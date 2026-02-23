//! In-memory cooldown store implementation.

use async_trait::async_trait;
use rand::seq::IndexedRandom;
use rand::Rng;
use std::collections::HashMap;
use std::sync::RwLock;

use crate::resilience::domain::{
    CooldownReason, FailoverError, FailoverStrategy, ProviderHealth, ProviderId,
};
use crate::resilience::ports::ProviderRegistry;

/// In-memory implementation of `ProviderRegistry`.
///
/// Suitable for single-instance deployments. For distributed systems,
/// consider implementing a SurrealDB or Redis-backed registry.
pub struct InMemoryCooldownStore {
    providers: RwLock<HashMap<String, ProviderHealth>>,
    /// Index for round-robin selection
    round_robin_index: RwLock<usize>,
}

impl InMemoryCooldownStore {
    /// Creates a new in-memory cooldown store.
    pub fn new() -> Self {
        Self {
            providers: RwLock::new(HashMap::new()),
            round_robin_index: RwLock::new(0),
        }
    }

    fn get_key(id: &ProviderId) -> String {
        id.key()
    }
}

impl Default for InMemoryCooldownStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ProviderRegistry for InMemoryCooldownStore {
    async fn register(&self, id: ProviderId) -> Result<(), FailoverError> {
        let key = Self::get_key(&id);
        let mut providers = self
            .providers
            .write()
            .map_err(|_| FailoverError::ConfigError("lock poisoned".into()))?;

        providers
            .entry(key)
            .or_insert_with(|| ProviderHealth::new(id));
        Ok(())
    }

    async fn get_health(&self, id: &ProviderId) -> Result<ProviderHealth, FailoverError> {
        let key = Self::get_key(id);
        let providers = self
            .providers
            .read()
            .map_err(|_| FailoverError::ConfigError("lock poisoned".into()))?;

        providers
            .get(&key)
            .cloned()
            .ok_or_else(|| FailoverError::ProviderError {
                provider: id.key(),
                message: "not registered".into(),
            })
    }

    async fn record_success(&self, id: &ProviderId) -> Result<(), FailoverError> {
        let key = Self::get_key(id);
        let mut providers = self
            .providers
            .write()
            .map_err(|_| FailoverError::ConfigError("lock poisoned".into()))?;

        if let Some(health) = providers.get_mut(&key) {
            health.record_success();
        }
        Ok(())
    }

    async fn record_failure(
        &self,
        id: &ProviderId,
        reason: CooldownReason,
    ) -> Result<(), FailoverError> {
        let key = Self::get_key(id);
        let mut providers = self
            .providers
            .write()
            .map_err(|_| FailoverError::ConfigError("lock poisoned".into()))?;

        if let Some(health) = providers.get_mut(&key) {
            health.record_failure(reason);
        }
        Ok(())
    }

    async fn list_available(&self) -> Result<Vec<ProviderHealth>, FailoverError> {
        let providers = self
            .providers
            .read()
            .map_err(|_| FailoverError::ConfigError("lock poisoned".into()))?;

        Ok(providers
            .values()
            .filter(|h| h.is_available())
            .cloned()
            .collect())
    }

    async fn list_all(&self) -> Result<Vec<ProviderHealth>, FailoverError> {
        let providers = self
            .providers
            .read()
            .map_err(|_| FailoverError::ConfigError("lock poisoned".into()))?;

        Ok(providers.values().cloned().collect())
    }

    async fn clear_cooldown(&self, id: &ProviderId) -> Result<(), FailoverError> {
        let key = Self::get_key(id);
        let mut providers = self
            .providers
            .write()
            .map_err(|_| FailoverError::ConfigError("lock poisoned".into()))?;

        if let Some(health) = providers.get_mut(&key) {
            health.cooldown = None;
        }
        Ok(())
    }

    async fn select_next(
        &self,
        strategy: FailoverStrategy,
        exclude: &[ProviderId],
    ) -> Result<ProviderId, FailoverError> {
        let providers = self
            .providers
            .read()
            .map_err(|_| FailoverError::ConfigError("lock poisoned".into()))?;

        let exclude_keys: Vec<String> = exclude.iter().map(Self::get_key).collect();

        let available: Vec<&ProviderHealth> = providers
            .values()
            .filter(|h| h.is_available() && !exclude_keys.contains(&h.id.key()))
            .collect();

        if available.is_empty() {
            return Err(FailoverError::AllProvidersExhausted {
                message: format!(
                    "{} providers registered, all in cooldown or excluded",
                    providers.len()
                ),
            });
        }

        let selected = match strategy {
            FailoverStrategy::Stochastic => self.select_stochastic(&available)?,
            FailoverStrategy::Priority => {
                // Priority: select highest score
                available
                    .iter()
                    .max_by(|a, b| a.score().partial_cmp(&b.score()).unwrap())
                    .map(|h| h.id.clone())
                    .ok_or_else(|| FailoverError::NoProviders)?
            }
            FailoverStrategy::RoundRobin => self.select_round_robin(&available)?,
        };

        Ok(selected)
    }
}

impl InMemoryCooldownStore {
    /// Stochastic selection weighted by health score.
    fn select_stochastic(
        &self,
        available: &[&ProviderHealth],
    ) -> Result<ProviderId, FailoverError> {
        let total_score: f64 = available.iter().map(|h| h.score()).sum();

        if total_score <= 0.0 {
            // Fallback to random if all scores are 0
            return available
                .choose(&mut rand::rng())
                .map(|h| h.id.clone())
                .ok_or(FailoverError::NoProviders);
        }

        let mut rng = rand::rng();
        let threshold = rng.random::<f64>() * total_score;
        let mut cumulative = 0.0;

        for health in available {
            cumulative += health.score();
            if cumulative >= threshold {
                return Ok(health.id.clone());
            }
        }

        // Fallback to last
        available
            .last()
            .map(|h| h.id.clone())
            .ok_or(FailoverError::NoProviders)
    }

    /// Round-robin selection.
    fn select_round_robin(
        &self,
        available: &[&ProviderHealth],
    ) -> Result<ProviderId, FailoverError> {
        let mut index = self
            .round_robin_index
            .write()
            .map_err(|_| FailoverError::ConfigError("lock poisoned".into()))?;

        let selected_index = *index % available.len();
        *index = (*index + 1) % available.len();

        available
            .get(selected_index)
            .map(|h| h.id.clone())
            .ok_or(FailoverError::NoProviders)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_register_and_get_health() {
        let store = InMemoryCooldownStore::new();
        let id = ProviderId::new("openai", "gpt-4o");

        store.register(id.clone()).await.unwrap();
        let health = store.get_health(&id).await.unwrap();

        assert!(health.is_available());
        assert_eq!(health.success_count, 0);
    }

    #[tokio::test]
    async fn test_record_failure_enters_cooldown() {
        let store = InMemoryCooldownStore::new();
        let id = ProviderId::new("anthropic", "claude-4");

        store.register(id.clone()).await.unwrap();
        store
            .record_failure(&id, CooldownReason::RateLimit)
            .await
            .unwrap();

        let health = store.get_health(&id).await.unwrap();
        assert!(!health.is_available());
    }

    #[tokio::test]
    async fn test_select_excludes_providers() {
        let store = InMemoryCooldownStore::new();
        let id1 = ProviderId::new("openai", "gpt-4o");
        let id2 = ProviderId::new("anthropic", "claude-4");

        store.register(id1.clone()).await.unwrap();
        store.register(id2.clone()).await.unwrap();

        let selected = store
            .select_next(FailoverStrategy::Priority, &[id1.clone()])
            .await
            .unwrap();

        assert_eq!(selected.key(), id2.key());
    }

    #[tokio::test]
    async fn test_concurrent_access() {
        let store = Arc::new(InMemoryCooldownStore::new());
        let id1 = ProviderId::new("openai", "gpt-4o");
        let id2 = ProviderId::new("anthropic", "claude-4");

        store.register(id1.clone()).await.unwrap();
        store.register(id2.clone()).await.unwrap();

        let mut handles = vec![];

        for _ in 0..100 {
            let store = Arc::clone(&store);
            let id1_clone = id1.clone();
            handles.push(tokio::spawn(async move {
                store.record_success(&id1_clone).await.unwrap();
            }));
        }

        for _ in 0..50 {
            let store = Arc::clone(&store);
            let id2_clone = id2.clone();
            handles.push(tokio::spawn(async move {
                store
                    .record_failure(&id2_clone, CooldownReason::Timeout)
                    .await
                    .unwrap();
            }));
        }

        for handle in handles {
            handle.await.unwrap();
        }

        let health1 = store.get_health(&id1).await.unwrap();
        assert_eq!(health1.success_count, 100);

        let health2 = store.get_health(&id2).await.unwrap();
        assert_eq!(health2.failure_count, 50);
        assert!(!health2.is_available());
    }

    #[tokio::test]
    async fn test_round_robin_strategy() {
        let store = InMemoryCooldownStore::new();
        let id1 = ProviderId::new("openai", "gpt-4o");
        let id2 = ProviderId::new("anthropic", "claude-4");
        let id3 = ProviderId::new("deepseek", "coder");

        store.register(id1.clone()).await.unwrap();
        store.register(id2.clone()).await.unwrap();
        store.register(id3.clone()).await.unwrap();

        let s1 = store
            .select_next(FailoverStrategy::RoundRobin, &[])
            .await
            .unwrap();
        let s2 = store
            .select_next(FailoverStrategy::RoundRobin, &[])
            .await
            .unwrap();
        let s3 = store
            .select_next(FailoverStrategy::RoundRobin, &[])
            .await
            .unwrap();
        let s4 = store
            .select_next(FailoverStrategy::RoundRobin, &[])
            .await
            .unwrap();

        assert_ne!(s1.key(), s2.key());
        assert_ne!(s2.key(), s3.key());
        assert_eq!(s4.key(), s1.key());
    }
}
