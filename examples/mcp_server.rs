use synapse_agentic::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    let _registry = McpRegistry::new();
    // In a real example we would register tools and run the server
    println!("MCP Server example placeholder");
    Ok(())
}
