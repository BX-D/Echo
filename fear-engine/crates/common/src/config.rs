//! Application configuration loaded from environment variables.
//!
//! All settings have sensible defaults except API keys, which default to empty
//! strings in development. Use [`AppConfig::from_env`] at startup to load the
//! configuration, then call [`AppConfig::validate`] before entering production
//! code paths that require real API keys.

use crate::error::{FearEngineError, Result};

/// Top-level application configuration.
///
/// # Example
///
/// ```
/// use fear_engine_common::config::AppConfig;
///
/// let config = AppConfig::from_env();
/// assert_eq!(config.server_port, 3001);
/// ```
#[derive(Debug, Clone)]
pub struct AppConfig {
    /// Anthropic API key for Claude narrative generation.
    pub anthropic_api_key: String,
    /// Stability AI / Replicate API key for image generation (optional).
    pub stability_api_key: Option<String>,
    /// SQLite database URL.
    pub database_url: String,
    /// Address the HTTP server binds to.
    pub server_host: String,
    /// Port the HTTP server binds to.
    pub server_port: u16,
    /// Allowed origin for CORS.
    pub frontend_url: String,
    /// Rust `tracing` / `env_logger` filter string.
    pub log_level: String,
}

impl AppConfig {
    /// Loads configuration from environment variables, falling back to defaults.
    ///
    /// | Variable | Default |
    /// |----------|---------|
    /// | `ANTHROPIC_API_KEY` | `""` |
    /// | `OPENAI_API_KEY` | *none* |
    /// | `DATABASE_URL` | `sqlite://fear_engine.db` |
    /// | `SERVER_HOST` | `127.0.0.1` |
    /// | `SERVER_PORT` | `3001` |
    /// | `FRONTEND_URL` | `http://localhost:5173` |
    /// | `RUST_LOG` | `fear_engine=debug` |
    ///
    /// # Example
    ///
    /// ```
    /// use fear_engine_common::config::AppConfig;
    /// let cfg = AppConfig::from_env();
    /// assert!(!cfg.database_url.is_empty());
    /// ```
    pub fn from_env() -> Self {
        let stability_key = std::env::var("OPENAI_API_KEY")
            .ok()
            .filter(|s| !s.is_empty());

        let server_port = std::env::var("SERVER_PORT")
            .ok()
            .and_then(|v| v.parse::<u16>().ok())
            .unwrap_or(3001);

        Self {
            anthropic_api_key: std::env::var("ANTHROPIC_API_KEY").unwrap_or_default(),
            stability_api_key: stability_key,
            database_url: std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| "sqlite://fear_engine.db".into()),
            server_host: std::env::var("SERVER_HOST").unwrap_or_else(|_| "127.0.0.1".into()),
            server_port,
            frontend_url: std::env::var("FRONTEND_URL")
                .unwrap_or_else(|_| "http://localhost:5173".into()),
            log_level: std::env::var("RUST_LOG").unwrap_or_else(|_| "fear_engine=debug".into()),
        }
    }

    /// Returns the socket address string (`host:port`).
    ///
    /// # Example
    ///
    /// ```
    /// use fear_engine_common::config::AppConfig;
    /// let cfg = AppConfig::from_env();
    /// assert!(cfg.socket_addr().contains(':'));
    /// ```
    pub fn socket_addr(&self) -> String {
        format!("{}:{}", self.server_host, self.server_port)
    }

    /// Validates that required configuration values are present.
    ///
    /// Returns an error if `anthropic_api_key` is empty.
    ///
    /// # Example
    ///
    /// ```
    /// use fear_engine_common::config::AppConfig;
    /// let mut cfg = AppConfig::from_env();
    /// cfg.anthropic_api_key = String::new();
    /// assert!(cfg.validate().is_err());
    /// ```
    pub fn validate(&self) -> Result<()> {
        if self.anthropic_api_key.is_empty() {
            return Err(FearEngineError::Configuration(
                "ANTHROPIC_API_KEY is required but not set".into(),
            ));
        }
        Ok(())
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            anthropic_api_key: String::new(),
            stability_api_key: None,
            database_url: "sqlite://fear_engine.db".into(),
            server_host: "127.0.0.1".into(),
            server_port: 3001,
            frontend_url: "http://localhost:5173".into(),
            log_level: "fear_engine=debug".into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_has_sensible_values() {
        let cfg = AppConfig::default();
        assert_eq!(cfg.server_host, "127.0.0.1");
        assert_eq!(cfg.server_port, 3001);
        assert_eq!(cfg.frontend_url, "http://localhost:5173");
        assert_eq!(cfg.database_url, "sqlite://fear_engine.db");
        assert_eq!(cfg.log_level, "fear_engine=debug");
        assert!(cfg.anthropic_api_key.is_empty());
        assert!(cfg.stability_api_key.is_none());
    }

    #[test]
    fn test_socket_addr_format() {
        let cfg = AppConfig::default();
        assert_eq!(cfg.socket_addr(), "127.0.0.1:3001");
    }

    #[test]
    fn test_validate_fails_without_api_key() {
        let cfg = AppConfig::default();
        let result = cfg.validate();
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("ANTHROPIC_API_KEY"));
    }

    #[test]
    fn test_validate_succeeds_with_api_key() {
        let mut cfg = AppConfig::default();
        cfg.anthropic_api_key = "sk-ant-test-key".into();
        assert!(cfg.validate().is_ok());
    }

    #[test]
    fn test_from_env_returns_defaults_when_vars_unset() {
        // from_env reads real env; defaults are tested via the Default impl.
        // We just verify it doesn't panic and returns a valid config.
        let cfg = AppConfig::from_env();
        assert!(!cfg.database_url.is_empty());
        assert!(!cfg.server_host.is_empty());
        assert!(cfg.server_port > 0);
    }

    #[test]
    fn test_config_clone() {
        let cfg = AppConfig::default();
        let clone = cfg.clone();
        assert_eq!(cfg.server_port, clone.server_port);
        assert_eq!(cfg.database_url, clone.database_url);
    }
}
