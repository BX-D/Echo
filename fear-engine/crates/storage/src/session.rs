//! Game session CRUD operations.

use chrono::{DateTime, Utc};
use fear_engine_common::types::GamePhase;
use fear_engine_common::{FearEngineError, Result};
use rusqlite::params;
use serde::{Deserialize, Serialize};

use crate::{format_timestamp, game_phase_from_str, game_phase_to_str, parse_timestamp, Database};

/// A persisted game session row.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub player_name: Option<String>,
    pub current_scene_id: String,
    pub game_phase: GamePhase,
    pub game_state_json: String,
    pub completed: bool,
}

impl Database {
    /// Creates a new game session and returns its UUID.
    ///
    /// The session starts in the [`GamePhase::Calibrating`] phase with
    /// `current_scene_id` set to `"intro"`.
    ///
    /// # Example
    ///
    /// ```
    /// use fear_engine_storage::Database;
    ///
    /// let db = Database::new_in_memory().unwrap();
    /// let id = db.create_session(Some("Alice")).unwrap();
    /// assert_eq!(id.len(), 36); // UUID v4
    /// ```
    /// Creates a session with a specific ID (for sharing IDs across DBs).
    pub fn create_session_with_id(&self, id: &str, player_name: Option<&str>) -> Result<()> {
        let now = format_timestamp(&Utc::now());
        let conn = self.get_conn()?;
        conn.execute(
            "INSERT INTO sessions (id, created_at, updated_at, player_name, current_scene_id, game_phase, game_state_json, completed)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![id, now, now, player_name, "intro", "calibrating", "{}", false],
        )?;
        Ok(())
    }

    pub fn create_session(&self, player_name: Option<&str>) -> Result<String> {
        let id = uuid::Uuid::new_v4().to_string();
        let now = format_timestamp(&Utc::now());
        let conn = self.get_conn()?;
        conn.execute(
            "INSERT INTO sessions (id, created_at, updated_at, player_name, current_scene_id, game_phase, game_state_json, completed)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![id, now, now, player_name, "intro", "calibrating", "{}", false],
        )?;
        Ok(id)
    }

    /// Retrieves a session by its ID.
    ///
    /// # Errors
    ///
    /// Returns [`FearEngineError::NotFound`] if no session exists with the given ID.
    ///
    /// # Example
    ///
    /// ```
    /// use fear_engine_storage::Database;
    ///
    /// let db = Database::new_in_memory().unwrap();
    /// let id = db.create_session(None).unwrap();
    /// let session = db.get_session(&id).unwrap();
    /// assert_eq!(session.id, id);
    /// ```
    pub fn get_session(&self, id: &str) -> Result<Session> {
        let conn = self.get_conn()?;
        conn.query_row(
            "SELECT id, created_at, updated_at, player_name, current_scene_id, game_phase, game_state_json, completed
             FROM sessions WHERE id = ?1",
            params![id],
            |row| {
                Ok(SessionRow {
                    id: row.get(0)?,
                    created_at: row.get(1)?,
                    updated_at: row.get(2)?,
                    player_name: row.get(3)?,
                    current_scene_id: row.get(4)?,
                    game_phase: row.get(5)?,
                    game_state_json: row.get(6)?,
                    completed: row.get(7)?,
                })
            },
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => FearEngineError::NotFound {
                entity: "Session".into(),
                id: id.into(),
            },
            other => FearEngineError::Database(other.to_string()),
        })
        .and_then(|row| row.into_session())
    }

    /// Advances a session to a new game phase.
    ///
    /// # Example
    ///
    /// ```
    /// use fear_engine_storage::Database;
    /// use fear_engine_common::types::GamePhase;
    ///
    /// let db = Database::new_in_memory().unwrap();
    /// let id = db.create_session(None).unwrap();
    /// db.update_session_phase(&id, GamePhase::Exploring).unwrap();
    /// let s = db.get_session(&id).unwrap();
    /// assert_eq!(s.game_phase, GamePhase::Exploring);
    /// ```
    pub fn update_session_phase(&self, id: &str, phase: GamePhase) -> Result<()> {
        let now = format_timestamp(&Utc::now());
        let conn = self.get_conn()?;
        let rows = conn.execute(
            "UPDATE sessions SET game_phase = ?1, updated_at = ?2 WHERE id = ?3",
            params![game_phase_to_str(&phase), now, id],
        )?;
        if rows == 0 {
            return Err(FearEngineError::NotFound {
                entity: "Session".into(),
                id: id.into(),
            });
        }
        Ok(())
    }

    /// Updates the current scene and arbitrary game-state JSON blob.
    ///
    /// # Example
    ///
    /// ```
    /// use fear_engine_storage::Database;
    ///
    /// let db = Database::new_in_memory().unwrap();
    /// let id = db.create_session(None).unwrap();
    /// db.update_session_state(&id, "hallway_02", r#"{"key":"val"}"#).unwrap();
    /// let s = db.get_session(&id).unwrap();
    /// assert_eq!(s.current_scene_id, "hallway_02");
    /// ```
    pub fn update_session_state(&self, id: &str, scene_id: &str, state_json: &str) -> Result<()> {
        let now = format_timestamp(&Utc::now());
        let conn = self.get_conn()?;
        let rows = conn.execute(
            "UPDATE sessions SET current_scene_id = ?1, game_state_json = ?2, updated_at = ?3 WHERE id = ?4",
            params![scene_id, state_json, now, id],
        )?;
        if rows == 0 {
            return Err(FearEngineError::NotFound {
                entity: "Session".into(),
                id: id.into(),
            });
        }
        Ok(())
    }

    pub fn update_session_player_name(&self, id: &str, player_name: Option<&str>) -> Result<()> {
        let now = format_timestamp(&Utc::now());
        let conn = self.get_conn()?;
        let rows = conn.execute(
            "UPDATE sessions SET player_name = ?1, updated_at = ?2 WHERE id = ?3",
            params![player_name, now, id],
        )?;
        if rows == 0 {
            return Err(FearEngineError::NotFound {
                entity: "Session".into(),
                id: id.into(),
            });
        }
        Ok(())
    }

    /// Marks a session as completed.
    ///
    /// # Example
    ///
    /// ```
    /// use fear_engine_storage::Database;
    ///
    /// let db = Database::new_in_memory().unwrap();
    /// let id = db.create_session(None).unwrap();
    /// db.complete_session(&id).unwrap();
    /// let s = db.get_session(&id).unwrap();
    /// assert!(s.completed);
    /// ```
    pub fn complete_session(&self, id: &str) -> Result<()> {
        let now = format_timestamp(&Utc::now());
        let conn = self.get_conn()?;
        let rows = conn.execute(
            "UPDATE sessions SET completed = TRUE, updated_at = ?1 WHERE id = ?2",
            params![now, id],
        )?;
        if rows == 0 {
            return Err(FearEngineError::NotFound {
                entity: "Session".into(),
                id: id.into(),
            });
        }
        Ok(())
    }

    /// Returns all sessions that have not been marked as completed.
    ///
    /// # Example
    ///
    /// ```
    /// use fear_engine_storage::Database;
    ///
    /// let db = Database::new_in_memory().unwrap();
    /// db.create_session(Some("Alice")).unwrap();
    /// let active = db.list_active_sessions().unwrap();
    /// assert_eq!(active.len(), 1);
    /// ```
    pub fn list_active_sessions(&self) -> Result<Vec<Session>> {
        let conn = self.get_conn()?;
        let mut stmt = conn.prepare(
            "SELECT id, created_at, updated_at, player_name, current_scene_id, game_phase, game_state_json, completed
             FROM sessions WHERE completed = FALSE ORDER BY created_at DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(SessionRow {
                id: row.get(0)?,
                created_at: row.get(1)?,
                updated_at: row.get(2)?,
                player_name: row.get(3)?,
                current_scene_id: row.get(4)?,
                game_phase: row.get(5)?,
                game_state_json: row.get(6)?,
                completed: row.get(7)?,
            })
        })?;
        let mut sessions = Vec::new();
        for row in rows {
            sessions.push(row?.into_session()?);
        }
        Ok(sessions)
    }
}

// ---------------------------------------------------------------------------
// Internal raw-row type (strings before conversion)
// ---------------------------------------------------------------------------

struct SessionRow {
    id: String,
    created_at: String,
    updated_at: String,
    player_name: Option<String>,
    current_scene_id: String,
    game_phase: String,
    game_state_json: String,
    completed: bool,
}

impl SessionRow {
    fn into_session(self) -> Result<Session> {
        Ok(Session {
            id: self.id,
            created_at: parse_timestamp(&self.created_at)?,
            updated_at: parse_timestamp(&self.updated_at)?,
            player_name: self.player_name,
            current_scene_id: self.current_scene_id,
            game_phase: game_phase_from_str(&self.game_phase)?,
            game_state_json: self.game_state_json,
            completed: self.completed,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use std::sync::Arc;

    #[test]
    fn test_create_session_with_name() {
        let db = Database::new_in_memory().unwrap();
        let id = db.create_session(Some("Alice")).unwrap();
        assert_eq!(id.len(), 36);
        let s = db.get_session(&id).unwrap();
        assert_eq!(s.player_name.as_deref(), Some("Alice"));
        assert_eq!(s.game_phase, GamePhase::Calibrating);
        assert!(!s.completed);
    }

    #[test]
    fn test_create_session_without_name() {
        let db = Database::new_in_memory().unwrap();
        let id = db.create_session(None).unwrap();
        let s = db.get_session(&id).unwrap();
        assert!(s.player_name.is_none());
    }

    #[test]
    fn test_get_session_not_found() {
        let db = Database::new_in_memory().unwrap();
        let result = db.get_session("nonexistent-id");
        assert!(result.is_err());
        match result.unwrap_err() {
            FearEngineError::NotFound { entity, id } => {
                assert_eq!(entity, "Session");
                assert_eq!(id, "nonexistent-id");
            }
            other => panic!("expected NotFound, got {other:?}"),
        }
    }

    #[test]
    fn test_update_session_phase() {
        let db = Database::new_in_memory().unwrap();
        let id = db.create_session(None).unwrap();
        db.update_session_phase(&id, GamePhase::Exploring).unwrap();
        let s = db.get_session(&id).unwrap();
        assert_eq!(s.game_phase, GamePhase::Exploring);
    }

    #[test]
    fn test_update_session_phase_not_found() {
        let db = Database::new_in_memory().unwrap();
        let result = db.update_session_phase("bad-id", GamePhase::Climax);
        assert!(matches!(result, Err(FearEngineError::NotFound { .. })));
    }

    #[test]
    fn test_update_session_state() {
        let db = Database::new_in_memory().unwrap();
        let id = db.create_session(None).unwrap();
        db.update_session_state(&id, "hallway_02", r#"{"items":["key"]}"#)
            .unwrap();
        let s = db.get_session(&id).unwrap();
        assert_eq!(s.current_scene_id, "hallway_02");
        assert!(s.game_state_json.contains("key"));
    }

    #[test]
    fn test_update_session_player_name() {
        let db = Database::new_in_memory().unwrap();
        let id = db.create_session(None).unwrap();
        db.update_session_player_name(&id, Some("Morgan")).unwrap();
        let s = db.get_session(&id).unwrap();
        assert_eq!(s.player_name.as_deref(), Some("Morgan"));
    }

    #[test]
    fn test_complete_session() {
        let db = Database::new_in_memory().unwrap();
        let id = db.create_session(None).unwrap();
        assert!(!db.get_session(&id).unwrap().completed);
        db.complete_session(&id).unwrap();
        assert!(db.get_session(&id).unwrap().completed);
    }

    #[test]
    fn test_complete_session_not_found() {
        let db = Database::new_in_memory().unwrap();
        let result = db.complete_session("bad-id");
        assert!(matches!(result, Err(FearEngineError::NotFound { .. })));
    }

    #[test]
    fn test_list_active_sessions() {
        let db = Database::new_in_memory().unwrap();
        let id1 = db.create_session(Some("A")).unwrap();
        let _id2 = db.create_session(Some("B")).unwrap();
        db.complete_session(&id1).unwrap();

        let active = db.list_active_sessions().unwrap();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].player_name.as_deref(), Some("B"));
    }

    #[test]
    fn test_list_active_sessions_empty() {
        let db = Database::new_in_memory().unwrap();
        let active = db.list_active_sessions().unwrap();
        assert!(active.is_empty());
    }

    #[test]
    fn test_concurrent_session_access() {
        let db = Arc::new(Database::new_in_memory().unwrap());
        let mut handles = vec![];
        for i in 0..10 {
            let db = db.clone();
            handles.push(std::thread::spawn(move || {
                let name = format!("player_{i}");
                db.create_session(Some(&name)).unwrap()
            }));
        }
        let ids: Vec<String> = handles.into_iter().map(|h| h.join().unwrap()).collect();
        assert_eq!(ids.len(), 10);
        let unique: HashSet<_> = ids.iter().collect();
        assert_eq!(unique.len(), 10);
    }
}
