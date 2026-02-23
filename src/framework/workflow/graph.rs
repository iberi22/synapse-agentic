use anyhow::{anyhow, Result};
use std::collections::HashMap;

use super::node::GraphNode;
use super::state::{ContextState, NodeResult};

/// Represents a directed edge between nodes.
#[derive(Debug, Clone)]
pub struct Edge {
    from: String,
    to: String,
}

/// The core Hexagonal StateGraph.
///
/// It orchestrates multiple `GraphNode` actors or adapters by passing
/// the `ContextState` through them based on explicit or conditional edges.
pub struct StateGraph {
    nodes: HashMap<String, Box<dyn GraphNode>>,
    edges: Vec<Edge>,
    entry_point: Option<String>,
    error_handler: Option<String>,
}

impl StateGraph {
    /// Creates a new empty StateGraph.
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            edges: Vec::new(),
            entry_point: None,
            error_handler: None,
        }
    }

    /// Adds a node to the graph.
    pub fn add_node(&mut self, node: Box<dyn GraphNode>) {
        self.nodes.insert(node.id().to_string(), node);
    }

    /// Explicit edge between an origin and destination node.
    pub fn add_edge(&mut self, from: &str, to: &str) {
        self.edges.push(Edge {
            from: from.to_string(),
            to: to.to_string(),
        });
    }

    /// Defines the starting node for graph execution.
    pub fn set_entry_point(&mut self, node_id: &str) {
        self.entry_point = Some(node_id.to_string());
    }

    /// Sets a node to handle `NodeResult::Error` outcomes globally.
    pub fn set_error_handler(&mut self, node_id: &str) {
        self.error_handler = Some(node_id.to_string());
    }

    /// Executes the state graph asynchronously.
    pub async fn execute(&mut self, mut state: ContextState) -> Result<ContextState> {
        let mut current_node_id = self.entry_point.clone()
            .ok_or_else(|| anyhow!("Entry point not defined for StateGraph"))?;

        loop {
            // Record step
            state.record_visit(&current_node_id);

            // Fetch node
            let mut node = self.nodes.remove(&current_node_id)
                .ok_or_else(|| anyhow!("Node not found: {}", current_node_id))?;

            // Execute node
            let result = node.execute(&mut state).await;

            // Re-insert node
            self.nodes.insert(current_node_id.clone(), node);

            // Process result
            match result? {
                NodeResult::Halt => {
                    break;
                }
                NodeResult::Error(err_msg) => {
                    state.last_error = Some(err_msg.clone());
                    if let Some(handler) = &self.error_handler {
                        // Route to the error handler (Reflection/Critic)
                        current_node_id = handler.clone();
                    } else {
                        return Err(anyhow!("Graph execution halted by node error: {} -> {}", current_node_id, err_msg));
                    }
                }
                NodeResult::Continue(Some(next_id)) => {
                    current_node_id = next_id;
                }
                NodeResult::Continue(None) => {
                    // Find default edge
                    let next_edge = self.edges.iter().find(|e| e.from == current_node_id);
                    if let Some(edge) = next_edge {
                        current_node_id = edge.to.clone();
                    } else {
                        // Dead end, finish graph naturally
                        break;
                    }
                }
            }
        }

        Ok(state)
    }
}
