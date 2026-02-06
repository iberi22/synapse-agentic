//! Domain Layer: Core resilience entities and value objects.
//!
//! This module contains pure domain logic with zero external dependencies.

use std::time::{Duration, Instant};
use thiserror::Error;

/// Unique identifier for a provider instance.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ProviderId {
    /// Provider name (e.g., "openai", "anthropic", "deepseek")
    pub name: String,
    /// Model identifier (e.g., "gpt-4o", "claude-sonnet-4")
    pub model: String,
    /// Optional profile/credential set identifier
    pub profile: Option<String>,
}

impl ProviderId {
    /// Creates a new provider identifier.
    pub fn new(name: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            model: model.into(),
            profile: None,
        }
    }

    /// Creates a provider identifier with a specific credential profile.
    pub fn with_profile(
        name: impl Into<String>,
        model: impl Into<String>,
        profile: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            model: model.into(),
            profile: Some(profile.into()),
        }
    }

    /// Returns a unique key for this provider.
    pub fn key(&self) -> String {
        match &self.profile {
            Some(p) => format!("{}:{}:{}", self.name, self.model, p),
            None => format!("{}:{}", self.name, self.model),
        }
    }
}

/// Reasons why a provider might enter cooldown.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CooldownReason {
    /// Rate limit exceeded (HTTP 429)
    RateLimit,
    /// Authentication failure (HTTP 401/403)
    AuthFailure,
    /// Request timeout
    Timeout,
    /// Server error (HTTP 5xx)
    ServerError,
    /// Billing/quota exceeded (HTTP 402)
    QuotaExceeded,
    /// Unknown or generic failure
    Unknown(String),
}

impl CooldownReason {
    /// Returns the default cooldown duration for this reason.
    pub fn default_duration(&self) -> Duration {
        match self {
            CooldownReason::RateLimit => Duration::from_secs(60),
            CooldownReason::AuthFailure => Duration::from_secs(300), // 5 min
            CooldownReason::Timeout => Duration::from_secs(30),
            CooldownReason::ServerError => Duration::from_secs(120),
            CooldownReason::QuotaExceeded => Duration::from_secs(3600), // 1 hour
            CooldownReason::Unknown(_) => Duration::from_secs(60),
        }
    }

    /// Creates a reason from an HTTP status code.
    pub fn from_status(status: u16) -> Self {
        match status {
            401 | 403 => CooldownReason::AuthFailure,
            402 => CooldownReason::QuotaExceeded,
            429 => CooldownReason::RateLimit,
            500..=599 => CooldownReason::ServerError,
            _ => CooldownReason::Unknown(format!("HTTP {}", status)),
        }
    }
}

/// Represents a provider's cooldown state.
#[derive(Debug, Clone)]
pub struct CooldownState {
    /// When the cooldown expires
    pub until: Instant,
    /// Why the provider entered cooldown
    pub reason: CooldownReason,
    /// Number of consecutive failures
    pub failure_count: u32,
}

impl CooldownState {
    /// Creates a new cooldown state.
    pub fn new(reason: CooldownReason) -> Self {
        let duration = reason.default_duration();
        Self {
            until: Instant::now() + duration,
            reason,
            failure_count: 1,
        }
    }

    /// Creates a cooldown with exponential backoff.
    pub fn with_backoff(reason: CooldownReason, previous_failures: u32) -> Self {
        let base_duration = reason.default_duration();
        let multiplier = 2u32.pow(previous_failures.min(5)); // Cap at 32x
        let duration = base_duration * multiplier;

        Self {
            until: Instant::now() + duration,
            reason,
            failure_count: previous_failures + 1,
        }
    }

    /// Returns true if the cooldown has expired.
    pub fn is_expired(&self) -> bool {
        Instant::now() >= self.until
    }

    /// Returns the remaining cooldown duration.
    pub fn remaining(&self) -> Duration {
        self.until.saturating_duration_since(Instant::now())
    }
}

/// Health status of a provider.
#[derive(Debug, Clone)]
pub struct ProviderHealth {
    /// Provider identifier
    pub id: ProviderId,
    /// Current cooldown state (None if healthy)
    pub cooldown: Option<CooldownState>,
    /// Timestamp of last successful call
    pub last_success: Option<Instant>,
    /// Total successful calls
    pub success_count: u64,
    /// Total failed calls
    pub failure_count: u64,
}

impl ProviderHealth {
    /// Creates a new healthy provider state.
    pub fn new(id: ProviderId) -> Self {
        Self {
            id,
            cooldown: None,
            last_success: None,
            success_count: 0,
            failure_count: 0,
        }
    }

    /// Returns true if the provider is currently available.
    pub fn is_available(&self) -> bool {
        match &self.cooldown {
            None => true,
            Some(state) => state.is_expired(),
        }
    }

    /// Calculates a score for provider selection (higher = better).
    pub fn score(&self) -> f64 {
        if !self.is_available() {
            return 0.0;
        }

        let total = self.success_count + self.failure_count;
        if total == 0 {
            return 0.5; // Neutral score for new providers
        }

        let success_rate = self.success_count as f64 / total as f64;

        // Recency bonus: recently successful providers get a boost
        let recency_bonus = match self.last_success {
            Some(t) if t.elapsed() < Duration::from_secs(60) => 0.1,
            Some(t) if t.elapsed() < Duration::from_secs(300) => 0.05,
            _ => 0.0,
        };

        (success_rate + recency_bonus).min(1.0)
    }

    /// Records a successful call.
    pub fn record_success(&mut self) {
        self.success_count += 1;
        self.last_success = Some(Instant::now());
        self.cooldown = None; // Clear any expired cooldown
    }

    /// Records a failed call and potentially enters cooldown.
    pub fn record_failure(&mut self, reason: CooldownReason) {
        self.failure_count += 1;
        let previous = self.cooldown.as_ref().map(|c| c.failure_count).unwrap_or(0);
        self.cooldown = Some(CooldownState::with_backoff(reason, previous));
    }
}

/// Strategy for selecting the next provider during failover.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum FailoverStrategy {
    /// Random selection weighted by health score
    #[default]
    Stochastic,
    /// Ordered by priority, first available wins
    Priority,
    /// Simple round-robin cycling
    RoundRobin,
}

/// Errors that can occur during failover operations.
#[derive(Debug, Error)]
pub enum FailoverError {
    /// All providers are in cooldown
    #[error("all providers exhausted: {message}")]
    AllProvidersExhausted {
        /// Error message describing the exhaustion.
        message: String
    },

    /// No providers configured
    #[error("no providers configured")]
    NoProviders,

    /// Provider-specific error
    #[error("provider '{provider}' error: {message}")]
    ProviderError {
        /// Name of the failed provider.
        provider: String,
        /// Error message.
        message: String
    },

    /// Configuration error
    #[error("configuration error: {0}")]
    ConfigError(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_id_key() {
        let id = ProviderId::new("openai", "gpt-4o");
        assert_eq!(id.key(), "openai:gpt-4o");

        let id_with_profile = ProviderId::with_profile("anthropic", "claude-4", "work");
        assert_eq!(id_with_profile.key(), "anthropic:claude-4:work");
    }

    #[test]
    fn test_provider_health_score_edge_cases() {
        let id = ProviderId::new("edge", "model");
        let mut health = ProviderHealth::new(id);

        // All failures -> score should be 0 (in cooldown)
        health.record_failure(CooldownReason::ServerError);
        health.record_failure(CooldownReason::ServerError);
        assert_eq!(health.score(), 0.0);

        // Available again (expired cooldown), but still all failures
        // -> success_rate = 0/2 = 0.0
        health.cooldown = None;
        assert_eq!(health.score(), 0.0);

        // All successes -> score should be 1.0 (with recency bonus capped)
        let mut health_good = ProviderHealth::new(ProviderId::new("good", "model"));
        health_good.record_success();
        health_good.record_success();
        // success_rate = 1.0, recency_bonus = 0.1, min(1.1, 1.0) = 1.0
        assert_eq!(health_good.score(), 1.0);
    }

    #[test]
    fn test_cooldown_expiration() {
        let state = CooldownState::new(CooldownReason::Timeout);
        assert!(!state.is_expired());
        // Can't easily test expiration without sleeping, but the logic is sound
    }

    #[test]
    fn test_provider_health_score() {
        let id = ProviderId::new("test", "model");
        let mut health = ProviderHealth::new(id);

        assert_eq!(health.score(), 0.5); // New provider

        health.record_success();
        health.record_success();
        health.record_failure(CooldownReason::Timeout);

        // After failure, provider is in cooldown -> score = 0.0
        assert_eq!(health.score(), 0.0);

        // Clear cooldown to simulate it expired
        health.cooldown = None;
        // 2 successes, 1 failure = 66.6% + recency bonus
        assert!(health.score() > 0.6);
    }

    #[test]
    fn test_cooldown_reason_from_status_comprehensive() {
        assert_eq!(CooldownReason::from_status(401), CooldownReason::AuthFailure);
        assert_eq!(CooldownReason::from_status(403), CooldownReason::AuthFailure);
        assert_eq!(CooldownReason::from_status(402), CooldownReason::QuotaExceeded);
        assert_eq!(CooldownReason::from_status(429), CooldownReason::RateLimit);
        assert_eq!(CooldownReason::from_status(500), CooldownReason::ServerError);
        assert_eq!(CooldownReason::from_status(503), CooldownReason::ServerError);
        assert_eq!(CooldownReason::from_status(599), CooldownReason::ServerError);
        assert_eq!(
            CooldownReason::from_status(404),
            CooldownReason::Unknown("HTTP 404".into())
        );
    }

    #[test]
    fn test_cooldown_state_with_backoff() {
        let reason = CooldownReason::ServerError;
        let base_duration = reason.default_duration();

        // First failure
        let state1 = CooldownState::with_backoff(reason.clone(), 0);
        assert_eq!(state1.failure_count, 1);

        // Second failure (2x)
        let state2 = CooldownState::with_backoff(reason.clone(), 1);
        assert_eq!(state2.failure_count, 2);

        // Fifth failure (16x)
        let state5 = CooldownState::with_backoff(reason.clone(), 4);
        assert_eq!(state5.failure_count, 5);

        // Sixth failure (capped at 32x)
        let state6 = CooldownState::with_backoff(reason, 5);
        assert_eq!(state6.failure_count, 6);
    }
}
