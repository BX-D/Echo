//! Game session management REST endpoints.

use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::app::AppState;

/// Request body for creating a new game session.
#[derive(Debug, Deserialize)]
pub struct CreateGameRequest {
    pub player_name: Option<String>,
}

/// Response body after creating a game session.
#[derive(Debug, Serialize)]
pub struct CreateGameResponse {
    pub session_id: String,
}

/// Creates a new game session and its associated fear profile in the database.
///
/// Returns the session ID that can be used to connect via WebSocket.
pub async fn create_game(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateGameRequest>,
) -> impl IntoResponse {
    let session_id = match state.db.create_session(req.player_name.as_deref()) {
        Ok(id) => id,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": e.to_string()})),
            )
                .into_response();
        }
    };

    if let Err(e) = state.db.create_fear_profile(&session_id) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response();
    }

    (
        StatusCode::CREATED,
        Json(serde_json::json!({
            "session_id": session_id
        })),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{build_app, AppState};
    use axum::body::Body;
    use fear_engine_storage::Database;
    use http_body_util::BodyExt;
    use tower::ServiceExt;
    use tower_http::cors::CorsLayer;

    fn test_state() -> Arc<AppState> {
        let db = Database::new_in_memory().unwrap();
        Arc::new(AppState::new(db))
    }

    #[tokio::test]
    async fn test_create_game_success() {
        let state = test_state();
        let app = build_app(state.clone(), CorsLayer::permissive());
        let body = serde_json::json!({"player_name": "Alice"});
        let req = axum::http::Request::builder()
            .method("POST")
            .uri("/api/game")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&body).unwrap()))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert!(json["session_id"].is_string());
    }

    #[tokio::test]
    async fn test_create_game_without_name() {
        let state = test_state();
        let app = build_app(state, CorsLayer::permissive());
        let req = axum::http::Request::builder()
            .method("POST")
            .uri("/api/game")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"player_name": null}"#))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);
    }
}
