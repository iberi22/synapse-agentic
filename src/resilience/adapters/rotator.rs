//! Stochastic Rotator: Resilient provider wrapper with automatic failover.

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tracing::{info, warn, instrument};

use crate::decision::LLMProvider;
use crate::resilience::domain::{
    CooldownReason, FailoverError, FailoverStrategy, ProviderId,
};
use crate::resilience::ports::{ProviderRegistry, ResilientProvider};

/// Maximum number of failover attempts before giving up.
const MAX_FAILOVER_ATTEMPTS: usize = 5;

/// A resilient LLM provider that automatically fails over between multiple providers.
///
/// # Example
///
/// ```rust,no_run
/// use synapse_agentic::resilience::{StochasticRotator, InMemoryCooldownStore, ProviderId};
/// use std::sync::Arc;
///
/// async fn example() {
///     let store = Arc::new(InMemoryCooldownStore::new());
///     let mut rotator = StochasticRotator::new(store);
///
///     // Add providers...
///     // let response = rotator.generate_with_failover("Hello").await?;
/// }
/// ```
pub struct StochasticRotator<R: ProviderRegistry> {
    /// The registry tracking provider health
    registry: Arc<R>,
    /// Map of provider ID to actual provider implementation
    providers: RwLock<HashMap<String, Arc<dyn LLMProvider>>>,
    /// Current failover strategy
    strategy: RwLock<FailoverStrategy>,
    /// Last successfully used provider
    last_used: RwLock<Option<ProviderId>>,
}

impl<R: ProviderRegistry> StochasticRotator<R> {
    /// Creates a new stochastic rotator with the given registry.
    pub fn new(registry: Arc<R>) -> Self {
        Self {
            registry,
            providers: RwLock::new(HashMap::new()),
            strategy: RwLock::new(FailoverStrategy::Stochastic),
            last_used: RwLock::new(None),
        }
    }

    /// Attempts to generate using a specific provider.
    async fn try_provider(
        &self,
        id: &ProviderId,
        prompt: &str,
    ) -> Result<String, FailoverError> {
        let provider = {
            let providers = self.providers.read().map_err(|_| {
                FailoverError::ConfigError("lock poisoned".into())
            })?;

            providers.get(&id.key()).cloned().ok_or_else(|| {
                FailoverError::ProviderError {
                    provider: id.key(),
                    message: "provider not found in pool".into(),
                }
            })?
        };

        match provider.generate(prompt).await {
            Ok(response) => {
                self.registry.record_success(id).await?;

                // Update last used
                if let Ok(mut last) = self.last_used.write() {
                    *last = Some(id.clone());
                }

                info!(provider = %id.key(), "generation successful");
                Ok(response)
            }
            Err(e) => {
                let reason = Self::classify_error(&e);
                warn!(
                    provider = %id.key(),
                    error = %e,
                    reason = ?reason,
                    "provider failed, entering cooldown"
                );

                self.registry.record_failure(id, reason).await?;

                Err(FailoverError::ProviderError {
                    provider: id.key(),
                    message: e.to_string(),
                })
            }
        }
    }

    /// Classifies an error to determine the appropriate cooldown reason.
    fn classify_error(error: &anyhow::Error) -> CooldownReason {
        let msg = error.to_string().to_lowercase();

        if msg.contains("429") || msg.contains("rate limit") || msg.contains("too many") {
            CooldownReason::RateLimit
        } else if msg.contains("401") || msg.contains("403") || msg.contains("unauthorized") || msg.contains("forbidden") {
            CooldownReason::AuthFailure
        } else if msg.contains("402") || msg.contains("quota") || msg.contains("billing") {
            CooldownReason::QuotaExceeded
        } else if msg.contains("timeout") || msg.contains("timed out") {
            CooldownReason::Timeout
        } else if msg.contains("500") || msg.contains("502") || msg.contains("503") || msg.contains("504") {
            CooldownReason::ServerError
        } else {
            CooldownReason::Unknown(msg.chars().take(100).collect())
        }
    }
}

#[async_trait]
impl<R: ProviderRegistry + 'static> ResilientProvider for StochasticRotator<R> {
    #[instrument(skip(self, prompt), fields(strategy = ?self.strategy()))]
    async fn generate_with_failover(&self, prompt: &str) -> Result<String, FailoverError> {
        let strategy = self.strategy();
        let mut excluded: Vec<ProviderId> = Vec::new();
        let mut last_error: Option<FailoverError> = None;

        for attempt in 0..MAX_FAILOVER_ATTEMPTS {
            let selected = self.registry.select_next(strategy, &excluded).await?;

            info!(
                attempt = attempt + 1,
                provider = %selected.key(),
                "attempting generation"
            );

            match self.try_provider(&selected, prompt).await {
                Ok(response) => return Ok(response),
                Err(e) => {
                    excluded.push(selected);
                    last_error = Some(e);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| FailoverError::AllProvidersExhausted {
            message: format!("exhausted after {} attempts", MAX_FAILOVER_ATTEMPTS),
        }))
    }

    fn last_used_provider(&self) -> Option<ProviderId> {
        self.last_used.read().ok().and_then(|g| g.clone())
    }

    fn strategy(&self) -> FailoverStrategy {
        self.strategy.read().map(|s| *s).unwrap_or_default()
    }

    fn set_strategy(&mut self, strategy: FailoverStrategy) {
        if let Ok(mut s) = self.strategy.write() {
            *s = strategy;
        }
    }

    fn add_provider(&mut self, id: ProviderId, provider: Arc<dyn LLMProvider>) {
        // Register with the registry (fire and forget for sync method)
        let registry = Arc::clone(&self.registry);
        let id_clone = id.clone();
        tokio::spawn(async move {
            let _ = registry.register(id_clone).await;
        });

        // Add to local pool
        if let Ok(mut providers) = self.providers.write() {
            providers.insert(id.key(), provider);
        }
    }

    fn remove_provider(&mut self, id: &ProviderId) -> bool {
        if let Ok(mut providers) = self.providers.write() {
            providers.remove(&id.key()).is_some()
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resilience::InMemoryCooldownStore;
    use std::sync::atomic::{AtomicU32, Ordering};

    #[derive(Debug)]
    struct MockProvider {
        name: String,
        fail_count: AtomicU32,
        max_failures: u32,
    }

    impl MockProvider {
        fn new(name: &str, max_failures: u32) -> Self {
            Self {
                name: name.to_string(),
                fail_count: AtomicU32::new(0),
                max_failures,
            }
        }
    }

    #[async_trait]
    impl LLMProvider for MockProvider {
        fn name(&self) -> &str { &self.name }
        fn cost_per_1k_tokens(&self) -> f64 { 0.01 }

        async fn generate(&self, _prompt: &str) -> anyhow::Result<String> {
            let count = self.fail_count.fetch_add(1, Ordering::SeqCst);
            if count < self.max_failures {
                anyhow::bail!("429 rate limit exceeded")
            }
            Ok(format!("Response from {}", self.name))
        }
    }

    #[tokio::test]
    async fn test_failover_on_rate_limit() {
        let store = Arc::new(InMemoryCooldownStore::new());
        let mut rotator = StochasticRotator::new(store);

        // First provider fails twice, second succeeds
        let id1 = ProviderId::new("failing", "model");
        let id2 = ProviderId::new("working", "model");

        rotator.add_provider(id1, Arc::new(MockProvider::new("failing", 10)));
        rotator.add_provider(id2, Arc::new(MockProvider::new("working", 0)));

        // Give time for registration
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        let result = rotator.generate_with_failover("test").await;
        assert!(result.is_ok());
        assert!(result.unwrap().contains("working"));
    }

    #[tokio::test]
    async fn test_multi_provider_failover() {
        let store = Arc::new(InMemoryCooldownStore::new());
        let mut rotator = StochasticRotator::new(store);

        let id1 = ProviderId::new("fail1", "model");
        let id2 = ProviderId::new("fail2", "model");
        let id3 = ProviderId::new("work", "model");

        rotator.add_provider(id1, Arc::new(MockProvider::new("fail1", 5)));
        rotator.add_provider(id2, Arc::new(MockProvider::new("fail2", 2)));
        rotator.add_provider(id3, Arc::new(MockProvider::new("work", 0)));

        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        let result = rotator.generate_with_failover("test").await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Response from work");
        assert_eq!(rotator.last_used_provider().unwrap().name, "work");
    }

    #[tokio::test]
    async fn test_empty_provider_pool() {
        let store = Arc::new(InMemoryCooldownStore::new());
        let rotator = StochasticRotator::new(store);

        let result = rotator.generate_with_failover("test").await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, FailoverError::AllProvidersExhausted { .. }));
    }

    #[tokio::test]
    async fn test_all_providers_in_cooldown() {
        let store = Arc::new(InMemoryCooldownStore::new());
        let mut rotator = StochasticRotator::new(store.clone());

        let id1 = ProviderId::new("fail1", "model");
        let id2 = ProviderId::new("fail2", "model");

        rotator.add_provider(id1.clone(), Arc::new(MockProvider::new("fail1", 5)));
        rotator.add_provider(id2.clone(), Arc::new(MockProvider::new("fail2", 2)));

        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        // Force both into cooldown
        store.record_failure(&id1, CooldownReason::ServerError).await.unwrap();
        store.record_failure(&id2, CooldownReason::ServerError).await.unwrap();

        let result = rotator.generate_with_failover("test").await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, FailoverError::AllProvidersExhausted { .. }));
    }
}
