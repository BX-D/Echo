//! Raw behavior event storage — persists every player-input signal for
//! downstream analysis by the fear-profiling engine.

use chrono::{DateTime, Utc};
use fear_engine_common::types::{BehaviorEvent, BehaviorEventType};
use fear_engine_common::Result;
use rusqlite::params;

use crate::{format_timestamp, parse_timestamp, Database};

/// Returns the discriminant tag used in the `event_type` column.
fn event_type_name(event: &BehaviorEventType) -> &'static str {
    match event {
        BehaviorEventType::Keystroke { .. } => "keystroke",
        BehaviorEventType::Pause { .. } => "pause",
        BehaviorEventType::Choice { .. } => "choice",
        BehaviorEventType::MouseMovement { .. } => "mouse_movement",
        BehaviorEventType::Scroll { .. } => "scroll",
        BehaviorEventType::ChoiceHoverPattern { .. } => "choice_hover_pattern",
        BehaviorEventType::MediaEngagement { .. } => "media_engagement",
        BehaviorEventType::CameraPresence { .. } => "camera_presence",
        BehaviorEventType::MicSilenceResponse { .. } => "mic_silence_response",
        BehaviorEventType::DevicePermission { .. } => "device_permission",
        BehaviorEventType::FocusChange { .. } => "focus_change",
    }
}

impl Database {
    /// Inserts a batch of behavior events within a single transaction.
    ///
    /// # Example
    ///
    /// ```
    /// use fear_engine_storage::Database;
    /// use fear_engine_common::types::{BehaviorEvent, BehaviorEventType};
    /// use chrono::Utc;
    ///
    /// let db = Database::new_in_memory().unwrap();
    /// let sid = db.create_session(None).unwrap();
    /// let events = vec![BehaviorEvent {
    ///     event_type: BehaviorEventType::Keystroke {
    ///         chars_per_second: 4.5,
    ///         backspace_count: 1,
    ///         total_chars: 20,
    ///     },
    ///     timestamp: Utc::now(),
    ///     scene_id: "scene_01".into(),
    /// }];
    /// db.insert_behavior_events(&sid, &events).unwrap();
    /// assert_eq!(db.count_behavior_events(&sid).unwrap(), 1);
    /// ```
    pub fn insert_behavior_events(&self, session_id: &str, events: &[BehaviorEvent]) -> Result<()> {
        let conn = self.get_conn()?;
        let tx = conn.unchecked_transaction()?;
        {
            let mut stmt = tx.prepare(
                "INSERT INTO behavior_events (session_id, event_type, event_data_json, scene_id, timestamp)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
            )?;
            for event in events {
                let type_name = event_type_name(&event.event_type);
                let data_json = serde_json::to_string(&event.event_type)?;
                let ts = format_timestamp(&event.timestamp);
                stmt.execute(params![
                    session_id,
                    type_name,
                    data_json,
                    event.scene_id,
                    ts
                ])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    /// Retrieves behavior events for a session, optionally filtered to those
    /// recorded after `since`.
    ///
    /// # Example
    ///
    /// ```
    /// use fear_engine_storage::Database;
    /// use fear_engine_common::types::{BehaviorEvent, BehaviorEventType};
    /// use chrono::Utc;
    ///
    /// let db = Database::new_in_memory().unwrap();
    /// let sid = db.create_session(None).unwrap();
    /// let events = vec![BehaviorEvent {
    ///     event_type: BehaviorEventType::Pause { duration_ms: 3000, scene_content_hash: "h1".into() },
    ///     timestamp: Utc::now(),
    ///     scene_id: "s1".into(),
    /// }];
    /// db.insert_behavior_events(&sid, &events).unwrap();
    /// let fetched = db.get_behavior_events(&sid, None).unwrap();
    /// assert_eq!(fetched.len(), 1);
    /// ```
    pub fn get_behavior_events(
        &self,
        session_id: &str,
        since: Option<DateTime<Utc>>,
    ) -> Result<Vec<BehaviorEvent>> {
        let conn = self.get_conn()?;

        let (sql, do_bind_since) = match since {
            Some(_) => (
                "SELECT event_data_json, scene_id, timestamp
                 FROM behavior_events
                 WHERE session_id = ?1 AND timestamp >= ?2
                 ORDER BY timestamp ASC",
                true,
            ),
            None => (
                "SELECT event_data_json, scene_id, timestamp
                 FROM behavior_events
                 WHERE session_id = ?1
                 ORDER BY timestamp ASC",
                false,
            ),
        };

        let mut stmt = conn.prepare(sql)?;

        let row_iter = if do_bind_since {
            let since_str = format_timestamp(&since.unwrap());
            stmt.query_map(params![session_id, since_str], map_row)?
        } else {
            stmt.query_map(params![session_id], map_row)?
        };

        let mut out = Vec::new();
        for row in row_iter {
            let (data_json, scene_id, ts_str): (String, String, String) = row?;
            let event_type: BehaviorEventType = serde_json::from_str(&data_json)?;
            let timestamp = parse_timestamp(&ts_str)?;
            out.push(BehaviorEvent {
                event_type,
                timestamp,
                scene_id,
            });
        }
        Ok(out)
    }

    /// Returns the total number of behavior events recorded for a session.
    ///
    /// # Example
    ///
    /// ```
    /// use fear_engine_storage::Database;
    ///
    /// let db = Database::new_in_memory().unwrap();
    /// let sid = db.create_session(None).unwrap();
    /// assert_eq!(db.count_behavior_events(&sid).unwrap(), 0);
    /// ```
    pub fn count_behavior_events(&self, session_id: &str) -> Result<u64> {
        let conn = self.get_conn()?;
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM behavior_events WHERE session_id = ?1",
            params![session_id],
            |row| row.get(0),
        )?;
        Ok(count as u64)
    }
}

/// Row mapper shared by both query branches.
fn map_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<(String, String, String)> {
    Ok((row.get(0)?, row.get(1)?, row.get(2)?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use fear_engine_common::types::{ChoiceApproach, ScrollDirection};

    fn make_db_and_session() -> (Database, String) {
        let db = Database::new_in_memory().unwrap();
        let sid = db.create_session(None).unwrap();
        (db, sid)
    }

    #[test]
    fn test_insert_and_get_behavior_events() {
        let (db, sid) = make_db_and_session();
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
                    duration_ms: 2500,
                    scene_content_hash: "abc".into(),
                },
                timestamp: Utc::now(),
                scene_id: "s1".into(),
            },
        ];
        db.insert_behavior_events(&sid, &events).unwrap();
        let fetched = db.get_behavior_events(&sid, None).unwrap();
        assert_eq!(fetched.len(), 2);
    }

    #[test]
    fn test_insert_empty_batch() {
        let (db, sid) = make_db_and_session();
        db.insert_behavior_events(&sid, &[]).unwrap();
        assert_eq!(db.count_behavior_events(&sid).unwrap(), 0);
    }

    #[test]
    fn test_get_events_since_timestamp() {
        let (db, sid) = make_db_and_session();
        let early = Utc::now() - chrono::Duration::seconds(10);
        let late = Utc::now();

        let events = vec![
            BehaviorEvent {
                event_type: BehaviorEventType::Keystroke {
                    chars_per_second: 3.0,
                    backspace_count: 0,
                    total_chars: 10,
                },
                timestamp: early,
                scene_id: "s1".into(),
            },
            BehaviorEvent {
                event_type: BehaviorEventType::Keystroke {
                    chars_per_second: 6.0,
                    backspace_count: 0,
                    total_chars: 20,
                },
                timestamp: late,
                scene_id: "s1".into(),
            },
        ];
        db.insert_behavior_events(&sid, &events).unwrap();

        // "since" a moment between the two events
        let mid = Utc::now() - chrono::Duration::seconds(5);
        let fetched = db.get_behavior_events(&sid, Some(mid)).unwrap();
        assert_eq!(fetched.len(), 1);
    }

    #[test]
    fn test_count_behavior_events() {
        let (db, sid) = make_db_and_session();
        assert_eq!(db.count_behavior_events(&sid).unwrap(), 0);
        let events: Vec<BehaviorEvent> = (0..5)
            .map(|_| BehaviorEvent {
                event_type: BehaviorEventType::MouseMovement {
                    velocity: 100.0,
                    tremor_score: 0.5,
                },
                timestamp: Utc::now(),
                scene_id: "s1".into(),
            })
            .collect();
        db.insert_behavior_events(&sid, &events).unwrap();
        assert_eq!(db.count_behavior_events(&sid).unwrap(), 5);
    }

    #[test]
    fn test_all_event_types_roundtrip() {
        let (db, sid) = make_db_and_session();
        let events = vec![
            BehaviorEvent {
                event_type: BehaviorEventType::Keystroke {
                    chars_per_second: 4.0,
                    backspace_count: 1,
                    total_chars: 30,
                },
                timestamp: Utc::now(),
                scene_id: "s1".into(),
            },
            BehaviorEvent {
                event_type: BehaviorEventType::Pause {
                    duration_ms: 1500,
                    scene_content_hash: "hash".into(),
                },
                timestamp: Utc::now(),
                scene_id: "s1".into(),
            },
            BehaviorEvent {
                event_type: BehaviorEventType::Choice {
                    choice_id: "c1".into(),
                    time_to_decide_ms: 3000,
                    approach: ChoiceApproach::Investigate,
                },
                timestamp: Utc::now(),
                scene_id: "s1".into(),
            },
            BehaviorEvent {
                event_type: BehaviorEventType::MouseMovement {
                    velocity: 200.0,
                    tremor_score: 0.9,
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
        db.insert_behavior_events(&sid, &events).unwrap();
        let fetched = db.get_behavior_events(&sid, None).unwrap();
        assert_eq!(fetched.len(), 5);
    }

    #[test]
    fn test_batch_insert_performance() {
        let (db, sid) = make_db_and_session();
        let events: Vec<BehaviorEvent> = (0..200)
            .map(|i| BehaviorEvent {
                event_type: BehaviorEventType::Keystroke {
                    chars_per_second: i as f64,
                    backspace_count: 0,
                    total_chars: 100,
                },
                timestamp: Utc::now(),
                scene_id: "scene_01".into(),
            })
            .collect();
        let start = std::time::Instant::now();
        db.insert_behavior_events(&sid, &events).unwrap();
        let elapsed = start.elapsed();
        assert!(
            elapsed.as_secs() < 5,
            "batch insert of 200 events took {elapsed:?}"
        );
        assert_eq!(db.count_behavior_events(&sid).unwrap(), 200);
    }
}
