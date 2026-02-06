//! # Resilience Module
//!
//! Provides fault-tolerant LLM provider management with automatic failover,
//! cooldown tracking, and stochastic rotation.
//!
//! ## Architecture (Hexagonal)
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    APPLICATION LAYER                        │
//! │  ┌─────────────────┐  ┌─────────────────────────────────┐  │
//! │  │ ResilientProvider│  │ ProviderRegistry (Port/Trait)  │  │
//! │  └─────────────────┘  └─────────────────────────────────┘  │
//! └─────────────────────────────────────────────────────────────┘
//!                              │
//! ┌─────────────────────────────────────────────────────────────┐
//! │                      DOMAIN LAYER                           │
//! │  ┌─────────────┐ ┌─────────────┐ ┌───────────────────────┐  │
//! │  │ ProviderId  │ │CooldownState│ │  FailoverStrategy     │  │
//! │  └─────────────┘ └─────────────┘ └───────────────────────┘  │
//! └─────────────────────────────────────────────────────────────┘
//!                              │
//! ┌─────────────────────────────────────────────────────────────┐
//! │                  INFRASTRUCTURE LAYER                       │
//! │  ┌─────────────────────┐  ┌───────────────────────────────┐ │
//! │  │ StochasticRotator   │  │ InMemoryCooldownStore         │ │
//! │  └─────────────────────┘  └───────────────────────────────┘ │
//! └─────────────────────────────────────────────────────────────┘
//! ```

// Domain: Pure business logic, no external dependencies
pub mod domain;

// Application: Ports (traits) and use cases
pub mod ports;

// Infrastructure: Concrete implementations
pub mod adapters;

// Re-exports for convenience
pub use domain::{
    ProviderId, ProviderHealth, CooldownState, CooldownReason,
    FailoverStrategy, FailoverError,
};
pub use ports::{ProviderRegistry, ResilientProvider};
pub use adapters::{StochasticRotator, InMemoryCooldownStore};
