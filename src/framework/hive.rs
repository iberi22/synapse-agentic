//! Hive - Agent supervisor and lifecycle manager.

use tokio::task::JoinSet;
use tokio::sync::mpsc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use anyhow::Result;
use tracing::{info, error};

use super::agent::{Agent, AgentHandle};

/// Simple cancellation token for graceful shutdown.
#[derive(Clone)]
pub struct CancellationToken {
    cancelled: Arc<AtomicBool>,
}

impl CancellationToken {
    fn new() -> Self {
        Self {
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }

    fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }
}

/// The `Hive` manages the lifecycle of a collection of Agents.
///
/// It provides supervision (monitoring for panics/errors) and graceful shutdown capabilities.
/// Uses `tokio::task::JoinSet` to collect all tasks and a `CancellationToken` to signal shutdown.
///
/// # Example
///
/// ```rust,no_run
/// use synapse_agentic::prelude::*;
///
/// #[derive(Debug)]
/// struct Ping;
///
/// struct PingAgent;
///
/// #[async_trait]
/// impl Agent for PingAgent {
///     type Input = Ping;
///     fn name(&self) -> &str { "Ping" }
///     async fn handle(&mut self, _: Ping) -> Result<()> { Ok(()) }
/// }
///
/// #[tokio::main]
/// async fn main() {
///     let mut hive = Hive::new();
///     let handle = hive.spawn(PingAgent);
///
///     handle.send(Ping).await.unwrap();
///
///     hive.shutdown().await;
/// }
/// ```
pub struct Hive {
    tasks: JoinSet<Result<()>>,
    cancel_token: CancellationToken,
}

impl Default for Hive {
    fn default() -> Self {
        Self::new()
    }
}

impl Hive {
    /// Creates a new empty Hive.
    pub fn new() -> Self {
        Self {
            tasks: JoinSet::new(),
            cancel_token: CancellationToken::new(),
        }
    }

    /// Spawns an agent into the Hive.
    ///
    /// Returns a typed `AgentHandle` for communicating with the agent.
    /// The agent will run until:
    /// - The Hive is shut down
    /// - All handles to the agent are dropped
    /// - The agent returns an error from `handle()`
    pub fn spawn<A>(&mut self, agent: A) -> AgentHandle<A::Input>
    where
        A: Agent + Send + Sync + 'static,
        A::Input: Send + Sync + 'static + std::fmt::Debug,
    {
        self.spawn_with_capacity(agent, 100)
    }

    /// Spawns an agent with a custom channel capacity.
    pub fn spawn_with_capacity<A>(&mut self, mut agent: A, capacity: usize) -> AgentHandle<A::Input>
    where
        A: Agent + Send + Sync + 'static,
        A::Input: Send + Sync + 'static + std::fmt::Debug,
    {
        let (tx, mut rx) = mpsc::channel(capacity);
        let name = agent.name().to_string();
        let name_clone = name.clone();
        let token = self.cancel_token.clone();

        self.tasks.spawn(async move {
            info!(agent = %name_clone, "Starting agent");

            if let Err(e) = agent.init().await {
                error!(agent = %name_clone, error = %e, "Agent init failed");
                return Err(e);
            }

            loop {
                // Check cancellation
                if token.is_cancelled() {
                    info!(agent = %name_clone, "Agent stopping (hive shutdown)");
                    break;
                }

                tokio::select! {
                    biased;
                    msg = rx.recv() => {
                        match msg {
                            Some(input) => {
                                if let Err(e) = agent.handle(input).await {
                                    error!(agent = %name_clone, error = %e, "Agent handle error");
                                    // Continue on errors by default
                                }
                            }
                            None => {
                                info!(agent = %name_clone, "Agent stopping (channel closed)");
                                break;
                            }
                        }
                    }
                    // Small sleep to allow cancellation check
                    _ = tokio::time::sleep(tokio::time::Duration::from_millis(10)) => {}
                }
            }

            if let Err(e) = agent.shutdown().await {
                error!(agent = %name_clone, error = %e, "Agent shutdown error");
            }

            info!(agent = %name_clone, "Agent stopped");
            Ok(())
        });

        AgentHandle::new(tx, name)
    }

    /// Spawns a generic background task into the Hive (supervised).
    ///
    /// The task will be cancelled when the Hive shuts down.
    pub fn spawn_task<F>(&mut self, name: &str, task: F)
    where
        F: std::future::Future<Output = Result<()>> + Send + 'static,
    {
        let name = name.to_string();
        let token = self.cancel_token.clone();

        self.tasks.spawn(async move {
            tokio::pin!(task);

            loop {
                if token.is_cancelled() {
                    info!(task = %name, "Task cancelled");
                    return Ok(());
                }

                tokio::select! {
                    biased;
                    result = &mut task => {
                        match &result {
                            Ok(_) => info!(task = %name, "Task completed"),
                            Err(e) => error!(task = %name, error = %e, "Task failed"),
                        }
                        return result;
                    }
                    _ = tokio::time::sleep(tokio::time::Duration::from_millis(10)) => {}
                }
            }
        });
    }

    /// Returns the cancellation token for this Hive.
    ///
    /// Can be used to check if shutdown has been requested.
    pub fn cancel_token(&self) -> CancellationToken {
        self.cancel_token.clone()
    }

    /// Initiates graceful shutdown of all agents.
    ///
    /// This signals all agents to stop and waits for them to complete.
    pub async fn shutdown(mut self) {
        info!("Hive shutdown initiated");
        self.cancel_token.cancel();

        while let Some(result) = self.tasks.join_next().await {
            match result {
                Ok(Ok(())) => {}
                Ok(Err(e)) => error!(error = %e, "Agent task error during shutdown"),
                Err(e) => error!(error = %e, "Agent task panic during shutdown"),
            }
        }

        info!("Hive shutdown complete");
    }

    /// Returns the number of active agents.
    pub fn agent_count(&self) -> usize {
        self.tasks.len()
    }

    /// Returns true if all agents have stopped.
    pub fn is_empty(&self) -> bool {
        self.tasks.is_empty()
    }
}
