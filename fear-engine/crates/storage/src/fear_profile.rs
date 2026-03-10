//! Fear profile persistence — stores and retrieves the per-session
//! Bayesian fear scores and meta-pattern values.

use chrono::{DateTime, Utc};
use fear_engine_common::{FearEngineError, Result};
use rusqlite::params;
use serde::{Deserialize, Serialize};

use crate::{parse_timestamp, Database};

/// A row in the `fear_profiles` table.
///
/// All ten fear axis scores plus three meta-pattern values are stored as
/// `f64` in the range `[0.0, 1.0]`, with a default prior of `0.5`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FearProfileRow {
    pub session_id: String,
    pub claustrophobia: f64,
    pub isolation: f64,
    pub body_horror: f64,
    pub stalking: f64,
    pub loss_of_control: f64,
    pub uncanny_valley: f64,
    pub darkness: f64,
    pub sound_based: f64,
    pub doppelganger: f64,
    pub abandonment: f64,
    pub anxiety_threshold: f64,
    pub recovery_speed: f64,
    pub curiosity_vs_avoidance: f64,
    pub confidence_json: String,
    pub updated_at: DateTime<Utc>,
}

impl Default for FearProfileRow {
    fn default() -> Self {
        Self {
            session_id: String::new(),
            claustrophobia: 0.5,
            isolation: 0.5,
            body_horror: 0.5,
            stalking: 0.5,
            loss_of_control: 0.5,
            uncanny_valley: 0.5,
            darkness: 0.5,
            sound_based: 0.5,
            doppelganger: 0.5,
            abandonment: 0.5,
            anxiety_threshold: 0.5,
            recovery_speed: 0.5,
            curiosity_vs_avoidance: 0.5,
            confidence_json: "{}".into(),
            updated_at: Utc::now(),
        }
    }
}

impl Database {
    /// Inserts a new fear profile for an existing session with default priors (0.5).
    ///
    /// # Example
    ///
    /// ```
    /// use fear_engine_storage::Database;
    ///
    /// let db = Database::new_in_memory().unwrap();
    /// let id = db.create_session(None).unwrap();
    /// db.create_fear_profile(&id).unwrap();
    /// let fp = db.get_fear_profile(&id).unwrap();
    /// assert!((fp.claustrophobia - 0.5).abs() < f64::EPSILON);
    /// ```
    pub fn create_fear_profile(&self, session_id: &str) -> Result<()> {
        let conn = self.get_conn()?;
        conn.execute(
            "INSERT INTO fear_profiles (session_id) VALUES (?1)",
            params![session_id],
        )?;
        Ok(())
    }

    /// Retrieves the fear profile for a session.
    ///
    /// # Errors
    ///
    /// Returns [`FearEngineError::NotFound`] if no profile exists for the session.
    ///
    /// # Example
    ///
    /// ```
    /// use fear_engine_storage::Database;
    ///
    /// let db = Database::new_in_memory().unwrap();
    /// let id = db.create_session(None).unwrap();
    /// db.create_fear_profile(&id).unwrap();
    /// let fp = db.get_fear_profile(&id).unwrap();
    /// assert_eq!(fp.session_id, id);
    /// ```
    pub fn get_fear_profile(&self, session_id: &str) -> Result<FearProfileRow> {
        let conn = self.get_conn()?;
        conn.query_row(
            "SELECT session_id, claustrophobia, isolation, body_horror, stalking,
                    loss_of_control, uncanny_valley, darkness, sound_based,
                    doppelganger, abandonment, anxiety_threshold, recovery_speed,
                    curiosity_vs_avoidance, confidence_json, updated_at
             FROM fear_profiles WHERE session_id = ?1",
            params![session_id],
            |row| {
                let updated_at_str: String = row.get(15)?;
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, f64>(1)?,
                    row.get::<_, f64>(2)?,
                    row.get::<_, f64>(3)?,
                    row.get::<_, f64>(4)?,
                    row.get::<_, f64>(5)?,
                    row.get::<_, f64>(6)?,
                    row.get::<_, f64>(7)?,
                    row.get::<_, f64>(8)?,
                    row.get::<_, f64>(9)?,
                    row.get::<_, f64>(10)?,
                    row.get::<_, f64>(11)?,
                    row.get::<_, f64>(12)?,
                    row.get::<_, f64>(13)?,
                    row.get::<_, String>(14)?,
                    updated_at_str,
                ))
            },
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => FearEngineError::NotFound {
                entity: "FearProfile".into(),
                id: session_id.into(),
            },
            other => FearEngineError::Database(other.to_string()),
        })
        .and_then(|t| {
            Ok(FearProfileRow {
                session_id: t.0,
                claustrophobia: t.1,
                isolation: t.2,
                body_horror: t.3,
                stalking: t.4,
                loss_of_control: t.5,
                uncanny_valley: t.6,
                darkness: t.7,
                sound_based: t.8,
                doppelganger: t.9,
                abandonment: t.10,
                anxiety_threshold: t.11,
                recovery_speed: t.12,
                curiosity_vs_avoidance: t.13,
                confidence_json: t.14,
                updated_at: parse_timestamp(&t.15)?,
            })
        })
    }

    /// Persists updated fear scores and meta-pattern values.
    ///
    /// # Example
    ///
    /// ```
    /// use fear_engine_storage::Database;
    /// use fear_engine_storage::fear_profile::FearProfileRow;
    ///
    /// let db = Database::new_in_memory().unwrap();
    /// let id = db.create_session(None).unwrap();
    /// db.create_fear_profile(&id).unwrap();
    /// let mut fp = db.get_fear_profile(&id).unwrap();
    /// fp.darkness = 0.85;
    /// db.update_fear_profile(&id, &fp).unwrap();
    /// let updated = db.get_fear_profile(&id).unwrap();
    /// assert!((updated.darkness - 0.85).abs() < f64::EPSILON);
    /// ```
    pub fn update_fear_profile(&self, session_id: &str, profile: &FearProfileRow) -> Result<()> {
        let now = crate::format_timestamp(&Utc::now());
        let conn = self.get_conn()?;
        let rows = conn.execute(
            "UPDATE fear_profiles SET
                claustrophobia = ?1, isolation = ?2, body_horror = ?3,
                stalking = ?4, loss_of_control = ?5, uncanny_valley = ?6,
                darkness = ?7, sound_based = ?8, doppelganger = ?9,
                abandonment = ?10, anxiety_threshold = ?11, recovery_speed = ?12,
                curiosity_vs_avoidance = ?13, confidence_json = ?14, updated_at = ?15
             WHERE session_id = ?16",
            params![
                profile.claustrophobia,
                profile.isolation,
                profile.body_horror,
                profile.stalking,
                profile.loss_of_control,
                profile.uncanny_valley,
                profile.darkness,
                profile.sound_based,
                profile.doppelganger,
                profile.abandonment,
                profile.anxiety_threshold,
                profile.recovery_speed,
                profile.curiosity_vs_avoidance,
                profile.confidence_json,
                now,
                session_id,
            ],
        )?;
        if rows == 0 {
            return Err(FearEngineError::NotFound {
                entity: "FearProfile".into(),
                id: session_id.into(),
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_and_get_fear_profile() {
        let db = Database::new_in_memory().unwrap();
        let id = db.create_session(None).unwrap();
        db.create_fear_profile(&id).unwrap();
        let fp = db.get_fear_profile(&id).unwrap();
        assert_eq!(fp.session_id, id);
        assert!((fp.claustrophobia - 0.5).abs() < f64::EPSILON);
        assert!((fp.darkness - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_get_fear_profile_not_found() {
        let db = Database::new_in_memory().unwrap();
        let result = db.get_fear_profile("no-such-session");
        assert!(matches!(result, Err(FearEngineError::NotFound { .. })));
    }

    #[test]
    fn test_update_fear_profile() {
        let db = Database::new_in_memory().unwrap();
        let id = db.create_session(None).unwrap();
        db.create_fear_profile(&id).unwrap();

        let mut fp = db.get_fear_profile(&id).unwrap();
        fp.darkness = 0.9;
        fp.isolation = 0.3;
        fp.anxiety_threshold = 0.7;
        fp.confidence_json = r#"{"darkness":0.8}"#.into();
        db.update_fear_profile(&id, &fp).unwrap();

        let updated = db.get_fear_profile(&id).unwrap();
        assert!((updated.darkness - 0.9).abs() < f64::EPSILON);
        assert!((updated.isolation - 0.3).abs() < f64::EPSILON);
        assert!((updated.anxiety_threshold - 0.7).abs() < f64::EPSILON);
        assert!(updated.confidence_json.contains("darkness"));
    }

    #[test]
    fn test_update_fear_profile_not_found() {
        let db = Database::new_in_memory().unwrap();
        let profile = FearProfileRow::default();
        let result = db.update_fear_profile("bad-id", &profile);
        assert!(matches!(result, Err(FearEngineError::NotFound { .. })));
    }

    #[test]
    fn test_duplicate_fear_profile_fails() {
        let db = Database::new_in_memory().unwrap();
        let id = db.create_session(None).unwrap();
        db.create_fear_profile(&id).unwrap();
        let result = db.create_fear_profile(&id);
        assert!(result.is_err());
    }

    #[test]
    fn test_fear_profile_all_scores_persisted() {
        let db = Database::new_in_memory().unwrap();
        let id = db.create_session(None).unwrap();
        db.create_fear_profile(&id).unwrap();

        let mut fp = db.get_fear_profile(&id).unwrap();
        fp.claustrophobia = 0.1;
        fp.isolation = 0.2;
        fp.body_horror = 0.3;
        fp.stalking = 0.4;
        fp.loss_of_control = 0.5;
        fp.uncanny_valley = 0.6;
        fp.darkness = 0.7;
        fp.sound_based = 0.8;
        fp.doppelganger = 0.9;
        fp.abandonment = 1.0;
        fp.recovery_speed = 0.15;
        fp.curiosity_vs_avoidance = 0.25;
        db.update_fear_profile(&id, &fp).unwrap();

        let u = db.get_fear_profile(&id).unwrap();
        assert!((u.claustrophobia - 0.1).abs() < f64::EPSILON);
        assert!((u.isolation - 0.2).abs() < f64::EPSILON);
        assert!((u.body_horror - 0.3).abs() < f64::EPSILON);
        assert!((u.stalking - 0.4).abs() < f64::EPSILON);
        assert!((u.loss_of_control - 0.5).abs() < f64::EPSILON);
        assert!((u.uncanny_valley - 0.6).abs() < f64::EPSILON);
        assert!((u.darkness - 0.7).abs() < f64::EPSILON);
        assert!((u.sound_based - 0.8).abs() < f64::EPSILON);
        assert!((u.doppelganger - 0.9).abs() < f64::EPSILON);
        assert!((u.abandonment - 1.0).abs() < f64::EPSILON);
        assert!((u.recovery_speed - 0.15).abs() < f64::EPSILON);
        assert!((u.curiosity_vs_avoidance - 0.25).abs() < f64::EPSILON);
    }
}
