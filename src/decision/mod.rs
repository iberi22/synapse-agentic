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

mod context;
mod engine;
mod provider;
mod skill;

pub use context::DecisionContext;
pub use engine::{Decision, DecisionEngine, EngineMode};
pub use provider::{GeminiProvider, LLMProvider, MinimaxProvider, RawLLMOutput};
pub use skill::{Skill, SkillOutput};
