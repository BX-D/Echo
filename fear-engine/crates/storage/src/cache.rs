//! Content cache — stores generated narrative and image responses with a
//! time-to-live so duplicate prompts can be served from cache.

use chrono::{DateTime, Utc};
use fear_engine_common::Result;
use rusqlite::params;
use serde::{Deserialize, Serialize};

use crate::{parse_timestamp, Database};

/// TTL-aware SQL predicate: entry is live when created_at + ttl > now (all as integers).
const TTL_LIVE: &str =
    "(CAST(strftime('%s', created_at) AS INTEGER) + ttl_seconds) > CAST(strftime('%s', 'now') AS INTEGER)";

/// TTL-aware SQL predicate: entry is expired (complement of [`TTL_LIVE`]).
const TTL_EXPIRED: &str =
    "(CAST(strftime('%s', created_at) AS INTEGER) + ttl_seconds) <= CAST(strftime('%s', 'now') AS INTEGER)";

/// A row in the `content_cache` table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    pub cache_key: String,
    pub content_type: String,
    pub content_json: String,
    pub created_at: DateTime<Utc>,
    pub ttl_seconds: u32,
}

impl Database {
    /// Inserts or replaces a cache entry.
    ///
    /// # Example
    ///
    /// ```
    /// use fear_engine_storage::Database;
    ///
    /// let db = Database::new_in_memory().unwrap();
    /// db.cache_set("prompt_abc", "narrative", r#"{"text":"hello"}"#, 3600).unwrap();
    /// let entry = db.cache_get("prompt_abc").unwrap().unwrap();
    /// assert_eq!(entry.content_type, "narrative");
    /// ```
    pub fn cache_set(
        &self,
        key: &str,
        content_type: &str,
        content_json: &str,
        ttl: u32,
    ) -> Result<()> {
        let conn = self.get_conn()?;
        conn.execute(
            "INSERT OR REPLACE INTO content_cache (cache_key, content_type, content_json, created_at, ttl_seconds)
             VALUES (?1, ?2, ?3, CURRENT_TIMESTAMP, ?4)",
            params![key, content_type, content_json, ttl],
        )?;
        Ok(())
    }

    /// Retrieves a cache entry if it exists **and** has not expired.
    ///
    /// Returns `Ok(None)` for missing or expired entries.
    ///
    /// # Example
    ///
    /// ```
    /// use fear_engine_storage::Database;
    ///
    /// let db = Database::new_in_memory().unwrap();
    /// assert!(db.cache_get("missing").unwrap().is_none());
    /// ```
    pub fn cache_get(&self, key: &str) -> Result<Option<CacheEntry>> {
        let conn = self.get_conn()?;
        let sql = format!(
            "SELECT cache_key, content_type, content_json, created_at, ttl_seconds
             FROM content_cache
             WHERE cache_key = ?1 AND {TTL_LIVE}"
        );
        let result = conn.query_row(&sql, params![key], |row| {
            Ok(CacheRow {
                cache_key: row.get(0)?,
                content_type: row.get(1)?,
                content_json: row.get(2)?,
                created_at: row.get(3)?,
                ttl_seconds: row.get(4)?,
            })
        });
        match result {
            Ok(row) => Ok(Some(CacheEntry {
                cache_key: row.cache_key,
                content_type: row.content_type,
                content_json: row.content_json,
                created_at: parse_timestamp(&row.created_at)?,
                ttl_seconds: row.ttl_seconds,
            })),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Deletes all expired cache entries, returning how many were removed.
    ///
    /// # Example
    ///
    /// ```
    /// use fear_engine_storage::Database;
    ///
    /// let db = Database::new_in_memory().unwrap();
    /// let removed = db.cache_cleanup_expired().unwrap();
    /// assert_eq!(removed, 0);
    /// ```
    pub fn cache_cleanup_expired(&self) -> Result<u64> {
        let conn = self.get_conn()?;
        let sql = format!("DELETE FROM content_cache WHERE {TTL_EXPIRED}");
        let deleted = conn.execute(&sql, [])?;
        Ok(deleted as u64)
    }
}

struct CacheRow {
    cache_key: String,
    content_type: String,
    content_json: String,
    created_at: String,
    ttl_seconds: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_set_and_get() {
        let db = Database::new_in_memory().unwrap();
        db.cache_set("k1", "narrative", r#"{"t":"hi"}"#, 3600)
            .unwrap();
        let entry = db.cache_get("k1").unwrap().unwrap();
        assert_eq!(entry.cache_key, "k1");
        assert_eq!(entry.content_type, "narrative");
        assert!(entry.content_json.contains("hi"));
    }

    #[test]
    fn test_cache_get_missing_key() {
        let db = Database::new_in_memory().unwrap();
        assert!(db.cache_get("nope").unwrap().is_none());
    }

    #[test]
    fn test_cache_set_overwrites() {
        let db = Database::new_in_memory().unwrap();
        db.cache_set("k1", "narrative", "v1", 3600).unwrap();
        db.cache_set("k1", "image", "v2", 7200).unwrap();
        let entry = db.cache_get("k1").unwrap().unwrap();
        assert_eq!(entry.content_type, "image");
        assert_eq!(entry.content_json, "v2");
    }

    #[test]
    fn test_content_cache_ttl_expiry() {
        let db = Database::new_in_memory().unwrap();
        // Insert an entry whose created_at is 10 seconds in the past with a 5s TTL
        // so it's already expired.
        {
            let conn = db.get_conn().unwrap();
            conn.execute(
                "INSERT INTO content_cache (cache_key, content_type, content_json, created_at, ttl_seconds)
                 VALUES ('expired_key', 'narrative', '{}', datetime('now', '-10 seconds'), 5)",
                [],
            )
            .unwrap();
        } // conn returned to pool

        assert!(db.cache_get("expired_key").unwrap().is_none());
    }

    #[test]
    fn test_cache_cleanup_expired() {
        let db = Database::new_in_memory().unwrap();
        // Insert two expired entries via raw SQL (past timestamps).
        {
            let conn = db.get_conn().unwrap();
            conn.execute(
                "INSERT INTO content_cache (cache_key, content_type, content_json, created_at, ttl_seconds)
                 VALUES ('old1', 'narrative', '{}', datetime('now', '-100 seconds'), 1)",
                [],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO content_cache (cache_key, content_type, content_json, created_at, ttl_seconds)
                 VALUES ('old2', 'image', '{}', datetime('now', '-100 seconds'), 1)",
                [],
            )
            .unwrap();
        } // conn returned to pool

        // Insert one live entry.
        db.cache_set("live", "narrative", "{}", 9999).unwrap();

        let removed = db.cache_cleanup_expired().unwrap();
        assert_eq!(removed, 2);

        // Live entry still accessible.
        assert!(db.cache_get("live").unwrap().is_some());
    }

    #[test]
    fn test_cache_cleanup_when_nothing_expired() {
        let db = Database::new_in_memory().unwrap();
        db.cache_set("fresh", "narrative", "{}", 99999).unwrap();
        let removed = db.cache_cleanup_expired().unwrap();
        assert_eq!(removed, 0);
    }
}
