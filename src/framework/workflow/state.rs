use serde_json::Value;

/// Represents the shared context that flows through the StateGraph.
///
/// In a hexagonal architecture, this is the core domain entity that
/// nodes read from and write to.
#[derive(Debug, Clone)]
pub struct ContextState {
    /// Arbitrary state data (can be mapped to specific structs by nodes).
    pub data: Value,

    /// Optional error captured during the flow, useful for Reflection loops.
    pub last_error: Option<String>,

    /// Trace of visited node IDs
    pub history: Vec<String>,
}

impl ContextState {
    /// Creates a new ContextState with initial data.
    pub fn new(initial_data: Value) -> Self {
        Self {
            data: initial_data,
            last_error: None,
            history: Vec::new(),
        }
    }

    /// Appends a node to the history trail.
    pub fn record_visit(&mut self, node_id: &str) {
        self.history.push(node_id.to_string());
    }

    /// Helper to get a string value from the state data.
    pub fn get_string(&self, key: &str) -> Option<String> {
        self.data.get(key).and_then(|v| v.as_str()).map(|s| s.to_string())
    }

    /// Helper to set a value in the state data.
    pub fn set_value(&mut self, key: &str, value: Value) {
        if let Some(obj) = self.data.as_object_mut() {
            obj.insert(key.to_string(), value);
        }
    }
}

/// Represents the possible outcomes of a Node execution.
#[derive(Debug, Clone)]
pub enum NodeResult {
    /// The node completed successfully, optionally returning the ID of the next node to branch to.
    /// If None, the default edge defined in the graph will be used.
    Continue(Option<String>),

    /// The node failed. The framework should route to the Error handler or Reflection node.
    Error(String),

    /// The node indicates the entire graph execution should halt successfully.
    Halt,
}
