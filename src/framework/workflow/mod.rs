//! Hexagonal Architecture extensions for StateGraph (Workflow) execution.
//!
//! This module introduces LangGraph-style workflows via the `StateGraph` and `GraphNode`.

pub mod graph;
pub mod node;
pub mod reflection;
pub mod state;

pub use graph::{StateGraph, Edge};
pub use node::{GraphNode, ConditionFn};
pub use reflection::ReflectionNode;
pub use state::{ContextState, NodeResult};
