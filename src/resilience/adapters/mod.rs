//! Infrastructure Layer: Concrete implementations of resilience ports.
//!
//! Contains adapters for in-memory storage and stochastic rotation.

mod cooldown_store;
mod rotator;

pub use cooldown_store::InMemoryCooldownStore;
pub use rotator::StochasticRotator;
