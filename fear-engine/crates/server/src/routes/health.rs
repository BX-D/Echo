//! Health check endpoint.

use axum::Json;

/// Returns the server status and version.
///
/// # Example response
///
/// ```json
/// { "status": "ok", "version": "0.1.0" }
/// ```
pub async fn health_check() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "version": "0.1.0"
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_health_check_body() {
        let Json(body) = health_check().await;
        assert_eq!(body["status"], "ok");
        assert_eq!(body["version"], "0.1.0");
    }
}
