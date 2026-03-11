//! WebSocket message serialization and deserialization helpers.

use fear_engine_common::types::{ClientMessage, ServerMessage};

/// Serializes a [`ServerMessage`] to a JSON string for sending over the WebSocket.
///
/// # Example
///
/// ```
/// use fear_engine_common::types::ServerMessage;
/// use fear_engine_server::ws::messages::encode_server_message;
///
/// let msg = ServerMessage::Error {
///     code: "TEST".into(),
///     message: "test".into(),
///     recoverable: true,
/// };
/// let json = encode_server_message(&msg).unwrap();
/// assert!(json.contains("error"));
/// ```
pub fn encode_server_message(msg: &ServerMessage) -> Result<String, serde_json::Error> {
    serde_json::to_string(msg)
}

/// Deserializes a JSON string from the client into a [`ClientMessage`].
///
/// # Example
///
/// ```
/// use fear_engine_server::ws::messages::decode_client_message;
///
/// let json = r#"{"type":"start_game","payload":{"player_name":"Alice"}}"#;
/// let msg = decode_client_message(json).unwrap();
/// ```
pub fn decode_client_message(text: &str) -> Result<ClientMessage, serde_json::Error> {
    serde_json::from_str(text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use fear_engine_common::types::{
        Atmosphere, GamePhase, SessionAct, SurfaceMedium, TrustPosture,
    };

    #[test]
    fn test_encode_server_message_narrative() {
        let msg = ServerMessage::Narrative {
            scene_id: "s1".into(),
            text: "Hello".into(),
            atmosphere: Atmosphere::Calm,
            choices: vec![],
            sound_cue: None,
            intensity: 0.1,
            effects: vec![],
            title: None,
            act: Some(SessionAct::Invitation),
            medium: Some(SurfaceMedium::Chat),
            trust_posture: Some(TrustPosture::Helpful),
            status_line: None,
            observation_notes: vec![],
            trace_items: vec![],
            transcript_lines: vec![],
            question_prompts: vec![],
            archive_entries: vec![],
            mirror_observations: vec![],
            surface_label: None,
            auxiliary_text: None,
            surface_purpose: None,
            system_intent: None,
            active_links: vec![],
            provisional: false,
        };
        let json = encode_server_message(&msg).unwrap();
        assert!(json.contains("narrative"));
        assert!(json.contains("Hello"));
    }

    #[test]
    fn test_encode_server_message_phase_change() {
        let msg = ServerMessage::PhaseChange {
            from: GamePhase::Calibrating,
            to: GamePhase::Exploring,
        };
        let json = encode_server_message(&msg).unwrap();
        assert!(json.contains("phase_change"));
    }

    #[test]
    fn test_decode_client_message_start_game() {
        let json = r#"{"type":"start_game","payload":{"player_name":"Bob"}}"#;
        let msg = decode_client_message(json).unwrap();
        match msg {
            ClientMessage::StartGame { player_name } => {
                assert_eq!(player_name.as_deref(), Some("Bob"));
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn test_decode_client_message_choice() {
        let json = r#"{"type":"choice","payload":{"scene_id":"s1","choice_id":"c1","time_to_decide_ms":1500,"approach":"investigate"}}"#;
        let msg = decode_client_message(json).unwrap();
        match msg {
            ClientMessage::Choice {
                time_to_decide_ms,
                approach,
                ..
            } => {
                assert_eq!(time_to_decide_ms, 1500);
                assert_eq!(
                    approach,
                    fear_engine_common::types::ChoiceApproach::Investigate
                );
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn test_decode_invalid_json_returns_error() {
        assert!(decode_client_message("not json").is_err());
    }

    #[test]
    fn test_decode_unknown_type_returns_error() {
        let json = r#"{"type":"unknown","payload":{}}"#;
        assert!(decode_client_message(json).is_err());
    }
}
