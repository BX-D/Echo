//! # Fear Engine — Storage
//!
//! SQLite persistence layer for the Fear Engine. Provides a connection-pooled
//! [`Database`] handle with methods for managing game sessions, fear profiles,
//! behavior event logs, content cache, and scene history.
//!
//! All public operations go through the [`Database`] struct, which wraps an
//! `r2d2` connection pool backed by `rusqlite`.

pub mod behavior_log;
pub mod cache;
pub mod fear_profile;
pub mod scene_history;
pub mod session;

use fear_engine_common::{FearEngineError, Result};
use r2d2_sqlite::SqliteConnectionManager;

/// Type alias for a pooled SQLite connection.
pub(crate) type PooledConnection = r2d2::PooledConnection<SqliteConnectionManager>;

/// SQL migration applied at startup.
const MIGRATION_001: &str = include_str!("migrations/001_initial.sql");

/// Configures every new connection with required PRAGMAs.
#[derive(Debug)]
struct ConnectionCustomizer;

impl r2d2::CustomizeConnection<rusqlite::Connection, rusqlite::Error> for ConnectionCustomizer {
    fn on_acquire(
        &self,
        conn: &mut rusqlite::Connection,
    ) -> std::result::Result<(), rusqlite::Error> {
        conn.execute_batch("PRAGMA foreign_keys = ON; PRAGMA journal_mode = WAL;")?;
        Ok(())
    }
}

/// Connection-pooled handle to the Fear Engine SQLite database.
///
/// # Example
///
/// ```
/// use fear_engine_storage::Database;
///
/// let db = Database::new_in_memory().unwrap();
/// // Database is ready — migrations have been applied.
/// ```
#[derive(Clone)]
pub struct Database {
    pool: r2d2::Pool<SqliteConnectionManager>,
}

impl Database {
    /// Opens (or creates) a file-backed database at the given URL and runs migrations.
    ///
    /// The URL may optionally start with `sqlite://`; the prefix is stripped.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use fear_engine_storage::Database;
    /// let db = Database::new("sqlite://fear_engine.db").unwrap();
    /// ```
    pub fn new(database_url: &str) -> Result<Self> {
        let path = database_url
            .strip_prefix("sqlite://")
            .unwrap_or(database_url);
        let manager = SqliteConnectionManager::file(path);
        let pool = r2d2::Pool::builder()
            .connection_customizer(Box::new(ConnectionCustomizer))
            .build(manager)
            .map_err(|e| FearEngineError::Database(e.to_string()))?;
        let db = Self { pool };
        db.initialize()?;
        Ok(db)
    }

    /// Creates a new in-memory database for testing.
    ///
    /// Uses a pool size of 1 so every checkout returns the same underlying
    /// SQLite connection (and therefore the same in-memory database).
    ///
    /// # Example
    ///
    /// ```
    /// use fear_engine_storage::Database;
    /// let db = Database::new_in_memory().unwrap();
    /// ```
    pub fn new_in_memory() -> Result<Self> {
        let manager = SqliteConnectionManager::memory();
        let pool = r2d2::Pool::builder()
            .max_size(1)
            .connection_customizer(Box::new(ConnectionCustomizer))
            .build(manager)
            .map_err(|e| FearEngineError::Database(e.to_string()))?;
        let db = Self { pool };
        db.initialize()?;
        Ok(db)
    }

    /// Runs all pending migrations against the database.
    ///
    /// Currently applies [`MIGRATION_001`] (the initial schema). Safe to call
    /// multiple times thanks to `IF NOT EXISTS` guards in the SQL.
    ///
    /// # Example
    ///
    /// ```
    /// use fear_engine_storage::Database;
    /// let db = Database::new_in_memory().unwrap();
    /// db.initialize().unwrap(); // idempotent
    /// ```
    pub fn initialize(&self) -> Result<()> {
        let conn = self.get_conn()?;
        conn.execute_batch(MIGRATION_001)?;
        Ok(())
    }

    /// Checks out a connection from the pool.
    pub(crate) fn get_conn(&self) -> Result<PooledConnection> {
        self.pool
            .get()
            .map_err(|e| FearEngineError::Database(e.to_string()))
    }
}

// ---------------------------------------------------------------------------
// Timestamp helpers (shared by all sub-modules)
// ---------------------------------------------------------------------------

/// Formats a `DateTime<Utc>` into the string format SQLite's `CURRENT_TIMESTAMP` produces.
pub(crate) fn format_timestamp(dt: &chrono::DateTime<chrono::Utc>) -> String {
    dt.format("%Y-%m-%d %H:%M:%S").to_string()
}

/// Parses a SQLite timestamp string (or RFC 3339) into `DateTime<Utc>`.
pub(crate) fn parse_timestamp(s: &str) -> Result<chrono::DateTime<chrono::Utc>> {
    chrono::DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .or_else(|_| {
            chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S").map(|ndt| ndt.and_utc())
        })
        .map_err(|e| FearEngineError::Serialization(format!("invalid timestamp '{s}': {e}")))
}

/// Converts a [`GamePhase`] to its database TEXT representation.
pub(crate) fn game_phase_to_str(phase: &fear_engine_common::types::GamePhase) -> &'static str {
    use fear_engine_common::types::GamePhase;
    match phase {
        GamePhase::Calibrating => "calibrating",
        GamePhase::Exploring => "exploring",
        GamePhase::Escalating => "escalating",
        GamePhase::Climax => "climax",
        GamePhase::Reveal => "reveal",
    }
}

/// Parses a database TEXT value back into a [`GamePhase`].
pub(crate) fn game_phase_from_str(s: &str) -> Result<fear_engine_common::types::GamePhase> {
    use fear_engine_common::types::GamePhase;
    match s {
        "calibrating" => Ok(GamePhase::Calibrating),
        "exploring" => Ok(GamePhase::Exploring),
        "escalating" => Ok(GamePhase::Escalating),
        "climax" => Ok(GamePhase::Climax),
        "reveal" => Ok(GamePhase::Reveal),
        _ => Err(FearEngineError::Serialization(format!(
            "unknown game phase: '{s}'"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn smoke_test_database_creation() {
        let db = Database::new_in_memory().unwrap();
        // Verify we can get a connection
        let _conn = db.get_conn().unwrap();
    }

    #[test]
    fn test_initialize_is_idempotent() {
        let db = Database::new_in_memory().unwrap();
        db.initialize().unwrap();
        db.initialize().unwrap();
    }

    #[test]
    fn test_format_parse_timestamp_roundtrip() {
        let now = Utc::now();
        let formatted = format_timestamp(&now);
        let parsed = parse_timestamp(&formatted).unwrap();
        // Precision limited to seconds
        assert_eq!(
            now.format("%Y-%m-%d %H:%M:%S").to_string(),
            parsed.format("%Y-%m-%d %H:%M:%S").to_string()
        );
    }

    #[test]
    fn test_parse_timestamp_rfc3339() {
        let dt = parse_timestamp("2026-01-15T10:30:00Z").unwrap();
        assert_eq!(dt.format("%Y-%m-%d").to_string(), "2026-01-15");
    }

    #[test]
    fn test_parse_timestamp_invalid() {
        assert!(parse_timestamp("not-a-date").is_err());
    }

    #[test]
    fn test_game_phase_str_roundtrip() {
        use fear_engine_common::types::GamePhase;
        let phases = [
            GamePhase::Calibrating,
            GamePhase::Exploring,
            GamePhase::Escalating,
            GamePhase::Climax,
            GamePhase::Reveal,
        ];
        for phase in &phases {
            let s = game_phase_to_str(phase);
            let back = game_phase_from_str(s).unwrap();
            assert_eq!(phase, &back);
        }
    }

    #[test]
    fn test_game_phase_from_str_invalid() {
        assert!(game_phase_from_str("unknown").is_err());
    }
}
