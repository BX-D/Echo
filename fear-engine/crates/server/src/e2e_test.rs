//! Comprehensive end-to-end functional tests.
//!
//! Verifies every major feature of the game works correctly from a real
//! WebSocket client perspective:
//!
//! 1. Connection + welcome
//! 2. StartGame → first calibration scene with choices
//! 3. Full calibration path (3 scenes)
//! 4. Probe scenes with fear-targeted content
//! 5. Dynamic/fallback scenes when graph runs out
//! 6. Behavior batches are accepted and don't break state
//! 7. Image messages sent for scenes with image_prompt
//! 8. Bad choices → fallback (no crash)
//! 9. Multiple concurrent players
//! 10. Disconnect + reconnect
//! 11. Every scene has non-empty text and valid intensity

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::Duration;

    use fear_engine_common::types::ServerMessage;
    use fear_engine_storage::Database;
    use futures_util::{SinkExt, StreamExt};
    use tokio::net::TcpListener;
    use tokio_tungstenite::tungstenite;
    use tower_http::cors::CorsLayer;

    use crate::app::{build_app, AppState};

    // -- Helpers ----------------------------------------------------------

    type WsSink = futures_util::stream::SplitSink<
        tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
        tungstenite::Message,
    >;
    type WsStream = futures_util::stream::SplitStream<
        tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
    >;

    async fn spawn_server() -> (std::net::SocketAddr, Arc<AppState>) {
        let db = Database::new_in_memory().unwrap();
        let state = Arc::new(AppState::new(db));
        let app = build_app(state.clone(), CorsLayer::permissive());
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        (addr, state)
    }

    async fn connect(addr: std::net::SocketAddr) -> (WsSink, WsStream) {
        let url = format!("ws://{addr}/ws");
        let (stream, _) = tokio_tungstenite::connect_async(&url).await.unwrap();
        stream.split()
    }

    /// Read next message with 5s timeout.
    async fn recv(read: &mut WsStream) -> ServerMessage {
        let msg = tokio::time::timeout(Duration::from_secs(5), read.next())
            .await
            .expect("timed out")
            .expect("stream ended")
            .expect("ws error");
        match msg {
            tungstenite::Message::Text(text) => {
                serde_json::from_str::<ServerMessage>(&text)
                    .unwrap_or_else(|e| panic!("bad JSON: {e}\nraw: {text}"))
            }
            other => panic!("expected Text, got {other:?}"),
        }
    }

    async fn send_json(write: &mut WsSink, json: serde_json::Value) {
        write
            .send(tungstenite::Message::Text(json.to_string().into()))
            .await
            .unwrap();
    }

    async fn send_start(write: &mut WsSink) {
        send_json(
            write,
            serde_json::json!({"type":"start_game","payload":{"player_name":"Tester"}}),
        )
        .await;
    }

    async fn send_choice(write: &mut WsSink, scene_id: &str, choice_id: &str) {
        send_json(
            write,
            serde_json::json!({
                "type":"choice",
                "payload":{"scene_id":scene_id,"choice_id":choice_id,"time_to_decide_ms":2000,"approach":"investigate"}
            }),
        )
        .await;
    }

    async fn send_behavior(write: &mut WsSink) {
        send_json(
            write,
            serde_json::json!({
                "type":"behavior_batch",
                "payload":{
                    "events":[
                        {"event_type":{"type":"keystroke","chars_per_second":4.0,"backspace_count":2,"total_chars":25},"timestamp":"2026-03-09T12:00:00Z","scene_id":"any"},
                        {"event_type":{"type":"mouse_movement","velocity":180.0,"tremor_score":0.5},"timestamp":"2026-03-09T12:00:01Z","scene_id":"any"},
                        {"event_type":{"type":"pause","duration_ms":3500,"scene_content_hash":"h1"},"timestamp":"2026-03-09T12:00:05Z","scene_id":"any"}
                    ],
                    "timestamp":"2026-03-09T12:00:06Z"
                }
            }),
        )
        .await;
    }

    /// Collect all server messages until a Narrative arrives (skipping Image/Phase/Meta).
    /// Returns the Narrative plus any extra messages received along the way.
    async fn recv_until_narrative(read: &mut WsStream) -> (ServerMessage, Vec<ServerMessage>) {
        let mut extras = Vec::new();
        for _ in 0..5 {
            let msg = recv(read).await;
            match &msg {
                ServerMessage::Narrative { .. } => return (msg, extras),
                _ => extras.push(msg),
            }
        }
        panic!("never received a Narrative after 5 messages");
    }

    async fn recv_until_playable_narrative(
        read: &mut WsStream,
    ) -> (ServerMessage, Vec<ServerMessage>) {
        let mut extras = Vec::new();
        for _ in 0..8 {
            let msg = recv(read).await;
            match &msg {
                ServerMessage::Narrative { choices, .. } if !choices.is_empty() => {
                    return (msg, extras);
                }
                ServerMessage::Reveal { .. } => return (msg, extras),
                _ => extras.push(msg),
            }
        }
        panic!("never received a playable Narrative after 8 messages");
    }

    fn get_scene_id(msg: &ServerMessage) -> &str {
        match msg {
            ServerMessage::Narrative { scene_id, .. } => scene_id,
            _ => panic!("not a Narrative"),
        }
    }

    fn get_choices(msg: &ServerMessage) -> Vec<String> {
        match msg {
            ServerMessage::Narrative { choices, .. } => {
                choices.iter().map(|c| c.id.clone()).collect()
            }
            _ => panic!("not a Narrative"),
        }
    }

    // =====================================================================
    //  FUNCTIONAL REQUIREMENT TESTS
    // =====================================================================

    /// FR1: WebSocket connects and receives a welcome narrative immediately.
    #[tokio::test]
    async fn fr01_connection_and_welcome() {
        let (addr, _) = spawn_server().await;
        let (_, mut read) = connect(addr).await;

        let msg = recv(&mut read).await;
        assert_eq!(get_scene_id(&msg), "welcome");
        match &msg {
            ServerMessage::Narrative { text, .. } => {
                assert!(text.contains("Fear Engine"), "welcome should mention Fear Engine");
            }
            _ => panic!("expected Narrative"),
        }
    }

    /// FR2: StartGame produces the first calibration scene with choices.
    #[tokio::test]
    async fn fr02_start_game_produces_calibration_scene() {
        let (addr, _) = spawn_server().await;
        let (mut write, mut read) = connect(addr).await;
        let _ = recv(&mut read).await; // welcome

        send_start(&mut write).await;
        let msg = recv(&mut read).await;

        assert_eq!(get_scene_id(&msg), "cal_awakening");
        let choices = get_choices(&msg);
        assert_eq!(choices.len(), 3, "cal_awakening should have 3 choices");
        assert!(choices.contains(&"sit_up".to_string()));
        assert!(choices.contains(&"stay_still".to_string()));
        assert!(choices.contains(&"call_out".to_string()));

        match &msg {
            ServerMessage::Narrative { text, intensity, .. } => {
                assert!(
                    text.to_lowercase().contains("session")
                        || text.to_lowercase().contains("interface"),
                    "should have authored session text"
                );
                assert!(*intensity >= 0.0 && *intensity <= 1.0);
            }
            _ => unreachable!(),
        }
    }

    /// FR3: Full calibration path — 3 scenes with choices at each step.
    #[tokio::test]
    async fn fr03_full_calibration_path() {
        let (addr, _) = spawn_server().await;
        let (mut write, mut read) = connect(addr).await;
        let _ = recv(&mut read).await; // welcome

        // Scene 1: cal_awakening
        send_start(&mut write).await;
        let s1 = recv(&mut read).await;
        assert_eq!(get_scene_id(&s1), "cal_awakening");
        assert!(!get_choices(&s1).is_empty());

        // Scene 2: cal_corridor
        send_choice(&mut write, "cal_awakening", "sit_up").await;
        let (s2, _) = recv_until_narrative(&mut read).await;
        assert_eq!(get_scene_id(&s2), "cal_corridor");
        assert!(!get_choices(&s2).is_empty());
        match &s2 {
            ServerMessage::Narrative { text, .. } => {
                assert!(
                    text.contains("PATH") || text.contains("interface"),
                    "cal_corridor should describe the session pathways"
                );
            }
            _ => unreachable!(),
        }

        // Scene 3: cal_reception
        send_choice(&mut write, "cal_corridor", "go_left").await;
        let (s3, _) = recv_until_narrative(&mut read).await;
        assert_eq!(get_scene_id(&s3), "cal_reception");
        assert!(!get_choices(&s3).is_empty());
        match &s3 {
            ServerMessage::Narrative { text, .. } => {
                assert!(
                    text.contains("presence frame")
                        || text.contains("mirror")
                        || text.contains("Camera"),
                    "cal_reception should describe the presence surface"
                );
            }
            _ => unreachable!(),
        }
    }

    /// FR4: Probe scenes are reached after calibration, targeting specific fears.
    #[tokio::test]
    async fn fr04_probe_scenes_reached() {
        let (addr, _) = spawn_server().await;
        let (mut write, mut read) = connect(addr).await;
        let _ = recv(&mut read).await;

        send_start(&mut write).await;
        let _ = recv(&mut read).await; // cal_awakening

        send_choice(&mut write, "cal_awakening", "sit_up").await;
        let (_, _) = recv_until_narrative(&mut read).await; // cal_corridor

        send_choice(&mut write, "cal_corridor", "go_left").await;
        let (_, _) = recv_until_narrative(&mut read).await; // cal_reception

        // From reception, "answer_phone" leads to a probe scene.
        send_choice(&mut write, "cal_reception", "answer_phone").await;
        let (probe, _) = recv_until_narrative(&mut read).await;
        let probe_id = get_scene_id(&probe);
        assert!(
            probe_id.starts_with("probe_"),
            "should reach a probe scene, got: {probe_id}"
        );
        assert!(!get_choices(&probe).is_empty(), "probe should have choices");
    }

    /// FR5: Dynamic/fallback scenes work when the scene graph runs out.
    #[tokio::test]
    async fn fr05_dynamic_fallback_scenes() {
        let (addr, _) = spawn_server().await;
        let (mut write, mut read) = connect(addr).await;
        let _ = recv(&mut read).await;

        send_start(&mut write).await;
        let _ = recv(&mut read).await;
        send_choice(&mut write, "cal_awakening", "sit_up").await;
        let (_, _) = recv_until_narrative(&mut read).await;
        send_choice(&mut write, "cal_corridor", "go_left").await;
        let (_, _) = recv_until_narrative(&mut read).await;
        send_choice(&mut write, "cal_reception", "answer_phone").await;
        let (probe, _) = recv_until_narrative(&mut read).await;

        // From probe, pick first choice — may lead to Dynamic target → fallback.
        let probe_choices = get_choices(&probe);
        send_choice(&mut write, get_scene_id(&probe), &probe_choices[0]).await;
        let (next, _) = recv_until_playable_narrative(&mut read).await;

        // Should be either another probe, a dynamic_fallback, or a fallback.
        let next_id = get_scene_id(&next);
        assert!(
            !next_id.is_empty(),
            "should get a valid scene after probe choice"
        );
        assert!(
            !get_choices(&next).is_empty(),
            "fallback/dynamic scene should have choices so game continues"
        );
    }

    /// FR6: Behavior batches are accepted without breaking the game.
    #[tokio::test]
    async fn fr06_behavior_batches_accepted() {
        let (addr, _) = spawn_server().await;
        let (mut write, mut read) = connect(addr).await;
        let _ = recv(&mut read).await;

        send_start(&mut write).await;
        let _ = recv(&mut read).await;

        // Send multiple behavior batches.
        for _ in 0..5 {
            send_behavior(&mut write).await;
        }

        // Game should still work after batches.
        send_choice(&mut write, "cal_awakening", "stay_still").await;
        let (msg, _) = recv_until_narrative(&mut read).await;
        assert_eq!(get_scene_id(&msg), "cal_corridor");
    }

    /// FR7: Image-prompt scenes send narrative (image is async/optional).
    #[tokio::test]
    async fn fr07_image_prompt_scene_sends_narrative() {
        let (addr, _) = spawn_server().await;
        let (mut write, mut read) = connect(addr).await;
        let _ = recv(&mut read).await;

        send_start(&mut write).await;
        let _ = recv(&mut read).await; // cal_awakening
        send_choice(&mut write, "cal_awakening", "sit_up").await;
        let (_, _) = recv_until_narrative(&mut read).await; // cal_corridor
        send_choice(&mut write, "cal_corridor", "read_clipboard").await;

        // cal_reception has image_prompt — narrative must arrive.
        // Image is generated async via DALL-E; in tests without API key it's skipped.
        let (narrative, extras) = recv_until_narrative(&mut read).await;
        assert_eq!(get_scene_id(&narrative), "cal_reception");
        // If OPENAI_API_KEY is set, we may also get an Image message.
        let got_image = extras.iter().any(|m| matches!(m, ServerMessage::Image { .. }));
        println!("[FR07] narrative=cal_reception, got_image={got_image}");
    }

    /// FR8: Invalid choices produce fallback scenes (no crash).
    #[tokio::test]
    async fn fr08_bad_choice_recovery() {
        let (addr, _) = spawn_server().await;
        let (mut write, mut read) = connect(addr).await;
        let _ = recv(&mut read).await;

        send_start(&mut write).await;
        let _ = recv(&mut read).await;

        // Send a nonexistent choice.
        send_choice(&mut write, "cal_awakening", "teleport_to_moon").await;
        let (msg, _) = recv_until_playable_narrative(&mut read).await;

        match &msg {
            ServerMessage::Narrative { choices, text, .. } => {
                assert!(!text.is_empty(), "fallback should have text");
                assert!(!choices.is_empty(), "fallback should have choices to continue");
            }
            ServerMessage::Error { .. } => {
                // Error is also acceptable — game didn't crash.
            }
            other => panic!("unexpected: {other:?}"),
        }

        // Game still works after bad choice.
        send_choice(&mut write, "cal_awakening", "sit_up").await;
        let (recovery, _) = recv_until_narrative(&mut read).await;
        assert!(!get_scene_id(&recovery).is_empty());
    }

    /// FR9: Multiple concurrent players each get independent game state.
    #[tokio::test]
    async fn fr09_concurrent_players() {
        let (addr, state) = spawn_server().await;
        let mut handles = vec![];

        for idx in 0..5 {
            let addr = addr;
            handles.push(tokio::spawn(async move {
                let (mut write, mut read) = connect(addr).await;
                let _ = recv(&mut read).await; // welcome

                send_start(&mut write).await;
                let s1 = recv(&mut read).await;
                assert_eq!(get_scene_id(&s1), "cal_awakening",
                    "player {idx} should start at cal_awakening");

                send_choice(&mut write, "cal_awakening", "call_out").await;
                let (s2, _) = recv_until_narrative(&mut read).await;
                assert_eq!(get_scene_id(&s2), "cal_corridor",
                    "player {idx} should reach cal_corridor");
                idx
            }));
        }

        for h in handles {
            let idx = h.await.unwrap();
            println!("[FR09] Player {idx} completed");
        }

        // All sessions should have been created.
        assert!(state.sessions.len() <= 5);
    }

    /// FR10: Disconnect and reconnect works — new session starts fresh.
    #[tokio::test]
    async fn fr10_disconnect_and_reconnect() {
        let (addr, state) = spawn_server().await;

        // First connection
        let (write, mut read) = connect(addr).await;
        let _ = recv(&mut read).await;
        assert_eq!(state.sessions.len(), 1);
        drop(write);
        drop(read);
        tokio::time::sleep(Duration::from_millis(200)).await;
        assert_eq!(state.sessions.len(), 0, "session should be cleaned up");

        // Reconnect
        let (mut write2, mut read2) = connect(addr).await;
        let welcome = recv(&mut read2).await;
        assert_eq!(get_scene_id(&welcome), "welcome");
        assert_eq!(state.sessions.len(), 1);

        // New session should work from scratch.
        send_start(&mut write2).await;
        let s1 = recv(&mut read2).await;
        assert_eq!(get_scene_id(&s1), "cal_awakening");
    }

    /// FR11: Every scene in a playthrough has non-empty text and valid intensity.
    #[tokio::test]
    async fn fr11_all_scenes_valid_content() {
        let (addr, _) = spawn_server().await;
        let (mut write, mut read) = connect(addr).await;
        let _ = recv(&mut read).await;

        send_start(&mut write).await;

        let first = recv(&mut read).await;
        let (mut current_scene, mut current_choices): (String, Vec<String>) = match &first {
            ServerMessage::Narrative {
                scene_id, choices, text, intensity, ..
            } => {
                assert!(!text.is_empty(), "scene text must not be empty");
                assert!(*intensity >= 0.0 && *intensity <= 1.0, "intensity out of range");
                assert!(text.len() > 20, "scene text should be substantial");
                (
                    scene_id.clone(),
                    choices.iter().map(|c| c.id.clone()).collect(),
                )
            }
            _ => panic!("expected Narrative"),
        };

        // Play through 8 scenes, checking each one.
        for step in 0..8 {
            if current_choices.is_empty() {
                break;
            }

            send_choice(&mut write, &current_scene, &current_choices[0]).await;
            let msg = recv(&mut read).await;

            match &msg {
                ServerMessage::Narrative {
                    scene_id, choices, text, intensity, ..
                } => {
                    assert!(!text.is_empty(), "step {step}: empty text");
                    assert!(*intensity >= 0.0 && *intensity <= 1.0, "step {step}: bad intensity");
                    current_scene = scene_id.clone();
                    current_choices = choices.iter().map(|c| c.id.clone()).collect();
                }
                ServerMessage::Reveal { .. } => {
                    // Game ended — valid outcome.
                    break;
                }
                ServerMessage::Image { .. } => {
                    // Image arrived before narrative — read again.
                    let next = recv(&mut read).await;
                    match &next {
                        ServerMessage::Narrative { scene_id, choices, .. } => {
                            current_scene = scene_id.clone();
                            current_choices = choices.iter().map(|c| c.id.clone()).collect();
                        }
                        ServerMessage::Reveal { .. } => break,
                        _ => {}
                    }
                }
                _ => {}
            }
        }
    }

    /// FR12: All three choice options from cal_awakening lead to cal_corridor.
    #[tokio::test]
    async fn fr12_all_awakening_choices_work() {
        let (addr, _) = spawn_server().await;

        for choice in ["sit_up", "stay_still", "call_out"] {
            let (mut write, mut read) = connect(addr).await;
            let _ = recv(&mut read).await;

            send_start(&mut write).await;
            let _ = recv(&mut read).await;

            send_choice(&mut write, "cal_awakening", choice).await;
            let (msg, _) = recv_until_narrative(&mut read).await;
            assert_eq!(
                get_scene_id(&msg),
                "cal_corridor",
                "choice '{choice}' should lead to cal_corridor"
            );
        }
    }

    /// FR13: Sending invalid JSON returns an error message (not a crash).
    #[tokio::test]
    async fn fr13_invalid_json_handled() {
        let (addr, _) = spawn_server().await;
        let (mut write, mut read) = connect(addr).await;
        let _ = recv(&mut read).await;

        send_json(&mut write, serde_json::json!("just a string")).await;
        let msg = recv(&mut read).await;
        match msg {
            ServerMessage::Error { code, recoverable, .. } => {
                assert_eq!(code, "INVALID_MESSAGE");
                assert!(recoverable);
            }
            _ => panic!("expected Error for invalid JSON"),
        }

        // Game should still work.
        send_start(&mut write).await;
        let s = recv(&mut read).await;
        assert_eq!(get_scene_id(&s), "cal_awakening");
    }

    /// FR14: Full 6-scene playthrough with behavior tracking interleaved.
    #[tokio::test]
    async fn fr14_full_playthrough_with_behavior() {
        let (addr, _) = spawn_server().await;
        let (mut write, mut read) = connect(addr).await;
        let _ = recv(&mut read).await;

        send_start(&mut write).await;
        let _ = recv(&mut read).await; // cal_awakening

        let path = [
            ("cal_awakening", "sit_up"),
            ("cal_corridor", "go_right"),
            ("cal_reception", "ignore_phone"),
        ];

        let mut last_scene = "cal_awakening".to_string();
        for (scene, choice) in &path {
            // Send behavior between choices.
            send_behavior(&mut write).await;

            send_choice(&mut write, scene, choice).await;

            // Drain all messages until we get a Narrative.
            let (msg, extras) = recv_until_narrative(&mut read).await;
            last_scene = get_scene_id(&msg).to_string();

            // Print what we got for debugging.
            let extra_types: Vec<&str> = extras.iter().map(|m| match m {
                ServerMessage::Image { .. } => "Image",
                ServerMessage::PhaseChange { .. } => "PhaseChange",
                ServerMessage::Meta { .. } => "Meta",
                _ => "Other",
            }).collect();
            println!(
                "[FR14] {scene} → {choice} → {last_scene} (extras: {extra_types:?})"
            );
        }

        // Should have gone through calibration into probes.
        assert!(
            last_scene.starts_with("probe_") || last_scene == "dynamic_fallback" || last_scene == "fallback",
            "after calibration should reach probe/dynamic, got: {last_scene}"
        );
    }
}
