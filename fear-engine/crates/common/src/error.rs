//! Unified error types for Echo Protocol runtime crates.
//!
//! Provides [`FearEngineError`], the single error enum used across all crates, and a
//! convenience [`Result`] type alias. Automatic conversions from third-party error types
//! (rusqlite, reqwest, serde_json) are gated behind optional features so downstream
//! crates only pay for what they use.

use thiserror::Error;

/// The unified error type for all Echo Protocol runtime operations.
///
/// Each variant captures a distinct failure domain so callers can match on the
/// category without inspecting error strings.
///
/// # Example
///
/// ```
/// use fear_engine_common::error::FearEngineError;
///
/// let err = FearEngineError::Database("connection refused".into());
/// assert_eq!(err.to_string(), "Database error: connection refused");
/// ```
#[derive(Debug, Error)]
pub enum FearEngineError {
    /// SQLite / persistence layer errors.
    #[error("Database error: {0}")]
    Database(String),

    /// WebSocket connection or message framing errors.
    #[error("WebSocket error: {0}")]
    WebSocket(String),

    /// JSON serialization / deserialization errors.
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// LLM (Claude) API errors.
    #[error("AI generation error: {0}")]
    AiGeneration(String),

    /// Image generation API errors.
    #[error("Image generation error: {0}")]
    ImageGeneration(String),

    /// Attempted an illegal state-machine transition.
    #[error("Invalid state transition: cannot move from '{current}' to '{attempted}'")]
    InvalidState {
        /// The state the machine is currently in.
        current: String,
        /// The state the caller tried to transition to.
        attempted: String,
    },

    /// A user-supplied or internal value failed validation.
    #[error("Invalid input for field '{field}': {reason}")]
    InvalidInput {
        /// Which field or parameter was invalid.
        field: String,
        /// Human-readable explanation.
        reason: String,
    },

    /// A requested entity was not found in storage.
    #[error("{entity} not found with id '{id}'")]
    NotFound {
        /// The kind of entity (e.g. "Session", "FearProfile").
        entity: String,
        /// The identifier that was looked up.
        id: String,
    },

    /// An upstream API returned a rate-limit response.
    #[error("Rate limited: retry after {retry_after_ms}ms")]
    RateLimit {
        /// Suggested wait time before retrying, in milliseconds.
        retry_after_ms: u64,
    },

    /// An operation exceeded its deadline.
    #[error("Operation '{operation}' timed out after {duration_ms}ms")]
    Timeout {
        /// A short label for the operation (e.g. "narrative_generation").
        operation: String,
        /// How long we waited, in milliseconds.
        duration_ms: u64,
    },

    /// Missing or invalid application configuration.
    #[error("Configuration error: {0}")]
    Configuration(String),
}

/// Convenience alias used throughout the Echo Protocol runtime crates.
///
/// # Example
///
/// ```
/// use fear_engine_common::error::Result;
///
/// fn example() -> Result<i32> {
///     Ok(42)
/// }
/// assert_eq!(example().unwrap(), 42);
/// ```
pub type Result<T> = std::result::Result<T, FearEngineError>;

// ---------------------------------------------------------------------------
// From conversions
// ---------------------------------------------------------------------------

impl From<serde_json::Error> for FearEngineError {
    fn from(err: serde_json::Error) -> Self {
        FearEngineError::Serialization(err.to_string())
    }
}

#[cfg(feature = "rusqlite")]
impl From<rusqlite::Error> for FearEngineError {
    fn from(err: rusqlite::Error) -> Self {
        FearEngineError::Database(err.to_string())
    }
}

#[cfg(feature = "reqwest")]
impl From<reqwest::Error> for FearEngineError {
    fn from(err: reqwest::Error) -> Self {
        FearEngineError::AiGeneration(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_database_error_display() {
        let err = FearEngineError::Database("connection refused".into());
        assert_eq!(err.to_string(), "Database error: connection refused");
    }

    #[test]
    fn test_websocket_error_display() {
        let err = FearEngineError::WebSocket("frame too large".into());
        assert_eq!(err.to_string(), "WebSocket error: frame too large");
    }

    #[test]
    fn test_serialization_error_display() {
        let err = FearEngineError::Serialization("unexpected token".into());
        assert_eq!(err.to_string(), "Serialization error: unexpected token");
    }

    #[test]
    fn test_ai_generation_error_display() {
        let err = FearEngineError::AiGeneration("model overloaded".into());
        assert_eq!(err.to_string(), "AI generation error: model overloaded");
    }

    #[test]
    fn test_image_generation_error_display() {
        let err = FearEngineError::ImageGeneration("NSFW filter triggered".into());
        assert_eq!(
            err.to_string(),
            "Image generation error: NSFW filter triggered"
        );
    }

    #[test]
    fn test_invalid_state_error_display() {
        let err = FearEngineError::InvalidState {
            current: "Calibrating".into(),
            attempted: "Reveal".into(),
        };
        assert_eq!(
            err.to_string(),
            "Invalid state transition: cannot move from 'Calibrating' to 'Reveal'"
        );
    }

    #[test]
    fn test_invalid_input_error_display() {
        let err = FearEngineError::InvalidInput {
            field: "intensity".into(),
            reason: "must be between 0.0 and 1.0".into(),
        };
        assert_eq!(
            err.to_string(),
            "Invalid input for field 'intensity': must be between 0.0 and 1.0"
        );
    }

    #[test]
    fn test_not_found_error_display() {
        let err = FearEngineError::NotFound {
            entity: "Session".into(),
            id: "abc-123".into(),
        };
        assert_eq!(err.to_string(), "Session not found with id 'abc-123'");
    }

    #[test]
    fn test_rate_limit_error_display() {
        let err = FearEngineError::RateLimit {
            retry_after_ms: 5000,
        };
        assert_eq!(err.to_string(), "Rate limited: retry after 5000ms");
    }

    #[test]
    fn test_timeout_error_display() {
        let err = FearEngineError::Timeout {
            operation: "narrative_generation".into(),
            duration_ms: 10000,
        };
        assert_eq!(
            err.to_string(),
            "Operation 'narrative_generation' timed out after 10000ms"
        );
    }

    #[test]
    fn test_configuration_error_display() {
        let err = FearEngineError::Configuration("ANTHROPIC_API_KEY not set".into());
        assert_eq!(
            err.to_string(),
            "Configuration error: ANTHROPIC_API_KEY not set"
        );
    }

    #[test]
    fn test_from_serde_json_error() {
        let bad_json = "{ not valid json }";
        let serde_err = serde_json::from_str::<serde_json::Value>(bad_json).unwrap_err();
        let err: FearEngineError = serde_err.into();
        match &err {
            FearEngineError::Serialization(msg) => {
                assert!(!msg.is_empty(), "error message should not be empty");
            }
            other => panic!("expected Serialization, got {other:?}"),
        }
    }

    #[test]
    fn test_result_alias_ok() {
        fn returns_ok() -> Result<u32> {
            Ok(42)
        }
        assert_eq!(returns_ok().unwrap(), 42);
    }

    #[test]
    fn test_result_alias_err() {
        fn returns_err() -> Result<u32> {
            Err(FearEngineError::Database("test".into()))
        }
        assert!(returns_err().is_err());
    }
}
