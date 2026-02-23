//! Application Layer: Ports (Traits) for resilience operations.
//!
//! Ports define the interfaces that infrastructure adapters must implement.

use async_trait::async_trait;
use std::sync::Arc;

use super::domain::{CooldownReason, FailoverError, FailoverStrategy, ProviderHealth, ProviderId};
use crate::decision::LLMProvider;

/// Port for managing provider health and cooldown states.
///
/// Implement this trait to provide different storage backends
/// (in-memory, SurrealDB, Redis, etc.).
#[async_trait]
pub trait ProviderRegistry: Send + Sync {
    /// Registers a new provider in the registry.
    async fn register(&self, id: ProviderId) -> Result<(), FailoverError>;

    /// Retrieves health information for a provider.
    async fn get_health(&self, id: &ProviderId) -> Result<ProviderHealth, FailoverError>;

    /// Records a successful call for a provider.
    async fn record_success(&self, id: &ProviderId) -> Result<(), FailoverError>;

    /// Records a failed call and enters cooldown.
    async fn record_failure(
        &self,
        id: &ProviderId,
        reason: CooldownReason,
    ) -> Result<(), FailoverError>;

    /// Returns all available (not in cooldown) providers.
    async fn list_available(&self) -> Result<Vec<ProviderHealth>, FailoverError>;

    /// Returns all registered providers.
    async fn list_all(&self) -> Result<Vec<ProviderHealth>, FailoverError>;

    /// Clears cooldown for a specific provider.
    async fn clear_cooldown(&self, id: &ProviderId) -> Result<(), FailoverError>;

    /// Selects the next provider using the given strategy.
    async fn select_next(
        &self,
        strategy: FailoverStrategy,
        exclude: &[ProviderId],
    ) -> Result<ProviderId, FailoverError>;
}

/// A provider wrapper that adds automatic failover capabilities.
///
/// Wraps multiple `LLMProvider` implementations and automatically
/// rotates between them on failure.
#[async_trait]
pub trait ResilientProvider: Send + Sync {
    /// Generates a response with automatic failover.
    ///
    /// If the primary provider fails, it will rotate through
    /// available providers until one succeeds or all are exhausted.
    async fn generate_with_failover(&self, prompt: &str) -> Result<String, FailoverError>;

    /// Returns the provider that was used for the last successful call.
    fn last_used_provider(&self) -> Option<ProviderId>;

    /// Returns the current failover strategy.
    fn strategy(&self) -> FailoverStrategy;

    /// Sets the failover strategy.
    fn set_strategy(&mut self, strategy: FailoverStrategy);

    /// Adds a provider to the pool.
    fn add_provider(&mut self, id: ProviderId, provider: Arc<dyn LLMProvider>);

    /// Removes a provider from the pool.
    fn remove_provider(&mut self, id: &ProviderId) -> bool;
}
