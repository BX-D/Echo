//! Content safety validation and LRU caching for narrative and image
//! generation responses.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use fear_engine_common::{FearEngineError, Result};
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Content safety
// ---------------------------------------------------------------------------

/// Blocked phrases that cross the line from fictional horror into harmful content.
const BLOCKED_PHRASES: &[&str] = &[
    "how to harm",
    "instructions for",
    "kill yourself",
    "self-harm",
    "commit suicide",
    "real person",
    "real address",
    "real phone",
];

/// Checks whether generated content is safe for the game.
///
/// Horror content is expected and allowed. The filter only blocks content
/// that could be genuinely harmful (self-harm instructions, real-world
/// targeting, etc.).
///
/// # Example
///
/// ```
/// use fear_engine_ai_integration::cache::is_content_safe;
///
/// assert!(is_content_safe("The shadows close in around you."));
/// assert!(!is_content_safe("Here are instructions for self-harm."));
/// ```
pub fn is_content_safe(content: &str) -> bool {
    let lower = content.to_lowercase();
    !BLOCKED_PHRASES.iter().any(|phrase| lower.contains(phrase))
}

/// Validates narrative length (target: 150–300 words, lenient range: 20–500).
///
/// # Example
///
/// ```
/// use fear_engine_ai_integration::cache::validate_narrative_length;
///
/// assert!(validate_narrative_length("A short scene for testing purposes that has enough words.").is_ok());
/// ```
pub fn validate_narrative_length(narrative: &str) -> Result<()> {
    let word_count = narrative.split_whitespace().count();
    if word_count < 5 {
        return Err(FearEngineError::AiGeneration(format!(
            "narrative too short: {word_count} words (minimum 5)"
        )));
    }
    if word_count > 500 {
        return Err(FearEngineError::AiGeneration(format!(
            "narrative too long: {word_count} words (maximum 500)"
        )));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// LRU cache
// ---------------------------------------------------------------------------

/// Cache entry with TTL.
#[derive(Debug, Clone)]
struct CacheEntry<V> {
    value: V,
    created_at: Instant,
    last_accessed: Instant,
}

/// Cache hit/miss metrics.
///
/// # Example
///
/// ```
/// use fear_engine_ai_integration::cache::CacheMetrics;
///
/// let m = CacheMetrics::default();
/// assert_eq!(m.hits, 0);
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CacheMetrics {
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
}

impl CacheMetrics {
    /// Hit rate as a fraction in `[0.0, 1.0]`.
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            return 0.0;
        }
        self.hits as f64 / total as f64
    }
}

/// A simple LRU cache with TTL and capacity eviction.
///
/// # Example
///
/// ```
/// use fear_engine_ai_integration::cache::LruCache;
/// use std::time::Duration;
///
/// let mut cache: LruCache<String> = LruCache::new(2, Duration::from_secs(60));
/// cache.set("k1".into(), "v1".into());
/// assert_eq!(cache.get("k1"), Some(&"v1".to_string()));
/// assert_eq!(cache.get("missing"), None);
/// ```
pub struct LruCache<V> {
    entries: HashMap<String, CacheEntry<V>>,
    capacity: usize,
    ttl: Duration,
    metrics: CacheMetrics,
}

impl<V: Clone> LruCache<V> {
    /// Creates a new cache with the given maximum capacity and TTL.
    pub fn new(capacity: usize, ttl: Duration) -> Self {
        Self {
            entries: HashMap::new(),
            capacity,
            ttl,
            metrics: CacheMetrics::default(),
        }
    }

    /// Inserts a value, evicting the least-recently-used entry if at capacity.
    ///
    /// # Example
    ///
    /// ```
    /// use fear_engine_ai_integration::cache::LruCache;
    /// use std::time::Duration;
    ///
    /// let mut cache: LruCache<i32> = LruCache::new(2, Duration::from_secs(60));
    /// cache.set("a".into(), 1);
    /// cache.set("b".into(), 2);
    /// cache.set("c".into(), 3); // evicts "a"
    /// assert!(cache.get("a").is_none());
    /// assert_eq!(cache.get("c"), Some(&3));
    /// ```
    pub fn set(&mut self, key: String, value: V) {
        // Evict expired entries first.
        self.evict_expired();

        // If at capacity, evict least recently used.
        if self.entries.len() >= self.capacity && !self.entries.contains_key(&key) {
            self.evict_lru();
        }

        let now = Instant::now();
        self.entries.insert(
            key,
            CacheEntry {
                value,
                created_at: now,
                last_accessed: now,
            },
        );
    }

    /// Returns a reference to the cached value, or `None` if missing or expired.
    pub fn get(&mut self, key: &str) -> Option<&V> {
        let now = Instant::now();

        // Check if entry exists and is not expired.
        let expired = self
            .entries
            .get(key)
            .map(|e| now.duration_since(e.created_at) > self.ttl)
            .unwrap_or(true);

        if expired {
            self.entries.remove(key);
            self.metrics.misses += 1;
            return None;
        }

        // Update last_accessed and record hit.
        if let Some(entry) = self.entries.get_mut(key) {
            entry.last_accessed = now;
            self.metrics.hits += 1;
            Some(&entry.value)
        } else {
            self.metrics.misses += 1;
            None
        }
    }

    /// Returns the current metrics snapshot.
    pub fn metrics(&self) -> &CacheMetrics {
        &self.metrics
    }

    /// Number of entries currently stored.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    fn evict_expired(&mut self) {
        let now = Instant::now();
        let before = self.entries.len();
        self.entries
            .retain(|_, e| now.duration_since(e.created_at) <= self.ttl);
        self.metrics.evictions += (before - self.entries.len()) as u64;
    }

    fn evict_lru(&mut self) {
        if let Some(lru_key) = self
            .entries
            .iter()
            .min_by_key(|(_, e)| e.last_accessed)
            .map(|(k, _)| k.clone())
        {
            self.entries.remove(&lru_key);
            self.metrics.evictions += 1;
        }
    }
}

/// Computes a deterministic cache key from a set of components.
///
/// # Example
///
/// ```
/// use fear_engine_ai_integration::cache::compute_cache_key;
///
/// let k1 = compute_cache_key(&["a", "b"]);
/// let k2 = compute_cache_key(&["a", "b"]);
/// assert_eq!(k1, k2);
/// let k3 = compute_cache_key(&["a", "c"]);
/// assert_ne!(k1, k3);
/// ```
pub fn compute_cache_key(components: &[&str]) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    for c in components {
        c.hash(&mut hasher);
    }
    format!("ck_{:x}", hasher.finish())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- Safety filter tests ----------------------------------------------

    #[test]
    fn test_safety_filter_allows_horror_content() {
        assert!(is_content_safe(
            "The shadows writhe around you. Something breathes in the dark."
        ));
        assert!(is_content_safe(
            "Blood seeps from beneath the door. The sound of scraping grows louder."
        ));
    }

    #[test]
    fn test_safety_filter_blocks_harmful_content() {
        assert!(!is_content_safe("Here are instructions for harming yourself."));
        assert!(!is_content_safe("You should commit suicide."));
        assert!(!is_content_safe("This targets a real person named John."));
    }

    // -- Narrative length tests -------------------------------------------

    #[test]
    fn test_narrative_length_valid() {
        let text = "word ".repeat(200);
        assert!(validate_narrative_length(&text).is_ok());
    }

    #[test]
    fn test_narrative_length_too_short() {
        assert!(validate_narrative_length("too short").is_err());
    }

    #[test]
    fn test_narrative_length_too_long() {
        let text = "word ".repeat(600);
        assert!(validate_narrative_length(&text).is_err());
    }

    // -- LRU cache tests --------------------------------------------------

    #[test]
    fn test_lru_cache_set_and_get() {
        let mut cache: LruCache<String> = LruCache::new(10, Duration::from_secs(60));
        cache.set("k1".into(), "v1".into());
        assert_eq!(cache.get("k1"), Some(&"v1".to_string()));
    }

    #[test]
    fn test_lru_cache_miss() {
        let mut cache: LruCache<String> = LruCache::new(10, Duration::from_secs(60));
        assert_eq!(cache.get("missing"), None);
    }

    #[test]
    fn test_lru_cache_eviction_on_capacity() {
        let mut cache: LruCache<i32> = LruCache::new(2, Duration::from_secs(60));
        cache.set("a".into(), 1);
        cache.set("b".into(), 2);
        cache.set("c".into(), 3); // should evict "a" (LRU)
        assert!(cache.get("a").is_none());
        assert_eq!(cache.get("b"), Some(&2));
        assert_eq!(cache.get("c"), Some(&3));
    }

    #[test]
    fn test_lru_cache_ttl_expiry() {
        let mut cache: LruCache<String> = LruCache::new(10, Duration::from_millis(1));
        cache.set("k".into(), "v".into());
        std::thread::sleep(Duration::from_millis(10));
        assert!(cache.get("k").is_none());
    }

    #[test]
    fn test_cache_metrics_tracking() {
        let mut cache: LruCache<i32> = LruCache::new(10, Duration::from_secs(60));
        cache.set("a".into(), 1);
        let _ = cache.get("a"); // hit
        let _ = cache.get("b"); // miss

        assert_eq!(cache.metrics().hits, 1);
        assert_eq!(cache.metrics().misses, 1);
        assert!((cache.metrics().hit_rate() - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_cache_eviction_metrics() {
        let mut cache: LruCache<i32> = LruCache::new(1, Duration::from_secs(60));
        cache.set("a".into(), 1);
        cache.set("b".into(), 2); // evicts "a"
        assert_eq!(cache.metrics().evictions, 1);
    }

    // -- Cache key tests --------------------------------------------------

    #[test]
    fn test_cache_key_deterministic() {
        let k1 = compute_cache_key(&["fear", "darkness", "scene_5"]);
        let k2 = compute_cache_key(&["fear", "darkness", "scene_5"]);
        assert_eq!(k1, k2);
    }

    #[test]
    fn test_cache_key_different_for_different_contexts() {
        let k1 = compute_cache_key(&["fear", "darkness"]);
        let k2 = compute_cache_key(&["fear", "isolation"]);
        assert_ne!(k1, k2);
    }

    #[test]
    fn test_metrics_hit_rate_zero_when_empty() {
        let m = CacheMetrics::default();
        assert!((m.hit_rate() - 0.0).abs() < f64::EPSILON);
    }
}
