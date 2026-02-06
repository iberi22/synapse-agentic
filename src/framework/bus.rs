//! EventBus - System-wide event broadcasting.

use tokio::sync::broadcast;
use anyhow::Result;

/// A system-wide event bus for broadcasting messages to multiple subscribers.
///
/// This is useful for system-level events like Shutdown, ConfigUpdate, or
/// cross-cutting concerns that multiple agents need to react to.
///
/// # Example
///
/// ```rust,no_run
/// use synapse_agentic::framework::EventBus;
///
/// #[derive(Clone, Debug)]
/// enum SystemEvent {
///     ConfigReloaded,
///     MaintenanceMode(bool),
/// }
///
/// #[tokio::main]
/// async fn main() {
///     let bus: EventBus<SystemEvent> = EventBus::new(100);
///
///     let mut rx = bus.subscribe();
///
///     bus.publish(SystemEvent::ConfigReloaded).unwrap();
///
///     if let Ok(event) = rx.recv().await {
///         println!("Received: {:?}", event);
///     }
/// }
/// ```
#[derive(Clone)]
pub struct EventBus<T: Clone + Send + Sync + 'static> {
    tx: broadcast::Sender<T>,
}

impl<T: Clone + Send + Sync + 'static> EventBus<T> {
    /// Creates a new EventBus with the specified channel capacity.
    ///
    /// # Arguments
    ///
    /// * `capacity` - Maximum number of messages that can be buffered.
    ///   When full, oldest messages are dropped for slow receivers.
    pub fn new(capacity: usize) -> Self {
        let (tx, _) = broadcast::channel(capacity);
        Self { tx }
    }

    /// Subscribes to the event bus.
    ///
    /// Returns a receiver that will get all future events.
    /// Note: Events published before subscription are not received.
    pub fn subscribe(&self) -> broadcast::Receiver<T> {
        self.tx.subscribe()
    }

    /// Publishes an event to all subscribers.
    ///
    /// Returns the number of subscribers that received the event.
    /// Returns an error if there are no subscribers.
    pub fn publish(&self, event: T) -> Result<usize> {
        self.tx
            .send(event)
            .map_err(|_| anyhow::anyhow!("Failed to publish event: no subscribers"))
    }

    /// Returns the number of active subscribers.
    pub fn subscriber_count(&self) -> usize {
        self.tx.receiver_count()
    }
}
