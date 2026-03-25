//! Agent trait and handle definitions.

use anyhow::Result;
use async_trait::async_trait;
use std::fmt::Debug;
use std::sync::Arc;
use tokio::sync::mpsc;

/// The `Agent` trait defines the lifecycle and behavior of an active component.
///
/// Each agent runs in its own task and processes messages of type `Input`.
///
/// # Example
///
/// ```rust
/// use synapse_agentic::prelude::*;
///
/// #[derive(Debug)]
/// enum CounterMsg {
///     Increment,
///     Decrement,
///     Get(tokio::sync::oneshot::Sender<i32>),
/// }
///
/// struct CounterAgent {
///     count: i32,
/// }
///
/// #[async_trait]
/// impl Agent for CounterAgent {
///     type Input = CounterMsg;
///
///     fn name(&self) -> &str { "Counter" }
///
///     async fn handle(&mut self, msg: Self::Input) -> Result<()> {
///         match msg {
///             CounterMsg::Increment => self.count += 1,
///             CounterMsg::Decrement => self.count -= 1,
///             CounterMsg::Get(reply) => { let _ = reply.send(self.count); }
///         }
///         Ok(())
///     }
/// }
/// ```
#[async_trait]
pub trait Agent: Send + Sync + 'static {
    /// The type of message this agent handles.
    type Input: Debug + Send;

    /// Called before the agent starts processing messages.
    ///
    /// Use this for initialization that requires async operations.
    async fn init(&mut self) -> Result<()> {
        Ok(())
    }

    /// Handles a single message.
    ///
    /// This is called for each message received by the agent.
    async fn handle(&mut self, message: Self::Input) -> Result<()>;

    /// Called when the agent is shutting down.
    ///
    /// Use this for cleanup operations.
    async fn shutdown(&mut self) -> Result<()> {
        Ok(())
    }

    /// Serializes the current internal state of the agent.
    ///
    /// Useful for persisting agent state across restarts (e.g. into memory or DB).
    /// Returns `None` if the agent is stateless or does not support snapshotting.
    fn snapshot(&self) -> Option<serde_json::Value> {
        None
    }

    /// Restores the internal state of the agent from a snapshot.
    ///
    /// This is called during initialization if a previous state snapshot was provided.
    fn restore(&mut self, _state: &serde_json::Value) -> Result<()> {
        Ok(())
    }

    /// Returns the name of the agent (for logging/debugging).
    fn name(&self) -> &str;
}

/// A generic handle to communicate with an agent.
///
/// This encapsulates the `mpsc::Sender` and provides a clean API for sending messages.
/// Handles are cheap to clone and can be shared across tasks.
///
/// # Example
///
/// ```rust,no_run
/// use synapse_agentic::prelude::*;
///
/// async fn example(handle: AgentHandle<String>) {
///     handle.send("Hello".to_string()).await.unwrap();
/// }
/// ```
pub struct AgentHandle<T> {
    tx: mpsc::Sender<T>,
    name: Arc<String>,
}

impl<T> Clone for AgentHandle<T> {
    fn clone(&self) -> Self {
        Self {
            tx: self.tx.clone(),
            name: self.name.clone(),
        }
    }
}

impl<T> AgentHandle<T>
where
    T: Debug + Send + 'static,
{
    /// Creates a new agent handle.
    pub fn new(tx: mpsc::Sender<T>, name: String) -> Self {
        Self {
            tx,
            name: Arc::new(name),
        }
    }

    /// Sends a message to the agent.
    ///
    /// Returns an error if the agent has been shut down.
    pub async fn send(&self, message: T) -> Result<()> {
        self.tx
            .send(message)
            .await
            .map_err(|_| anyhow::anyhow!("Failed to send message to agent: {}", self.name))
    }

    /// Attempts to send a message without waiting.
    ///
    /// Returns an error if the channel is full or closed.
    pub fn try_send(&self, message: T) -> Result<()> {
        self.tx
            .try_send(message)
            .map_err(|e| anyhow::anyhow!("Failed to send message to agent {}: {}", self.name, e))
    }

    /// Returns the name of the agent this handle points to.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns true if the agent is still alive.
    pub fn is_alive(&self) -> bool {
        !self.tx.is_closed()
    }
}

impl<T: Debug + Send + 'static> Debug for AgentHandle<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AgentHandle")
            .field("name", &self.name)
            .field("alive", &self.is_alive())
            .finish()
    }
}
