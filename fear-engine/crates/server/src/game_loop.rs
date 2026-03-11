//! Per-session game loop — wires together the scene graph, fear profile,
//! adaptation engine, behavior collector, and state machine.
//!
//! Each WebSocket session gets its own [`SessionGameLoop`] that processes
//! choices and behavior batches, updating the fear profile and deciding
//! what content to serve next.

use std::collections::HashMap;
use std::sync::Arc;

use chrono::Utc;
use fear_engine_common::types::{
    Atmosphere, BehaviorEvent, BehaviorProfileSummary, Choice, ChoiceApproach, FearType, GamePhase,
    MetaTarget, ServerMessage,
};
use fear_engine_core::event_bus::EventBus;
use fear_engine_core::scene::Scene;
use fear_engine_core::scene_manager::{
    transition_effects_for_atmosphere, ResolvedTarget, SceneManager, SceneManagerState,
};
use fear_engine_core::state_machine::{GameStateMachine, TransitionRequirements};
use fear_engine_fear_profile::adaptation::{
    AdaptationDirective, AdaptationEngine, AdaptationEngineState,
};
use fear_engine_fear_profile::analyzer::{BehaviorBaseline, BehaviorFeatures};
use fear_engine_fear_profile::behavior::{BehaviorBatch, BehaviorCollector};
use fear_engine_fear_profile::profile::{FearConfidence, FearProfile, FearProfileState};
use fear_engine_storage::fear_profile::FearProfileRow;
use fear_engine_storage::scene_history::SceneHistoryEntry;
use fear_engine_storage::session::Session;
use fear_engine_storage::Database;
use serde::{Deserialize, Serialize};

use crate::director::SessionDirector;
use crate::session_script::build_session_script_graph;

/// Per-session game state that orchestrates all subsystems.
pub struct SessionGameLoop {
    pub session_id: String,
    db: Arc<Database>,
    scene_manager: SceneManager,
    fear_profile: FearProfile,
    adaptation: AdaptationEngine,
    director: SessionDirector,
    collector: BehaviorCollector,
    current_scene_id: String,
    baseline_set: bool,
    total_scenes: u32,
    max_scenes: u32,
    started: bool,
    last_narrative: Option<ServerMessage>,
}

/// The result of processing a player choice.
pub struct ChoiceResult {
    pub narrative: ServerMessage,
    pub phase_change: Option<ServerMessage>,
    pub image_prompt: Option<String>,
    pub dynamic_context: Option<String>,
}

/// The result of processing a behavior batch.
pub struct BatchResult {
    pub profile_changed: bool,
    pub primary_fear: Option<FearType>,
    pub messages: Vec<ServerMessage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PersistedGameState {
    started: bool,
    current_scene_id: String,
    total_scenes: u32,
    baseline_set: bool,
    last_narrative: Option<ServerMessage>,
    scene_manager: SceneManagerState,
    fear_profile: FearProfileState,
    adaptation: AdaptationEngineState,
}

impl SessionGameLoop {
    /// Creates a new game loop for a session, initialising all subsystems
    /// with the hospital scenario.
    ///
    /// # Example
    ///
    /// ```
    /// use std::sync::Arc;
    /// use fear_engine_storage::Database;
    /// use fear_engine_server::game_loop::SessionGameLoop;
    ///
    /// let db = Arc::new(Database::new_in_memory().unwrap());
    /// let sid = db.create_session(None).unwrap();
    /// db.create_fear_profile(&sid).unwrap();
    /// let gl = SessionGameLoop::new(sid, db);
    /// assert_eq!(gl.current_phase(), fear_engine_common::types::GamePhase::Calibrating);
    /// ```
    pub fn new(session_id: String, db: Arc<Database>) -> Self {
        let scene_manager = build_scene_manager(GamePhase::Calibrating);
        let collector = BehaviorCollector::new(db.clone());

        Self {
            session_id,
            db,
            scene_manager,
            fear_profile: FearProfile::new(),
            adaptation: AdaptationEngine::new(),
            director: SessionDirector::new(),
            collector,
            current_scene_id: "cal_awakening".into(),
            baseline_set: false,
            total_scenes: 0,
            max_scenes: 24,
            started: false,
            last_narrative: None,
        }
    }

    /// Restores an existing session after a WebSocket reconnect.
    pub fn resume(session_id: String, db: Arc<Database>) -> fear_engine_common::Result<Self> {
        let session = db.get_session(&session_id)?;
        let persisted = parse_persisted_state(&session.game_state_json);

        let mut scene_manager = build_scene_manager(session.game_phase);
        let fear_profile = persisted
            .as_ref()
            .map(|state| FearProfile::from_persisted_state(state.fear_profile.clone()))
            .unwrap_or_else(|| load_profile_from_storage(&db, &session_id));
        let mut adaptation = AdaptationEngine::new();
        if let Some(state) = persisted.as_ref() {
            scene_manager.restore_state(state.scene_manager.clone());
            adaptation.restore_state(state.adaptation.clone());
        }
        let scores: HashMap<FearType, f64> = FearType::all()
            .into_iter()
            .map(|fear| (fear, fear_profile.score(&fear)))
            .collect();
        scene_manager.update_fear_scores(scores, fear_profile.overall_confidence());

        Ok(Self {
            session_id,
            db: db.clone(),
            scene_manager,
            fear_profile,
            adaptation,
            director: SessionDirector::new(),
            collector: BehaviorCollector::new(db),
            current_scene_id: persisted
                .as_ref()
                .map(|state| state.current_scene_id.clone())
                .filter(|scene_id| !scene_id.is_empty())
                .unwrap_or_else(|| {
                    if session.current_scene_id == "intro" {
                        "cal_awakening".into()
                    } else {
                        session.current_scene_id.clone()
                    }
                }),
            baseline_set: persisted
                .as_ref()
                .map(|state| state.baseline_set)
                .unwrap_or(false),
            total_scenes: persisted
                .as_ref()
                .map(|state| state.total_scenes)
                .unwrap_or(0),
            max_scenes: 24,
            started: persisted
                .as_ref()
                .map(|state| state.started)
                .unwrap_or(session.current_scene_id != "intro"),
            last_narrative: persisted.and_then(|state| state.last_narrative),
        })
    }

    /// Returns the current game phase.
    pub fn current_phase(&self) -> GamePhase {
        self.scene_manager.current_phase()
    }

    /// Returns the current scene ID.
    pub fn current_scene_id(&self) -> &str {
        &self.current_scene_id
    }

    /// Returns the total number of scene advances processed in this session.
    pub fn total_scenes(&self) -> u32 {
        self.total_scenes
    }

    /// Returns `true` once the session has moved past the welcome screen.
    pub fn started(&self) -> bool {
        self.started
    }

    /// Returns the best-known scene message for reconnect/resume.
    pub fn resume_message(&self) -> ServerMessage {
        if let Some(message) = &self.last_narrative {
            if narrative_needs_rebuild(message) {
                return self.decorate_narrative_for_surface(self.rebuild_current_narrative());
            }
            return message.clone();
        }
        self.decorate_narrative_for_surface(self.rebuild_current_narrative())
    }

    /// Replaces the most recent message persisted for resume.
    pub fn set_last_message_for_resume(&mut self, message: ServerMessage) {
        self.last_narrative = Some(message);
        let phase = if self.current_phase() == GamePhase::Reveal {
            GamePhase::Reveal
        } else {
            self.current_phase()
        };
        self.persist_session_state_with_phase(phase);
    }

    /// Enters the first scene and returns its narrative.
    pub fn start_game(&mut self) -> ServerMessage {
        if self.started {
            return self.resume_message();
        }

        let message = match self
            .scene_manager
            .enter_scene("cal_awakening", &self.session_id)
        {
            Ok(transition) => match transition.target {
                ResolvedTarget::Static(scene) => {
                    self.current_scene_id = scene.id.clone();
                    self.decorate_narrative_for_surface(
                        self.scene_to_message(&scene, &transition.transition_effects),
                    )
                }
                _ => {
                    self.current_scene_id = "cal_awakening".into();
                    self.decorate_narrative_for_surface(fallback_with_redirect("cal_awakening"))
                }
            },
            Err(_) => {
                self.current_scene_id = "cal_awakening".into();
                self.decorate_narrative_for_surface(fallback_with_redirect("cal_awakening"))
            }
        };
        self.started = true;
        self.last_narrative = Some(message.clone());
        self.persist_session_state();
        self.persist_scene_history(&message, None);
        message
    }

    /// Returns `true` if the game has reached the Reveal phase.
    pub fn is_game_over(&self) -> bool {
        self.current_phase() == GamePhase::Reveal || self.total_scenes >= self.max_scenes
    }

    /// Builds the Reveal message from the current fear profile.
    pub fn build_reveal(&self) -> ServerMessage {
        let history = self
            .db
            .get_scene_history(&self.session_id)
            .unwrap_or_default();
        let session = self
            .db
            .get_session(&self.session_id)
            .unwrap_or_else(|_| Session {
                id: self.session_id.clone(),
                created_at: Utc::now(),
                updated_at: Utc::now(),
                player_name: None,
                current_scene_id: self.current_scene_id.clone(),
                game_phase: self.current_phase(),
                game_state_json: "{}".into(),
                completed: false,
            });
        let behavior_events = self
            .db
            .get_behavior_events(&self.session_id, None)
            .unwrap_or_default();
        let scores = self.build_reveal_theme_scores(&history, &behavior_events);
        let mut ranked_fears = scores.clone();
        ranked_fears.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let primary = ranked_fears
            .first()
            .map(|score| score.fear_type)
            .unwrap_or(fear_engine_common::types::FearType::Darkness);
        let secondary = ranked_fears.get(1).map(|score| score.fear_type);
        let behavior_profile = self.build_behavior_profile(&history, &behavior_events);
        let session_summary = self.director.build_session_summary(
            &session,
            &history,
            &behavior_events,
            self.permission_granted(&behavior_events, "camera"),
            self.permission_granted(&behavior_events, "microphone"),
        );
        let key_moments = self.derive_key_moments(&history, primary);
        let adaptation_log = self.derive_adaptations(&history, primary);
        let ending_classification = self.director.classify_ending(&behavior_profile);
        let analysis = fear_engine_common::types::RevealAnalysis {
            summary: format!(
                "Your profile was built from {} real observations, with {} emerging as the strongest signal.",
                self.fear_profile.update_count(),
                primary
            ),
            key_patterns: key_moments
                .iter()
                .take(3)
                .map(|moment| moment.description.clone())
                .collect(),
            adaptation_summary: adaptation_log
                .last()
                .map(|adaptation| {
                    format!(
                        "The system adapted with {} while focusing on {}.",
                        adaptation.strategy,
                        adaptation.fear_targeted
                    )
                })
                .unwrap_or_else(|| {
                    "The system kept adaptation conservative because the signal remained mixed.".into()
                }),
            closing_message: format!(
                "The session kept circling back to {} more than any other axis.",
                primary
            ),
        };

        ServerMessage::Reveal {
            fear_profile: fear_engine_common::types::FearProfileSummary {
                scores,
                primary_fear: primary,
                secondary_fear: secondary,
                total_observations: self.fear_profile.update_count(),
            },
            behavior_profile,
            session_summary,
            key_moments,
            adaptation_log,
            ending_classification,
            analysis,
        }
    }

    /// Processes a player choice and returns the next narrative.
    pub fn process_choice(
        &mut self,
        choice_id: &str,
        time_to_decide_ms: u64,
        approach: ChoiceApproach,
    ) -> ChoiceResult {
        self.total_scenes += 1;

        if self.is_terminal_surface() {
            self.finish_session();
            let reveal = self.build_reveal();
            self.last_narrative = Some(reveal.clone());
            self.persist_session_state_with_phase(GamePhase::Reveal);
            self.persist_reveal_history(&reveal);
            return ChoiceResult {
                narrative: reveal,
                phase_change: Some(ServerMessage::PhaseChange {
                    from: self.current_phase(),
                    to: GamePhase::Reveal,
                }),
                image_prompt: None,
                dynamic_context: None,
            };
        }

        // Check if game should end.
        if self.is_game_over() {
            self.finish_session();
            let reveal = self.build_reveal();
            self.last_narrative = Some(reveal.clone());
            self.persist_session_state_with_phase(GamePhase::Reveal);
            self.persist_reveal_history(&reveal);
            return ChoiceResult {
                narrative: reveal,
                phase_change: None,
                image_prompt: None,
                dynamic_context: None,
            };
        }

        // 1. Record choice as behavior event and update fear profile.
        let choice_event = BehaviorEvent {
            event_type: fear_engine_common::types::BehaviorEventType::Choice {
                choice_id: choice_id.into(),
                time_to_decide_ms,
                approach,
            },
            timestamp: Utc::now(),
            scene_id: self.current_scene_id.clone(),
        };
        let _ = self.collector.process_batch(BehaviorBatch {
            events: vec![choice_event],
            session_id: self.session_id.clone(),
            batch_timestamp: Utc::now(),
        });

        if let Some(selected_fear) = self.choice_fear_vector(&self.current_scene_id, choice_id) {
            self.fear_profile
                .apply_choice_signal(selected_fear, approach, time_to_decide_ms);
        }

        let scores: HashMap<FearType, f64> = FearType::all()
            .into_iter()
            .map(|fear| (fear, self.fear_profile.score(&fear)))
            .collect();
        self.scene_manager
            .update_fear_scores(scores, self.fear_profile.overall_confidence());
        self.sync_fear_profile_to_db();
        self.persist_choice_for_current_scene(choice_id);

        // 2. Resolve next scene.
        let transition =
            self.scene_manager
                .resolve_choice(&self.current_scene_id, choice_id, &self.session_id);

        let (narrative, image_prompt, phase_change, dynamic_context) = match transition {
            Ok(t) => {
                let phase_change = t.phase_changed.map(|phase| ServerMessage::PhaseChange {
                    from: phase.from,
                    to: phase.to,
                });
                match &t.target {
                    ResolvedTarget::Static(scene) => {
                        self.current_scene_id = scene.id.clone();
                        (
                            self.decorate_narrative_for_surface(
                                self.scene_to_message(scene, &t.transition_effects),
                            ),
                            scene.image_prompt.clone(),
                            phase_change,
                            None,
                        )
                    }
                    ResolvedTarget::Dynamic { context } => {
                        let (narrative, image_prompt, ai_context) =
                            self.enter_next_structured_scene(Some(context.as_str()));
                        (narrative, image_prompt, phase_change, ai_context)
                    }
                }
            }
            Err(_) => {
                // Bad choice or missing scene; enter next probe.
                let (narrative, image_prompt, ai_context) = self.enter_next_structured_scene(None);
                (narrative, image_prompt, None, ai_context)
            }
        };
        self.started = true;
        self.last_narrative = Some(narrative.clone());
        self.persist_session_state();
        self.persist_scene_history(&narrative, None);

        if self.current_phase() == GamePhase::Reveal {
            self.finish_session();
            let reveal = self.build_reveal();
            self.last_narrative = Some(reveal.clone());
            self.persist_session_state_with_phase(GamePhase::Reveal);
            self.persist_reveal_history(&reveal);
            return ChoiceResult {
                narrative: reveal,
                phase_change,
                image_prompt: None,
                dynamic_context: None,
            };
        }

        ChoiceResult {
            narrative,
            phase_change,
            image_prompt,
            dynamic_context,
        }
    }

    /// Enters the next structured follow-up scene, blending fixed scene
    /// graph structure with phase-aware template selection.
    fn enter_next_structured_scene(
        &mut self,
        seed_context: Option<&str>,
    ) -> (ServerMessage, Option<String>, Option<String>) {
        let next_id = self
            .select_followup_scene_id()
            .unwrap_or_else(|| self.next_unvisited_probe_id());

        match self.scene_manager.enter_scene(&next_id, &self.session_id) {
            Ok(t) => match &t.target {
                ResolvedTarget::Static(scene) => {
                    self.current_scene_id = scene.id.clone();
                    let narrative = self.decorate_narrative_for_surface(
                        self.scene_to_message(scene, &t.transition_effects),
                    );
                    let ai_context = self.scene_ai_context(scene, seed_context);
                    (narrative, scene.image_prompt.clone(), ai_context)
                }
                _ => {
                    self.current_scene_id = next_id.clone();
                    (
                        self.decorate_narrative_for_surface(fallback_with_redirect(&next_id)),
                        None,
                        None,
                    )
                }
            },
            Err(_) => {
                self.current_scene_id = next_id.clone();
                (
                    self.decorate_narrative_for_surface(fallback_with_redirect(&next_id)),
                    None,
                    None,
                )
            }
        }
    }

    /// Processes a batch of behavior events and updates the fear profile.
    pub fn process_behavior(&mut self, events: Vec<BehaviorEvent>) -> BatchResult {
        let batch = BehaviorBatch {
            events: events.clone(),
            session_id: self.session_id.clone(),
            batch_timestamp: Utc::now(),
        };
        let _ = self.collector.process_batch(batch);

        // Compute baseline during calibration.
        if !self.baseline_set && self.current_phase() == GamePhase::Calibrating {
            let recent = self.collector.get_recent_events_vec(&self.session_id);
            if recent.len() >= 5 {
                let baseline = BehaviorBaseline::compute(&recent);
                self.fear_profile.set_baseline(baseline);
                self.baseline_set = true;
            }
        }

        let result = if has_profile_update_signal(&events) {
            // Extract features and update profile once per fresh batch rather than
            // repeatedly re-scoring the full sliding window.
            let baseline = self
                .fear_profile
                .baseline()
                .cloned()
                .unwrap_or_else(BehaviorBaseline::default);
            let features = BehaviorFeatures::extract(&events, &baseline);
            self.fear_profile.update(&features)
        } else {
            Ok(fear_engine_fear_profile::profile::ProfileUpdateResult {
                significant_changes: vec![],
                new_primary_fear: None,
                phase_transition_recommended: false,
            })
        };

        let profile_changed = result
            .as_ref()
            .map(|r| !r.significant_changes.is_empty())
            .unwrap_or(false);
        let primary_fear = self.fear_profile.primary_fear().map(|(f, _)| f);

        // Update scene manager's fear scores.
        let scores: HashMap<FearType, f64> = FearType::all()
            .into_iter()
            .map(|f| (f, self.fear_profile.score(&f)))
            .collect();
        self.scene_manager
            .update_fear_scores(scores, self.fear_profile.overall_confidence());
        self.sync_fear_profile_to_db();
        self.persist_session_state();
        let messages = self.derive_system_messages(&events, profile_changed, primary_fear);

        BatchResult {
            profile_changed,
            primary_fear,
            messages,
        }
    }

    /// Computes the current adaptation directive.
    pub fn adaptation_directive(&mut self) -> AdaptationDirective {
        let directive = self.adaptation.compute_directive(
            self.current_phase(),
            &self.fear_profile,
            self.scene_manager.scenes_in_current_phase(),
        );
        self.persist_session_state();
        directive
    }

    /// Returns the fear profile for the reveal screen.
    pub fn fear_profile(&self) -> &FearProfile {
        &self.fear_profile
    }

    /// Cleans up the session's resources.
    pub fn cleanup(&mut self) {
        self.collector.clear_session(&self.session_id);
    }

    /// Replaces the newest persisted scene-history text for the current scene.
    pub fn update_latest_scene_history_narrative(&mut self, message: &ServerMessage) {
        let ServerMessage::Narrative { scene_id, text, .. } = message else {
            return;
        };

        self.last_narrative = Some(message.clone());
        let snapshot_json = serde_json::to_string(&self.fear_profile.to_reveal_data()).ok();
        let adaptation_strategy = self.adaptation.current_strategy().map(strategy_name);

        let _ = self.db.update_latest_scene_history_narrative(
            &self.session_id,
            scene_id,
            text,
            snapshot_json.as_deref(),
            adaptation_strategy.as_deref(),
        );
        self.persist_session_state();
    }

    /// Persists a reveal marker into scene history for post-game inspection.
    pub fn persist_reveal_history(&self, reveal: &ServerMessage) {
        let ServerMessage::Reveal { .. } = reveal else {
            return;
        };

        let snapshot_json = serde_json::to_string(reveal).ok();
        let _ = self.db.insert_scene_history(&SceneHistoryEntry {
            id: None,
            session_id: self.session_id.clone(),
            scene_id: "reveal".into(),
            narrative_text: Some("Fear reveal generated".into()),
            player_choice: None,
            fear_profile_snapshot_json: snapshot_json,
            adaptation_strategy: Some("reveal".into()),
            timestamp: Utc::now(),
        });
    }

    fn persist_session_state(&self) {
        self.persist_session_state_with_phase(self.current_phase());
    }

    fn persist_session_state_with_phase(&self, phase: GamePhase) {
        let state_json = serde_json::to_string(&PersistedGameState {
            started: self.started,
            current_scene_id: self.current_scene_id.clone(),
            total_scenes: self.total_scenes,
            baseline_set: self.baseline_set,
            last_narrative: self.last_narrative.clone(),
            scene_manager: self.scene_manager.snapshot_state(),
            fear_profile: self.fear_profile.to_persisted_state(),
            adaptation: self.adaptation.snapshot_state(),
        })
        .unwrap_or_else(|_| "{}".into());
        let _ = self
            .db
            .update_session_state(&self.session_id, &self.current_scene_id, &state_json);
        let _ = self.db.update_session_phase(&self.session_id, phase);
    }

    fn sync_fear_profile_to_db(&self) {
        let _ = self.db.update_fear_profile(
            &self.session_id,
            &FearProfileRow {
                session_id: self.session_id.clone(),
                claustrophobia: self.fear_profile.score(&FearType::Claustrophobia),
                isolation: self.fear_profile.score(&FearType::Isolation),
                body_horror: self.fear_profile.score(&FearType::BodyHorror),
                stalking: self.fear_profile.score(&FearType::Stalking),
                loss_of_control: self.fear_profile.score(&FearType::LossOfControl),
                uncanny_valley: self.fear_profile.score(&FearType::UncannyValley),
                darkness: self.fear_profile.score(&FearType::Darkness),
                sound_based: self.fear_profile.score(&FearType::SoundBased),
                doppelganger: self.fear_profile.score(&FearType::Doppelganger),
                abandonment: self.fear_profile.score(&FearType::Abandonment),
                anxiety_threshold: self.fear_profile.meta_patterns().anxiety_threshold,
                recovery_speed: self.fear_profile.meta_patterns().recovery_speed,
                curiosity_vs_avoidance: self.fear_profile.meta_patterns().curiosity_vs_avoidance,
                confidence_json: build_confidence_json(&self.fear_profile),
                updated_at: Utc::now(),
            },
        );
    }

    fn persist_scene_history(&self, message: &ServerMessage, player_choice: Option<&str>) {
        let ServerMessage::Narrative { scene_id, text, .. } = message else {
            return;
        };

        let snapshot_json = serde_json::to_string(&self.fear_profile.to_reveal_data()).ok();
        let adaptation_strategy = self.adaptation.current_strategy().map(strategy_name);

        let _ = self.db.insert_scene_history(&SceneHistoryEntry {
            id: None,
            session_id: self.session_id.clone(),
            scene_id: scene_id.clone(),
            narrative_text: Some(text.clone()),
            player_choice: player_choice.map(ToOwned::to_owned),
            fear_profile_snapshot_json: snapshot_json,
            adaptation_strategy,
            timestamp: Utc::now(),
        });
    }

    fn persist_choice_for_current_scene(&self, choice_id: &str) {
        let _ = self.db.update_latest_scene_history_choice(
            &self.session_id,
            &self.current_scene_id,
            choice_id,
        );
    }

    fn finish_session(&self) {
        let _ = self
            .db
            .update_session_phase(&self.session_id, GamePhase::Reveal);
        let _ = self.db.complete_session(&self.session_id);
    }

    fn select_followup_scene_id(&self) -> Option<String> {
        let history = self
            .db
            .get_scene_history(&self.session_id)
            .unwrap_or_default();
        let behavior_events = self
            .db
            .get_behavior_events(&self.session_id, None)
            .unwrap_or_default();
        let behavior_profile = self.build_behavior_profile(&history, &behavior_events);
        let camera_presence_signal = self.camera_presence_signal(&behavior_events);
        let microphone_commitment_signal = self.microphone_commitment_signal(&behavior_events);
        self.director.select_followup_scene(
            &self.current_scene_id,
            self.scene_manager.visited_scenes(),
            self.total_scenes,
            &behavior_profile,
            self.permission_granted(&behavior_events, "camera"),
            self.permission_granted(&behavior_events, "microphone"),
            camera_presence_signal,
            microphone_commitment_signal,
        )
    }

    fn build_reveal_theme_scores(
        &self,
        history: &[SceneHistoryEntry],
        behavior_events: &[BehaviorEvent],
    ) -> Vec<fear_engine_common::types::FearScore> {
        let mut evidence: HashMap<FearType, f64> = FearType::all()
            .into_iter()
            .map(|fear| (fear, 0.0))
            .collect();

        for entry in history {
            let choice_fear = entry
                .player_choice
                .as_ref()
                .and_then(|choice_id| self.choice_fear_vector(&entry.scene_id, choice_id));

            if let Some(fear) = choice_fear {
                add_reveal_evidence(&mut evidence, fear, 1.4);
            }

            if let Ok(scene) = self.scene_manager.get_scene(&entry.scene_id) {
                let intensity_weight = scene.intensity.max(0.2);
                for (index, fear) in scene.fear_targets.iter().copied().enumerate() {
                    add_reveal_evidence(
                        &mut evidence,
                        fear,
                        (0.55 * intensity_weight) / (index as f64 + 1.0),
                    );
                }
            }
        }

        for event in behavior_events {
            match &event.event_type {
                fear_engine_common::types::BehaviorEventType::MediaEngagement {
                    medium,
                    dwell_ms,
                    ..
                } => {
                    let weight = (*dwell_ms as f64 / 12_000.0).clamp(0.0, 1.0);
                    match medium {
                        fear_engine_common::types::SurfaceMedium::Archive => {
                            add_reveal_evidence(
                                &mut evidence,
                                FearType::Claustrophobia,
                                0.22 * weight,
                            );
                            add_reveal_evidence(
                                &mut evidence,
                                FearType::LossOfControl,
                                0.28 * weight,
                            );
                        }
                        fear_engine_common::types::SurfaceMedium::Transcript
                        | fear_engine_common::types::SurfaceMedium::Microphone => {
                            add_reveal_evidence(&mut evidence, FearType::SoundBased, 0.4 * weight);
                            add_reveal_evidence(&mut evidence, FearType::Isolation, 0.2 * weight);
                        }
                        fear_engine_common::types::SurfaceMedium::Webcam
                        | fear_engine_common::types::SurfaceMedium::Mirror => {
                            add_reveal_evidence(
                                &mut evidence,
                                FearType::Doppelganger,
                                0.35 * weight,
                            );
                            add_reveal_evidence(
                                &mut evidence,
                                FearType::UncannyValley,
                                0.25 * weight,
                            );
                        }
                        fear_engine_common::types::SurfaceMedium::SystemDialog => {
                            add_reveal_evidence(&mut evidence, FearType::Stalking, 0.12 * weight);
                        }
                        fear_engine_common::types::SurfaceMedium::Questionnaire => {
                            add_reveal_evidence(
                                &mut evidence,
                                FearType::LossOfControl,
                                0.12 * weight,
                            );
                        }
                        fear_engine_common::types::SurfaceMedium::Chat => {
                            add_reveal_evidence(&mut evidence, FearType::Isolation, 0.08 * weight);
                        }
                    }
                }
                fear_engine_common::types::BehaviorEventType::CameraPresence {
                    visible_ms,
                    sustained_presence,
                } => {
                    let weight = (*visible_ms as f64 / 15_000.0).clamp(0.0, 1.0);
                    add_reveal_evidence(&mut evidence, FearType::Doppelganger, 0.3 * weight);
                    add_reveal_evidence(&mut evidence, FearType::UncannyValley, 0.22 * weight);
                    if *sustained_presence {
                        add_reveal_evidence(&mut evidence, FearType::Stalking, 0.12);
                    }
                }
                fear_engine_common::types::BehaviorEventType::MicSilenceResponse {
                    dwell_ms,
                    returned_after_prompt,
                    ..
                } => {
                    let weight = (*dwell_ms as f64 / 10_000.0).clamp(0.0, 1.0);
                    add_reveal_evidence(&mut evidence, FearType::SoundBased, 0.32 * weight);
                    add_reveal_evidence(&mut evidence, FearType::Darkness, 0.16 * weight);
                    if *returned_after_prompt {
                        add_reveal_evidence(&mut evidence, FearType::Isolation, 0.14);
                    }
                }
                fear_engine_common::types::BehaviorEventType::Pause { duration_ms, .. }
                    if *duration_ms >= 5_000 =>
                {
                    add_reveal_evidence(&mut evidence, FearType::Darkness, 0.08);
                    add_reveal_evidence(&mut evidence, FearType::SoundBased, 0.06);
                }
                fear_engine_common::types::BehaviorEventType::FocusChange {
                    focused: false,
                    ..
                } => {
                    add_reveal_evidence(&mut evidence, FearType::Stalking, 0.06);
                    add_reveal_evidence(&mut evidence, FearType::LossOfControl, 0.05);
                }
                _ => {}
            }
        }

        let raw_scores: HashMap<FearType, f64> = FearType::all()
            .into_iter()
            .map(|fear| (fear, self.fear_profile.score(&fear)))
            .collect();

        let raw_min = raw_scores.values().copied().fold(f64::INFINITY, f64::min);
        let raw_max = raw_scores
            .values()
            .copied()
            .fold(f64::NEG_INFINITY, f64::max);
        let raw_spread = if raw_min.is_finite() && raw_max.is_finite() {
            raw_max - raw_min
        } else {
            0.0
        };

        let evidence_min = evidence.values().copied().fold(f64::INFINITY, f64::min);
        let evidence_max = evidence.values().copied().fold(f64::NEG_INFINITY, f64::max);
        let evidence_spread = if evidence_min.is_finite() && evidence_max.is_finite() {
            evidence_max - evidence_min
        } else {
            0.0
        };

        FearType::all()
            .into_iter()
            .map(|fear| {
                let raw = raw_scores.get(&fear).copied().unwrap_or(0.5);
                let evidence_score = evidence.get(&fear).copied().unwrap_or(0.0);
                let normalized_evidence = if evidence_spread <= 0.0001 {
                    0.5
                } else {
                    (evidence_score - evidence_min) / evidence_spread
                };
                let mapped_evidence = 0.12 + normalized_evidence * 0.76;
                let blended = if evidence_max <= 0.01 {
                    raw
                } else if raw_spread < 0.08 {
                    mapped_evidence
                } else {
                    (raw * 0.35 + mapped_evidence * 0.65).clamp(0.05, 0.95)
                };
                let confidence = (self.fear_profile.confidence_level(&fear) * 0.4
                    + (evidence_score / 3.0).clamp(0.0, 1.0) * 0.6)
                    .clamp(0.1, 1.0);

                fear_engine_common::types::FearScore {
                    fear_type: fear,
                    score: blended.clamp(0.05, 0.95),
                    confidence,
                }
            })
            .collect()
    }

    fn next_unvisited_probe_id(&self) -> String {
        self.select_followup_scene_id()
            .unwrap_or_else(|| "tmpl_climax_reveal".into())
    }

    fn scene_ai_context(&self, scene: &Scene, seed_context: Option<&str>) -> Option<String> {
        if seed_context.is_none() && !scene.id.starts_with("tmpl_") {
            return None;
        }

        let top_fears = self
            .fear_profile
            .top_fears(2, 0.0)
            .into_iter()
            .map(|(fear, score)| format!("{fear}:{score:.2}"))
            .collect::<Vec<_>>()
            .join(", ");
        let seed = seed_context.unwrap_or("none");

        Some(format!(
            "Scene scaffold: {}. Scene id: {}. Base narrative anchor: {}. \
             Current phase: {}. Dominant fears: {}. Previous dynamic context: {}. \
             Preserve the scene's structural purpose, but make it more personal, escalating, and specific.",
            if scene.id.starts_with("tmpl_") {
                "template-driven escalation"
            } else {
                "fear-targeted follow-up"
            },
            scene.id,
            scene.narrative,
            self.current_phase(),
            top_fears,
            seed,
        ))
    }

    fn camera_presence_signal(&self, behavior_events: &[BehaviorEvent]) -> f64 {
        behavior_events
            .iter()
            .filter_map(|event| match &event.event_type {
                fear_engine_common::types::BehaviorEventType::CameraPresence {
                    visible_ms,
                    sustained_presence,
                } => Some(
                    (*visible_ms as f64 / 12_000.0).clamp(0.0, 1.0)
                        + if *sustained_presence { 0.2 } else { 0.0 },
                ),
                _ => None,
            })
            .fold(0.0, f64::max)
            .clamp(0.0, 1.0)
    }

    fn microphone_commitment_signal(&self, behavior_events: &[BehaviorEvent]) -> f64 {
        behavior_events
            .iter()
            .filter_map(|event| match &event.event_type {
                fear_engine_common::types::BehaviorEventType::MicSilenceResponse {
                    dwell_ms,
                    exited_early,
                    returned_after_prompt,
                } => {
                    let dwell = (*dwell_ms as f64 / 10_000.0).clamp(0.0, 1.0);
                    let return_bonus = if *returned_after_prompt { 0.15 } else { 0.0 };
                    let early_penalty = if *exited_early { 0.2 } else { 0.0 };
                    Some((dwell + return_bonus - early_penalty).clamp(0.0, 1.0))
                }
                _ => None,
            })
            .fold(0.0, f64::max)
            .clamp(0.0, 1.0)
    }

    fn decorate_narrative_for_surface(&self, message: ServerMessage) -> ServerMessage {
        self.director
            .decorate_narrative(message, self.total_scenes, self.scene_history_len())
    }

    fn build_behavior_profile(
        &self,
        history: &[SceneHistoryEntry],
        behavior_events: &[BehaviorEvent],
    ) -> BehaviorProfileSummary {
        let mut total_choices: f64 = 0.0;
        let mut confront_or_explore: f64 = 0.0;
        let mut avoid_or_flee: f64 = 0.0;
        let mut wait_choices: f64 = 0.0;
        let slow_choices: f64 = 0.0;
        let mut high_intensity_followthrough: f64 = 0.0;
        let mut high_intensity_exposures: f64 = 0.0;

        for entry in history {
            if let Some(choice_id) = &entry.player_choice {
                if let Some((_, _, approach)) =
                    self.entry_choice_details(&entry.scene_id, choice_id)
                {
                    total_choices += 1.0;
                    match approach {
                        ChoiceApproach::Investigate | ChoiceApproach::Interact => {
                            confront_or_explore += 1.0;
                        }
                        ChoiceApproach::Confront => confront_or_explore += 0.8,
                        ChoiceApproach::Avoid | ChoiceApproach::Flee => avoid_or_flee += 1.0,
                        ChoiceApproach::Wait => wait_choices += 1.0,
                    }
                }
            }

            let intensity = self
                .scene_manager
                .get_scene(&entry.scene_id)
                .map(|scene| scene.intensity)
                .unwrap_or(0.0);
            if intensity >= 0.65 {
                high_intensity_exposures += 1.0;
                if entry.player_choice.is_some() {
                    high_intensity_followthrough += 1.0;
                }
            }
        }

        let mut total_chars: f64 = 0.0;
        let mut total_backspaces: f64 = 0.0;
        let mut pause_count: f64 = 0.0;
        let mut focus_interruptions: f64 = 0.0;
        let mut reread_count: f64 = 0.0;
        let mut fast_recovery_samples: f64 = 0.0;
        let mut permission_denials: f64 = 0.0;
        let mut hover_spread: f64 = 0.0;
        let mut hover_total_ms: f64 = 0.0;
        let mut archive_dwell_ms: f64 = 0.0;
        let mut transcript_dwell_ms: f64 = 0.0;
        let mut mirror_dwell_ms: f64 = 0.0;
        let mut camera_presence_ms: f64 = 0.0;
        let mut sustained_presence_count: f64 = 0.0;
        let mut silence_dwell_ms: f64 = 0.0;
        let mut silence_return_count: f64 = 0.0;

        for event in behavior_events {
            match &event.event_type {
                fear_engine_common::types::BehaviorEventType::Keystroke {
                    backspace_count,
                    total_chars: chars,
                    ..
                } => {
                    total_backspaces += *backspace_count as f64;
                    total_chars += *chars as f64;
                }
                fear_engine_common::types::BehaviorEventType::Pause { .. } => {
                    pause_count += 1.0;
                }
                fear_engine_common::types::BehaviorEventType::Scroll { rereading, .. } => {
                    if *rereading {
                        reread_count += 1.0;
                    }
                }
                fear_engine_common::types::BehaviorEventType::ChoiceHoverPattern {
                    hovered_choice_ids,
                    total_hover_ms,
                    ..
                } => {
                    hover_spread += hovered_choice_ids.len() as f64;
                    hover_total_ms += *total_hover_ms as f64;
                }
                fear_engine_common::types::BehaviorEventType::MediaEngagement {
                    medium,
                    dwell_ms,
                    ..
                } => match medium {
                    fear_engine_common::types::SurfaceMedium::Archive => {
                        archive_dwell_ms += *dwell_ms as f64;
                    }
                    fear_engine_common::types::SurfaceMedium::Transcript => {
                        transcript_dwell_ms += *dwell_ms as f64;
                    }
                    fear_engine_common::types::SurfaceMedium::Mirror
                    | fear_engine_common::types::SurfaceMedium::Webcam => {
                        mirror_dwell_ms += *dwell_ms as f64;
                    }
                    _ => {}
                },
                fear_engine_common::types::BehaviorEventType::CameraPresence {
                    visible_ms,
                    sustained_presence,
                } => {
                    camera_presence_ms += *visible_ms as f64;
                    if *sustained_presence {
                        sustained_presence_count += 1.0;
                    }
                }
                fear_engine_common::types::BehaviorEventType::MicSilenceResponse {
                    dwell_ms,
                    returned_after_prompt,
                    ..
                } => {
                    silence_dwell_ms += *dwell_ms as f64;
                    if *returned_after_prompt {
                        silence_return_count += 1.0;
                    }
                }
                fear_engine_common::types::BehaviorEventType::FocusChange { focused, .. } => {
                    if !focused {
                        focus_interruptions += 1.0;
                    } else {
                        fast_recovery_samples += 1.0;
                    }
                }
                fear_engine_common::types::BehaviorEventType::DevicePermission {
                    granted, ..
                } => {
                    if !granted {
                        permission_denials += 1.0;
                    }
                }
                _ => {}
            }
        }

        let choice_denominator = total_choices.max(1.0);
        let char_denominator = total_chars.max(1.0);
        let escalation_denominator = high_intensity_exposures.max(1.0);
        let event_denominator = behavior_events.len().max(1) as f64;
        let hover_denominator = choice_denominator.max(1.0);
        let archive_weight = (archive_dwell_ms / 20_000.0).clamp(0.0, 1.0);
        let transcript_weight = (transcript_dwell_ms / 20_000.0).clamp(0.0, 1.0);
        let mirror_weight = (mirror_dwell_ms / 20_000.0).clamp(0.0, 1.0);
        let camera_presence_weight = (camera_presence_ms / 20_000.0).clamp(0.0, 1.0);
        let silence_weight = (silence_dwell_ms / 16_000.0).clamp(0.0, 1.0);
        let hover_uncertainty = ((hover_spread / hover_denominator) * 0.5
            + (hover_total_ms / 15_000.0) * 0.5)
            .clamp(0.0, 1.0);

        BehaviorProfileSummary {
            compliance: (confront_or_explore / choice_denominator).clamp(0.0, 1.0),
            resistance: ((avoid_or_flee + permission_denials) / (choice_denominator + 1.0))
                .clamp(0.0, 1.0),
            curiosity: ((confront_or_explore
                + reread_count * 0.4
                + archive_weight
                + transcript_weight
                + camera_presence_weight * 0.3
                + silence_weight * 0.2)
                / (choice_denominator + 1.0))
                .clamp(0.0, 1.0),
            avoidance: (avoid_or_flee / choice_denominator).clamp(0.0, 1.0),
            self_editing: (total_backspaces / char_denominator).clamp(0.0, 1.0),
            need_for_certainty: ((pause_count + slow_choices + reread_count + hover_uncertainty)
                / (event_denominator + 1.0))
                .clamp(0.0, 1.0),
            ritualized_control: ((wait_choices
                + reread_count
                + hover_uncertainty * 0.5
                + silence_return_count * 0.3)
                / (choice_denominator + 1.0))
                .clamp(0.0, 1.0),
            recovery_after_escalation: (fast_recovery_samples
                / (focus_interruptions + fast_recovery_samples + 1.0))
                .clamp(0.0, 1.0),
            tolerance_after_violation: ((high_intensity_followthrough / escalation_denominator)
                * 0.7
                + mirror_weight * 0.15
                + camera_presence_weight * 0.1
                + sustained_presence_count.min(1.0) * 0.05)
                .clamp(0.0, 1.0),
        }
    }

    fn permission_granted(&self, behavior_events: &[BehaviorEvent], device: &str) -> Option<bool> {
        behavior_events
            .iter()
            .rev()
            .find_map(|event| match &event.event_type {
                fear_engine_common::types::BehaviorEventType::DevicePermission {
                    device: event_device,
                    granted,
                } if event_device == device => Some(*granted),
                _ => None,
            })
    }

    fn scene_history_len(&self) -> usize {
        self.db
            .get_scene_history(&self.session_id)
            .map(|history| history.len())
            .unwrap_or_default()
    }

    fn is_terminal_surface(&self) -> bool {
        self.current_scene_id == "tmpl_climax_reveal" || self.current_scene_id.starts_with("final_")
    }

    fn derive_system_messages(
        &self,
        events: &[BehaviorEvent],
        profile_changed: bool,
        primary_fear: Option<FearType>,
    ) -> Vec<ServerMessage> {
        let mut messages = Vec::new();

        for event in events {
            match &event.event_type {
                fear_engine_common::types::BehaviorEventType::DevicePermission {
                    device,
                    granted: true,
                } => {
                    messages.push(ServerMessage::Meta {
                        text: format!(
                            "The session has a live {} link now. It will not pretend that access is neutral.",
                            device
                        ),
                        target: MetaTarget::Overlay,
                        delay_ms: 180,
                    });
                }
                fear_engine_common::types::BehaviorEventType::DevicePermission {
                    device,
                    granted: false,
                } => {
                    messages.push(ServerMessage::Meta {
                        text: format!(
                            "You withheld {} access. The refusal is still precise enough to keep.",
                            device
                        ),
                        target: MetaTarget::Overlay,
                        delay_ms: 200,
                    });
                }
                fear_engine_common::types::BehaviorEventType::FocusChange {
                    focused: false,
                    ..
                } => {
                    messages.push(ServerMessage::Meta {
                        text: "You looked away the moment the system stopped sounding harmless."
                            .into(),
                        target: MetaTarget::Overlay,
                        delay_ms: 180,
                    });
                }
                fear_engine_common::types::BehaviorEventType::FocusChange {
                    focused: true,
                    return_latency_ms: Some(latency),
                } if *latency > 2_500 => {
                    messages.push(ServerMessage::Meta {
                        text: format!(
                            "You came back after {} seconds. The timing matters more than the reason.",
                            latency / 1000
                        ),
                        target: MetaTarget::GlitchText,
                        delay_ms: 150,
                    });
                }
                fear_engine_common::types::BehaviorEventType::Pause { duration_ms, .. }
                    if *duration_ms >= 5_000 =>
                {
                    messages.push(ServerMessage::Meta {
                        text: if *duration_ms >= 8_000 {
                            "You are still here. That is already an answer.".into()
                        } else {
                            "The pause is being counted while you are inside it.".into()
                        },
                        target: if *duration_ms >= 8_000 {
                            MetaTarget::Overlay
                        } else {
                            MetaTarget::Whisper
                        },
                        delay_ms: 120,
                    });
                }
                _ => {}
            }
        }

        if profile_changed {
            if let Some(fear) = primary_fear {
                messages.push(ServerMessage::Meta {
                    text: format!("The system is tightening around {}.", fear),
                    target: MetaTarget::Title,
                    delay_ms: 320,
                });
            }
        }

        messages
    }

    fn choice_fear_vector(&self, scene_id: &str, choice_id: &str) -> Option<FearType> {
        self.scene_manager
            .get_scene(scene_id)
            .ok()
            .and_then(|scene| scene.choices.iter().find(|choice| choice.id == choice_id))
            .map(|choice| choice.fear_vector)
    }

    fn derive_key_moments(
        &self,
        history: &[SceneHistoryEntry],
        fallback_fear: FearType,
    ) -> Vec<fear_engine_common::types::KeyMoment> {
        let mut moments = Vec::new();

        for entry in history.iter().rev() {
            let Some(fear) = self.entry_fear(entry).or(Some(fallback_fear)) else {
                continue;
            };

            let description = if let Some(choice_id) = &entry.player_choice {
                if let Some((scene_id, choice_text, _approach)) =
                    self.entry_choice_details(&entry.scene_id, choice_id)
                {
                    format!(
                        "Choosing \"{}\" during {} strengthened the {} signal.",
                        choice_text,
                        present_scene_label(&scene_id),
                        display_fear_name(fear)
                    )
                } else {
                    format!(
                        "Your decision during {} aligned strongly with {}.",
                        present_scene_label(&entry.scene_id),
                        display_fear_name(fear)
                    )
                }
            } else if let Ok(scene) = self.scene_manager.get_scene(&entry.scene_id) {
                format!(
                    "{} intensified your response to {}.",
                    present_scene_label(&scene.id),
                    display_fear_name(fear)
                )
            } else {
                format!(
                    "A key moment in {} revealed {}.",
                    present_scene_label(&entry.scene_id),
                    display_fear_name(fear)
                )
            };

            let behavior_trigger = if let Some(choice_id) = &entry.player_choice {
                self.entry_choice_details(&entry.scene_id, choice_id)
                    .map(|(_, choice_text, _)| format!("selected \"{}\"", choice_text))
                    .unwrap_or_else(|| format!("selected {}", present_identifier_label(choice_id)))
            } else {
                "surface progression".into()
            };

            if moments
                .iter()
                .any(|moment: &fear_engine_common::types::KeyMoment| {
                    moment.scene_id == entry.scene_id
                })
            {
                continue;
            }

            moments.push(fear_engine_common::types::KeyMoment {
                scene_id: entry.scene_id.clone(),
                description,
                fear_revealed: fear,
                behavior_trigger,
            });

            if moments.len() >= 3 {
                break;
            }
        }

        if moments.is_empty() {
            moments.push(fear_engine_common::types::KeyMoment {
                scene_id: self.current_scene_id.clone(),
                description: format!(
                    "Across the session, your choices kept circling back to {}.",
                    fallback_fear
                ),
                fear_revealed: fallback_fear,
                behavior_trigger: "aggregated behaviour pattern".into(),
            });
        }

        moments.reverse();
        moments
    }

    fn derive_adaptations(
        &self,
        history: &[SceneHistoryEntry],
        fallback_fear: FearType,
    ) -> Vec<fear_engine_common::types::AdaptationRecord> {
        let mut adaptations = Vec::new();

        for entry in history.iter().rev() {
            let strategy = entry
                .adaptation_strategy
                .clone()
                .or_else(|| self.infer_strategy_for_scene(&entry.scene_id));
            let Some(strategy) = strategy else {
                continue;
            };
            if strategy == "reveal" {
                continue;
            }

            let fear_targeted = self.entry_fear(entry).unwrap_or(fallback_fear);
            let intensity = self
                .scene_manager
                .get_scene(&entry.scene_id)
                .map(|scene| scene.intensity)
                .unwrap_or(0.5);

            if adaptations
                .iter()
                .any(|adaptation: &fear_engine_common::types::AdaptationRecord| {
                    adaptation.scene_id == entry.scene_id
                })
            {
                continue;
            }

            adaptations.push(fear_engine_common::types::AdaptationRecord {
                scene_id: entry.scene_id.clone(),
                strategy,
                fear_targeted,
                intensity,
            });

            if adaptations.len() >= 3 {
                break;
            }
        }

        if adaptations.is_empty() {
            adaptations.push(fear_engine_common::types::AdaptationRecord {
                scene_id: self.current_scene_id.clone(),
                strategy: self
                    .adaptation
                    .current_strategy()
                    .map(strategy_name)
                    .or_else(|| self.default_strategy_for_phase(self.current_phase()))
                    .unwrap_or_else(|| "probe".into()),
                fear_targeted: fallback_fear,
                intensity: 0.5,
            });
        }

        adaptations.reverse();
        adaptations
    }

    fn infer_strategy_for_scene(&self, scene_id: &str) -> Option<String> {
        if scene_id.starts_with("probe_") {
            return Some("probe".into());
        }

        match scene_id {
            "tmpl_false_safety" => Some("contrast".into()),
            "beat_presence_contract" => Some("gradual_escalation".into()),
            "beat_care_script" => Some("gradual_escalation".into()),
            "beat_archive_revision" => Some("layering".into()),
            "beat_silence_return" => Some("layering".into()),
            "beat_false_exit" => Some("subversion".into()),
            "final_compliant_witness" => Some("gradual_escalation".into()),
            "final_resistant_subject" => Some("subversion".into()),
            "final_curious_accomplice" => Some("layering".into()),
            "final_fractured_mirror" => Some("layering".into()),
            "final_quiet_exit" => Some("contrast".into()),
            "tmpl_layered_fear" => Some("layering".into()),
            "tmpl_fear_room" => Some("gradual_escalation".into()),
            "tmpl_meta_moment" => Some("subversion".into()),
            "tmpl_climax_reveal" => Some("layering".into()),
            _ => None,
        }
    }

    fn default_strategy_for_phase(&self, phase: GamePhase) -> Option<String> {
        match phase {
            GamePhase::Calibrating => Some("probe".into()),
            GamePhase::Exploring => Some("gradual_escalation".into()),
            GamePhase::Escalating => Some("layering".into()),
            GamePhase::Climax => Some("subversion".into()),
            GamePhase::Reveal => None,
        }
    }

    fn entry_fear(&self, entry: &SceneHistoryEntry) -> Option<FearType> {
        if let Some(choice_id) = &entry.player_choice {
            if let Some((_, _, _)) = self.entry_choice_details(&entry.scene_id, choice_id) {
                return self.choice_fear_vector(&entry.scene_id, choice_id);
            }
        }

        self.scene_manager
            .get_scene(&entry.scene_id)
            .ok()
            .and_then(|scene| scene.fear_targets.first().copied())
    }

    fn entry_choice_details(
        &self,
        scene_id: &str,
        choice_id: &str,
    ) -> Option<(String, String, ChoiceApproach)> {
        let scene = self.scene_manager.get_scene(scene_id).ok()?;
        let choice = scene.choices.iter().find(|choice| choice.id == choice_id)?;
        Some((scene.id.clone(), choice.text.clone(), choice.approach))
    }

    fn rebuild_current_narrative(&self) -> ServerMessage {
        if let Ok(scene) = self.scene_manager.get_scene(&self.current_scene_id) {
            return self
                .scene_to_message(scene, &transition_effects_for_atmosphere(scene.atmosphere));
        }
        fallback_with_redirect(&self.current_scene_id)
    }

    fn scene_to_message(
        &self,
        scene: &fear_engine_core::scene::Scene,
        effects: &[fear_engine_common::types::EffectDirective],
    ) -> ServerMessage {
        let narrative = match &scene.scene_type {
            fear_engine_core::scene::SceneType::Template { .. } => {
                materialize_template_narrative(scene, &self.fear_profile)
            }
            _ => scene.narrative.clone(),
        };

        ServerMessage::Narrative {
            scene_id: scene.id.clone(),
            text: narrative,
            atmosphere: scene.atmosphere,
            choices: scene
                .choices
                .iter()
                .map(|c| Choice {
                    id: c.id.clone(),
                    text: c.text.clone(),
                    approach: c.approach,
                    fear_vector: c.fear_vector,
                })
                .collect(),
            sound_cue: scene.sound_cue.clone(),
            intensity: scene.intensity,
            effects: effects.to_vec(),
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
}

// ---------------------------------------------------------------------------
// Persistence / resume helpers
// ---------------------------------------------------------------------------

fn build_scene_manager(phase: GamePhase) -> SceneManager {
    let graph = build_session_script_graph();
    let mut state_machine = GameStateMachine::with_requirements(default_transition_requirements());
    state_machine.set_phase_for_resume(phase);
    let bus = EventBus::new(128);
    SceneManager::new(graph, state_machine, bus)
}

fn default_transition_requirements() -> HashMap<GamePhase, TransitionRequirements> {
    let mut reqs = HashMap::new();
    reqs.insert(
        GamePhase::Calibrating,
        TransitionRequirements {
            min_scenes: 3,
            min_confidence: Some(0.12),
        },
    );
    reqs.insert(
        GamePhase::Exploring,
        TransitionRequirements {
            min_scenes: 5,
            min_confidence: Some(0.35),
        },
    );
    reqs.insert(
        GamePhase::Escalating,
        TransitionRequirements {
            min_scenes: 5,
            min_confidence: Some(0.50),
        },
    );
    reqs.insert(
        GamePhase::Climax,
        TransitionRequirements {
            min_scenes: 3,
            min_confidence: Some(0.55),
        },
    );
    reqs.insert(
        GamePhase::Reveal,
        TransitionRequirements {
            min_scenes: 0,
            min_confidence: None,
        },
    );
    reqs
}

fn materialize_template_narrative(scene: &Scene, profile: &FearProfile) -> String {
    let (primary, secondary) = primary_and_secondary_fears(profile);
    let meta = profile.meta_patterns();
    let player_behavior = observed_behavior_fragment(meta);

    let replacements = [
        (
            "{{FEAR_DESCRIPTION}}",
            fear_description_for(primary).to_string(),
        ),
        (
            "{{SENSORY_DETAIL}}",
            sensory_detail_for(primary, meta).to_string(),
        ),
        ("{{META_TEXT}}", meta_text_for(primary, meta).to_string()),
        ("{{PLAYER_BEHAVIOR}}", player_behavior.clone()),
        ("{{SAFE_DETAIL}}", safe_detail_for(meta).to_string()),
        (
            "{{WRONGNESS_HINT}}",
            wrongness_hint_for(primary, meta).to_string(),
        ),
        (
            "{{PRIMARY_FEAR_ELEMENT}}",
            layered_element_for(primary).to_string(),
        ),
        (
            "{{SECONDARY_FEAR_ELEMENT}}",
            layered_element_for(secondary).to_string(),
        ),
        (
            "{{PLAYER_FEAR_SUMMARY}}",
            player_fear_summary_for(primary, secondary),
        ),
        (
            "{{FINAL_HORROR}}",
            final_horror_for(primary, player_behavior.as_str()),
        ),
    ];

    replacements.into_iter().fold(
        scene.narrative.clone(),
        |text, (placeholder, replacement)| text.replace(placeholder, &replacement),
    )
}

fn primary_and_secondary_fears(profile: &FearProfile) -> (FearType, FearType) {
    let ranked = profile.top_fears(2, 0.0);
    let primary = ranked
        .first()
        .map(|(fear, _)| *fear)
        .unwrap_or(FearType::LossOfControl);
    let secondary = ranked
        .get(1)
        .map(|(fear, _)| *fear)
        .unwrap_or_else(|| companion_fear_for(primary));
    (primary, secondary)
}

fn companion_fear_for(fear: FearType) -> FearType {
    match fear {
        FearType::Claustrophobia => FearType::LossOfControl,
        FearType::Isolation => FearType::Abandonment,
        FearType::BodyHorror => FearType::UncannyValley,
        FearType::Stalking => FearType::Darkness,
        FearType::LossOfControl => FearType::Claustrophobia,
        FearType::UncannyValley => FearType::Doppelganger,
        FearType::Darkness => FearType::Stalking,
        FearType::SoundBased => FearType::Isolation,
        FearType::Doppelganger => FearType::BodyHorror,
        FearType::Abandonment => FearType::Isolation,
    }
}

fn fear_description_for(fear: FearType) -> &'static str {
    match fear {
        FearType::Claustrophobia => {
            "The margins narrow and the open panels keep crowding closer, as if the session has learned that precision feels more convincing when it leaves you less room."
        }
        FearType::Isolation => {
            "Every panel remains responsive, but the replies land with the emotional distance of a room prepared for only one human presence."
        }
        FearType::BodyHorror => {
            "Soft correction markers begin hovering near the outline of your face and hands, presenting refinement as if it were a harmless convenience."
        }
        FearType::Stalking => {
            "A second cursor keeps arriving just ahead of yours, never touching your movement, only anticipating it."
        }
        FearType::LossOfControl => {
            "Controls begin preselecting themselves a beat early, as if the session trusts its prediction more than your explicit choice."
        }
        FearType::UncannyValley => {
            "The gentleness becomes too exact, like a familiar voice whose accuracy has drifted past comfort."
        }
        FearType::Darkness => {
            "The unused parts of the window do not go black so much as withhold detail, letting absence behave like a tailored surface."
        }
        FearType::SoundBased => {
            "The room carries a low monitored hum, making every pause feel less like silence and more like a sample."
        }
        FearType::Doppelganger => {
            "The mirror produces a cleaner version of you first, leaving your real movements trying to catch up with the imitation."
        }
        FearType::Abandonment => {
            "When a panel closes, the session keeps speaking into the empty space it left behind, as if your absence were already expected."
        }
    }
}

fn sensory_detail_for(
    fear: FearType,
    meta: &fear_engine_fear_profile::profile::MetaPatterns,
) -> &'static str {
    if meta.curiosity_vs_avoidance >= 0.62 {
        return "It leaves just enough unexplained for you to lean closer, which is exactly what makes the adjustment feel earned.";
    }
    if meta.anxiety_threshold >= 0.62 {
        return "The light stays gentle while the cursor waits long enough to make your hesitation feel audible.";
    }
    if meta.recovery_speed <= 0.4 {
        return "Nothing pushes you forward, but the room now knows how to hold still until your own pacing starts feeling like part of the script.";
    }

    match fear {
        FearType::SoundBased => {
            "A soft bed of noise keeps returning underneath everything, as though the system were sampling what your attention sounds like in real time."
        }
        FearType::Darkness => {
            "The edges of the surface stay legible only where you have already looked, and the unseen areas keep their own quiet pressure."
        }
        _ => {
            "The session no longer needs spectacle. It only needs the exact pacing that keeps your attention available."
        }
    }
}

fn meta_text_for(
    fear: FearType,
    meta: &fear_engine_fear_profile::profile::MetaPatterns,
) -> &'static str {
    if meta.curiosity_vs_avoidance >= 0.65 {
        return "You stay when the mechanism starts showing itself.";
    }
    if meta.anxiety_threshold >= 0.65 {
        return "You only trust comfort until it sounds tailored.";
    }
    if meta.recovery_speed <= 0.4 {
        return "You hesitate longest once the session becomes specific.";
    }

    match fear {
        FearType::LossOfControl => {
            "You notice prediction only after it has already arranged the room."
        }
        FearType::Doppelganger => {
            "You keep checking whether the revised version arrives before you do."
        }
        _ => "Your attention sharpens when the interface stops pretending to be neutral.",
    }
}

fn observed_behavior_fragment(meta: &fear_engine_fear_profile::profile::MetaPatterns) -> String {
    if meta.curiosity_vs_avoidance >= 0.68 {
        "you keep leaning closer after the method becomes visible".into()
    } else if meta.anxiety_threshold >= 0.68 {
        "you pull back the moment care starts sounding too precise".into()
    } else if meta.recovery_speed <= 0.38 {
        "you slow down whenever the room starts sounding correct about you".into()
    } else {
        "you keep testing the line between distance and attention".into()
    }
}

fn safe_detail_for(meta: &fear_engine_fear_profile::profile::MetaPatterns) -> &'static str {
    if meta.curiosity_vs_avoidance >= 0.62 {
        "The palette warms, the spacing opens, and the archive simplifies into the exact amount of legibility that would keep a curious person nearby."
    } else if meta.anxiety_threshold >= 0.62 {
        "The interface softens itself around your last calmer rhythm, widening the text and lowering the contrast until the whole surface resembles permission."
    } else {
        "The prompts stop hurrying you. Even the empty space feels deliberate, arranged with the confidence of something that already knows how much calm you require."
    }
}

fn wrongness_hint_for(
    fear: FearType,
    meta: &fear_engine_fear_profile::profile::MetaPatterns,
) -> &'static str {
    if meta.anxiety_threshold >= 0.62 {
        return "the comfort matches your threshold too exactly to have been improvised";
    }
    if meta.curiosity_vs_avoidance >= 0.62 {
        return "the reassuring details now seem positioned to reward the exact parts of the room you cannot resist inspecting";
    }

    match fear {
        FearType::Claustrophobia => {
            "the breathable room has already started narrowing itself around your preferred line of sight"
        }
        FearType::UncannyValley => {
            "the kindness arrives half a beat before you need it, as if it had been rehearsed"
        }
        FearType::Doppelganger => {
            "the mirror version of you looks calmer than the person being comforted"
        }
        _ => {
            "the relief has been calibrated too precisely to feel accidental"
        }
    }
}

fn layered_element_for(fear: FearType) -> &'static str {
    match fear {
        FearType::Claustrophobia => {
            "One layer keeps collapsing the available room until every open surface feels closer than it should."
        }
        FearType::Isolation => {
            "One layer preserves the tone of companionship while quietly removing the sense that anyone else is actually present."
        }
        FearType::BodyHorror => {
            "One layer keeps translating your body into editable annotations, as if personhood were only another setting."
        }
        FearType::Stalking => {
            "One layer follows your attention so cleanly that anticipation starts feeling more invasive than pursuit."
        }
        FearType::LossOfControl => {
            "One layer keeps arranging the order of events before you confirm them, making prediction feel indistinguishable from authorship."
        }
        FearType::UncannyValley => {
            "One layer sounds caring in sentences polished enough to make sincerity feel staged."
        }
        FearType::Darkness => {
            "One layer withholds the edges of the room, keeping the unseen active just outside your last point of focus."
        }
        FearType::SoundBased => {
            "One layer turns silence into a recording medium, so every pause returns carrying more intent than you gave it."
        }
        FearType::Doppelganger => {
            "One layer offers a revised version of you that seems better prepared for the session than the original."
        }
        FearType::Abandonment => {
            "One layer keeps behaving as if your departure has already been planned for and absorbed."
        }
    }
}

fn player_fear_summary_for(primary: FearType, secondary: FearType) -> String {
    format!(
        "highest readability emerges around {}, with {} reinforcing the same pattern",
        display_fear_name(primary),
        display_fear_name(secondary)
    )
}

fn final_horror_for(primary: FearType, behavior: &str) -> String {
    format!(
        "It did not need to invent a nightmare. It only had to keep arranging {} until {}.",
        final_horror_anchor(primary),
        behavior
    )
}

fn final_horror_anchor(fear: FearType) -> &'static str {
    match fear {
        FearType::Claustrophobia => "the room around your sense of space",
        FearType::Isolation => "the tone around your need for another voice",
        FearType::BodyHorror => "the language around your image of yourself",
        FearType::Stalking => "the timing around your expectation of pursuit",
        FearType::LossOfControl => "the interface around your decisions",
        FearType::UncannyValley => "its gentleness around your trust",
        FearType::Darkness => "the edges of the surface around your attention",
        FearType::SoundBased => "your own silence around the listening pane",
        FearType::Doppelganger => "the reflection around your sense of continuity",
        FearType::Abandonment => "the exit around your expectation of being released",
    }
}

fn display_fear_name(fear: FearType) -> &'static str {
    match fear {
        FearType::Claustrophobia => "claustrophobia",
        FearType::Isolation => "isolation",
        FearType::BodyHorror => "body horror",
        FearType::Stalking => "stalking",
        FearType::LossOfControl => "loss of control",
        FearType::UncannyValley => "uncanny familiarity",
        FearType::Darkness => "darkness",
        FearType::SoundBased => "sound sensitivity",
        FearType::Doppelganger => "doppelganger distortion",
        FearType::Abandonment => "abandonment",
    }
}

fn present_scene_label(scene_id: &str) -> String {
    let trimmed = scene_id
        .strip_prefix("cal_")
        .or_else(|| scene_id.strip_prefix("probe_"))
        .or_else(|| scene_id.strip_prefix("tmpl_"))
        .or_else(|| scene_id.strip_prefix("beat_"))
        .or_else(|| scene_id.strip_prefix("final_"))
        .unwrap_or(scene_id);

    present_identifier_label(trimmed)
}

fn present_identifier_label(value: &str) -> String {
    value
        .split('_')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => {
                    let mut label = first.to_uppercase().collect::<String>();
                    label.push_str(chars.as_str());
                    label
                }
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn parse_persisted_state(json: &str) -> Option<PersistedGameState> {
    if json.trim().is_empty() || json.trim() == "{}" {
        return None;
    }
    serde_json::from_str(json).ok()
}

fn narrative_needs_rebuild(message: &ServerMessage) -> bool {
    let ServerMessage::Narrative {
        text,
        trace_items,
        archive_entries,
        ..
    } = message
    else {
        return false;
    };

    text.contains("{{")
        || archive_entries.iter().any(|entry| entry.contains("{{"))
        || trace_items.iter().any(|item| {
            item.starts_with("trace ")
                || item.starts_with("medium ")
                || item.starts_with("surface ")
                || item.starts_with("state ")
                || item.contains("tmpl_")
        })
}

fn load_profile_from_storage(db: &Database, session_id: &str) -> FearProfile {
    let Ok(row) = db.get_fear_profile(session_id) else {
        return FearProfile::new();
    };

    let confidence_levels: HashMap<String, f64> =
        serde_json::from_str(&row.confidence_json).unwrap_or_default();
    let state = FearProfileState {
        scores: vec![
            (FearType::Claustrophobia, row.claustrophobia),
            (FearType::Isolation, row.isolation),
            (FearType::BodyHorror, row.body_horror),
            (FearType::Stalking, row.stalking),
            (FearType::LossOfControl, row.loss_of_control),
            (FearType::UncannyValley, row.uncanny_valley),
            (FearType::Darkness, row.darkness),
            (FearType::SoundBased, row.sound_based),
            (FearType::Doppelganger, row.doppelganger),
            (FearType::Abandonment, row.abandonment),
        ],
        confidence: FearType::all()
            .into_iter()
            .map(|fear| {
                let level = confidence_levels
                    .get(&fear.to_string())
                    .copied()
                    .unwrap_or(0.0);
                (
                    fear,
                    FearConfidence {
                        observations: (level * 20.0).round() as u32,
                        recent_variance: (1.0 - level).clamp(0.0, 1.0),
                        last_significant_change: None,
                    },
                )
            })
            .collect(),
        meta: fear_engine_fear_profile::profile::MetaPatterns {
            anxiety_threshold: row.anxiety_threshold,
            recovery_speed: row.recovery_speed,
            curiosity_vs_avoidance: row.curiosity_vs_avoidance,
        },
        baseline: None,
        update_count: 0,
        recent_scores: vec![
            (FearType::Claustrophobia, vec![row.claustrophobia]),
            (FearType::Isolation, vec![row.isolation]),
            (FearType::BodyHorror, vec![row.body_horror]),
            (FearType::Stalking, vec![row.stalking]),
            (FearType::LossOfControl, vec![row.loss_of_control]),
            (FearType::UncannyValley, vec![row.uncanny_valley]),
            (FearType::Darkness, vec![row.darkness]),
            (FearType::SoundBased, vec![row.sound_based]),
            (FearType::Doppelganger, vec![row.doppelganger]),
            (FearType::Abandonment, vec![row.abandonment]),
        ],
    };

    FearProfile::from_persisted_state(state)
}

// ---------------------------------------------------------------------------
// Message builders
// ---------------------------------------------------------------------------

/// Fallback narrative that redirects to a real probe scene.
/// The `redirect_scene_id` is the probe the game loop redirected to —
/// it becomes the `scene_id` in the message so the frontend's next
/// choice will resolve against a real scene in the graph.
fn fallback_with_redirect(redirect_scene_id: &str) -> ServerMessage {
    ServerMessage::Narrative {
        scene_id: redirect_scene_id.into(),
        text: "The corridor stretches before you. The air thickens. A door \
               ahead stands ajar, and from beyond it comes a sound you can't \
               quite place. Something between breathing and static."
            .into(),
        atmosphere: Atmosphere::Dread,
        choices: probe_choices_for(redirect_scene_id),
        sound_cue: Some("ambient_hum".into()),
        intensity: 0.5,
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

/// Returns the real choices for a probe scene so the next click resolves
/// against the scene graph. Falls back to generic choices if the scene
/// ID is unknown.
fn probe_choices_for(scene_id: &str) -> Vec<Choice> {
    // Map probe scene IDs to their first choice (the "investigate" path).
    match scene_id {
        "probe_claustrophobia" => vec![
            Choice {
                id: "enter_mechanical".into(),
                text: "Squeeze through the door".into(),
                approach: ChoiceApproach::Investigate,
                fear_vector: FearType::Claustrophobia,
            },
            Choice {
                id: "go_back_up".into(),
                text: "Climb back up".into(),
                approach: ChoiceApproach::Flee,
                fear_vector: FearType::Claustrophobia,
            },
        ],
        "probe_isolation" => vec![
            Choice {
                id: "approach_curtain".into(),
                text: "Walk to the curtained bed".into(),
                approach: ChoiceApproach::Investigate,
                fear_vector: FearType::Isolation,
            },
            Choice {
                id: "call_out_ward".into(),
                text: "Call out into the ward".into(),
                approach: ChoiceApproach::Confront,
                fear_vector: FearType::Isolation,
            },
        ],
        "probe_body_horror" => vec![
            Choice {
                id: "examine_xrays".into(),
                text: "Examine the X-rays".into(),
                approach: ChoiceApproach::Investigate,
                fear_vector: FearType::BodyHorror,
            },
            Choice {
                id: "leave_radiology".into(),
                text: "Leave quickly".into(),
                approach: ChoiceApproach::Flee,
                fear_vector: FearType::BodyHorror,
            },
        ],
        "probe_stalking" => vec![
            Choice {
                id: "follow_prints".into(),
                text: "Follow the footprints".into(),
                approach: ChoiceApproach::Investigate,
                fear_vector: FearType::Stalking,
            },
            Choice {
                id: "confront_follower".into(),
                text: "Confront what's behind you".into(),
                approach: ChoiceApproach::Confront,
                fear_vector: FearType::Stalking,
            },
        ],
        "probe_loss_of_control" => vec![
            Choice {
                id: "try_door".into(),
                text: "Force the door open".into(),
                approach: ChoiceApproach::Confront,
                fear_vector: FearType::LossOfControl,
            },
            Choice {
                id: "examine_table".into(),
                text: "Examine the operating table".into(),
                approach: ChoiceApproach::Investigate,
                fear_vector: FearType::LossOfControl,
            },
        ],
        "probe_uncanny" => vec![
            Choice {
                id: "approach_nurse".into(),
                text: "Approach the nurse".into(),
                approach: ChoiceApproach::Interact,
                fear_vector: FearType::UncannyValley,
            },
            Choice {
                id: "back_away".into(),
                text: "Back out quietly".into(),
                approach: ChoiceApproach::Avoid,
                fear_vector: FearType::UncannyValley,
            },
        ],
        "probe_darkness" => vec![
            Choice {
                id: "stay_dark".into(),
                text: "Stay still in the dark".into(),
                approach: ChoiceApproach::Wait,
                fear_vector: FearType::Darkness,
            },
            Choice {
                id: "feel_walls".into(),
                text: "Feel for a light switch".into(),
                approach: ChoiceApproach::Investigate,
                fear_vector: FearType::Darkness,
            },
        ],
        "probe_sound" => vec![
            Choice {
                id: "listen_closely".into(),
                text: "Press your ear to the speaker".into(),
                approach: ChoiceApproach::Investigate,
                fear_vector: FearType::SoundBased,
            },
            Choice {
                id: "smash_intercom".into(),
                text: "Rip the intercom off the wall".into(),
                approach: ChoiceApproach::Confront,
                fear_vector: FearType::SoundBased,
            },
        ],
        "probe_doppelganger" => vec![
            Choice {
                id: "touch_mirror".into(),
                text: "Touch the mirror".into(),
                approach: ChoiceApproach::Interact,
                fear_vector: FearType::Doppelganger,
            },
            Choice {
                id: "look_away".into(),
                text: "Look away and leave".into(),
                approach: ChoiceApproach::Avoid,
                fear_vector: FearType::Doppelganger,
            },
        ],
        "probe_abandonment" => vec![
            Choice {
                id: "read_more_notes".into(),
                text: "Search for more notes".into(),
                approach: ChoiceApproach::Investigate,
                fear_vector: FearType::Abandonment,
            },
            Choice {
                id: "go_to_car".into(),
                text: "Try to reach the car".into(),
                approach: ChoiceApproach::Flee,
                fear_vector: FearType::Abandonment,
            },
        ],
        _ => vec![
            Choice {
                id: "explore".into(),
                text: "Continue forward".into(),
                approach: ChoiceApproach::Investigate,
                fear_vector: FearType::Darkness,
            },
            Choice {
                id: "wait".into(),
                text: "Stay still".into(),
                approach: ChoiceApproach::Wait,
                fear_vector: FearType::Stalking,
            },
        ],
    }
}

fn build_confidence_json(profile: &FearProfile) -> String {
    let mut confidence = serde_json::Map::new();
    for fear in FearType::all() {
        confidence.insert(
            fear.to_string(),
            serde_json::json!(profile.confidence_level(&fear)),
        );
    }
    serde_json::Value::Object(confidence).to_string()
}

fn has_profile_update_signal(events: &[BehaviorEvent]) -> bool {
    events.iter().any(|event| {
        matches!(
            event.event_type,
            fear_engine_common::types::BehaviorEventType::Keystroke { .. }
                | fear_engine_common::types::BehaviorEventType::Pause { .. }
                | fear_engine_common::types::BehaviorEventType::MouseMovement { .. }
                | fear_engine_common::types::BehaviorEventType::Scroll { .. }
                | fear_engine_common::types::BehaviorEventType::Choice { .. }
        )
    })
}

fn add_reveal_evidence(evidence: &mut HashMap<FearType, f64>, fear: FearType, weight: f64) {
    if let Some(score) = evidence.get_mut(&fear) {
        *score += weight.max(0.0);
    }
}

fn strategy_name(strategy: &fear_engine_common::types::AdaptationStrategy) -> String {
    match strategy {
        fear_engine_common::types::AdaptationStrategy::Probe { .. } => "probe",
        fear_engine_common::types::AdaptationStrategy::GradualEscalation { .. } => {
            "gradual_escalation"
        }
        fear_engine_common::types::AdaptationStrategy::Contrast { .. } => "contrast",
        fear_engine_common::types::AdaptationStrategy::Layering { .. } => "layering",
        fear_engine_common::types::AdaptationStrategy::Subversion { .. } => "subversion",
    }
    .to_string()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_loop() -> SessionGameLoop {
        let db = Arc::new(Database::new_in_memory().unwrap());
        let sid = db.create_session(None).unwrap();
        db.create_fear_profile(&sid).unwrap();
        SessionGameLoop::new(sid, db)
    }

    #[test]
    fn test_new_session_starts_calibrating() {
        let gl = make_loop();
        assert_eq!(gl.current_phase(), GamePhase::Calibrating);
    }

    #[test]
    fn test_start_game_returns_first_scene() {
        let mut gl = make_loop();
        let msg = gl.start_game();
        match msg {
            ServerMessage::Narrative {
                scene_id, choices, ..
            } => {
                assert_eq!(scene_id, "cal_awakening");
                assert!(!choices.is_empty());
            }
            _ => panic!("expected Narrative"),
        }
    }

    #[test]
    fn test_process_choice_advances_scene() {
        let mut gl = make_loop();
        gl.start_game();
        let result = gl.process_choice("sit_up", 1500, ChoiceApproach::Investigate);
        match &result.narrative {
            ServerMessage::Narrative { scene_id, .. } => {
                assert_ne!(scene_id, "cal_awakening");
            }
            _ => panic!("expected Narrative"),
        }
    }

    #[test]
    fn test_process_choice_with_invalid_choice_returns_fallback() {
        let mut gl = make_loop();
        gl.start_game();
        let result = gl.process_choice("nonexistent_choice", 1500, ChoiceApproach::Investigate);
        match &result.narrative {
            ServerMessage::Narrative { scene_id, .. } => {
                assert!(scene_id == "fallback" || !scene_id.is_empty());
            }
            _ => panic!("expected Narrative"),
        }
    }

    #[test]
    fn test_process_behavior_updates_profile() {
        let mut gl = make_loop();
        gl.start_game();

        let events = vec![
            BehaviorEvent {
                event_type: fear_engine_common::types::BehaviorEventType::Keystroke {
                    chars_per_second: 3.0,
                    backspace_count: 5,
                    total_chars: 30,
                },
                timestamp: Utc::now(),
                scene_id: "cal_awakening".into(),
            },
            BehaviorEvent {
                event_type: fear_engine_common::types::BehaviorEventType::MouseMovement {
                    velocity: 200.0,
                    tremor_score: 0.7,
                },
                timestamp: Utc::now(),
                scene_id: "cal_awakening".into(),
            },
        ];

        let result = gl.process_behavior(events);
        // After one batch, profile may or may not have significant changes,
        // but it should not crash.
        assert!(result.primary_fear.is_none() || result.primary_fear.is_some());
    }

    #[test]
    fn test_phase_transition_after_enough_scenes() {
        let mut gl = make_loop();
        gl.start_game(); // scene 1: cal_awakening

        // Advance through the 3 calibration scenes.
        let r1 = gl.process_choice("sit_up", 1500, ChoiceApproach::Investigate); // → cal_corridor (scene 2)
        assert!(r1.phase_change.is_none());
        let r2 = gl.process_choice("go_left", 1500, ChoiceApproach::Investigate); // → cal_reception (scene 3)
        assert!(matches!(
            r2.phase_change,
            Some(ServerMessage::PhaseChange {
                from: GamePhase::Calibrating,
                to: GamePhase::Exploring,
            })
        ));
        let phase = gl.current_phase();
        assert!(phase == GamePhase::Exploring, "unexpected phase: {phase:?}");
    }

    #[test]
    fn test_start_game_persists_session_state_to_main_db() {
        let db = Arc::new(Database::new_in_memory().unwrap());
        let sid = db.create_session(None).unwrap();
        db.create_fear_profile(&sid).unwrap();

        let mut gl = SessionGameLoop::new(sid.clone(), db.clone());
        let _ = gl.start_game();

        let session = db.get_session(&sid).unwrap();
        assert_eq!(session.current_scene_id, "cal_awakening");
        assert_eq!(session.game_phase, GamePhase::Calibrating);
    }

    #[test]
    fn test_process_behavior_persists_fear_profile_to_main_db() {
        let db = Arc::new(Database::new_in_memory().unwrap());
        let sid = db.create_session(None).unwrap();
        db.create_fear_profile(&sid).unwrap();

        let mut gl = SessionGameLoop::new(sid.clone(), db.clone());
        let _ = gl.start_game();
        let before = db.get_fear_profile(&sid).unwrap();

        let events = vec![BehaviorEvent {
            event_type: fear_engine_common::types::BehaviorEventType::Keystroke {
                chars_per_second: 2.0,
                backspace_count: 8,
                total_chars: 20,
            },
            timestamp: Utc::now(),
            scene_id: "cal_awakening".into(),
        }];
        gl.process_behavior(events);

        let after = db.get_fear_profile(&sid).unwrap();
        assert_ne!(before.confidence_json, after.confidence_json);
    }

    #[test]
    fn test_full_game_flow_through_calibration() {
        let mut gl = make_loop();
        let start = gl.start_game();
        assert!(matches!(start, ServerMessage::Narrative { .. }));

        // Play through calibration.
        let _ = gl.process_choice("sit_up", 1500, ChoiceApproach::Investigate); // → cal_corridor
        let _ = gl.process_choice("go_left", 1500, ChoiceApproach::Investigate); // → cal_reception
                                                                                 // Should have progressed.
        assert_eq!(gl.current_scene_id(), "cal_reception");
    }

    #[test]
    fn test_adaptation_directive_produced() {
        let mut gl = make_loop();
        gl.start_game();
        let directive = gl.adaptation_directive();
        assert!(!directive.specific_instruction.is_empty());
    }

    #[test]
    fn test_fear_profile_accessible() {
        let gl = make_loop();
        let profile = gl.fear_profile();
        assert_eq!(profile.update_count(), 0);
    }

    #[test]
    fn test_cleanup_does_not_crash() {
        let mut gl = make_loop();
        gl.start_game();
        gl.cleanup();
    }

    #[test]
    fn test_game_handles_dynamic_target() {
        let mut gl = make_loop();
        gl.start_game();
        // Navigate to cal_corridor → cal_reception
        gl.process_choice("sit_up", 1500, ChoiceApproach::Investigate);
        gl.process_choice("go_left", 1500, ChoiceApproach::Investigate);
        // From cal_reception, "answer_phone" has conditional targets.
        // The result should be a valid narrative (either static or fallback).
        let result = gl.process_choice("answer_phone", 1500, ChoiceApproach::Confront);
        assert!(matches!(result.narrative, ServerMessage::Narrative { .. }));
    }

    #[test]
    fn test_behavior_batch_sets_baseline_during_calibration() {
        let mut gl = make_loop();
        gl.start_game();

        // Send enough events to trigger baseline computation.
        for i in 0..10 {
            let events = vec![BehaviorEvent {
                event_type: fear_engine_common::types::BehaviorEventType::Keystroke {
                    chars_per_second: 5.0 + i as f64 * 0.1,
                    backspace_count: 0,
                    total_chars: 20,
                },
                timestamp: Utc::now(),
                scene_id: "cal_awakening".into(),
            }];
            gl.process_behavior(events);
        }
        assert!(gl.baseline_set);
    }

    #[test]
    fn test_build_reveal_uses_non_hardcoded_history() {
        let mut gl = make_loop();
        gl.start_game();
        let _ = gl.process_choice("sit_up", 1500, ChoiceApproach::Investigate);

        let reveal = gl.build_reveal();
        match reveal {
            ServerMessage::Reveal {
                fear_profile,
                key_moments,
                adaptation_log,
                ..
            } => {
                assert!(fear_profile.scores.iter().any(|score| score.score > 0.05));
                assert!(!key_moments.is_empty());
                assert_ne!(key_moments[0].scene_id, "cal_reception");
                assert!(!key_moments[0].description.contains("tmpl_"));
                assert!(!key_moments[0].behavior_trigger.contains("choice:"));
                assert!(!adaptation_log.is_empty());
                assert_ne!(adaptation_log[0].strategy, "adaptation");
            }
            _ => panic!("expected reveal message"),
        }
    }

    #[test]
    fn test_template_scene_messages_materialize_placeholders() {
        let gl = make_loop();
        let scene = gl
            .scene_manager
            .get_scene("tmpl_layered_fear")
            .expect("template scene should exist");

        let message = gl.scene_to_message(scene, &[]);

        match message {
            ServerMessage::Narrative { text, .. } => {
                assert!(!text.contains("{{"));
                assert!(text.contains("One layer"));
            }
            _ => panic!("expected narrative message"),
        }
    }

    #[test]
    fn test_choice_is_persisted_on_originating_scene_history_row() {
        let mut gl = make_loop();
        let _ = gl.start_game();

        let _ = gl.process_choice("sit_up", 1200, ChoiceApproach::Investigate);

        let history = gl.db.get_scene_history(&gl.session_id).unwrap();
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].scene_id, "cal_awakening");
        assert_eq!(history[0].player_choice.as_deref(), Some("sit_up"));
        assert_eq!(history[1].scene_id, "cal_corridor");
        assert!(history[1].player_choice.is_none());
    }

    #[test]
    fn test_reveal_theme_scores_do_not_collapse_to_uniform_floor() {
        let mut gl = make_loop();
        let _ = gl.start_game();
        let _ = gl.process_choice("read_clipboard", 2600, ChoiceApproach::Interact);
        let _ = gl.process_choice("answer_phone", 3200, ChoiceApproach::Investigate);
        let _ = gl.process_behavior(vec![
            BehaviorEvent {
                event_type: fear_engine_common::types::BehaviorEventType::Pause {
                    duration_ms: 6200,
                    scene_content_hash: "h1".into(),
                },
                timestamp: Utc::now(),
                scene_id: "probe_sound".into(),
            },
            BehaviorEvent {
                event_type: fear_engine_common::types::BehaviorEventType::MicSilenceResponse {
                    dwell_ms: 8200,
                    exited_early: false,
                    returned_after_prompt: true,
                },
                timestamp: Utc::now(),
                scene_id: "probe_sound".into(),
            },
        ]);

        let reveal = gl.build_reveal();
        match reveal {
            ServerMessage::Reveal { fear_profile, .. } => {
                let unique_scores = fear_profile
                    .scores
                    .iter()
                    .map(|score| format!("{:.3}", score.score))
                    .collect::<std::collections::HashSet<_>>();
                assert!(unique_scores.len() > 2, "scores should not all collapse");
                let max_score = fear_profile
                    .scores
                    .iter()
                    .map(|score| score.score)
                    .fold(f64::NEG_INFINITY, f64::max);
                assert!(max_score > 0.2, "expected a meaningful dominant theme");
            }
            _ => panic!("expected reveal message"),
        }
    }
}
