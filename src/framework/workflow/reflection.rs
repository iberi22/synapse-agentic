use anyhow::Result;
use async_trait::async_trait;

use super::node::GraphNode;
use super::state::{ContextState, NodeResult};

/// The ReflectionNode (Critic) evaluates the state after an execution step.
///
/// If it detects an error (e.g., from a tool failure or compilation error), it
/// redirects the flow back to the Agent for self-correction.
#[derive(Debug)]
pub struct ReflectionNode {
    id: String,
    max_iterations: usize,
    current_iterations: usize,
    target_node_id: String, // Node to route back to for correction
}

impl ReflectionNode {
    /// Creates a new ReflectionNode (Critic).
    ///
    /// * `id`: Unique ID for this node.
    /// * `target_node_id`: The ID of the node that should receive the feedback for correction.
    /// * `max_iterations`: Maximum number of retries before returning a hard error.
    pub fn new(id: &str, target_node_id: &str, max_iterations: usize) -> Self {
        Self {
            id: id.to_string(),
            max_iterations,
            current_iterations: 0,
            target_node_id: target_node_id.to_string(),
        }
    }
}

#[async_trait]
impl GraphNode for ReflectionNode {
    fn id(&self) -> &str {
        &self.id
    }

    async fn execute(&mut self, state: &mut ContextState) -> Result<NodeResult> {
        // If there is an error in the state, we reflect on it
        if let Some(error) = &state.last_error {
            self.current_iterations += 1;

            if self.current_iterations > self.max_iterations {
                return Ok(NodeResult::Error(format!(
                    "Reflection failed: Max iterations ({}) reached. Last error: {}",
                    self.max_iterations, error
                )));
            }

            // We prepare the context for the agent so it knows it failed
            let reflection_prompt = format!(
                "CRITIC FEEDBACK: Your previous action failed with error:\n{}\n\nPlease correct your approach and try again. Iteration {}/{}",
                error, self.current_iterations, self.max_iterations
            );

            // Append reflection feedback to the state
            state.set_value("critic_feedback", serde_json::Value::String(reflection_prompt));

            // Clear the actual error so the system doesn't trap again immediately
            state.last_error = None;

            // Route back to the agent for fixing
            return Ok(NodeResult::Continue(Some(self.target_node_id.clone())));
        }

        // If there's no error, we consider the execution successful and move on.
        self.current_iterations = 0; // Reset for future passes
        Ok(NodeResult::Continue(None))
    }
}
