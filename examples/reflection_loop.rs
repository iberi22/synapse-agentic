use synapse_agentic::framework::workflow::*;
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;

/// A simple Agent node that tries to perform a "flawed" calculation.
/// If it detects it has been criticized, it fixes its behavior.
#[derive(Debug)]
struct MathAgentNode {
    id: String,
}

#[async_trait]
impl GraphNode for MathAgentNode {
    fn id(&self) -> &str {
        &self.id
    }

    async fn execute(&mut self, state: &mut ContextState) -> Result<NodeResult> {
        println!("[{}] Executing...", self.id);

        let iter_count = state.data.get("calc_attempts").and_then(|v| v.as_u64()).unwrap_or(0);
        state.set_value("calc_attempts", json!(iter_count + 1));

        // Let's pretend the agent needs 3 iterations to "realize" it's doing math wrong
        // through the critic feedback
        if let Some(feedback) = state.get_string("critic_feedback") {
            println!("[{}] Received feedback from critic:\n >> {}", self.id, feedback);
            if iter_count >= 2 {
                println!("[{}] Ah! I will fix the math now. 2 + 2 = 4.", self.id);
                state.set_value("result", json!(4));
                return Ok(NodeResult::Continue(None)); // This ends the graph cleanly
            }
        }

        // Simulate agent making a mistake
        println!("[{}] Calculating 2 + 2 = 5...", self.id);

        // Return an error which will be caught by the framework/graph
        // and routed to Reflection if configured.
        Ok(NodeResult::Error("Math logic failure: 2 + 2 is not 5. Are you crazy?".to_string()))
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("--- Synapse-Agentic: Reflection/Self-Correction Example ---");

    let agent = MathAgentNode { id: "math_agent".to_string() };

    // The critic will intercept errors from `math_agent` and route back to it up to 3 times
    let critic = ReflectionNode::new("critic_node", "math_agent", 3);

    let mut graph = StateGraph::new();

    // Add nodes
    graph.add_node(Box::new(agent));
    graph.add_node(Box::new(critic));

    // Define flow
    graph.set_entry_point("math_agent");
    graph.set_error_handler("critic_node");

    // Initialize state
    let initial_state = ContextState::new(json!({}));

    // Execute!
    println!("Starting graph execution...");
    let final_state = graph.execute(initial_state).await?;

    println!("\nExecution Finished!");
    println!("Path taken: {:?}", final_state.history);
    println!("Final Result: {:?}", final_state.data.get("result"));

    Ok(())
}
