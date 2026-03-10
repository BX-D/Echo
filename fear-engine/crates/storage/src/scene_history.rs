//! Scene history — records every scene the player visits, the narrative shown,
//! the choice made, and a snapshot of the fear profile at that moment.

use chrono::{DateTime, Utc};
use fear_engine_common::{FearEngineError, Result};
use rusqlite::params;
use serde::{Deserialize, Serialize};

use crate::{format_timestamp, parse_timestamp, Database};

/// A row in the `scene_history` table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneHistoryEntry {
    pub id: Option<i64>,
    pub session_id: String,
    pub scene_id: String,
    pub narrative_text: Option<String>,
    pub player_choice: Option<String>,
    pub fear_profile_snapshot_json: Option<String>,
    pub adaptation_strategy: Option<String>,
    pub timestamp: DateTime<Utc>,
}

impl Database {
    /// Appends a new entry to a session's scene history.
    ///
    /// # Example
    ///
    /// ```
    /// use fear_engine_storage::Database;
    /// use fear_engine_storage::scene_history::SceneHistoryEntry;
    /// use chrono::Utc;
    ///
    /// let db = Database::new_in_memory().unwrap();
    /// let sid = db.create_session(None).unwrap();
    /// let entry = SceneHistoryEntry {
    ///     id: None,
    ///     session_id: sid.clone(),
    ///     scene_id: "lobby".into(),
    ///     narrative_text: Some("You enter the lobby.".into()),
    ///     player_choice: None,
    ///     fear_profile_snapshot_json: None,
    ///     adaptation_strategy: None,
    ///     timestamp: Utc::now(),
    /// };
    /// db.insert_scene_history(&entry).unwrap();
    /// let history = db.get_scene_history(&sid).unwrap();
    /// assert_eq!(history.len(), 1);
    /// ```
    pub fn insert_scene_history(&self, entry: &SceneHistoryEntry) -> Result<()> {
        let ts = format_timestamp(&entry.timestamp);
        let conn = self.get_conn()?;
        conn.execute(
            "INSERT INTO scene_history (session_id, scene_id, narrative_text, player_choice, fear_profile_snapshot_json, adaptation_strategy, timestamp)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                entry.session_id,
                entry.scene_id,
                entry.narrative_text,
                entry.player_choice,
                entry.fear_profile_snapshot_json,
                entry.adaptation_strategy,
                ts,
            ],
        )?;
        Ok(())
    }

    /// Returns the full scene history for a session, ordered chronologically.
    ///
    /// # Example
    ///
    /// ```
    /// use fear_engine_storage::Database;
    ///
    /// let db = Database::new_in_memory().unwrap();
    /// let sid = db.create_session(None).unwrap();
    /// let history = db.get_scene_history(&sid).unwrap();
    /// assert!(history.is_empty());
    /// ```
    pub fn get_scene_history(&self, session_id: &str) -> Result<Vec<SceneHistoryEntry>> {
        let conn = self.get_conn()?;
        let mut stmt = conn.prepare(
            "SELECT id, session_id, scene_id, narrative_text, player_choice,
                    fear_profile_snapshot_json, adaptation_strategy, timestamp
             FROM scene_history
             WHERE session_id = ?1
             ORDER BY timestamp ASC",
        )?;
        let rows = stmt.query_map(params![session_id], |row| {
            Ok(SceneHistoryRow {
                id: row.get(0)?,
                session_id: row.get(1)?,
                scene_id: row.get(2)?,
                narrative_text: row.get(3)?,
                player_choice: row.get(4)?,
                fear_profile_snapshot_json: row.get(5)?,
                adaptation_strategy: row.get(6)?,
                timestamp: row.get(7)?,
            })
        })?;
        let mut out = Vec::new();
        for row in rows {
            let r = row?;
            out.push(SceneHistoryEntry {
                id: Some(r.id),
                session_id: r.session_id,
                scene_id: r.scene_id,
                narrative_text: r.narrative_text,
                player_choice: r.player_choice,
                fear_profile_snapshot_json: r.fear_profile_snapshot_json,
                adaptation_strategy: r.adaptation_strategy,
                timestamp: parse_timestamp(&r.timestamp)?,
            });
        }
        Ok(out)
    }

    /// Returns the most recent scene history entry for a session, or `None`
    /// if the session has no history yet.
    ///
    /// # Example
    ///
    /// ```
    /// use fear_engine_storage::Database;
    ///
    /// let db = Database::new_in_memory().unwrap();
    /// let sid = db.create_session(None).unwrap();
    /// assert!(db.get_latest_scene(&sid).unwrap().is_none());
    /// ```
    pub fn get_latest_scene(&self, session_id: &str) -> Result<Option<SceneHistoryEntry>> {
        let conn = self.get_conn()?;
        let result = conn.query_row(
            "SELECT id, session_id, scene_id, narrative_text, player_choice,
                    fear_profile_snapshot_json, adaptation_strategy, timestamp
             FROM scene_history
             WHERE session_id = ?1
             ORDER BY timestamp DESC
             LIMIT 1",
            params![session_id],
            |row| {
                Ok(SceneHistoryRow {
                    id: row.get(0)?,
                    session_id: row.get(1)?,
                    scene_id: row.get(2)?,
                    narrative_text: row.get(3)?,
                    player_choice: row.get(4)?,
                    fear_profile_snapshot_json: row.get(5)?,
                    adaptation_strategy: row.get(6)?,
                    timestamp: row.get(7)?,
                })
            },
        );
        match result {
            Ok(r) => Ok(Some(SceneHistoryEntry {
                id: Some(r.id),
                session_id: r.session_id,
                scene_id: r.scene_id,
                narrative_text: r.narrative_text,
                player_choice: r.player_choice,
                fear_profile_snapshot_json: r.fear_profile_snapshot_json,
                adaptation_strategy: r.adaptation_strategy,
                timestamp: parse_timestamp(&r.timestamp)?,
            })),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(FearEngineError::Database(e.to_string())),
        }
    }

    /// Updates the newest scene-history row for a session/scene pair.
    ///
    /// This is used when a placeholder narrative is persisted first and then
    /// replaced by AI-generated text after asynchronous generation completes.
    pub fn update_latest_scene_history_narrative(
        &self,
        session_id: &str,
        scene_id: &str,
        narrative_text: &str,
        fear_profile_snapshot_json: Option<&str>,
        adaptation_strategy: Option<&str>,
    ) -> Result<()> {
        let conn = self.get_conn()?;
        let rows = conn.execute(
            "UPDATE scene_history
             SET narrative_text = ?1,
                 fear_profile_snapshot_json = COALESCE(?2, fear_profile_snapshot_json),
                 adaptation_strategy = COALESCE(?3, adaptation_strategy)
             WHERE id = (
                 SELECT id
                 FROM scene_history
                 WHERE session_id = ?4 AND scene_id = ?5
                 ORDER BY timestamp DESC, id DESC
                 LIMIT 1
             )",
            params![
                narrative_text,
                fear_profile_snapshot_json,
                adaptation_strategy,
                session_id,
                scene_id,
            ],
        )?;
        if rows == 0 {
            return Err(FearEngineError::NotFound {
                entity: "SceneHistory".into(),
                id: format!("{session_id}:{scene_id}"),
            });
        }
        Ok(())
    }

    /// Updates the newest scene-history row for a session/scene pair with
    /// the player's chosen response.
    pub fn update_latest_scene_history_choice(
        &self,
        session_id: &str,
        scene_id: &str,
        player_choice: &str,
    ) -> Result<()> {
        let conn = self.get_conn()?;
        let rows = conn.execute(
            "UPDATE scene_history
             SET player_choice = ?1
             WHERE id = (
                 SELECT id
                 FROM scene_history
                 WHERE session_id = ?2 AND scene_id = ?3
                 ORDER BY timestamp DESC, id DESC
                 LIMIT 1
             )",
            params![player_choice, session_id, scene_id],
        )?;
        if rows == 0 {
            return Err(FearEngineError::NotFound {
                entity: "SceneHistory".into(),
                id: format!("{session_id}:{scene_id}"),
            });
        }
        Ok(())
    }
}

struct SceneHistoryRow {
    id: i64,
    session_id: String,
    scene_id: String,
    narrative_text: Option<String>,
    player_choice: Option<String>,
    fear_profile_snapshot_json: Option<String>,
    adaptation_strategy: Option<String>,
    timestamp: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_entry(session_id: &str, scene: &str) -> SceneHistoryEntry {
        SceneHistoryEntry {
            id: None,
            session_id: session_id.into(),
            scene_id: scene.into(),
            narrative_text: Some(format!("Narrative for {scene}")),
            player_choice: Some("choice_a".into()),
            fear_profile_snapshot_json: Some(r#"{"darkness":0.7}"#.into()),
            adaptation_strategy: Some("probe".into()),
            timestamp: Utc::now(),
        }
    }

    #[test]
    fn test_insert_and_get_scene_history() {
        let db = Database::new_in_memory().unwrap();
        let sid = db.create_session(None).unwrap();

        db.insert_scene_history(&make_entry(&sid, "lobby")).unwrap();
        db.insert_scene_history(&make_entry(&sid, "hallway")).unwrap();

        let history = db.get_scene_history(&sid).unwrap();
        assert_eq!(history.len(), 2);
        assert!(history[0].id.is_some());
        assert_eq!(history[0].scene_id, "lobby");
        assert_eq!(history[1].scene_id, "hallway");
    }

    #[test]
    fn test_get_scene_history_empty() {
        let db = Database::new_in_memory().unwrap();
        let sid = db.create_session(None).unwrap();
        let history = db.get_scene_history(&sid).unwrap();
        assert!(history.is_empty());
    }

    #[test]
    fn test_get_latest_scene() {
        let db = Database::new_in_memory().unwrap();
        let sid = db.create_session(None).unwrap();

        db.insert_scene_history(&make_entry(&sid, "lobby")).unwrap();
        // Small sleep not needed — SQLite CURRENT_TIMESTAMP is second-precision
        // and we supply our own timestamp, so ordering works.
        let mut later = make_entry(&sid, "basement");
        later.timestamp = Utc::now() + chrono::Duration::seconds(1);
        db.insert_scene_history(&later).unwrap();

        let latest = db.get_latest_scene(&sid).unwrap().unwrap();
        assert_eq!(latest.scene_id, "basement");
    }

    #[test]
    fn test_get_latest_scene_empty() {
        let db = Database::new_in_memory().unwrap();
        let sid = db.create_session(None).unwrap();
        assert!(db.get_latest_scene(&sid).unwrap().is_none());
    }

    #[test]
    fn test_scene_history_nullable_fields() {
        let db = Database::new_in_memory().unwrap();
        let sid = db.create_session(None).unwrap();

        let entry = SceneHistoryEntry {
            id: None,
            session_id: sid.clone(),
            scene_id: "intro".into(),
            narrative_text: None,
            player_choice: None,
            fear_profile_snapshot_json: None,
            adaptation_strategy: None,
            timestamp: Utc::now(),
        };
        db.insert_scene_history(&entry).unwrap();

        let history = db.get_scene_history(&sid).unwrap();
        assert_eq!(history.len(), 1);
        assert!(history[0].narrative_text.is_none());
        assert!(history[0].player_choice.is_none());
    }

    #[test]
    fn test_scene_history_preserves_all_fields() {
        let db = Database::new_in_memory().unwrap();
        let sid = db.create_session(None).unwrap();

        let entry = make_entry(&sid, "operating_room");
        db.insert_scene_history(&entry).unwrap();

        let history = db.get_scene_history(&sid).unwrap();
        let h = &history[0];
        assert_eq!(h.narrative_text.as_deref(), Some("Narrative for operating_room"));
        assert_eq!(h.player_choice.as_deref(), Some("choice_a"));
        assert!(h
            .fear_profile_snapshot_json
            .as_ref()
            .unwrap()
            .contains("darkness"));
        assert_eq!(h.adaptation_strategy.as_deref(), Some("probe"));
    }

    #[test]
    fn test_update_latest_scene_history_narrative() {
        let db = Database::new_in_memory().unwrap();
        let sid = db.create_session(None).unwrap();

        db.insert_scene_history(&make_entry(&sid, "ward_a")).unwrap();
        let mut later = make_entry(&sid, "ward_a");
        later.narrative_text = Some("Fallback text".into());
        later.timestamp = Utc::now() + chrono::Duration::seconds(1);
        db.insert_scene_history(&later).unwrap();

        db.update_latest_scene_history_narrative(
            &sid,
            "ward_a",
            "Final AI text",
            Some(r#"{"scores":[]}"#),
            Some("layering"),
        )
        .unwrap();

        let latest = db.get_latest_scene(&sid).unwrap().unwrap();
        assert_eq!(latest.scene_id, "ward_a");
        assert_eq!(latest.narrative_text.as_deref(), Some("Final AI text"));
        assert_eq!(latest.adaptation_strategy.as_deref(), Some("layering"));
        assert_eq!(
            latest.fear_profile_snapshot_json.as_deref(),
            Some(r#"{"scores":[]}"#)
        );
    }

    #[test]
    fn test_update_latest_scene_history_choice() {
        let db = Database::new_in_memory().unwrap();
        let sid = db.create_session(None).unwrap();

        let entry = SceneHistoryEntry {
            id: None,
            session_id: sid.clone(),
            scene_id: "mirror_surface".into(),
            narrative_text: Some("Narrative for mirror_surface".into()),
            player_choice: None,
            fear_profile_snapshot_json: None,
            adaptation_strategy: None,
            timestamp: Utc::now(),
        };
        db.insert_scene_history(&entry).unwrap();

        db.update_latest_scene_history_choice(&sid, "mirror_surface", "hold_gaze")
            .unwrap();

        let latest = db.get_latest_scene(&sid).unwrap().unwrap();
        assert_eq!(latest.player_choice.as_deref(), Some("hold_gaze"));
    }
}
