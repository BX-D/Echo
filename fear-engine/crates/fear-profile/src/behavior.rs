//! Behavior event validation, batching, and collection.
//!
//! The [`BehaviorBatch`] type validates incoming events from the frontend.
//! The [`BehaviorCollector`] persists them to the database and maintains a
//! per-session sliding window of recent events for downstream analysis.

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

use chrono::{DateTime, Duration, Utc};
use fear_engine_common::types::{BehaviorEvent, BehaviorEventType};
use fear_engine_common::{FearEngineError, Result};
use fear_engine_storage::Database;

// ---------------------------------------------------------------------------
// Batch
// ---------------------------------------------------------------------------

/// A timestamped batch of behavior events from one session.
///
/// # Example
///
/// ```
/// use chrono::Utc;
/// use fear_engine_common::types::{BehaviorEvent, BehaviorEventType};
/// use fear_engine_fear_profile::behavior::BehaviorBatch;
///
/// let batch = BehaviorBatch {
///     events: vec![BehaviorEvent {
///         event_type: BehaviorEventType::Keystroke {
///             chars_per_second: 4.0,
///             backspace_count: 1,
///             total_chars: 20,
///         },
///         timestamp: Utc::now(),
///         scene_id: "intro".into(),
///     }],
///     session_id: "sess-1".into(),
///     batch_timestamp: Utc::now(),
/// };
/// batch.validate().unwrap();
/// ```
#[derive(Debug, Clone)]
pub struct BehaviorBatch {
    /// The individual behavior events in this batch.
    pub events: Vec<BehaviorEvent>,
    /// Session this batch belongs to.
    pub session_id: String,
    /// When the batch was assembled on the client.
    pub batch_timestamp: DateTime<Utc>,
}

impl BehaviorBatch {
    /// Validates the batch, rejecting impossible or malformed data.
    ///
    /// Checks:
    /// - Batch is not empty.
    /// - No event has a timestamp in the future (with 5 s tolerance).
    /// - No negative durations or speeds inside events.
    /// - Events are roughly ordered by timestamp.
    ///
    /// # Errors
    ///
    /// Returns [`FearEngineError::InvalidInput`] on the first violation found.
    pub fn validate(&self) -> Result<()> {
        if self.events.is_empty() {
            return Err(FearEngineError::InvalidInput {
                field: "events".into(),
                reason: "batch must contain at least one event".into(),
            });
        }

        let now_plus_tolerance = Utc::now() + Duration::seconds(5);

        let mut prev_ts: Option<DateTime<Utc>> = None;
        for (i, event) in self.events.iter().enumerate() {
            // Future timestamp check.
            if event.timestamp > now_plus_tolerance {
                return Err(FearEngineError::InvalidInput {
                    field: format!("events[{i}].timestamp"),
                    reason: "timestamp is in the future".into(),
                });
            }

            // Per-variant value checks.
            validate_event_type(&event.event_type, i)?;

            // Rough ordering: each timestamp >= previous - 1 s tolerance.
            if let Some(prev) = prev_ts {
                if event.timestamp < prev - Duration::seconds(1) {
                    return Err(FearEngineError::InvalidInput {
                        field: format!("events[{i}].timestamp"),
                        reason: "events are not in chronological order".into(),
                    });
                }
            }
            prev_ts = Some(event.timestamp);
        }

        Ok(())
    }
}

/// Validates the numeric fields inside a single event type.
fn validate_event_type(et: &BehaviorEventType, index: usize) -> Result<()> {
    match et {
        BehaviorEventType::Keystroke {
            chars_per_second, ..
        } => {
            if *chars_per_second < 0.0 {
                return Err(FearEngineError::InvalidInput {
                    field: format!("events[{index}].chars_per_second"),
                    reason: "chars_per_second cannot be negative".into(),
                });
            }
        }
        BehaviorEventType::Pause { duration_ms, .. } => {
            // duration_ms is u64, so it can't be negative at the type level,
            // but we still guard against unreasonably large values.
            if *duration_ms > 600_000 {
                return Err(FearEngineError::InvalidInput {
                    field: format!("events[{index}].duration_ms"),
                    reason: "pause duration exceeds 10 minutes".into(),
                });
            }
        }
        BehaviorEventType::MouseMovement {
            velocity,
            tremor_score,
        } => {
            if *velocity < 0.0 {
                return Err(FearEngineError::InvalidInput {
                    field: format!("events[{index}].velocity"),
                    reason: "velocity cannot be negative".into(),
                });
            }
            if *tremor_score < 0.0 || *tremor_score > 1.0 {
                return Err(FearEngineError::InvalidInput {
                    field: format!("events[{index}].tremor_score"),
                    reason: "tremor_score must be in [0.0, 1.0]".into(),
                });
            }
        }
        BehaviorEventType::Scroll { to_position, .. } => {
            if *to_position < 0.0 || *to_position > 1.0 {
                return Err(FearEngineError::InvalidInput {
                    field: format!("events[{index}].to_position"),
                    reason: "to_position must be in [0.0, 1.0]".into(),
                });
            }
        }
        BehaviorEventType::ChoiceHoverPattern {
            hovered_choice_ids,
            total_hover_ms,
            ..
        } => {
            if hovered_choice_ids.is_empty() {
                return Err(FearEngineError::InvalidInput {
                    field: format!("events[{index}].hovered_choice_ids"),
                    reason: "hovered_choice_ids cannot be empty".into(),
                });
            }
            if *total_hover_ms > 600_000 {
                return Err(FearEngineError::InvalidInput {
                    field: format!("events[{index}].total_hover_ms"),
                    reason: "choice hover duration exceeds 10 minutes".into(),
                });
            }
        }
        BehaviorEventType::MediaEngagement {
            dwell_ms,
            interaction_count,
            ..
        } => {
            if *dwell_ms > 3_600_000 {
                return Err(FearEngineError::InvalidInput {
                    field: format!("events[{index}].dwell_ms"),
                    reason: "media dwell exceeds 1 hour".into(),
                });
            }
            if *interaction_count > 10_000 {
                return Err(FearEngineError::InvalidInput {
                    field: format!("events[{index}].interaction_count"),
                    reason: "interaction_count is implausibly large".into(),
                });
            }
        }
        BehaviorEventType::CameraPresence { visible_ms, .. } => {
            if *visible_ms > 3_600_000 {
                return Err(FearEngineError::InvalidInput {
                    field: format!("events[{index}].visible_ms"),
                    reason: "camera presence exceeds 1 hour".into(),
                });
            }
        }
        BehaviorEventType::MicSilenceResponse { dwell_ms, .. } => {
            if *dwell_ms > 3_600_000 {
                return Err(FearEngineError::InvalidInput {
                    field: format!("events[{index}].dwell_ms"),
                    reason: "mic silence dwell exceeds 1 hour".into(),
                });
            }
        }
        BehaviorEventType::DevicePermission { device, .. } => {
            if device.trim().is_empty() {
                return Err(FearEngineError::InvalidInput {
                    field: format!("events[{index}].device"),
                    reason: "device cannot be empty".into(),
                });
            }
        }
        BehaviorEventType::FocusChange {
            return_latency_ms,
            ..
        } => {
            if let Some(latency) = return_latency_ms {
                if *latency > 3_600_000 {
                    return Err(FearEngineError::InvalidInput {
                        field: format!("events[{index}].return_latency_ms"),
                        reason: "focus return latency exceeds 1 hour".into(),
                    });
                }
            }
        }
        BehaviorEventType::Choice { .. } => {}
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Collector
// ---------------------------------------------------------------------------

/// Receives validated batches, persists them, and maintains a per-session
/// sliding window of recent events.
///
/// # Example
///
/// ```
/// use std::sync::Arc;
/// use fear_engine_storage::Database;
/// use fear_engine_fear_profile::behavior::BehaviorCollector;
///
/// let db = Arc::new(Database::new_in_memory().unwrap());
/// let collector = BehaviorCollector::new(db);
/// assert!(collector.get_recent_events("nonexistent").is_empty());
/// ```
pub struct BehaviorCollector {
    db: Arc<Database>,
    recent_events: HashMap<String, VecDeque<BehaviorEvent>>,
    window_duration: Duration,
}

impl BehaviorCollector {
    /// Creates a new collector with a 60-second sliding window.
    pub fn new(db: Arc<Database>) -> Self {
        Self {
            db,
            recent_events: HashMap::new(),
            window_duration: Duration::seconds(60),
        }
    }

    /// Creates a collector with a custom window duration (for testing).
    ///
    /// # Example
    ///
    /// ```
    /// use std::sync::Arc;
    /// use chrono::Duration;
    /// use fear_engine_storage::Database;
    /// use fear_engine_fear_profile::behavior::BehaviorCollector;
    ///
    /// let db = Arc::new(Database::new_in_memory().unwrap());
    /// let collector = BehaviorCollector::with_window(db, Duration::seconds(10));
    /// ```
    pub fn with_window(db: Arc<Database>, window_duration: Duration) -> Self {
        Self {
            db,
            recent_events: HashMap::new(),
            window_duration,
        }
    }

    /// Validates, persists, and indexes a batch of behavior events.
    ///
    /// 1. Calls [`BehaviorBatch::validate`].
    /// 2. Stores the events in the database.
    /// 3. Appends events to the sliding window for the session.
    /// 4. Evicts events older than the window duration.
    ///
    /// # Errors
    ///
    /// Propagates validation or database errors.
    ///
    /// # Example
    ///
    /// ```
    /// # use std::sync::Arc;
    /// # use chrono::Utc;
    /// # use fear_engine_storage::Database;
    /// # use fear_engine_common::types::{BehaviorEvent, BehaviorEventType};
    /// # use fear_engine_fear_profile::behavior::{BehaviorBatch, BehaviorCollector};
    /// let db = Arc::new(Database::new_in_memory().unwrap());
    /// let sid = db.create_session(None).unwrap();
    /// let mut collector = BehaviorCollector::new(db);
    /// let batch = BehaviorBatch {
    ///     events: vec![BehaviorEvent {
    ///         event_type: BehaviorEventType::Keystroke {
    ///             chars_per_second: 5.0, backspace_count: 0, total_chars: 30,
    ///         },
    ///         timestamp: Utc::now(),
    ///         scene_id: "s1".into(),
    ///     }],
    ///     session_id: sid.clone(),
    ///     batch_timestamp: Utc::now(),
    /// };
    /// collector.process_batch(batch).unwrap();
    /// assert_eq!(collector.get_recent_events(&sid).len(), 1);
    /// ```
    pub fn process_batch(&mut self, batch: BehaviorBatch) -> Result<()> {
        batch.validate()?;

        self.db
            .insert_behavior_events(&batch.session_id, &batch.events)?;

        let window = self
            .recent_events
            .entry(batch.session_id.clone())
            .or_default();

        for event in &batch.events {
            window.push_back(event.clone());
        }

        self.evict_old_events(&batch.session_id);

        Ok(())
    }

    /// Returns the recent events for a session (within the sliding window).
    ///
    /// Returns an empty slice if the session has no recorded events.
    pub fn get_recent_events(&self, session_id: &str) -> &[BehaviorEvent] {
        match self.recent_events.get(session_id) {
            Some(deque) => deque.as_slices().0, // contiguous front slice
            None => &[],
        }
    }

    /// Returns all recent events for a session as a `Vec` (avoids
    /// split-deque issues).
    ///
    /// # Example
    ///
    /// ```
    /// # use std::sync::Arc;
    /// # use fear_engine_storage::Database;
    /// # use fear_engine_fear_profile::behavior::BehaviorCollector;
    /// let db = Arc::new(Database::new_in_memory().unwrap());
    /// let collector = BehaviorCollector::new(db);
    /// assert!(collector.get_recent_events_vec("x").is_empty());
    /// ```
    pub fn get_recent_events_vec(&self, session_id: &str) -> Vec<BehaviorEvent> {
        match self.recent_events.get(session_id) {
            Some(deque) => deque.iter().cloned().collect(),
            None => Vec::new(),
        }
    }

    /// Removes all tracked state for a session (called on disconnect).
    pub fn clear_session(&mut self, session_id: &str) {
        self.recent_events.remove(session_id);
    }

    /// Evicts events older than the window duration from the front of the
    /// deque for the given session.
    fn evict_old_events(&mut self, session_id: &str) {
        let cutoff = Utc::now() - self.window_duration;
        if let Some(deque) = self.recent_events.get_mut(session_id) {
            while let Some(front) = deque.front() {
                if front.timestamp < cutoff {
                    deque.pop_front();
                } else {
                    break;
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;
    use fear_engine_common::types::{ChoiceApproach, ScrollDirection};
    use proptest::prelude::*;

    fn keystroke_event(cps: f64) -> BehaviorEvent {
        BehaviorEvent {
            event_type: BehaviorEventType::Keystroke {
                chars_per_second: cps,
                backspace_count: 0,
                total_chars: 10,
            },
            timestamp: Utc::now(),
            scene_id: "s1".into(),
        }
    }

    fn make_batch(events: Vec<BehaviorEvent>) -> BehaviorBatch {
        BehaviorBatch {
            events,
            session_id: "test-session".into(),
            batch_timestamp: Utc::now(),
        }
    }

    fn db_and_session() -> (Arc<Database>, String) {
        let db = Arc::new(Database::new_in_memory().unwrap());
        let sid = db.create_session(None).unwrap();
        (db, sid)
    }

    // -- Validation -------------------------------------------------------

    #[test]
    fn test_all_event_types_validate() {
        let events = vec![
            BehaviorEvent {
                event_type: BehaviorEventType::Keystroke {
                    chars_per_second: 5.0,
                    backspace_count: 2,
                    total_chars: 40,
                },
                timestamp: Utc::now(),
                scene_id: "s1".into(),
            },
            BehaviorEvent {
                event_type: BehaviorEventType::Pause {
                    duration_ms: 3000,
                    scene_content_hash: "abc".into(),
                },
                timestamp: Utc::now(),
                scene_id: "s1".into(),
            },
            BehaviorEvent {
                event_type: BehaviorEventType::Choice {
                    choice_id: "c1".into(),
                    time_to_decide_ms: 1500,
                    approach: ChoiceApproach::Investigate,
                },
                timestamp: Utc::now(),
                scene_id: "s1".into(),
            },
            BehaviorEvent {
                event_type: BehaviorEventType::MouseMovement {
                    velocity: 120.0,
                    tremor_score: 0.5,
                },
                timestamp: Utc::now(),
                scene_id: "s1".into(),
            },
            BehaviorEvent {
                event_type: BehaviorEventType::Scroll {
                    direction: ScrollDirection::Down,
                    to_position: 0.75,
                    rereading: false,
                },
                timestamp: Utc::now(),
                scene_id: "s1".into(),
            },
        ];
        make_batch(events).validate().unwrap();
    }

    #[test]
    fn test_batch_validation_catches_future_timestamps() {
        let future = Utc::now() + Duration::minutes(10);
        let batch = BehaviorBatch {
            events: vec![BehaviorEvent {
                event_type: BehaviorEventType::Keystroke {
                    chars_per_second: 5.0,
                    backspace_count: 0,
                    total_chars: 10,
                },
                timestamp: future,
                scene_id: "s1".into(),
            }],
            session_id: "sess".into(),
            batch_timestamp: Utc::now(),
        };
        let err = batch.validate().unwrap_err();
        assert!(err.to_string().contains("future"));
    }

    #[test]
    fn test_batch_validation_catches_negative_speed() {
        let batch = make_batch(vec![keystroke_event(-1.0)]);
        let err = batch.validate().unwrap_err();
        assert!(err.to_string().contains("negative"));
    }

    #[test]
    fn test_batch_validation_catches_negative_velocity() {
        let batch = make_batch(vec![BehaviorEvent {
            event_type: BehaviorEventType::MouseMovement {
                velocity: -50.0,
                tremor_score: 0.5,
            },
            timestamp: Utc::now(),
            scene_id: "s1".into(),
        }]);
        let err = batch.validate().unwrap_err();
        assert!(err.to_string().contains("negative"));
    }

    #[test]
    fn test_batch_validation_catches_invalid_tremor_score() {
        let batch = make_batch(vec![BehaviorEvent {
            event_type: BehaviorEventType::MouseMovement {
                velocity: 100.0,
                tremor_score: 1.5,
            },
            timestamp: Utc::now(),
            scene_id: "s1".into(),
        }]);
        let err = batch.validate().unwrap_err();
        assert!(err.to_string().contains("tremor_score"));
    }

    #[test]
    fn test_batch_validation_catches_invalid_scroll_position() {
        let batch = make_batch(vec![BehaviorEvent {
            event_type: BehaviorEventType::Scroll {
                direction: ScrollDirection::Up,
                to_position: 1.5,
                rereading: false,
            },
            timestamp: Utc::now(),
            scene_id: "s1".into(),
        }]);
        let err = batch.validate().unwrap_err();
        assert!(err.to_string().contains("to_position"));
    }

    #[test]
    fn test_batch_validation_catches_empty_batch() {
        let batch = make_batch(vec![]);
        let err = batch.validate().unwrap_err();
        assert!(err.to_string().contains("at least one"));
    }

    #[test]
    fn test_batch_validation_catches_out_of_order() {
        let now = Utc::now();
        let batch = BehaviorBatch {
            events: vec![
                BehaviorEvent {
                    event_type: BehaviorEventType::Keystroke {
                        chars_per_second: 5.0,
                        backspace_count: 0,
                        total_chars: 10,
                    },
                    timestamp: now,
                    scene_id: "s1".into(),
                },
                BehaviorEvent {
                    event_type: BehaviorEventType::Keystroke {
                        chars_per_second: 5.0,
                        backspace_count: 0,
                        total_chars: 10,
                    },
                    timestamp: now - Duration::seconds(30),
                    scene_id: "s1".into(),
                },
            ],
            session_id: "sess".into(),
            batch_timestamp: now,
        };
        let err = batch.validate().unwrap_err();
        assert!(err.to_string().contains("chronological"));
    }

    #[test]
    fn test_batch_validation_catches_excessive_pause() {
        let batch = make_batch(vec![BehaviorEvent {
            event_type: BehaviorEventType::Pause {
                duration_ms: 700_000,
                scene_content_hash: "h".into(),
            },
            timestamp: Utc::now(),
            scene_id: "s1".into(),
        }]);
        let err = batch.validate().unwrap_err();
        assert!(err.to_string().contains("10 minutes"));
    }

    // -- Collector --------------------------------------------------------

    #[test]
    fn test_collector_stores_to_database() {
        let (db, sid) = db_and_session();
        let mut collector = BehaviorCollector::new(db.clone());
        let batch = BehaviorBatch {
            events: vec![keystroke_event(5.0), keystroke_event(6.0)],
            session_id: sid.clone(),
            batch_timestamp: Utc::now(),
        };
        collector.process_batch(batch).unwrap();
        assert_eq!(db.count_behavior_events(&sid).unwrap(), 2);
    }

    #[test]
    fn test_collector_maintains_sliding_window() {
        let (db, sid) = db_and_session();
        let mut collector = BehaviorCollector::new(db);
        let batch = BehaviorBatch {
            events: vec![keystroke_event(5.0), keystroke_event(6.0)],
            session_id: sid.clone(),
            batch_timestamp: Utc::now(),
        };
        collector.process_batch(batch).unwrap();
        let recent = collector.get_recent_events_vec(&sid);
        assert_eq!(recent.len(), 2);
    }

    #[test]
    fn test_sliding_window_evicts_old_events() {
        let (db, sid) = db_and_session();
        let mut collector = BehaviorCollector::with_window(db, Duration::seconds(5));

        // Insert an old event (timestamp 30s ago) + a recent one.
        let old_event = BehaviorEvent {
            event_type: BehaviorEventType::Keystroke {
                chars_per_second: 3.0,
                backspace_count: 0,
                total_chars: 10,
            },
            timestamp: Utc::now() - Duration::seconds(30),
            scene_id: "s1".into(),
        };
        let new_event = keystroke_event(5.0);

        let batch = BehaviorBatch {
            events: vec![old_event, new_event],
            session_id: sid.clone(),
            batch_timestamp: Utc::now(),
        };
        collector.process_batch(batch).unwrap();

        // Old event should have been evicted.
        let recent = collector.get_recent_events_vec(&sid);
        assert_eq!(recent.len(), 1);
    }

    #[test]
    fn test_clear_session() {
        let (db, sid) = db_and_session();
        let mut collector = BehaviorCollector::new(db);
        let batch = BehaviorBatch {
            events: vec![keystroke_event(5.0)],
            session_id: sid.clone(),
            batch_timestamp: Utc::now(),
        };
        collector.process_batch(batch).unwrap();
        assert!(!collector.get_recent_events_vec(&sid).is_empty());

        collector.clear_session(&sid);
        assert!(collector.get_recent_events_vec(&sid).is_empty());
    }

    #[test]
    fn test_get_recent_events_empty_session() {
        let db = Arc::new(Database::new_in_memory().unwrap());
        let collector = BehaviorCollector::new(db);
        assert!(collector.get_recent_events("nonexistent").is_empty());
    }

    #[test]
    fn test_process_invalid_batch_returns_error() {
        let (db, sid) = db_and_session();
        let mut collector = BehaviorCollector::new(db);
        let batch = BehaviorBatch {
            events: vec![],
            session_id: sid,
            batch_timestamp: Utc::now(),
        };
        assert!(collector.process_batch(batch).is_err());
    }

    // -- Property test ----------------------------------------------------

    proptest! {
        #[test]
        fn test_valid_events_serialize_deserialize(
            cps in 0.0..100.0f64,
            bc in 0u32..50,
            tc in 1u32..1000,
        ) {
            let event = BehaviorEvent {
                event_type: BehaviorEventType::Keystroke {
                    chars_per_second: cps,
                    backspace_count: bc,
                    total_chars: tc,
                },
                timestamp: Utc::now(),
                scene_id: "s1".into(),
            };
            let json = serde_json::to_string(&event).unwrap();
            let back: BehaviorEvent = serde_json::from_str(&json).unwrap();
            match back.event_type {
                BehaviorEventType::Keystroke { chars_per_second, backspace_count, total_chars } => {
                    prop_assert!((chars_per_second - cps).abs() < 1e-10);
                    prop_assert_eq!(backspace_count, bc);
                    prop_assert_eq!(total_chars, tc);
                }
                _ => prop_assert!(false, "wrong variant"),
            }
        }
    }
}
