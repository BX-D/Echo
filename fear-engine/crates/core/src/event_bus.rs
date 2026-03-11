//! Async publish/subscribe event bus for decoupled communication between
//! game subsystems.
//!
//! The [`EventBus`] is thread-safe (`Send + Sync`) and delivers every published
//! event to all active subscribers via `tokio::sync::broadcast`.  A bounded
//! history ring-buffer is maintained for debugging.
//!
//! # Example
//!
//! ```
//! # tokio_test::block_on(async {
//! use fear_engine_core::event_bus::{EventBus, GameEvent};
//!
//! let bus = EventBus::new(64);
//! let mut rx = bus.subscribe();
//! bus.publish(GameEvent::PhaseChanged {
//!     from: "calibrating".into(),
//!     to: "exploring".into(),
//! });
//! let event = rx.recv().await.unwrap();
//! assert!(matches!(event, GameEvent::PhaseChanged { .. }));
//! # });
//! ```

use std::sync::{Arc, Mutex};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

// ---------------------------------------------------------------------------
// Game events
// ---------------------------------------------------------------------------

/// Every significant occurrence in the game that subsystems may react to.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GameEvent {
    /// Player entered a new scene.
    SceneEntered {
        scene_id: String,
        session_id: String,
    },
    /// Player made a choice.
    ChoiceMade {
        scene_id: String,
        choice_id: String,
        session_id: String,
    },
    /// A batch of behavior events was recorded.
    BehaviorRecorded {
        session_id: String,
        event_count: usize,
    },
    /// The fear profile was recalculated.
    FearProfileUpdated {
        session_id: String,
        top_fear: String,
        confidence: f64,
    },
    /// A narrative was generated (by AI or from static content).
    NarrativeGenerated {
        session_id: String,
        scene_id: String,
    },
    /// The game phase changed.
    PhaseChanged { from: String, to: String },
    /// An image was generated.
    ImageGenerated {
        session_id: String,
        scene_id: String,
    },
}

// ---------------------------------------------------------------------------
// History entry
// ---------------------------------------------------------------------------

/// A timestamped record of a published event.
#[derive(Debug, Clone)]
pub struct EventRecord {
    /// The event that was published.
    pub event: GameEvent,
    /// When the event was published.
    pub timestamp: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// Subscriber handle
// ---------------------------------------------------------------------------

/// A unique identifier for a subscription, used for unsubscribing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SubscriptionId(u64);

// ---------------------------------------------------------------------------
// Event bus
// ---------------------------------------------------------------------------

/// An async, thread-safe publish/subscribe event bus.
///
/// Internally uses [`tokio::sync::broadcast`] for fan-out delivery and a
/// [`Mutex`]-protected ring-buffer for event history.
///
/// # Example
///
/// ```
/// use fear_engine_core::event_bus::EventBus;
///
/// let bus = EventBus::new(128);
/// let _rx = bus.subscribe();
/// assert_eq!(bus.subscriber_count(), 1);
/// ```
pub struct EventBus {
    sender: broadcast::Sender<GameEvent>,
    history: Arc<Mutex<Vec<EventRecord>>>,
    history_capacity: usize,
    next_sub_id: Arc<Mutex<u64>>,
}

impl EventBus {
    /// Creates a new event bus with the given broadcast channel capacity.
    ///
    /// `capacity` controls how many events can be buffered before slow
    /// subscribers start lagging.
    ///
    /// # Example
    ///
    /// ```
    /// use fear_engine_core::event_bus::EventBus;
    /// let bus = EventBus::new(32);
    /// ```
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self {
            sender,
            history: Arc::new(Mutex::new(Vec::new())),
            history_capacity: capacity,
            next_sub_id: Arc::new(Mutex::new(0)),
        }
    }

    /// Publishes an event to all current subscribers and records it in
    /// the history buffer.
    ///
    /// Does **not** panic if there are zero subscribers — the event is
    /// still recorded in history.
    ///
    /// # Example
    ///
    /// ```
    /// use fear_engine_core::event_bus::{EventBus, GameEvent};
    ///
    /// let bus = EventBus::new(16);
    /// bus.publish(GameEvent::PhaseChanged {
    ///     from: "calibrating".into(),
    ///     to: "exploring".into(),
    /// });
    /// assert_eq!(bus.history().len(), 1);
    /// ```
    pub fn publish(&self, event: GameEvent) {
        // Record in history.
        {
            let mut hist = self.history.lock().expect("history lock poisoned");
            if hist.len() >= self.history_capacity {
                hist.remove(0);
            }
            hist.push(EventRecord {
                event: event.clone(),
                timestamp: Utc::now(),
            });
        }

        // Deliver to subscribers (ignore "no receivers" error).
        let _ = self.sender.send(event);
    }

    /// Creates a new subscriber that will receive all future events.
    ///
    /// Returns a [`broadcast::Receiver`] that can be `.recv().await`-ed.
    ///
    /// # Example
    ///
    /// ```
    /// use fear_engine_core::event_bus::EventBus;
    ///
    /// let bus = EventBus::new(16);
    /// let _rx = bus.subscribe();
    /// assert_eq!(bus.subscriber_count(), 1);
    /// ```
    pub fn subscribe(&self) -> broadcast::Receiver<GameEvent> {
        self.sender.subscribe()
    }

    /// Creates a named subscription and returns both the receiver and an
    /// ID that can be passed to [`Self::unsubscribe`].
    ///
    /// # Example
    ///
    /// ```
    /// use fear_engine_core::event_bus::EventBus;
    ///
    /// let bus = EventBus::new(16);
    /// let (id, _rx) = bus.subscribe_with_id();
    /// ```
    pub fn subscribe_with_id(&self) -> (SubscriptionId, broadcast::Receiver<GameEvent>) {
        let mut counter = self.next_sub_id.lock().expect("sub id lock poisoned");
        let id = SubscriptionId(*counter);
        *counter += 1;
        (id, self.sender.subscribe())
    }

    /// Returns the number of active subscribers.
    ///
    /// # Example
    ///
    /// ```
    /// use fear_engine_core::event_bus::EventBus;
    ///
    /// let bus = EventBus::new(16);
    /// assert_eq!(bus.subscriber_count(), 0);
    /// let _rx = bus.subscribe();
    /// assert_eq!(bus.subscriber_count(), 1);
    /// ```
    pub fn subscriber_count(&self) -> usize {
        self.sender.receiver_count()
    }

    /// Returns a snapshot of the event history (oldest first).
    ///
    /// # Example
    ///
    /// ```
    /// use fear_engine_core::event_bus::{EventBus, GameEvent};
    ///
    /// let bus = EventBus::new(16);
    /// bus.publish(GameEvent::SceneEntered {
    ///     scene_id: "intro".into(),
    ///     session_id: "s1".into(),
    /// });
    /// assert_eq!(bus.history().len(), 1);
    /// ```
    pub fn history(&self) -> Vec<EventRecord> {
        self.history.lock().expect("history lock poisoned").clone()
    }

    /// Clears the event history.
    pub fn clear_history(&self) {
        self.history.lock().expect("history lock poisoned").clear();
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn scene_entered(scene: &str) -> GameEvent {
        GameEvent::SceneEntered {
            scene_id: scene.into(),
            session_id: "test-session".into(),
        }
    }

    fn phase_changed(from: &str, to: &str) -> GameEvent {
        GameEvent::PhaseChanged {
            from: from.into(),
            to: to.into(),
        }
    }

    // -- Required tests ---------------------------------------------------

    #[tokio::test]
    async fn test_subscribe_and_receive_event() {
        let bus = EventBus::new(16);
        let mut rx = bus.subscribe();

        bus.publish(scene_entered("lobby"));

        let event = rx.recv().await.unwrap();
        assert!(matches!(
            event,
            GameEvent::SceneEntered { scene_id, .. } if scene_id == "lobby"
        ));
    }

    #[tokio::test]
    async fn test_multiple_subscribers_all_receive() {
        let bus = EventBus::new(16);
        let mut rx1 = bus.subscribe();
        let mut rx2 = bus.subscribe();
        let mut rx3 = bus.subscribe();

        bus.publish(phase_changed("calibrating", "exploring"));

        let e1 = rx1.recv().await.unwrap();
        let e2 = rx2.recv().await.unwrap();
        let e3 = rx3.recv().await.unwrap();

        for e in [&e1, &e2, &e3] {
            assert!(matches!(e, GameEvent::PhaseChanged { to, .. } if to == "exploring"));
        }
    }

    #[tokio::test]
    async fn test_unsubscribe_stops_receiving() {
        let bus = EventBus::new(16);
        let mut rx = bus.subscribe();

        // Receive first event.
        bus.publish(scene_entered("a"));
        let _ = rx.recv().await.unwrap();

        // Drop the receiver (= unsubscribe).
        drop(rx);

        // Publish another event — should not panic.
        bus.publish(scene_entered("b"));

        // Confirm subscriber count dropped.
        assert_eq!(bus.subscriber_count(), 0);
    }

    #[tokio::test]
    async fn test_event_history_tracks_all_events() {
        let bus = EventBus::new(64);

        bus.publish(scene_entered("a"));
        bus.publish(scene_entered("b"));
        bus.publish(phase_changed("calibrating", "exploring"));

        let history = bus.history();
        assert_eq!(history.len(), 3);

        assert!(matches!(
            &history[0].event,
            GameEvent::SceneEntered { scene_id, .. } if scene_id == "a"
        ));
        assert!(matches!(&history[2].event, GameEvent::PhaseChanged { .. }));
    }

    #[tokio::test]
    async fn test_publish_to_no_subscribers_doesnt_panic() {
        let bus = EventBus::new(16);
        assert_eq!(bus.subscriber_count(), 0);

        // Must not panic.
        bus.publish(scene_entered("solo"));
        bus.publish(phase_changed("a", "b"));

        // History still records the events.
        assert_eq!(bus.history().len(), 2);
    }

    #[tokio::test]
    async fn test_concurrent_publish_subscribe() {
        let bus = Arc::new(EventBus::new(256));
        let mut handles = Vec::new();

        // Spawn 5 subscriber tasks, each collecting 10 events.
        for _ in 0..5 {
            let mut rx = bus.subscribe();
            handles.push(tokio::spawn(async move {
                let mut received = Vec::new();
                for _ in 0..10 {
                    if let Ok(e) = rx.recv().await {
                        received.push(e);
                    }
                }
                received
            }));
        }

        // Publish 10 events from the main task.
        for i in 0..10 {
            bus.publish(scene_entered(&format!("scene_{i}")));
        }

        // All subscribers should have received all 10 events.
        for handle in handles {
            let received = handle.await.unwrap();
            assert_eq!(received.len(), 10);
        }
    }

    #[tokio::test]
    async fn test_event_ordering_preserved() {
        let bus = EventBus::new(64);
        let mut rx = bus.subscribe();

        for i in 0..5 {
            bus.publish(scene_entered(&format!("scene_{i}")));
        }

        for i in 0..5 {
            let event = rx.recv().await.unwrap();
            let expected = format!("scene_{i}");
            assert!(
                matches!(&event, GameEvent::SceneEntered { scene_id, .. } if *scene_id == expected),
                "event {i} out of order"
            );
        }
    }

    // -- Additional tests -------------------------------------------------

    #[tokio::test]
    async fn test_history_ring_buffer_evicts_oldest() {
        let bus = EventBus::new(3);
        bus.publish(scene_entered("a"));
        bus.publish(scene_entered("b"));
        bus.publish(scene_entered("c"));
        bus.publish(scene_entered("d")); // evicts "a"

        let history = bus.history();
        assert_eq!(history.len(), 3);
        assert!(matches!(
            &history[0].event,
            GameEvent::SceneEntered { scene_id, .. } if scene_id == "b"
        ));
    }

    #[tokio::test]
    async fn test_clear_history() {
        let bus = EventBus::new(16);
        bus.publish(scene_entered("a"));
        assert_eq!(bus.history().len(), 1);
        bus.clear_history();
        assert_eq!(bus.history().len(), 0);
    }

    #[tokio::test]
    async fn test_subscribe_with_id_returns_unique_ids() {
        let bus = EventBus::new(16);
        let (id1, _rx1) = bus.subscribe_with_id();
        let (id2, _rx2) = bus.subscribe_with_id();
        assert_ne!(id1, id2);
    }

    #[tokio::test]
    async fn test_all_event_variants_can_be_published() {
        let bus = EventBus::new(32);
        let mut rx = bus.subscribe();

        let events = vec![
            GameEvent::SceneEntered {
                scene_id: "s".into(),
                session_id: "x".into(),
            },
            GameEvent::ChoiceMade {
                scene_id: "s".into(),
                choice_id: "c".into(),
                session_id: "x".into(),
            },
            GameEvent::BehaviorRecorded {
                session_id: "x".into(),
                event_count: 5,
            },
            GameEvent::FearProfileUpdated {
                session_id: "x".into(),
                top_fear: "darkness".into(),
                confidence: 0.8,
            },
            GameEvent::NarrativeGenerated {
                session_id: "x".into(),
                scene_id: "s".into(),
            },
            GameEvent::PhaseChanged {
                from: "a".into(),
                to: "b".into(),
            },
            GameEvent::ImageGenerated {
                session_id: "x".into(),
                scene_id: "s".into(),
            },
        ];

        for event in &events {
            bus.publish(event.clone());
        }

        for _ in 0..7 {
            let _ = rx.recv().await.unwrap();
        }

        assert_eq!(bus.history().len(), 7);
    }

    #[tokio::test]
    async fn test_history_timestamps_are_monotonic() {
        let bus = EventBus::new(16);
        bus.publish(scene_entered("a"));
        bus.publish(scene_entered("b"));
        bus.publish(scene_entered("c"));

        let history = bus.history();
        for window in history.windows(2) {
            assert!(window[0].timestamp <= window[1].timestamp);
        }
    }
}
