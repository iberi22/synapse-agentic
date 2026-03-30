use std::time::Duration;
use synapse_agentic::prelude::*;

#[derive(Debug)]
enum Ping {
    Ping,
}

struct PingAgent;

#[async_trait]
impl Agent for PingAgent {
    type Input = Ping;

    fn name(&self) -> &str {
        "Ping"
    }

    async fn handle(&mut self, _msg: Self::Input) -> Result<()> {
        println!("Pong!");
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let mut hive = Hive::new();
    let agent = PingAgent;
    let _addr = hive.spawn(agent);

    println!("Agent spawned!");
    tokio::time::sleep(Duration::from_millis(100)).await;

    Ok(())
}
