//! # Agent Framework
//!
//! Core primitives for building actor-based agents.
//!
//! ## Components
//!
//! - [`Agent`] - Trait for defining agent behavior
//! - [`AgentHandle`] - Type-safe handle for communicating with agents
//! - [`Hive`] - Supervisor that manages agent lifecycles
//! - [`EventBus`] - Broadcast channel for system-wide events
//! - [`MemoryStore`] - Trait for agent memory backends

mod agent;
mod hive;
mod bus;
mod memory;

pub use agent::{Agent, AgentHandle};
pub use hive::Hive;
pub use bus::EventBus;
pub use memory::{MemoryStore, MemoryFragment};
