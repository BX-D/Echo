//! WebSocket connection handler for the Echo Protocol runtime.

use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Query, State};
use axum::response::IntoResponse;
use fear_engine_common::types::{ClientMessage, MetaTarget, ServerMessage};
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use tokio::sync::mpsc;
use tokio::time::Instant;

use crate::app::AppState;
use crate::echo_protocol::runtime::{EchoSessionRuntime, RuntimeOutput};
use crate::ws::messages::{decode_client_message, encode_server_message};
use crate::ws::session::GameSession;

#[derive(Debug, Default, Deserialize)]
pub struct WsConnectParams {
    pub session_id: Option<String>,
}

pub async fn ws_upgrade(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
    Query(params): Query<WsConnectParams>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state, params.session_id))
}

async fn handle_socket(
    socket: WebSocket,
    state: Arc<AppState>,
    requested_session_id: Option<String>,
) {
    let (mut ws_sender, mut ws_receiver) = socket.split();
    let (session_id, mut runtime) = match requested_session_id {
        Some(session_id) => match state.db.get_session(&session_id) {
            Ok(_) => match EchoSessionRuntime::resume(
                session_id.clone(),
                Arc::new(state.db.clone()),
                state.echo_content.clone(),
            ) {
                Ok(runtime) => (session_id, runtime),
                Err(error) => {
                    let _ = ws_sender
                        .send(Message::Text(
                            encode_server_message(&error_msg(
                                "SESSION_RESUME_FAILED",
                                &error.to_string(),
                                false,
                            ))
                            .unwrap_or_else(|_| "{}".into())
                            .into(),
                        ))
                        .await;
                    return;
                }
            },
            Err(error) => {
                let _ = ws_sender
                    .send(Message::Text(
                        encode_server_message(&error_msg(
                            "SESSION_RESUME_FAILED",
                            &error.to_string(),
                            false,
                        ))
                        .unwrap_or_else(|_| "{}".into())
                        .into(),
                    ))
                    .await;
                return;
            }
        },
        None => {
            let session_id = match state.db.create_session(None) {
                Ok(id) => id,
                Err(error) => {
                    let _ = ws_sender
                        .send(Message::Text(
                            encode_server_message(&error_msg(
                                "SESSION_CREATE_FAILED",
                                &error.to_string(),
                                false,
                            ))
                            .unwrap_or_else(|_| "{}".into())
                            .into(),
                        ))
                        .await;
                    return;
                }
            };
            let _ = state.db.create_fear_profile(&session_id);
            (
                session_id.clone(),
                EchoSessionRuntime::new(
                    session_id,
                    Arc::new(state.db.clone()),
                    state.echo_content.clone(),
                ),
            )
        }
    };

    let (tx, mut rx) = mpsc::channel::<ServerMessage>(32);
    let send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if let Ok(json) = encode_server_message(&msg) {
                if ws_sender.send(Message::Text(json.into())).await.is_err() {
                    break;
                }
            }
        }
    });

    state.sessions.insert(
        session_id.clone(),
        GameSession {
            session_id: session_id.clone(),
            chapter: runtime.current_chapter(),
            sender: tx.clone(),
            created_at: Instant::now(),
        },
    );

    if runtime.started() {
        let _ =
            send_runtime_output(&tx, runtime.current_surface().map(RuntimeOutput::Surface)).await;
        if let Some(ending) = runtime.current_ending() {
            let _ = tx.send(ServerMessage::Ending { ending }).await;
        }
    }

    while let Some(result) = ws_receiver.next().await {
        match result {
            Ok(Message::Text(text)) => match decode_client_message(&text) {
                Ok(client_msg) => {
                    handle_client_message(client_msg, &state, &session_id, &tx, &mut runtime).await;
                    if let Some(mut session) = state.sessions.get_mut(&session_id) {
                        session.chapter = runtime.current_chapter();
                    }
                }
                Err(error) => {
                    let _ = tx
                        .send(error_msg("INVALID_MESSAGE", &error.to_string(), true))
                        .await;
                }
            },
            Ok(Message::Close(_)) => break,
            Err(_) => break,
            _ => {}
        }
    }

    state.sessions.remove(&session_id);
    send_task.abort();
}

async fn handle_client_message(
    msg: ClientMessage,
    state: &Arc<AppState>,
    session_id: &str,
    tx: &mpsc::Sender<ServerMessage>,
    runtime: &mut EchoSessionRuntime,
) {
    match msg {
        ClientMessage::StartGame { player_name } => {
            if let Err(error) = runtime.set_player_name(player_name.as_deref()) {
                let _ = tx
                    .send(error_msg("START_FAILED", &error.to_string(), false))
                    .await;
                return;
            }
            match runtime.start_game() {
                Ok(surface) => {
                    let _ = tx
                        .send(ServerMessage::SessionSurface {
                            surface: surface.clone(),
                        })
                        .await;
                    for meta in meta_interruptions(&surface) {
                        let _ = tx.send(meta).await;
                    }
                }
                Err(error) => {
                    let _ = tx
                        .send(error_msg("START_FAILED", &error.to_string(), false))
                        .await;
                }
            }
        }
        ClientMessage::PlayerMessage {
            beat_id,
            text,
            typing_duration_ms,
            backspace_count,
        } => {
            let result = runtime
                .process_player_message(&beat_id, &text, typing_duration_ms, backspace_count)
                .await;
            let _ = send_runtime_output(tx, result).await;
        }
        ClientMessage::Choice {
            scene_id,
            choice_id,
            time_to_decide_ms,
            approach,
        } => {
            let result = runtime.process_choice(&choice_id, &scene_id, time_to_decide_ms, approach);
            let _ = send_runtime_output(tx, result).await;
        }
        ClientMessage::BehaviorBatch { events, .. } => {
            let _ = state.db.insert_behavior_events(session_id, &events);
        }
        ClientMessage::TextInput {
            scene_id,
            text,
            typing_duration_ms,
            backspace_count,
        } => {
            let event = fear_engine_common::types::BehaviorEvent {
                event_type: fear_engine_common::types::BehaviorEventType::Keystroke {
                    chars_per_second: if typing_duration_ms > 0 {
                        text.len() as f64 / (typing_duration_ms as f64 / 1000.0)
                    } else {
                        0.0
                    },
                    backspace_count,
                    total_chars: text.len() as u32,
                },
                timestamp: chrono::Utc::now(),
                scene_id,
            };
            let _ = state.db.insert_behavior_events(session_id, &[event]);
        }
    }
}

async fn send_runtime_output(
    tx: &mpsc::Sender<ServerMessage>,
    result: fear_engine_common::Result<RuntimeOutput>,
) -> fear_engine_common::Result<()> {
    match result? {
        RuntimeOutput::Surface(surface) => {
            tx.send(ServerMessage::SessionSurface {
                surface: surface.clone(),
            })
            .await
            .ok();
            for meta in meta_interruptions(&surface) {
                tx.send(meta).await.ok();
            }
        }
        RuntimeOutput::Ending(ending) => {
            tx.send(ServerMessage::Ending { ending }).await.ok();
        }
    }
    Ok(())
}

fn error_msg(code: &str, message: &str, recoverable: bool) -> ServerMessage {
    ServerMessage::Error {
        code: code.into(),
        message: message.into(),
        recoverable,
    }
}

fn meta_interruptions(surface: &fear_engine_common::types::SessionSurface) -> Vec<ServerMessage> {
    let mut out = Vec::new();
    if surface.glitch_level >= 0.6 {
        out.push(ServerMessage::Meta {
            text: "The client title bar changes a fraction too late, as if something else is naming the window."
                .into(),
            target: MetaTarget::Overlay,
            delay_ms: 2800,
        });
    } else if surface.glitch_level >= 0.3 {
        out.push(ServerMessage::Meta {
            text: "A line in the terminal blinks twice, then pretends it only appeared once."
                .into(),
            target: MetaTarget::GlitchText,
            delay_ms: 2200,
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{build_app, AppState};
    use fear_engine_storage::Database;
    use futures_util::{SinkExt, StreamExt};
    use tokio::net::TcpListener;
    use tokio_tungstenite::tungstenite;
    use tower_http::cors::CorsLayer;

    #[tokio::test]
    async fn start_game_emits_session_surface() {
        let db = Database::new_in_memory().unwrap();
        let app = build_app(Arc::new(AppState::new(db)), CorsLayer::permissive());
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

        let (stream, _) = tokio_tungstenite::connect_async(format!("ws://{addr}/ws"))
            .await
            .unwrap();
        let (mut write, mut read) = stream.split();
        write
            .send(tungstenite::Message::Text(
                serde_json::json!({"type":"start_game","payload":{"player_name":"Tester"}})
                    .to_string()
                    .into(),
            ))
            .await
            .unwrap();

        let text = read.next().await.unwrap().unwrap().into_text().unwrap();
        assert!(text.contains("session_surface"));
        assert!(text.contains("prologue_boot_sequence"));
    }
}
