//! # Decision Engine
//!
//! Multi-LLM decision making with consensus and skills.
//!
//! ## Components
//!
//! - [`LLMProvider`] - Trait for LLM API integrations
//! - [`DecisionEngine`] - Orchestrates multi-LLM decisions
//! - [`DecisionContext`] - Context for decision requests
//! - [`Skill`] - Reusable AI capabilities

mod provider;
mod engine;
mod context;
mod skill;

pub use provider::{LLMProvider, RawLLMOutput};
pub use engine::{DecisionEngine, EngineMode, Decision};
pub use context::DecisionContext;
pub use skill::{Skill, SkillOutput};
