use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, RwLock};
use tokio::sync::broadcast;

/// How many events a slow subscriber may fall behind before it starts losing them.
const CHANNEL_CAPACITY: usize = 256;

pub use tokio::sync::broadcast::error::{RecvError, TryRecvError};

/// Anything publishable on the [`EventBus`].
///
/// Typically an enum per domain (`PlatformEvent`, `GitHubEvent`), so that adding a
/// variant forces every exhaustive subscriber to acknowledge it.
pub trait Event: fmt::Debug + Clone + Send + Sync + 'static {
    /// Stable name used for logging and for forwarding across IPC.
    ///
    /// It is never used for dispatch — dispatch is by type.
    fn name(&self) -> &'static str;
}

#[derive(Debug, thiserror::Error)]
pub enum PublishError {
    #[error("event bus lock poisoned")]
    Poisoned,
}

/// Cloneable handle to the in-process bus. Cloning shares the same channels.
#[derive(Clone, Default)]
pub struct EventBus {
    channels: Arc<RwLock<HashMap<TypeId, Box<dyn Any + Send + Sync>>>>,
}

impl fmt::Debug for EventBus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let channels = self.channels.read().map(|c| c.len()).unwrap_or(0);
        f.debug_struct("EventBus")
            .field("event_types", &channels)
            .finish()
    }
}

impl EventBus {
    pub fn new() -> Self {
        Self::default()
    }

    /// Publish an event. Returns the number of live subscribers that received it.
    ///
    /// Publishing with no subscribers is not an error — a connector must not care
    /// whether anyone is listening.
    pub fn publish<E: Event>(&self, event: E) -> Result<usize, PublishError> {
        let name = event.name();
        let sender = self.sender::<E>()?;
        let delivered = sender.send(event).unwrap_or(0);
        tracing::debug!(event = name, subscribers = delivered, "event published");
        Ok(delivered)
    }

    /// Subscribe to every future event of type `E`.
    ///
    /// Events published before subscribing are not replayed.
    pub fn subscribe<E: Event>(&self) -> Result<EventStream<E>, PublishError> {
        Ok(EventStream {
            receiver: self.sender::<E>()?.subscribe(),
        })
    }

    fn sender<E: Event>(&self) -> Result<broadcast::Sender<E>, PublishError> {
        let type_id = TypeId::of::<E>();

        {
            let channels = self.channels.read().map_err(|_| PublishError::Poisoned)?;
            if let Some(existing) = channels.get(&type_id) {
                return Ok(Self::downcast::<E>(existing));
            }
        }

        let mut channels = self.channels.write().map_err(|_| PublishError::Poisoned)?;
        let entry = channels.entry(type_id).or_insert_with(|| {
            let (sender, _) = broadcast::channel::<E>(CHANNEL_CAPACITY);
            Box::new(sender)
        });
        Ok(Self::downcast::<E>(entry))
    }

    fn downcast<E: Event>(entry: &Box<dyn Any + Send + Sync>) -> broadcast::Sender<E> {
        entry
            .downcast_ref::<broadcast::Sender<E>>()
            // The map is keyed by `TypeId::of::<E>()` and only ever written with the
            // matching sender, so this cannot fail.
            .expect("event channel registered under a mismatched type id")
            .clone()
    }
}

/// A subscription to one event type.
#[derive(Debug)]
pub struct EventStream<E: Event> {
    receiver: broadcast::Receiver<E>,
}

impl<E: Event> EventStream<E> {
    /// Wait for the next event.
    ///
    /// Returns [`RecvError::Lagged`] when this subscriber fell too far behind; the
    /// stream stays usable and resumes at the oldest retained event.
    pub async fn recv(&mut self) -> Result<E, RecvError> {
        self.receiver.recv().await
    }

    /// Take an event only if one is already waiting.
    ///
    /// Used to drain a backlog without awaiting, and in tests to assert that
    /// *nothing* was published.
    pub fn try_recv(&mut self) -> Result<E, TryRecvError> {
        self.receiver.try_recv()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    struct Ping(u32);
    impl Event for Ping {
        fn name(&self) -> &'static str {
            "test.ping"
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    struct Pong;
    impl Event for Pong {
        fn name(&self) -> &'static str {
            "test.pong"
        }
    }

    #[tokio::test]
    async fn subscribers_receive_events_of_their_own_type() {
        let bus = EventBus::new();
        let mut pings = bus.subscribe::<Ping>().unwrap();

        assert_eq!(bus.publish(Ping(7)).unwrap(), 1);

        assert_eq!(pings.recv().await.unwrap(), Ping(7));
    }

    #[tokio::test]
    async fn events_of_a_different_type_are_not_delivered() {
        let bus = EventBus::new();
        let mut pings = bus.subscribe::<Ping>().unwrap();

        bus.publish(Pong).unwrap();
        bus.publish(Ping(1)).unwrap();

        // Pong did not end up in the Ping stream.
        assert_eq!(pings.recv().await.unwrap(), Ping(1));
    }

    #[tokio::test]
    async fn publishing_without_subscribers_is_not_an_error() {
        let bus = EventBus::new();
        assert_eq!(bus.publish(Ping(1)).unwrap(), 0);
    }

    #[tokio::test]
    async fn every_subscriber_receives_a_copy() {
        let bus = EventBus::new();
        let mut first = bus.subscribe::<Ping>().unwrap();
        let mut second = bus.subscribe::<Ping>().unwrap();

        assert_eq!(bus.publish(Ping(42)).unwrap(), 2);

        assert_eq!(first.recv().await.unwrap(), Ping(42));
        assert_eq!(second.recv().await.unwrap(), Ping(42));
    }
}
