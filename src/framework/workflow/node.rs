use super::state::{ContextState, NodeResult};
use anyhow::Result;
use async_trait::async_trait;
use std::fmt::Debug;

/// Defines a node that can be executed within a StateGraph.
///
/// This is a Port in the Hexagonal Architecture. Agents can be wrapped
/// in Adapters that implement this trait to participate in workflow graphs.
#[async_trait]
pub trait GraphNode: Send + Sync + Debug + 'static {
    /// Returns the unique identifier of this node.
    fn id(&self) -> &str;

    /// Executes the node's logic using the provided state.
    ///
    /// The state can be mutated. Returns a `NodeResult` dictating flow.
    async fn execute(&mut self, state: &mut ContextState) -> Result<NodeResult>;
}

/// A Condition is an edge function that determines the next node based on state.
pub type ConditionFn = Box<dyn Fn(&ContextState) -> Option<String> + Send + Sync + 'static>;
