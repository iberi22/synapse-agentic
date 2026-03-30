//! EventBus - System-wide event broadcasting.

use anyhow::Result;
use parking_lot::RwLock;
use std::sync::Arc;
use tokio::sync::broadcast;

/// Trait for intercepting events published to the EventBus.
/// This allows external systems (like Gestalt) to observe and persist framework-level events.
pub trait EventInterceptor<T>: Send + Sync + 'static {
    fn intercept(&self, event: &T);
}

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
///     let mut bus: EventBus<SystemEvent> = EventBus::new(100);
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
    interceptors: Arc<RwLock<Vec<Box<dyn EventInterceptor<T>>>>>,
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
        Self {
            tx,
            interceptors: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Registers a new interceptor that will be called for every event published.
    pub fn add_interceptor(&self, interceptor: Box<dyn EventInterceptor<T>>) {
        let mut writers = self.interceptors.write();
        writers.push(interceptor);
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
    /// The event is first passed to all registered interceptors.
    /// Returns the number of subscribers that received the event.
    /// Returns an error if there are no subscribers.
    pub fn publish(&self, event: T) -> Result<usize> {
        let interceptors = self.interceptors.read();
        for interceptor in interceptors.iter() {
            interceptor.intercept(&event);
        }

        self.tx
            .send(event)
            .map_err(|_| anyhow::anyhow!("Failed to publish event: no subscribers"))
    }

    /// Returns the number of active subscribers.
    pub fn subscriber_count(&self) -> usize {
        self.tx.receiver_count()
    }
}
