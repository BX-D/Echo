//! Per-connection game session state held in memory while a WebSocket is active.

use fear_engine_common::types::{ServerMessage, StoryChapter};
use tokio::sync::mpsc;
use tokio::time::Instant;

/// In-memory state for one active WebSocket connection.
///
/// Stored in [`AppState::sessions`](crate::app::AppState) while the player is
/// connected. Removed on disconnect.
pub struct GameSession {
    /// Database session ID (UUID).
    pub session_id: String,
    /// Current story chapter for fast access without a DB round-trip.
    pub chapter: StoryChapter,
    /// Channel for pushing [`ServerMessage`]s to this connection's send task.
    pub sender: mpsc::Sender<ServerMessage>,
    /// When this connection was established.
    pub created_at: Instant,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_game_session_fields() {
        let (tx, _rx) = mpsc::channel(8);
        let session = GameSession {
            session_id: "test-id".into(),
            chapter: StoryChapter::Onboarding,
            sender: tx,
            created_at: Instant::now(),
        };
        assert_eq!(session.session_id, "test-id");
        assert_eq!(session.chapter, StoryChapter::Onboarding);
    }
}
