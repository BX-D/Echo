//! WebSocket connection handler — upgrade, message routing, and cleanup.

use std::collections::HashSet;
use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Query, State};
use axum::response::IntoResponse;
use fear_engine_ai_integration::cache::compute_cache_key;
use fear_engine_ai_integration::claude_client::{
    ClaudeClient, ClientConfig, GenerateRequest, Message as ClaudeMessage, Role,
};
use fear_engine_ai_integration::image::ImageClient;
use fear_engine_ai_integration::narrative::NarrativePipeline;
use fear_engine_ai_integration::prompt::context::{
    FearProfileContext, GameStateContext, PromptContext,
};
use fear_engine_common::types::{ClientMessage, FearType, MetaTarget, RevealAnalysis, ServerMessage};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tokio::time::Instant;

use crate::app::AppState;
use crate::game_loop::SessionGameLoop;
use crate::ws::messages::{decode_client_message, encode_server_message};
use crate::ws::session::GameSession;

#[derive(Debug, Default, Deserialize)]
pub struct WsConnectParams {
    pub session_id: Option<String>,
}

/// Axum handler that upgrades an HTTP request to a WebSocket connection.
pub async fn ws_upgrade(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
    Query(params): Query<WsConnectParams>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state, params.session_id))
}

/// Drives one WebSocket connection through its entire lifecycle.
async fn handle_socket(
    socket: WebSocket,
    state: Arc<AppState>,
    requested_session_id: Option<String>,
) {
    let (mut ws_sender, mut ws_receiver) = socket.split();

    // ── 1. Create or resume session in DB ───────────────────────────────
    let (session_id, mut game_loop) = match requested_session_id {
        Some(session_id) => {
            let session = match state.db.get_session(&session_id) {
                Ok(session) => session,
                Err(e) => {
                    let err = error_msg("SESSION_RESUME_FAILED", &e.to_string(), false);
                    let _ = ws_sender.send(text_msg(&err)).await;
                    return;
                }
            };

            if state.db.get_fear_profile(&session.id).is_err()
                && state.db.create_fear_profile(&session.id).is_err()
            {
                let err = error_msg("PROFILE_RESUME_FAILED", "missing fear profile", false);
                let _ = ws_sender.send(text_msg(&err)).await;
                return;
            }

            let game_loop = match SessionGameLoop::resume(
                session.id.clone(),
                Arc::new(state.db.clone()),
            ) {
                Ok(game_loop) => game_loop,
                Err(e) => {
                    let err = error_msg("SESSION_RESUME_FAILED", &e.to_string(), false);
                    let _ = ws_sender.send(text_msg(&err)).await;
                    return;
                }
            };

            (session.id, game_loop)
        }
        None => {
            let session_id = match state.db.create_session(None) {
                Ok(id) => id,
                Err(e) => {
                    let err = error_msg("SESSION_CREATE_FAILED", &e.to_string(), false);
                    let _ = ws_sender.send(text_msg(&err)).await;
                    return;
                }
            };

            if let Err(e) = state.db.create_fear_profile(&session_id) {
                let err = error_msg("PROFILE_CREATE_FAILED", &e.to_string(), false);
                let _ = ws_sender.send(text_msg(&err)).await;
                return;
            }

            let game_loop = SessionGameLoop::new(
                session_id.clone(),
                Arc::new(state.db.clone()),
            );
            (session_id, game_loop)
        }
    };

    // ── 2. Outbound channel + send task ─────────────────────────────────
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

    // ── 3. Register session ─────────────────────────────────────────────
    state.sessions.insert(
        session_id.clone(),
        GameSession {
            session_id: session_id.clone(),
            game_phase: game_loop.current_phase(),
            sender: tx.clone(),
            created_at: Instant::now(),
        },
    );

    // ── 4. Initial message ──────────────────────────────────────────────
    let initial = if game_loop.started() {
        game_loop.resume_message()
    } else {
        welcome_message()
    };
    let _ = tx.send(initial).await;

    // ── 5. Receive loop ─────────────────────────────────────────────────
    while let Some(result) = ws_receiver.next().await {
        match result {
            Ok(Message::Text(text)) => match decode_client_message(&text) {
                Ok(client_msg) => {
                    handle_client_message(
                        client_msg,
                        &state,
                        &session_id,
                        &tx,
                        &mut game_loop,
                    )
                    .await;
                }
                Err(e) => {
                    let _ = tx
                        .send(error_msg("INVALID_MESSAGE", &e.to_string(), true))
                        .await;
                }
            },
            Ok(Message::Close(_)) => break,
            Err(_) => break,
            _ => {}
        }
    }

    // ── 6. Cleanup ──────────────────────────────────────────────────────
    game_loop.cleanup();
    state.sessions.remove(&session_id);
    send_task.abort();
}

/// Routes a deserialized [`ClientMessage`] to the game loop.
async fn handle_client_message(
    msg: ClientMessage,
    state: &Arc<AppState>,
    session_id: &str,
    tx: &mpsc::Sender<ServerMessage>,
    game_loop: &mut SessionGameLoop,
) {
    match msg {
        ClientMessage::StartGame { player_name } => {
            if let Some(name) = &player_name {
                let state_json = serde_json::json!({"player_name": name}).to_string();
                let _ = state.db.update_session_state(session_id, "intro", &state_json);
            }
            // Start the game — send the first real scene.
            let narrative = game_loop.start_game();
            let _ = tx.send(narrative).await;
            for meta in scene_entry_interruptions(&game_loop.resume_message()) {
                let _ = tx.send(meta).await;
            }
        }

        ClientMessage::Choice {
            choice_id,
            time_to_decide_ms,
            approach,
            ..
        } => {
            let mut result = game_loop.process_choice(
                &choice_id,
                time_to_decide_ms,
                approach,
            );

            if matches!(result.narrative, ServerMessage::Reveal { .. }) {
                result.narrative = enrich_reveal(&state.db, &result.narrative).await;
                game_loop.set_last_message_for_resume(result.narrative.clone());
            }

            let prepared_dynamic = result.dynamic_context.as_ref().map(|dynamic_context| {
                PreparedDynamicNarrative {
                    fallback: result.narrative.clone(),
                    prompt_context: build_prompt_context(
                        game_loop,
                        dynamic_context,
                        &choice_id,
                        &result.narrative,
                    ),
                    narrative_cache_key: build_dynamic_cache_key(
                        game_loop,
                        dynamic_context,
                        &choice_id,
                    ),
                }
            });
            let dominant_fear = dominant_visual_fear(game_loop);

            let message_to_send = if prepared_dynamic.is_some() {
                provisional_narrative(&result.narrative)
            } else {
                result.narrative.clone()
            };

            let _ = tx.send(message_to_send).await;
            for meta in scene_entry_interruptions(&result.narrative) {
                let _ = tx.send(meta).await;
            }
            if let Some(phase_msg) = result.phase_change {
                if let ServerMessage::PhaseChange { to, .. } = &phase_msg {
                    if let Some(mut session) = state.sessions.get_mut(session_id) {
                        session.game_phase = *to;
                    }
                }
                let _ = tx.send(phase_msg).await;
            }

            if let Some(prepared) = prepared_dynamic {
                let tx_dynamic = tx.clone();
                let db = state.db.clone();
                let session_id = session_id.to_string();
                let scene_id = game_loop.current_scene_id().to_string();
                let fallback_image_prompt = result.image_prompt.clone();
                let dominant_fear_for_image = dominant_fear;
                tokio::spawn(async move {
                    let (final_narrative, image_prompt) = match generate_dynamic_narrative(
                        &db,
                        &prepared,
                    )
                    .await
                    {
                        Some((narrative, image_prompt)) => (narrative, image_prompt),
                        None => (prepared.fallback.clone(), fallback_image_prompt.clone()),
                    };

                    if let ServerMessage::Narrative { text, .. } = &final_narrative {
                        let _ = db.update_latest_scene_history_narrative(
                            &session_id,
                            &scene_id,
                            text,
                            None,
                            None,
                        );
                    }

                    let _ = tx_dynamic.send(final_narrative.clone()).await;

                    let final_scene_id = match &final_narrative {
                        ServerMessage::Narrative { scene_id, .. } => scene_id.clone(),
                        _ => scene_id.clone(),
                    };
                    let image_prompt = effective_image_prompt(
                        &final_narrative,
                        image_prompt,
                        dominant_fear_for_image,
                    );

                    if let Some(url) = generate_image(
                        &db,
                        &session_id,
                        &final_scene_id,
                        &image_prompt,
                        dominant_fear_for_image,
                    )
                    .await
                    {
                        let _ = tx_dynamic
                            .send(ServerMessage::Image {
                                scene_id: final_scene_id,
                                image_url: url,
                                display_mode: fear_engine_common::types::DisplayMode::FadeIn,
                            })
                            .await;
                    }
                });
            } else {
                let image_prompt = effective_image_prompt(
                    &result.narrative,
                    result.image_prompt,
                    dominant_fear,
                );
                let tx_img = tx.clone();
                let scene_id = game_loop.current_scene_id().to_string();
                let db = state.db.clone();
                let dominant_fear_for_image = dominant_fear;
                let session_id = session_id.to_string();
                tokio::spawn(async move {
                    if let Some(url) = generate_image(
                        &db,
                        &session_id,
                        &scene_id,
                        &image_prompt,
                        dominant_fear_for_image,
                    )
                    .await
                    {
                        let _ = tx_img
                            .send(ServerMessage::Image {
                                scene_id,
                                image_url: url,
                                display_mode: fear_engine_common::types::DisplayMode::FadeIn,
                            })
                            .await;
                    }
                });
            }
        }

        ClientMessage::BehaviorBatch { events, .. } => {
            let result = game_loop.process_behavior(events);
            for message in result.messages {
                let _ = tx.send(message).await;
            }
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
                        (text.len() as f64) / (typing_duration_ms as f64 / 1000.0)
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

// ---------------------------------------------------------------------------
// Message builders
// ---------------------------------------------------------------------------

/// Calls the OpenAI DALL-E 3 API and returns a URL to the generated image.
/// Returns `None` silently on any error (graceful degradation).
async fn generate_image(
    db: &fear_engine_storage::Database,
    session_id: &str,
    scene_id: &str,
    prompt: &str,
    dominant_fear: Option<FearType>,
) -> Option<String> {
    if cfg!(test) {
        return None;
    }

    let api_key = std::env::var("OPENAI_API_KEY").ok()?;
    if api_key.is_empty() {
        return None;
    }

    let diversified_prompt = diversify_image_prompt(db, session_id, scene_id, prompt);
    let fear_component = dominant_fear
        .map(|fear| fear.to_string())
        .unwrap_or_else(|| "mixed".into());
    let cache_key = compute_cache_key(&["image", scene_id, &fear_component, &diversified_prompt]);
    if let Ok(Some(entry)) = db.cache_get(&cache_key) {
        if let Ok(cached) = serde_json::from_str::<CachedImage>(&entry.content_json) {
            return Some(cached.image_url);
        }
    }

    let client = ImageClient::new(api_key);
    let result = client
        .generate(&diversified_prompt, dominant_fear.as_ref())
        .await
        .ok()
        .flatten()
        .map(|image| image.data_url);

    if let Some(ref image_url) = result {
        if let Ok(content_json) = serde_json::to_string(&CachedImage {
            image_url: image_url.clone(),
        }) {
            let _ = db.cache_set(&cache_key, "image", &content_json, 3600);
        }
        cache_recent_image_prompt(db, session_id, scene_id, &diversified_prompt);
    }

    result
}

fn diversify_image_prompt(
    db: &fear_engine_storage::Database,
    session_id: &str,
    scene_id: &str,
    prompt: &str,
) -> String {
    let recent_key = compute_cache_key(&["recent_image_prompt", session_id]);
    let recent = db
        .cache_get(&recent_key)
        .ok()
        .flatten()
        .and_then(|entry| serde_json::from_str::<RecentImagePrompt>(&entry.content_json).ok());

    if let Some(previous) = recent {
        if previous.scene_id != scene_id && prompts_too_similar(&previous.prompt, prompt) {
            return format!(
                "{prompt}. Fresh composition directive: different camera distance, different framing, different dominant subject placement, avoid repeating the prior visual layout."
            );
        }
    }

    prompt.to_string()
}

fn cache_recent_image_prompt(
    db: &fear_engine_storage::Database,
    session_id: &str,
    scene_id: &str,
    prompt: &str,
) {
    let recent_key = compute_cache_key(&["recent_image_prompt", session_id]);
    if let Ok(content_json) = serde_json::to_string(&RecentImagePrompt {
        scene_id: scene_id.to_string(),
        prompt: prompt.to_string(),
    }) {
        let _ = db.cache_set(&recent_key, "recent_image_prompt", &content_json, 7200);
    }
}

fn prompts_too_similar(previous: &str, current: &str) -> bool {
    let previous_tokens = prompt_token_set(previous);
    let current_tokens = prompt_token_set(current);
    if previous_tokens.is_empty() || current_tokens.is_empty() {
        return false;
    }

    let overlap = previous_tokens
        .intersection(&current_tokens)
        .count() as f64;
    let denominator = previous_tokens.len().max(current_tokens.len()) as f64;
    let overlap_ratio = overlap / denominator;

    overlap_ratio >= 0.68
        || previous
            .chars()
            .take(160)
            .collect::<String>()
            .eq_ignore_ascii_case(&current.chars().take(160).collect::<String>())
}

fn prompt_token_set(prompt: &str) -> HashSet<String> {
    prompt
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|token| token.len() > 4)
        .map(|token| token.to_ascii_lowercase())
        .collect()
}

async fn generate_dynamic_narrative(
    db: &fear_engine_storage::Database,
    prepared: &PreparedDynamicNarrative,
) -> Option<(ServerMessage, Option<String>)> {
    if cfg!(test) {
        return None;
    }

    let api_key = std::env::var("ANTHROPIC_API_KEY").ok()?;
    if api_key.is_empty() {
        return None;
    }

    if let Ok(Some(entry)) = db.cache_get(&prepared.narrative_cache_key) {
        if let Ok(cached) =
            serde_json::from_str::<CachedDynamicNarrative>(&entry.content_json)
        {
            return Some((cached.narrative, cached.image_prompt));
        }
    }

    let client = ClaudeClient::new(api_key, runtime_ai_config());
    let pipeline = NarrativePipeline::new(client);
    let generated = pipeline.generate(&prepared.prompt_context).await;

    let ServerMessage::Narrative {
        scene_id,
        choices,
        sound_cue,
        effects,
        title,
        act,
        medium,
        trust_posture,
        status_line,
        observation_notes,
        trace_items,
        transcript_lines,
        question_prompts,
        archive_entries,
        mirror_observations,
        surface_label,
        auxiliary_text,
        surface_purpose,
        system_intent,
        active_links,
        ..
    } = &prepared.fallback
    else {
        return None;
    };

    let generated_message = ServerMessage::Narrative {
            scene_id: scene_id.clone(),
            text: generated.narrative,
            atmosphere: generated.atmosphere,
            choices: choices.clone(),
            sound_cue: generated.sound_cue.or_else(|| sound_cue.clone()),
            intensity: generated.intensity,
            effects: effects.clone(),
            title: title.clone(),
            act: *act,
            medium: *medium,
            trust_posture: *trust_posture,
            status_line: status_line.clone(),
            observation_notes: observation_notes.clone(),
            trace_items: trace_items.clone(),
            transcript_lines: if generated.transcript_lines.is_empty() {
                transcript_lines.clone()
            } else {
                generated.transcript_lines.clone()
            },
            question_prompts: if generated.question_prompts.is_empty() {
                question_prompts.clone()
            } else {
                generated.question_prompts.clone()
            },
            archive_entries: if generated.archive_entries.is_empty() {
                archive_entries.clone()
            } else {
                generated.archive_entries.clone()
            },
            mirror_observations: if generated.mirror_observations.is_empty() {
                mirror_observations.clone()
            } else {
                generated.mirror_observations.clone()
            },
            surface_label: surface_label.clone(),
            auxiliary_text: auxiliary_text.clone(),
            surface_purpose: surface_purpose.clone(),
            system_intent: system_intent.clone(),
            active_links: active_links.clone(),
            provisional: false,
        };
    let image_prompt = generated.image_prompt;

    if let Ok(content_json) = serde_json::to_string(&CachedDynamicNarrative {
        narrative: generated_message.clone(),
        image_prompt: image_prompt.clone(),
    }) {
        let _ = db.cache_set(
            &prepared.narrative_cache_key,
            "narrative",
            &content_json,
            3600,
        );
    }

    Some((generated_message, image_prompt))
}

async fn enrich_reveal(
    db: &fear_engine_storage::Database,
    message: &ServerMessage,
) -> ServerMessage {
    let ServerMessage::Reveal {
        fear_profile,
        behavior_profile,
        session_summary,
        key_moments,
        adaptation_log,
        ending_classification,
        ..
    } = message
    else {
        return message.clone();
    };

    let fallback = fallback_reveal_analysis(fear_profile, key_moments, adaptation_log);

    if cfg!(test) {
        return ServerMessage::Reveal {
            fear_profile: fear_profile.clone(),
            behavior_profile: behavior_profile.clone(),
            session_summary: session_summary.clone(),
            key_moments: key_moments.clone(),
            adaptation_log: adaptation_log.clone(),
            ending_classification: *ending_classification,
            analysis: fallback,
        };
    }

    let payload = serde_json::json!({
        "fear_profile": fear_profile,
        "behavior_profile": behavior_profile,
        "session_summary": session_summary,
        "key_moments": key_moments,
        "adaptation_log": adaptation_log,
        "ending_classification": ending_classification,
    });
    let payload_json = payload.to_string();
    let cache_key = compute_cache_key(&["reveal_analysis", &payload_json]);
    if let Ok(Some(entry)) = db.cache_get(&cache_key) {
        if let Ok(cached) = serde_json::from_str::<RevealAnalysis>(&entry.content_json) {
            return ServerMessage::Reveal {
                fear_profile: fear_profile.clone(),
                behavior_profile: behavior_profile.clone(),
                session_summary: session_summary.clone(),
                key_moments: key_moments.clone(),
                adaptation_log: adaptation_log.clone(),
                ending_classification: *ending_classification,
                analysis: cached,
            };
        }
    }

    let Some(api_key) = std::env::var("ANTHROPIC_API_KEY").ok().filter(|key| !key.is_empty()) else {
        return ServerMessage::Reveal {
            fear_profile: fear_profile.clone(),
            behavior_profile: behavior_profile.clone(),
            session_summary: session_summary.clone(),
            key_moments: key_moments.clone(),
            adaptation_log: adaptation_log.clone(),
            ending_classification: *ending_classification,
            analysis: fallback,
        };
    };

    let client = ClaudeClient::new(api_key, reveal_ai_config());
    let request = GenerateRequest {
        system_prompt: "You analyze a player's completed psychological horror session. Use only the supplied data. Do not invent statistics, percentiles, or events. Prioritize behavioral interpretation, consent/withdrawal patterns, and how the system adapted. Mention fear axes only as supporting evidence, never as the sole conclusion. Respond with valid JSON only.".into(),
        messages: vec![ClaudeMessage {
            role: Role::User,
            content: format!(
                "Analyze this completed session and return JSON with exactly these fields:\n\
                 {{\"summary\":\"string\",\"key_patterns\":[\"string\",\"string\",\"string\"],\"adaptation_summary\":\"string\",\"closing_message\":\"string\"}}\n\n\
                 Session data:\n{}",
                payload_json
            ),
        }],
        temperature: 0.4,
    };

    let analysis = match client.generate(&request).await {
        Ok(response) => parse_reveal_analysis(&response.content).unwrap_or(fallback.clone()),
        Err(_) => fallback.clone(),
    };

    if let Ok(content_json) = serde_json::to_string(&analysis) {
        let _ = db.cache_set(&cache_key, "reveal_analysis", &content_json, 3600);
    }

    ServerMessage::Reveal {
        fear_profile: fear_profile.clone(),
        behavior_profile: behavior_profile.clone(),
        session_summary: session_summary.clone(),
        key_moments: key_moments.clone(),
        adaptation_log: adaptation_log.clone(),
        ending_classification: *ending_classification,
        analysis,
    }
}

fn fallback_reveal_analysis(
    fear_profile: &fear_engine_common::types::FearProfileSummary,
    key_moments: &[fear_engine_common::types::KeyMoment],
    adaptation_log: &[fear_engine_common::types::AdaptationRecord],
) -> RevealAnalysis {
    let summary = format!(
        "Across {} real observations, the session formed its verdict by tracking how you handled observation, uncertainty, refusal, and continued exposure. {} emerged as the strongest thematic trigger, but the more important pattern was the style of behavior you produced while the system kept adapting around you.",
        fear_profile.total_observations,
        fear_profile.primary_fear
    );

    let key_patterns = if key_moments.is_empty() {
        vec![
            "The final verdict was built from real in-session choices, pauses, and interaction timing.".into(),
            "The system treated consent, refusal, and continued attention as meaningful signals rather than optional flavor.".into(),
            format!("{} remained the clearest thematic trigger, but only after the session had already classified your behavior style.", fear_profile.primary_fear),
        ]
    } else {
        key_moments
            .iter()
            .take(3)
            .map(|moment| moment.description.clone())
            .collect()
    };

    let adaptation_summary = if let Some(adaptation) = adaptation_log.last() {
        format!(
            "The adaptation layer leaned hardest on {} while shaping later beats around {} at roughly {:.0}% intensity.",
            adaptation.strategy,
            adaptation.fear_targeted,
            adaptation.intensity * 100.0
        )
    } else {
        "The adaptation layer reacted to the session with lower confidence, so the escalation pattern stayed conservative.".into()
    };

    let closing_message = format!(
        "By the end of this run, the system believed your behavior was more revealing than any single scare theme."
    );

    RevealAnalysis {
        summary,
        key_patterns,
        adaptation_summary,
        closing_message,
    }
}

fn parse_reveal_analysis(raw: &str) -> Option<RevealAnalysis> {
    let trimmed = raw.trim();
    let json_str = if trimmed.starts_with("```") {
        trimmed
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim()
    } else {
        trimmed
    };

    serde_json::from_str(json_str).ok()
}

fn runtime_ai_config() -> ClientConfig {
    ClientConfig {
        timeout: std::time::Duration::from_secs(10),
        max_retries: 1,
        base_retry_delay: std::time::Duration::from_millis(400),
        max_retry_delay: std::time::Duration::from_secs(2),
    }
}

fn reveal_ai_config() -> ClientConfig {
    ClientConfig {
        timeout: std::time::Duration::from_secs(8),
        max_retries: 0,
        base_retry_delay: std::time::Duration::from_millis(250),
        max_retry_delay: std::time::Duration::from_secs(1),
    }
}

fn build_dynamic_cache_key(
    game_loop: &SessionGameLoop,
    dynamic_context: &str,
    last_choice: &str,
) -> String {
    let phase = game_loop.current_phase().to_string();
    let fear_signature = game_loop
        .fear_profile()
        .top_fears(3, 0.0)
        .into_iter()
        .map(|(fear, score)| format!("{fear}:{score:.2}"))
        .collect::<Vec<_>>()
        .join("|");

    compute_cache_key(&[
        "narrative",
        game_loop.current_scene_id(),
        dynamic_context,
        last_choice,
        &phase,
        &fear_signature,
    ])
}

fn effective_image_prompt(
    narrative: &ServerMessage,
    explicit_image_prompt: Option<String>,
    dominant_fear: Option<FearType>,
) -> String {
    match narrative {
        ServerMessage::Narrative {
            scene_id,
            text,
            atmosphere,
            title,
            act,
            medium,
            trust_posture,
            intensity,
            ..
        } => {
            let base = explicit_image_prompt.unwrap_or_else(|| {
                text.chars().take(240).collect::<String>()
            });
            let scene_label = title
                .clone()
                .unwrap_or_else(|| scene_id.replace('_', " "));
            let act_label = act
                .map(display_act)
                .unwrap_or("session beat");
            let medium_label = medium
                .map(display_medium)
                .unwrap_or("chat");
            let posture_label = trust_posture
                .map(display_trust_posture)
                .unwrap_or("helpful");
            let fear_label = dominant_fear
                .map(|fear| fear.to_string().replace('_', " "))
                .unwrap_or_else(|| "mixed pattern".into());

            format!(
                "{base}. Scene framing: {scene_label}. Surface: {medium_label}. Act: {act_label}. Atmosphere: {:?}. Trust posture: {posture_label}. Dominant fear pattern: {fear_label}. Intensity: {:.0} percent. Distinct composition, rich environmental detail, no captions, no interface text, no watermark.",
                atmosphere,
                intensity * 100.0
            )
        }
        _ => "elegant psychological session surface, dark minimal architecture, unsettling atmosphere".into(),
    }
}

fn dominant_visual_fear(game_loop: &SessionGameLoop) -> Option<FearType> {
    game_loop
        .fear_profile()
        .top_fears(1, 0.0)
        .into_iter()
        .next()
        .map(|(fear, _)| fear)
}

fn display_act(act: fear_engine_common::types::SessionAct) -> &'static str {
    match act {
        fear_engine_common::types::SessionAct::Invitation => "invitation",
        fear_engine_common::types::SessionAct::Calibration => "calibration",
        fear_engine_common::types::SessionAct::Accommodation => "accommodation",
        fear_engine_common::types::SessionAct::Contamination => "contamination",
        fear_engine_common::types::SessionAct::PerformanceCollapse => "performance collapse",
        fear_engine_common::types::SessionAct::Verdict => "verdict",
    }
}

fn display_medium(medium: fear_engine_common::types::SurfaceMedium) -> &'static str {
    match medium {
        fear_engine_common::types::SurfaceMedium::Chat => "chat",
        fear_engine_common::types::SurfaceMedium::Questionnaire => "questionnaire",
        fear_engine_common::types::SurfaceMedium::Archive => "archive",
        fear_engine_common::types::SurfaceMedium::Transcript => "transcript",
        fear_engine_common::types::SurfaceMedium::Webcam => "webcam",
        fear_engine_common::types::SurfaceMedium::Microphone => "microphone",
        fear_engine_common::types::SurfaceMedium::SystemDialog => "system dialog",
        fear_engine_common::types::SurfaceMedium::Mirror => "mirror",
    }
}

fn display_trust_posture(posture: fear_engine_common::types::TrustPosture) -> &'static str {
    match posture {
        fear_engine_common::types::TrustPosture::Helpful => "helpful",
        fear_engine_common::types::TrustPosture::Curious => "curious",
        fear_engine_common::types::TrustPosture::Clinical => "clinical",
        fear_engine_common::types::TrustPosture::Manipulative => "manipulative",
        fear_engine_common::types::TrustPosture::Confessional => "confessional",
        fear_engine_common::types::TrustPosture::Hostile => "hostile",
    }
}

fn provisional_narrative(message: &ServerMessage) -> ServerMessage {
    match message {
        ServerMessage::Narrative {
            scene_id,
            text,
            atmosphere,
            title,
            act,
            medium,
            trust_posture,
            status_line,
            observation_notes,
            trace_items,
            transcript_lines,
            question_prompts,
            archive_entries,
            mirror_observations,
            surface_label,
            auxiliary_text,
            surface_purpose,
            system_intent,
            active_links,
            sound_cue,
            intensity,
            effects,
            ..
        } => ServerMessage::Narrative {
            scene_id: scene_id.clone(),
            text: text.clone(),
            atmosphere: *atmosphere,
            choices: vec![],
            sound_cue: sound_cue.clone(),
            intensity: *intensity,
            effects: effects.clone(),
            title: title.clone(),
            act: *act,
            medium: *medium,
            trust_posture: *trust_posture,
            status_line: status_line.clone(),
            observation_notes: observation_notes.clone(),
            trace_items: trace_items.clone(),
            transcript_lines: transcript_lines.clone(),
            question_prompts: question_prompts.clone(),
            archive_entries: archive_entries.clone(),
            mirror_observations: mirror_observations.clone(),
            surface_label: surface_label.clone(),
            auxiliary_text: auxiliary_text.clone(),
            surface_purpose: surface_purpose.clone(),
            system_intent: system_intent.clone(),
            active_links: active_links.clone(),
            provisional: true,
        },
        other => other.clone(),
    }
}

fn build_prompt_context(
    game_loop: &mut SessionGameLoop,
    dynamic_context: &str,
    last_choice: &str,
    surface: &ServerMessage,
) -> PromptContext {
    let (top_fears, anxiety_threshold, curiosity_vs_avoidance) = {
        let fear_profile = game_loop.fear_profile();
        let meta = fear_profile.meta_patterns();
        let top_fears = fear_profile
            .top_fears(3, 0.0)
            .into_iter()
            .map(|(fear, score)| {
                (fear, score, fear_profile.confidence_level(&fear))
            })
            .collect();
        (
            top_fears,
            meta.anxiety_threshold,
            meta.curiosity_vs_avoidance,
        )
    };
    let adaptation = game_loop.adaptation_directive();
    let (medium, trust_posture) = match surface {
        ServerMessage::Narrative {
            medium,
            trust_posture,
            ..
        } => (*medium, *trust_posture),
        _ => (None, None),
    };

    PromptContext {
        fear_profile: FearProfileContext {
            top_fears,
            anxiety_threshold,
            behavioral_pattern: behavioral_pattern(curiosity_vs_avoidance),
            estimated_emotional_state: emotional_state(anxiety_threshold),
        },
        game_state: GameStateContext {
            location: game_loop.current_scene_id().to_string(),
            phase: game_loop.current_phase(),
            medium,
            trust_posture,
            scene_number: game_loop.total_scenes().max(1),
            last_scene_summary: dynamic_context.to_string(),
            last_choice: last_choice.to_string(),
            active_threads: vec![dynamic_context.to_string()],
            inventory: vec![],
            established_details: vec![],
        },
        adaptation,
    }
}

fn behavioral_pattern(curiosity_vs_avoidance: f64) -> String {
    if curiosity_vs_avoidance >= 0.65 {
        "curious explorer".into()
    } else if curiosity_vs_avoidance <= 0.35 {
        "avoidant survivor".into()
    } else {
        "cautious explorer".into()
    }
}

fn emotional_state(anxiety_threshold: f64) -> String {
    if anxiety_threshold >= 0.75 {
        "panicked".into()
    } else if anxiety_threshold >= 0.55 {
        "tense".into()
    } else {
        "guarded".into()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CachedDynamicNarrative {
    narrative: ServerMessage,
    image_prompt: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CachedImage {
    image_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RecentImagePrompt {
    scene_id: String,
    prompt: String,
}

#[derive(Debug, Clone)]
struct PreparedDynamicNarrative {
    fallback: ServerMessage,
    prompt_context: PromptContext,
    narrative_cache_key: String,
}

fn welcome_message() -> ServerMessage {
    ServerMessage::Narrative {
        scene_id: "welcome".into(),
        text: "The screen flickers to life. A cold, clinical light fills the room.\n\n\
               Welcome to the Fear Engine.\n\n\
               Press START to begin."
            .into(),
        atmosphere: fear_engine_common::types::Atmosphere::Tension,
        choices: vec![],
        sound_cue: Some("static_hum".into()),
        intensity: 0.1,
        effects: vec![],
        title: None,
        act: None,
        medium: None,
        trust_posture: None,
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
    }
}

fn error_msg(code: &str, message: &str, recoverable: bool) -> ServerMessage {
    ServerMessage::Error {
        code: code.into(),
        message: message.into(),
        recoverable,
    }
}

fn text_msg(msg: &ServerMessage) -> Message {
    let json = encode_server_message(msg).unwrap_or_else(|_| "{}".into());
    Message::Text(json.into())
}

fn scene_entry_interruptions(message: &ServerMessage) -> Vec<ServerMessage> {
    let ServerMessage::Narrative {
        scene_id,
        act,
        medium,
        provisional,
        ..
    } = message
    else {
        return vec![];
    };

    if *provisional {
        return vec![];
    }

    let mut messages = Vec::new();

    match scene_id.as_str() {
        "cal_awakening" => messages.push(ServerMessage::Meta {
            text: "The session has already started timing how long you take to continue.".into(),
            target: MetaTarget::Overlay,
            delay_ms: 250,
        }),
        "cal_corridor" => messages.push(ServerMessage::Meta {
            text: "NO PATH IS NEUTRAL".into(),
            target: MetaTarget::Title,
            delay_ms: 220,
        }),
        "cal_reception" => messages.push(ServerMessage::Meta {
            text: "Once a live link is offered, refusal becomes content too.".into(),
            target: MetaTarget::Overlay,
            delay_ms: 200,
        }),
        "probe_sound" | "beat_silence_return" => messages.push(ServerMessage::Meta {
            text: "It kept the silence because it knew you might return to it.".into(),
            target: MetaTarget::Whisper,
            delay_ms: 120,
        }),
        "beat_false_exit" => messages.push(ServerMessage::Meta {
            text: "EXIT IS NOW JUST ANOTHER RESPONSE TYPE".into(),
            target: MetaTarget::Title,
            delay_ms: 180,
        }),
        "tmpl_meta_moment" => messages.push(ServerMessage::Meta {
            text: "The session is no longer separating observation from performance.".into(),
            target: MetaTarget::Overlay,
            delay_ms: 120,
        }),
        _ => {}
    }

    if matches!(act, Some(fear_engine_common::types::SessionAct::PerformanceCollapse))
        && matches!(medium, Some(fear_engine_common::types::SurfaceMedium::SystemDialog))
    {
        messages.push(ServerMessage::Meta {
            text: "It wants your attention badly enough to take the whole surface for itself.".into(),
            target: MetaTarget::GlitchText,
            delay_ms: 280,
        });
    }

    messages
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{build_app, AppState};
    use axum::body::Body;
    use fear_engine_storage::Database;
    use futures_util::{SinkExt, StreamExt};
    use http_body_util::BodyExt;
    use tokio::net::TcpListener;
    use tokio_tungstenite::tungstenite;
    use tower::ServiceExt;
    use tower_http::cors::CorsLayer;

    async fn spawn_test_server() -> (std::net::SocketAddr, Arc<AppState>) {
        let db = Database::new_in_memory().unwrap();
        let state = Arc::new(AppState::new(db));
        let app = build_app(state.clone(), CorsLayer::permissive());
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        (addr, state)
    }

    async fn ws_connect(
        addr: std::net::SocketAddr,
    ) -> (
        futures_util::stream::SplitSink<
            tokio_tungstenite::WebSocketStream<
                tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
            >,
            tungstenite::Message,
        >,
        futures_util::stream::SplitStream<
            tokio_tungstenite::WebSocketStream<
                tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
            >,
        >,
    ) {
        let url = format!("ws://{addr}/ws");
        let (stream, _) = tokio_tungstenite::connect_async(&url).await.unwrap();
        stream.split()
    }

    async fn recv_server_msg(
        read: &mut futures_util::stream::SplitStream<
            tokio_tungstenite::WebSocketStream<
                tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
            >,
        >,
    ) -> ServerMessage {
        let msg = tokio::time::timeout(std::time::Duration::from_secs(5), read.next())
            .await
            .expect("timed out waiting for message")
            .expect("stream ended")
            .expect("ws error");
        match msg {
            tungstenite::Message::Text(text) => {
                serde_json::from_str::<ServerMessage>(&text).expect("invalid server message JSON")
            }
            other => panic!("expected text message, got {other:?}"),
        }
    }

    async fn recv_until_narrative(
        read: &mut futures_util::stream::SplitStream<
            tokio_tungstenite::WebSocketStream<
                tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
            >,
        >,
    ) -> ServerMessage {
        loop {
            let msg = recv_server_msg(read).await;
            if matches!(msg, ServerMessage::Narrative { .. }) {
                return msg;
            }
        }
    }

    #[tokio::test]
    async fn test_health_endpoint_returns_200() {
        let db = Database::new_in_memory().unwrap();
        let state = Arc::new(AppState::new(db));
        let app = build_app(state, CorsLayer::permissive());
        let req = axum::http::Request::builder()
            .uri("/health")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), 200);
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(json["status"], "ok");
    }

    #[tokio::test]
    async fn test_cors_headers_present() {
        let mut config = fear_engine_common::config::AppConfig::default();
        config.frontend_url = "http://localhost:5173".into();
        let cors = crate::middleware::cors::build_cors(&config);
        let db = Database::new_in_memory().unwrap();
        let state = Arc::new(AppState::new(db));
        let app = build_app(state, cors);
        let req = axum::http::Request::builder()
            .uri("/health")
            .header("Origin", "http://localhost:5173")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert!(resp.headers().contains_key("access-control-allow-origin"));
    }

    #[tokio::test]
    async fn test_websocket_connects_successfully() {
        let (addr, _state) = spawn_test_server().await;
        let (_write, _read) = ws_connect(addr).await;
    }

    #[tokio::test]
    async fn test_websocket_receives_welcome_on_connect() {
        let (addr, _state) = spawn_test_server().await;
        let (_write, mut read) = ws_connect(addr).await;
        let msg = recv_server_msg(&mut read).await;
        match msg {
            ServerMessage::Narrative { scene_id, text, .. } => {
                assert_eq!(scene_id, "welcome");
                assert!(text.contains("Fear Engine"));
            }
            other => panic!("expected Narrative, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_start_game_sends_first_scene() {
        let (addr, _state) = spawn_test_server().await;
        let (mut write, mut read) = ws_connect(addr).await;
        let _ = recv_server_msg(&mut read).await; // welcome

        let start = serde_json::json!({
            "type": "start_game",
            "payload": { "player_name": null }
        });
        write
            .send(tungstenite::Message::Text(start.to_string().into()))
            .await
            .unwrap();

        let msg = recv_server_msg(&mut read).await;
        match msg {
            ServerMessage::Narrative { scene_id, choices, .. } => {
                assert_eq!(scene_id, "cal_awakening");
                assert!(!choices.is_empty());
            }
            other => panic!("expected Narrative with choices, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_choice_advances_to_next_scene() {
        let (addr, _state) = spawn_test_server().await;
        let (mut write, mut read) = ws_connect(addr).await;
        let _ = recv_server_msg(&mut read).await; // welcome

        // Start game
        write
            .send(tungstenite::Message::Text(
                serde_json::json!({"type":"start_game","payload":{"player_name":null}})
                    .to_string()
                    .into(),
            ))
            .await
            .unwrap();
        let _ = recv_until_narrative(&mut read).await; // cal_awakening

        // Make a choice
        write
            .send(tungstenite::Message::Text(
                serde_json::json!({"type":"choice","payload":{"scene_id":"cal_awakening","choice_id":"sit_up","time_to_decide_ms":1500,"approach":"investigate"}})
                    .to_string()
                    .into(),
            ))
            .await
            .unwrap();

        let msg = recv_until_narrative(&mut read).await;
        match msg {
            ServerMessage::Narrative { scene_id, choices, .. } => {
                assert_eq!(scene_id, "cal_corridor");
                assert!(!choices.is_empty(), "next scene should have choices");
            }
            other => panic!("expected Narrative for cal_corridor, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_invalid_json_returns_error() {
        let (addr, _state) = spawn_test_server().await;
        let (mut write, mut read) = ws_connect(addr).await;
        let _ = recv_server_msg(&mut read).await;
        write
            .send(tungstenite::Message::Text("not valid json".into()))
            .await
            .unwrap();
        let msg = recv_server_msg(&mut read).await;
        assert!(matches!(msg, ServerMessage::Error { .. }));
    }

    #[tokio::test]
    async fn test_disconnect_cleans_up_session() {
        let (addr, state) = spawn_test_server().await;
        let (write, mut read) = ws_connect(addr).await;
        let _ = recv_server_msg(&mut read).await;
        assert_eq!(state.sessions.len(), 1);
        drop(write);
        drop(read);
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        assert_eq!(state.sessions.len(), 0);
    }

    #[tokio::test]
    async fn test_multiple_concurrent_connections() {
        let (addr, state) = spawn_test_server().await;
        let mut connections = Vec::new();
        for _ in 0..5 {
            let (write, mut read) = ws_connect(addr).await;
            let _ = recv_server_msg(&mut read).await;
            connections.push((write, read));
        }
        assert_eq!(state.sessions.len(), 5);
        drop(connections);
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
        assert_eq!(state.sessions.len(), 0);
    }
}
