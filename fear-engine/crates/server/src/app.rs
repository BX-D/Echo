//! Application state and Axum router builder.

use std::sync::Arc;

use axum::routing::{get, post};
use axum::Router;
use dashmap::DashMap;
use fear_engine_storage::Database;
use tower_http::cors::CorsLayer;

use crate::echo_protocol::content::EchoContent;
use crate::routes;
use crate::ws;

/// Shared application state accessible from all handlers.
pub struct AppState {
    /// Database handle.
    pub db: Database,
    /// Echo Protocol authored content bundle.
    pub echo_content: Arc<EchoContent>,
    /// Currently active WebSocket game sessions, keyed by session ID.
    pub sessions: DashMap<String, ws::session::GameSession>,
}

impl AppState {
    /// Creates a new application state from a database handle.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use fear_engine_storage::Database;
    /// use fear_engine_server::app::AppState;
    ///
    /// let db = Database::new("sqlite://fear.db").unwrap();
    /// let state = AppState::new(db);
    /// ```
    pub fn new(db: Database) -> Self {
        Self {
            db,
            echo_content: Arc::new(
                EchoContent::load().expect("failed to load Echo Protocol content"),
            ),
            sessions: DashMap::new(),
        }
    }
}

/// Builds the Axum [`Router`] with all routes and middleware.
///
/// # Example
///
/// ```
/// use std::sync::Arc;
/// use fear_engine_storage::Database;
/// use fear_engine_server::app::{AppState, build_app};
/// use tower_http::cors::CorsLayer;
///
/// let db = Database::new_in_memory().unwrap();
/// let state = Arc::new(AppState::new(db));
/// let app = build_app(state, CorsLayer::permissive());
/// ```
pub fn build_app(state: Arc<AppState>, cors: CorsLayer) -> Router {
    Router::new()
        .route("/health", get(routes::health::health_check))
        .route("/ws", get(ws::handler::ws_upgrade))
        .route("/api/game", post(routes::game::create_game))
        .layer(cors)
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_state_creation() {
        let db = Database::new_in_memory().unwrap();
        let state = AppState::new(db);
        assert!(state.sessions.is_empty());
    }

    #[test]
    fn test_build_app_returns_router() {
        let db = Database::new_in_memory().unwrap();
        let state = Arc::new(AppState::new(db));
        let _app = build_app(state, CorsLayer::permissive());
    }
}
