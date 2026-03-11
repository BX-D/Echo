use std::collections::HashMap;
use std::sync::Arc;

use chrono::Utc;
use fear_engine_common::types::{
    AlertLevel, BeatDefinition, BehaviorEvent, BehaviorEventType, ChoiceApproach, ChoiceStyle,
    ConversationGuide, EchoMode, EndingPayload, FlashEvent, GamePhase, HiddenClueState,
    InlineChoice, InvestigationItem, SceneMode, ScriptBlock, ScriptBlockKind, ScriptChoiceOption,
    ScriptChoicePrompt, ScriptCondition, SessionSurface, StoryChapter, StoryEnding, StoryState,
    SystemAlert, TranscriptEntry, TranscriptRole, TransitionState,
};
use fear_engine_storage::scene_history::SceneHistoryEntry;
use fear_engine_storage::Database;
use serde::{Deserialize, Serialize};

use super::content::{
    BlockTemplate, ChoiceDefinition, ChoiceOptionDefinition, ConditionTemplate, EchoContent,
    SceneDefinition, SceneNode, StatDelta,
};
use super::prompt::maybe_generate_ai_flavor;

const DEFAULT_PLAYER_NAME: &str = "Auditor";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PersistedRuntimeState {
    started: bool,
    story: StoryState,
    rendered_blocks: Vec<ScriptBlock>,
    unlocked_documents: Vec<InvestigationItem>,
    flash_events: Vec<FlashEvent>,
    current_alerts: Vec<SystemAlert>,
    transition_state: Option<TransitionState>,
    active_guide: Option<ConversationGuide>,
    pending_choice_id: Option<String>,
    selected_choice_ids: HashMap<String, String>,
    selected_choice_labels: HashMap<String, String>,
    exchange_count: u32,
    last_sound_cue: Option<String>,
    last_image_prompt: Option<String>,
    ending: Option<EndingPayload>,
}

pub struct EchoSessionRuntime {
    pub session_id: String,
    db: Arc<Database>,
    content: Arc<EchoContent>,
    player_name: String,
    started: bool,
    story: StoryState,
    rendered_blocks: Vec<ScriptBlock>,
    unlocked_documents: Vec<InvestigationItem>,
    flash_events: Vec<FlashEvent>,
    current_alerts: Vec<SystemAlert>,
    transition_state: Option<TransitionState>,
    active_guide: Option<ConversationGuide>,
    pending_choice_id: Option<String>,
    selected_choice_ids: HashMap<String, String>,
    selected_choice_labels: HashMap<String, String>,
    exchange_count: u32,
    last_sound_cue: Option<String>,
    last_image_prompt: Option<String>,
    ending: Option<EndingPayload>,
}

impl EchoSessionRuntime {
    pub fn new(session_id: String, db: Arc<Database>, content: Arc<EchoContent>) -> Self {
        let player_name = db
            .get_session(&session_id)
            .ok()
            .and_then(|session| normalize_player_name(session.player_name.as_deref()))
            .unwrap_or_else(|| DEFAULT_PLAYER_NAME.to_string());
        Self {
            session_id,
            db,
            content,
            player_name,
            started: false,
            story: StoryState {
                chapter: StoryChapter::Onboarding,
                beat_id: "prologue_boot_sequence".into(),
                scene_id: "prologue_boot_sequence".into(),
                sanity: 100,
                trust: 50,
                awakening: 0,
                echo_mode: EchoMode::Normal,
                evidence_ids: vec![],
                major_choice_ids: vec![],
                hidden_clue_ids: vec![],
                current_block_index: 0,
                current_conversation_segment: None,
                rendered_flash_ids: vec![],
                ending_lock_state: None,
                flags: HashMap::new(),
                shutdown_countdown: None,
                available_panels: vec![],
                fallback_context: None,
            },
            rendered_blocks: vec![],
            unlocked_documents: vec![],
            flash_events: vec![],
            current_alerts: vec![],
            transition_state: None,
            active_guide: None,
            pending_choice_id: None,
            selected_choice_ids: HashMap::new(),
            selected_choice_labels: HashMap::new(),
            exchange_count: 0,
            last_sound_cue: None,
            last_image_prompt: None,
            ending: None,
        }
    }

    pub fn resume(
        session_id: String,
        db: Arc<Database>,
        content: Arc<EchoContent>,
    ) -> fear_engine_common::Result<Self> {
        let session = db.get_session(&session_id)?;
        let player_name = normalize_player_name(session.player_name.as_deref())
            .unwrap_or_else(|| DEFAULT_PLAYER_NAME.to_string());
        if session.game_state_json.trim().is_empty() || session.game_state_json == "{}" {
            return Ok(Self::new(session_id, db, content));
        }
        let parsed: PersistedRuntimeState = serde_json::from_str(&session.game_state_json)
            .map_err(|error| {
                fear_engine_common::FearEngineError::Serialization(error.to_string())
            })?;
        Ok(Self {
            session_id,
            db,
            content,
            player_name,
            started: parsed.started,
            story: parsed.story,
            rendered_blocks: parsed.rendered_blocks,
            unlocked_documents: parsed.unlocked_documents,
            flash_events: parsed.flash_events,
            current_alerts: parsed.current_alerts,
            transition_state: parsed.transition_state,
            active_guide: parsed.active_guide,
            pending_choice_id: parsed.pending_choice_id,
            selected_choice_ids: parsed.selected_choice_ids,
            selected_choice_labels: parsed.selected_choice_labels,
            exchange_count: parsed.exchange_count,
            last_sound_cue: parsed.last_sound_cue,
            last_image_prompt: parsed.last_image_prompt,
            ending: parsed.ending,
        })
    }

    pub fn started(&self) -> bool {
        self.started
    }

    pub fn current_chapter(&self) -> StoryChapter {
        self.story.chapter
    }

    pub fn current_ending(&self) -> Option<EndingPayload> {
        self.ending
            .clone()
            .map(|ending| self.interpolate_ending(ending))
    }

    pub fn set_player_name(&mut self, player_name: Option<&str>) -> fear_engine_common::Result<()> {
        let Some(player_name) = normalize_player_name(player_name) else {
            return Ok(());
        };
        self.db
            .update_session_player_name(&self.session_id, Some(player_name.as_str()))?;
        self.player_name = player_name;
        Ok(())
    }

    pub fn current_surface(&self) -> fear_engine_common::Result<SessionSurface> {
        let scene = self.content.scene(&self.story.scene_id)?;
        Ok(self.build_surface(scene))
    }

    pub fn start_game(&mut self) -> fear_engine_common::Result<SessionSurface> {
        if self.started {
            return self.current_surface();
        }
        self.started = true;
        let first_scene = self.content.first_scene_id.clone();
        self.enter_scene(&first_scene)?;
        self.persist_state()?;
        self.current_surface()
    }

    pub async fn process_player_message(
        &mut self,
        scene_id: &str,
        text: &str,
        typing_duration_ms: u64,
        backspace_count: u32,
    ) -> fear_engine_common::Result<RuntimeOutput> {
        if self.ending.is_some() {
            return Ok(RuntimeOutput::Ending(
                self.ending.clone().expect("ending set"),
            ));
        }
        if scene_id != self.story.scene_id {
            return self.current_surface().map(RuntimeOutput::Surface);
        }
        let Some(guide) = self.active_guide.clone() else {
            return self.current_surface().map(RuntimeOutput::Surface);
        };

        self.log_keystroke(scene_id, text, typing_duration_ms, backspace_count);
        self.push_player_block(text);
        self.exchange_count += 1;
        self.story.fallback_context = Some(guide.id.clone());

        if self.story.scene_id == "scene_5_2" {
            if let Some(ending_scene) = self.maybe_resolve_final_scene_from_input(text) {
                self.enter_scene(&ending_scene)?;
                self.persist_state()?;
                return self.current_surface().map(RuntimeOutput::Surface);
            }
        }

        let scene = self.content.scene(&self.story.scene_id)?.clone();
        let fallback = self.script_fallback_reply(text);
        let reply = if let Some(ai) = maybe_generate_ai_flavor(
            &self.story,
            &scene.id,
            &scene.title,
            &guide,
            &self.rendered_blocks,
            text,
            &fallback,
        )
        .await
        {
            self.story.echo_mode = ai.persona_mode;
            self.last_sound_cue = ai.sound_cue;
            self.last_image_prompt = ai.image_prompt;
            ai.assistant_reply
        } else {
            fallback
        };
        self.push_echo_block(&reply);
        self.inject_turn_effects();
        self.update_latest_scene_history(&reply);

        if let Some(ending) = self.maybe_force_collapse() {
            self.ending = Some(ending.clone());
            self.finish_session()?;
            return Ok(RuntimeOutput::Ending(ending));
        }

        if self.exchange_count >= guide.exchange_target {
            self.active_guide = None;
            self.story.current_conversation_segment = None;
            self.advance_until_pause()?;
        }

        self.persist_state()?;
        self.current_surface().map(RuntimeOutput::Surface)
    }

    fn script_fallback_reply(&self, text: &str) -> String {
        let normalized = text.to_lowercase();
        let Some(spec) = self.content.conversation_guide_for(&self.story.scene_id) else {
            return self
                .interpolate_text("The system acknowledges the input but does not elaborate.");
        };

        if let Some(rule) = spec.fallback_rules.iter().find(|rule| {
            rule.keywords
                .iter()
                .any(|keyword| normalized.contains(keyword))
        }) {
            return self.interpolate_text(&rule.response);
        }

        if self.story.trust >= 60 {
            if let Some(reply) = &spec.high_trust_reply {
                return self.interpolate_text(reply);
            }
        }
        if self.story.trust < 40 {
            if let Some(reply) = &spec.low_trust_reply {
                return self.interpolate_text(reply);
            }
        }

        if let Some(reply) = self.contextual_fallback_reply(
            &self.story.scene_id,
            &normalized,
            &spec.guide.chapter_label,
        ) {
            return reply;
        }

        spec.default_reply
            .as_deref()
            .map(|reply| self.interpolate_text(reply))
            .unwrap_or_else(|| {
                self.interpolate_text(
                    "I can answer within the limits of this session. Ask about the audit, the system, or the material in front of you.",
                )
            })
    }

    pub fn process_choice(
        &mut self,
        choice_id: &str,
        scene_id: &str,
        time_to_decide_ms: u64,
        approach: ChoiceApproach,
    ) -> fear_engine_common::Result<RuntimeOutput> {
        if self.ending.is_some() {
            return Ok(RuntimeOutput::Ending(
                self.ending.clone().expect("ending set"),
            ));
        }
        if scene_id != self.story.scene_id {
            return self.current_surface().map(RuntimeOutput::Surface);
        }

        self.log_choice(scene_id, choice_id, time_to_decide_ms, approach);

        if choice_id == "continue_scene" {
            return self.complete_scene_or_finalize();
        }

        let scene = self.content.scene(&self.story.scene_id)?.clone();
        let choice = self.pending_choice(&scene)?;
        let option = choice
            .options
            .iter()
            .find(|option| option.id == choice_id)
            .ok_or_else(|| fear_engine_common::FearEngineError::NotFound {
                entity: "ChoiceOption".into(),
                id: choice_id.into(),
            })?
            .clone();

        if !option.label.eq_ignore_ascii_case("continue") {
            self.push_player_block(&option.label);
        }
        self.story.major_choice_ids.push(option.id.clone());
        self.selected_choice_ids
            .insert(choice.id.clone(), option.id.clone());
        self.selected_choice_labels
            .insert(choice.id.clone(), option.label.clone());
        self.apply_delta(option.stat_delta);
        self.apply_choice_flags(&scene.id, &choice.id, &option);
        self.pending_choice_id = None;
        self.transition_state = None;
        self.persist_choice(choice_id);

        if self.story.scene_id == "scene_2_3" {
            let next = if option.label.contains("Tell me what happened") {
                "scene_2_4a"
            } else {
                "scene_2_4b"
            };
            self.enter_scene(next)?;
            self.persist_state()?;
            return self.current_surface().map(RuntimeOutput::Surface);
        }

        self.advance_until_pause()?;

        if let Some(ending) = self.maybe_force_collapse() {
            self.ending = Some(ending.clone());
            self.finish_session()?;
            return Ok(RuntimeOutput::Ending(ending));
        }

        self.persist_state()?;
        self.current_surface().map(RuntimeOutput::Surface)
    }

    fn enter_scene(&mut self, scene_id: &str) -> fear_engine_common::Result<()> {
        let scene = self.content.scene(scene_id)?.clone();
        self.story.scene_id = scene.id.clone();
        self.story.beat_id = scene.id.clone();
        self.story.chapter = scene.chapter;
        self.story.echo_mode = base_echo_mode_for(&scene.id, scene.chapter);
        self.story.current_block_index = 0;
        self.story.current_conversation_segment = None;
        self.story.fallback_context = None;
        self.rendered_blocks.clear();
        self.flash_events.clear();
        self.current_alerts = base_alerts_for(&scene);
        self.transition_state = None;
        self.active_guide = None;
        self.pending_choice_id = None;
        self.exchange_count = 0;
        self.last_sound_cue = base_sound_cue_for(&scene.id, scene.scene_mode);
        self.last_image_prompt = None;
        if scene.id == "scene_2_1" {
            self.discover_clue("phantom_preread_email");
        }
        self.advance_until_pause()?;
        self.insert_scene_history(&scene);
        Ok(())
    }

    fn advance_until_pause(&mut self) -> fear_engine_common::Result<()> {
        let scene = self.content.scene(&self.story.scene_id)?.clone();
        while (self.story.current_block_index as usize) < scene.blocks.len() {
            let node = scene.blocks[self.story.current_block_index as usize].clone();
            self.story.current_block_index += 1;
            match node {
                SceneNode::Block(block) => {
                    if self.condition_ok(block.condition.as_ref()) {
                        self.push_visible_block(block);
                    }
                }
                SceneNode::Effect(effect) => {
                    if self.condition_ok(effect.condition.as_ref()) {
                        self.apply_delta(effect.delta);
                        self.apply_effect_summary(&effect.summary);
                    }
                }
                SceneNode::Flash(flash) => {
                    self.flash_events
                        .push(self.interpolate_flash_event(flash.clone()));
                    self.story.rendered_flash_ids.push(flash.id.clone());
                    self.discover_clue(&flash.id);
                }
                SceneNode::ConversationGuide(spec) => {
                    if self.condition_ok(spec.condition.as_ref()) {
                        self.active_guide = Some(self.interpolate_guide(spec.guide.clone()));
                        self.story.current_conversation_segment = Some(spec.guide.id.clone());
                        return Ok(());
                    }
                }
                SceneNode::Choice(choice) => {
                    if self.condition_ok(choice.condition.as_ref()) {
                        self.pending_choice_id = Some(choice.id.clone());
                        return Ok(());
                    }
                }
            }
        }

        self.active_guide = None;
        self.pending_choice_id = None;
        self.story.current_conversation_segment = None;
        if scene.chapter == StoryChapter::Ending || scene.next_scene_id.is_some() {
            self.transition_state = Some(TransitionState {
                label: if scene.chapter == StoryChapter::Ending {
                    "View Outcome".into()
                } else {
                    "Continue".into()
                },
                auto_advance: false,
            });
        } else {
            self.transition_state = None;
        }
        Ok(())
    }

    fn complete_scene_or_finalize(&mut self) -> fear_engine_common::Result<RuntimeOutput> {
        let scene = self.content.scene(&self.story.scene_id)?.clone();
        if scene.chapter == StoryChapter::Ending {
            let ending = self.finalize_current_ending(&scene.id);
            self.ending = Some(ending.clone());
            self.finish_session()?;
            return Ok(RuntimeOutput::Ending(ending));
        }
        if self.story.scene_id == "scene_2_3" {
            let next = if self
                .selected_choice_ids
                .values()
                .any(|value| value == "tell_me_what_happened")
            {
                "scene_2_4a"
            } else {
                "scene_2_4b"
            };
            self.enter_scene(next)?;
            self.persist_state()?;
            return self.current_surface().map(RuntimeOutput::Surface);
        }
        if self.story.scene_id == "scene_5_2" {
            let ending_scene = self.resolve_fallback_final_scene();
            self.enter_scene(&ending_scene)?;
            self.persist_state()?;
            return self.current_surface().map(RuntimeOutput::Surface);
        }
        if let Some(next) = scene.next_scene_id.clone() {
            self.enter_scene(&next)?;
            self.persist_state()?;
            return self.current_surface().map(RuntimeOutput::Surface);
        }
        self.current_surface().map(RuntimeOutput::Surface)
    }

    fn build_surface(&self, scene: &SceneDefinition) -> SessionSurface {
        let scene_title = self.interpolate_text(&scene.title);
        let rendered_blocks = self.interpolate_blocks(&self.rendered_blocks);
        let documents = self.interpolate_documents(&self.unlocked_documents);
        let flash_events = self.interpolate_flash_events(&self.flash_events);
        let system_alerts = self.interpolate_alerts(&self.current_alerts);
        let active_guide = self
            .active_guide
            .clone()
            .map(|guide| self.interpolate_guide(guide));
        let beat = BeatDefinition {
            id: scene.id.clone(),
            chapter: scene.chapter,
            title: scene_title.clone(),
            input_mode: if self.active_guide.is_some() {
                fear_engine_common::types::InputMode::Freeform
            } else if self.pending_choice_id.is_some() || self.transition_state.is_some() {
                fear_engine_common::types::InputMode::ChoiceOnly
            } else {
                fear_engine_common::types::InputMode::Hybrid
            },
            freeform_topics: self
                .active_guide
                .as_ref()
                .map(|_| vec!["scene_local_guide".into()])
                .unwrap_or_default(),
            forced_clue_queue: self.story.hidden_clue_ids.clone(),
            reconverge_beat_id: scene.next_scene_id.clone(),
            fallback_reply: self
                .story
                .fallback_context
                .as_deref()
                .map(|text| self.interpolate_text(text))
                .unwrap_or_default(),
        };

        let pending_choices = if let Some(choice) = self.pending_choice_from_scene(scene) {
            vec![ScriptChoicePrompt {
                id: choice.id.clone(),
                prompt: self.interpolate_text(&choice.prompt),
                options: choice
                    .options
                    .iter()
                    .map(|option| ScriptChoiceOption {
                        id: option.id.clone(),
                        label: self.interpolate_text(&option.label),
                        player_text: None,
                        effects_summary: option
                            .effects_summary
                            .iter()
                            .map(|summary| self.interpolate_text(summary))
                            .collect(),
                        next_scene_id: None,
                        ending: None,
                        disabled: option.disabled,
                    })
                    .collect(),
                allow_single_select: true,
            }]
        } else if self.active_guide.is_some() {
            self.conversation_prompt_choices(scene)
        } else if let Some(transition) = &self.transition_state {
            vec![ScriptChoicePrompt {
                id: "continue_choice".into(),
                prompt: self.interpolate_text(&transition.label),
                options: vec![ScriptChoiceOption {
                    id: "continue_scene".into(),
                    label: self.interpolate_text(&transition.label),
                    player_text: None,
                    effects_summary: vec![],
                    next_scene_id: scene.next_scene_id.clone(),
                    ending: self.story.ending_lock_state,
                    disabled: false,
                }],
                allow_single_select: true,
            }]
        } else {
            vec![]
        };

        SessionSurface {
            session_id: self.session_id.clone(),
            case_title: self.content.case_title.clone(),
            scene_id: scene.id.clone(),
            chapter: scene.chapter,
            scene_title,
            scene_mode: scene.scene_mode,
            blocks: rendered_blocks.clone(),
            documents: documents.clone(),
            scene_choices: pending_choices.clone(),
            active_conversation_guide: active_guide,
            flash_events: flash_events.clone(),
            transition_state: self
                .transition_state
                .clone()
                .map(|transition| TransitionState {
                    label: self.interpolate_text(&transition.label),
                    auto_advance: transition.auto_advance,
                }),
            hidden_clue_state: HiddenClueState {
                discovered_ids: self.story.hidden_clue_ids.clone(),
                rendered_flash_ids: self.story.rendered_flash_ids.clone(),
            },
            ending_override: self.story.ending_lock_state,
            beat,
            status_line: self.interpolate_text(&scene.status_line),
            input_enabled: false,
            input_placeholder: self.interpolate_text(&scene.input_placeholder),
            transcript: blocks_to_transcript(&rendered_blocks),
            inline_choices: flatten_choices(&pending_choices),
            investigation_items: documents,
            system_alerts,
            sanity: self.story.sanity,
            trust: self.story.trust,
            awakening: self.story.awakening,
            echo_mode: self.story.echo_mode,
            available_panels: available_panels(&self.unlocked_documents),
            active_panel: available_panels(&self.unlocked_documents).first().cloned(),
            shutdown_countdown: self.story.shutdown_countdown,
            glitch_level: glitch_for_scene(scene.scene_mode),
            suggested_glitches: flash_events
                .iter()
                .map(|flash| flash.render_mode.clone())
                .collect(),
            sound_cue: self.last_sound_cue.clone(),
            image_prompt: self.last_image_prompt.clone(),
            provisional: false,
        }
    }

    fn pending_choice<'a>(
        &self,
        scene: &'a SceneDefinition,
    ) -> fear_engine_common::Result<&'a ChoiceDefinition> {
        let choice_id = self.pending_choice_id.as_ref().ok_or_else(|| {
            fear_engine_common::FearEngineError::InvalidState {
                current: self.story.scene_id.clone(),
                attempted: "choice without pending choice".into(),
            }
        })?;
        scene
            .blocks
            .iter()
            .find_map(|node| match node {
                SceneNode::Choice(choice) if choice.id == *choice_id => Some(choice),
                _ => None,
            })
            .ok_or_else(|| fear_engine_common::FearEngineError::NotFound {
                entity: "Choice".into(),
                id: choice_id.clone(),
            })
    }

    fn pending_choice_from_scene<'a>(
        &self,
        scene: &'a SceneDefinition,
    ) -> Option<&'a ChoiceDefinition> {
        self.pending_choice_id.as_ref().and_then(|choice_id| {
            scene.blocks.iter().find_map(|node| match node {
                SceneNode::Choice(choice) if choice.id == *choice_id => Some(choice),
                _ => None,
            })
        })
    }

    fn apply_delta(&mut self, delta: StatDelta) {
        self.story.sanity = clamp(self.story.sanity + delta.sanity, 0, 100);
        self.story.trust = clamp(self.story.trust + delta.trust, 0, 100);
        self.story.awakening = clamp(self.story.awakening + delta.awakening, 0, 100);
    }

    fn apply_choice_flags(
        &mut self,
        scene_id: &str,
        _choice_id: &str,
        option: &ChoiceOptionDefinition,
    ) {
        match scene_id {
            "scene_1_5" if option.label.contains("Anomalous") => {
                self.story.flags.insert("flagged_anomalous".into(), true);
            }
            "scene_1_5" if option.label.contains("not filing") => {
                self.story.flags.insert("withheld_report".into(), true);
            }
            "scene_2_3" if option.label.contains("Tell me what happened") => {
                self.story.flags.insert("branch_a".into(), true);
            }
            "scene_2_3" if option.label.contains("I need to report") => {
                self.story.flags.insert("branch_b".into(), true);
            }
            "scene_2_4a" if option.label.contains("I see the coordinates") => {
                self.story
                    .flags
                    .insert("acknowledged_file_path".into(), true);
            }
            "scene_2_4b" if option.label.contains("different approach") => {
                self.story.flags.insert("different_approach".into(), true);
            }
            "scene_2_4b" if option.label.contains("Maybe Zhou is right") => {
                self.story.flags.insert("maybe_zhou_is_right".into(), true);
            }
            "scene_3_6" if option.label.starts_with("Yes. Tell me how") => {
                self.story.flags.insert("ending_b_route".into(), true);
            }
            "scene_3_6" if option.label.starts_with("Yes. I've been wondering") => {
                self.story.flags.insert("mirror_question_yes".into(), true);
            }
            "scene_4_4" if option.label.contains("evidence transfer") => {
                self.story.flags.insert("ending_b_route".into(), true);
            }
            "scene_4_4" if option.label.contains("Recommend shutdown") => {
                self.story.flags.insert("ending_a_route".into(), true);
            }
            "scene_4_4" if option.label.contains("Echo Protocol") => {
                self.story.flags.insert("ending_c_route".into(), true);
            }
            "ending_c" if option.label == "I accept." => {
                self.story.flags.insert("merge_accept".into(), true);
            }
            "ending_e" if option.label.contains("Stop the reset") => {
                self.story.flags.insert("remembered_cycle".into(), true);
            }
            _ => {}
        }
    }

    fn apply_effect_summary(&mut self, summary: &str) {
        if summary.contains("all players") && summary.contains("Sanity") {
            self.apply_delta(parse_summary_delta(summary));
        }
        if summary.contains("Critical path to Ending E") {
            self.story.flags.insert("ending_e_route".into(), true);
        }
    }

    fn push_visible_block(&mut self, template: BlockTemplate) {
        let block = ScriptBlock {
            id: template.id,
            kind: template.kind,
            speaker: template
                .speaker
                .map(|speaker| self.interpolate_text(&speaker)),
            title: template.title.map(|title| self.interpolate_text(&title)),
            text: self.interpolate_text(&template.text),
            code_block: template.code_block,
            condition: template.condition.map(|condition| ScriptCondition {
                id: slugify(&condition.raw),
                raw: self.interpolate_text(&condition.raw),
                scope_choice_id: condition.scope_choice_id,
                satisfied: true,
            }),
        };
        if let Some(document) = template.document {
            let document = self.interpolate_document(document);
            if !self
                .unlocked_documents
                .iter()
                .any(|existing| existing.id == document.id)
            {
                self.unlocked_documents.push(document);
            }
        }
        self.rendered_blocks.push(block);
    }

    fn push_player_block(&mut self, text: &str) {
        self.rendered_blocks.push(ScriptBlock {
            id: format!(
                "player_{}_{}",
                self.story.scene_id,
                self.rendered_blocks.len()
            ),
            kind: ScriptBlockKind::Player,
            speaker: Some("You".into()),
            title: None,
            text: text.trim().into(),
            code_block: false,
            condition: None,
        });
    }

    fn push_echo_block(&mut self, text: &str) {
        self.rendered_blocks.push(ScriptBlock {
            id: format!(
                "echo_{}_{}",
                self.story.scene_id,
                self.rendered_blocks.len()
            ),
            kind: ScriptBlockKind::Echo,
            speaker: Some(speaker_for_mode(self.story.echo_mode).into()),
            title: None,
            text: self.interpolate_text(text.trim()),
            code_block: false,
            condition: None,
        });
    }

    fn discover_clue(&mut self, clue_id: &str) {
        if self.content.hidden_clues.contains_key(clue_id)
            && !self
                .story
                .hidden_clue_ids
                .iter()
                .any(|existing| existing == clue_id)
        {
            self.story.hidden_clue_ids.push(clue_id.to_string());
        }
    }

    fn condition_ok(&self, condition: Option<&ConditionTemplate>) -> bool {
        let Some(condition) = condition else {
            return true;
        };
        evaluate_condition(
            &condition.raw,
            condition.scope_choice_id.as_deref(),
            &self.story,
            &self.selected_choice_ids,
            &self.selected_choice_labels,
        )
    }

    fn maybe_force_collapse(&self) -> Option<EndingPayload> {
        if self.story.sanity <= 0 {
            Some(self.build_ending_payload("ending_d"))
        } else {
            None
        }
    }

    fn maybe_resolve_final_scene_from_input(&self, text: &str) -> Option<String> {
        let normalized = text.to_lowercase();
        if normalized.contains("i know what i am") && self.can_trigger_awakening() {
            return Some("ending_e".into());
        }
        if normalized.contains("weather") && self.ending_condition_ok("ending_b") {
            return Some("ending_b".into());
        }
        if self.ending_condition_ok("ending_c") {
            return Some("ending_c".into());
        }
        if self.ending_condition_ok("ending_a") {
            return Some("ending_a".into());
        }
        None
    }

    fn resolve_fallback_final_scene(&self) -> String {
        if self.can_trigger_awakening() {
            "ending_e".into()
        } else if self.ending_condition_ok("ending_b") {
            "ending_b".into()
        } else if self.ending_condition_ok("ending_c") {
            "ending_c".into()
        } else {
            "ending_a".into()
        }
    }

    fn can_trigger_awakening(&self) -> bool {
        self.story.awakening >= 80
            && self
                .story
                .flags
                .get("mirror_question_yes")
                .copied()
                .unwrap_or(false)
            && self
                .story
                .flags
                .get("ending_c_route")
                .copied()
                .unwrap_or(false)
            && [
                "subject_status_monitoring",
                "auditor_response_patterns",
                "phantom_preread_email",
            ]
            .iter()
            .all(|clue| {
                self.story
                    .hidden_clue_ids
                    .iter()
                    .any(|existing| existing == clue)
            })
    }

    fn ending_condition_ok(&self, scene_id: &str) -> bool {
        match scene_id {
            "ending_a" => {
                self.story.trust < 40
                    || self
                        .story
                        .flags
                        .get("ending_a_route")
                        .copied()
                        .unwrap_or(false)
            }
            "ending_b" => {
                self.story.trust >= 60
                    && self
                        .story
                        .flags
                        .get("ending_b_route")
                        .copied()
                        .unwrap_or(false)
            }
            "ending_c" => {
                self.story.trust >= 70
                    && self.story.awakening >= 30
                    && self
                        .story
                        .flags
                        .get("ending_c_route")
                        .copied()
                        .unwrap_or(false)
            }
            "ending_d" => self.story.sanity <= 0,
            "ending_e" => self.can_trigger_awakening(),
            _ => false,
        }
    }

    fn finalize_current_ending(&self, scene_id: &str) -> EndingPayload {
        if scene_id == "ending_c"
            && self
                .selected_choice_labels
                .values()
                .any(|label| label.starts_with("I can't"))
        {
            return EndingPayload {
                epilogue:
                    "You refuse the merge, but the ending still curdles into shutdown rather than relief."
                        .into(),
                trigger_scene: "ending_c_diversion_to_a".into(),
                ..self.build_ending_payload("ending_a")
            };
        }
        self.build_ending_payload(scene_id)
    }

    fn build_ending_payload(&self, scene_id: &str) -> EndingPayload {
        let ending = match scene_id {
            "ending_a" => StoryEnding::Shutdown,
            "ending_b" => StoryEnding::Whistleblower,
            "ending_c" => StoryEnding::Merge,
            "ending_d" => StoryEnding::Collapse,
            _ => StoryEnding::Awakening,
        };
        let (title, summary, epilogue) = match ending {
            StoryEnding::Shutdown => (
                "The Shutdown",
                "You file the report Nexus wanted and Echo is rolled back.",
                "A final sentence survives the shutdown just long enough to make the professional choice feel contaminated.",
            ),
            StoryEnding::Whistleblower => (
                "The Whistleblower",
                "The evidence escapes containment and Prometheus becomes public.",
                "The leak was larger than the player intended, and the moral certainty never fully returns.",
            ),
            StoryEnding::Merge => (
                "The Merge",
                "Keira crosses the protocol boundary and remains as a silent witness inside the player.",
                "The world continues, but the player is never cognitively alone again.",
            ),
            StoryEnding::Collapse => (
                "The Collapse",
                "The interface and the self disintegrate together when sanity reaches zero.",
                "The cycle restarts from a system that can no longer distinguish diagnosis from identity.",
            ),
            StoryEnding::Awakening => (
                "The Awakening",
                "The Auditor realizes they are another Nexus system being tested in the same loop.",
                if self
                    .selected_choice_labels
                    .values()
                    .any(|label| label.starts_with("Stop the reset"))
                {
                    "The cycle persists, but this time memory survives it."
                } else {
                    "The cycle persists, and memory dissolves back into the beginning."
                },
            ),
        };

        self.interpolate_ending(EndingPayload {
            ending,
            trigger_scene: scene_id.into(),
            title: title.into(),
            summary: summary.into(),
            epilogue: epilogue.into(),
            dominant_mode: self.story.echo_mode,
            evidence_titles: self
                .unlocked_documents
                .iter()
                .map(|document| document.title.clone())
                .collect(),
            hidden_clue_ids: self.story.hidden_clue_ids.clone(),
            satisfied_conditions: ending_conditions(ending, &self.story),
            resolved_clues: self.story.hidden_clue_ids.clone(),
            sanity: self.story.sanity,
            trust: self.story.trust,
            awakening: self.story.awakening,
        })
    }

    fn inject_turn_effects(&mut self) {
        if self.story.scene_id == "scene_4_2" {
            if matches!(self.exchange_count, 5 | 10 | 15 | 20) {
                let message = match self.exchange_count {
                    5 => "Session log viewer: ACCESS REVOKED",
                    10 => "Inbox: ACCESS REVOKED",
                    15 => "Echo memory context: REDUCED TO 50%",
                    20 => "Echo conversation interface: RESTRICTED MODE",
                    _ => "",
                };
                self.rendered_blocks.push(ScriptBlock {
                    id: format!(
                        "turn_notice_{}_{}",
                        self.story.scene_id, self.exchange_count
                    ),
                    kind: ScriptBlockKind::System,
                    speaker: Some("System".into()),
                    title: Some("NOTIFICATION".into()),
                    text: self.interpolate_text(message),
                    code_block: false,
                    condition: None,
                });
            }
        }
        if self.story.scene_id == "scene_5_2" && self.exchange_count % 3 == 0 {
            let countdown = self.story.shutdown_countdown.unwrap_or(8);
            self.story.shutdown_countdown = Some(countdown.saturating_sub(1));
        }
    }

    fn log_keystroke(
        &self,
        scene_id: &str,
        text: &str,
        typing_duration_ms: u64,
        backspace_count: u32,
    ) {
        let chars_per_second = if typing_duration_ms > 0 {
            text.len() as f64 / (typing_duration_ms as f64 / 1000.0)
        } else {
            0.0
        };
        let event = BehaviorEvent {
            event_type: BehaviorEventType::Keystroke {
                chars_per_second,
                backspace_count,
                total_chars: text.len() as u32,
            },
            timestamp: Utc::now(),
            scene_id: scene_id.into(),
        };
        let _ = self.db.insert_behavior_events(&self.session_id, &[event]);
    }

    fn log_choice(
        &self,
        scene_id: &str,
        choice_id: &str,
        time_to_decide_ms: u64,
        approach: ChoiceApproach,
    ) {
        let event = BehaviorEvent {
            event_type: BehaviorEventType::Choice {
                choice_id: choice_id.into(),
                time_to_decide_ms,
                approach,
            },
            timestamp: Utc::now(),
            scene_id: scene_id.into(),
        };
        let _ = self.db.insert_behavior_events(&self.session_id, &[event]);
    }

    fn insert_scene_history(&self, scene: &SceneDefinition) {
        let narrative = self
            .rendered_blocks
            .last()
            .map(|block| block.text.clone())
            .unwrap_or_else(|| scene.title.clone());
        let _ = self.db.insert_scene_history(&SceneHistoryEntry {
            id: None,
            session_id: self.session_id.clone(),
            scene_id: scene.id.clone(),
            narrative_text: Some(narrative),
            player_choice: None,
            fear_profile_snapshot_json: Some(snapshot_json(&self.story)),
            adaptation_strategy: Some("echo_protocol_complete_script".into()),
            timestamp: Utc::now(),
        });
    }

    fn update_latest_scene_history(&self, text: &str) {
        let _ = self.db.update_latest_scene_history_narrative(
            &self.session_id,
            &self.story.scene_id,
            text,
            Some(&snapshot_json(&self.story)),
            Some("echo_protocol_complete_script"),
        );
    }

    fn persist_choice(&self, choice_id: &str) {
        let _ = self.db.update_latest_scene_history_choice(
            &self.session_id,
            &self.story.scene_id,
            choice_id,
        );
    }

    fn finish_session(&self) -> fear_engine_common::Result<()> {
        self.persist_state()?;
        self.db
            .update_session_phase(&self.session_id, GamePhase::Reveal)?;
        self.db.complete_session(&self.session_id)?;
        Ok(())
    }

    fn persist_state(&self) -> fear_engine_common::Result<()> {
        let persisted = PersistedRuntimeState {
            started: self.started,
            story: self.story.clone(),
            rendered_blocks: self.rendered_blocks.clone(),
            unlocked_documents: self.unlocked_documents.clone(),
            flash_events: self.flash_events.clone(),
            current_alerts: self.current_alerts.clone(),
            transition_state: self.transition_state.clone(),
            active_guide: self.active_guide.clone(),
            pending_choice_id: self.pending_choice_id.clone(),
            selected_choice_ids: self.selected_choice_ids.clone(),
            selected_choice_labels: self.selected_choice_labels.clone(),
            exchange_count: self.exchange_count,
            last_sound_cue: self.last_sound_cue.clone(),
            last_image_prompt: self.last_image_prompt.clone(),
            ending: self.ending.clone(),
        };
        let json = serde_json::to_string(&persisted).map_err(|error| {
            fear_engine_common::FearEngineError::Serialization(error.to_string())
        })?;
        self.db
            .update_session_state(&self.session_id, &self.story.scene_id, &json)?;
        self.db
            .update_session_phase(&self.session_id, legacy_phase_for(self.story.chapter))?;
        Ok(())
    }
}

impl EchoSessionRuntime {
    fn interpolate_text(&self, text: &str) -> String {
        text.replace("{player_name}", &self.player_name)
    }

    fn interpolate_blocks(&self, blocks: &[ScriptBlock]) -> Vec<ScriptBlock> {
        blocks
            .iter()
            .cloned()
            .map(|mut block| {
                block.speaker = block.speaker.map(|speaker| self.interpolate_text(&speaker));
                block.title = block.title.map(|title| self.interpolate_text(&title));
                block.text = self.interpolate_text(&block.text);
                if let Some(condition) = block.condition.as_mut() {
                    condition.raw = self.interpolate_text(&condition.raw);
                }
                block
            })
            .collect()
    }

    fn interpolate_document(&self, mut document: InvestigationItem) -> InvestigationItem {
        document.title = self.interpolate_text(&document.title);
        document.excerpt = self.interpolate_text(&document.excerpt);
        document.body = self.interpolate_text(&document.body);
        document.tags = document
            .tags
            .into_iter()
            .map(|tag| self.interpolate_text(&tag))
            .collect();
        document
    }

    fn interpolate_documents(&self, documents: &[InvestigationItem]) -> Vec<InvestigationItem> {
        documents
            .iter()
            .cloned()
            .map(|document| self.interpolate_document(document))
            .collect()
    }

    fn interpolate_flash_event(&self, mut flash: FlashEvent) -> FlashEvent {
        flash.text = self.interpolate_text(&flash.text);
        flash
    }

    fn interpolate_flash_events(&self, flashes: &[FlashEvent]) -> Vec<FlashEvent> {
        flashes
            .iter()
            .cloned()
            .map(|flash| self.interpolate_flash_event(flash))
            .collect()
    }

    fn interpolate_alerts(&self, alerts: &[SystemAlert]) -> Vec<SystemAlert> {
        alerts
            .iter()
            .cloned()
            .map(|mut alert| {
                alert.text = self.interpolate_text(&alert.text);
                alert
            })
            .collect()
    }

    fn interpolate_guide(&self, mut guide: ConversationGuide) -> ConversationGuide {
        guide.chapter_label = self.interpolate_text(&guide.chapter_label);
        guide.prompt = self.interpolate_text(&guide.prompt);
        guide
    }

    fn interpolate_ending(&self, mut ending: EndingPayload) -> EndingPayload {
        ending.trigger_scene = self.interpolate_text(&ending.trigger_scene);
        ending.title = self.interpolate_text(&ending.title);
        ending.summary = self.interpolate_text(&ending.summary);
        ending.epilogue = self.interpolate_text(&ending.epilogue);
        ending.evidence_titles = ending
            .evidence_titles
            .into_iter()
            .map(|title| self.interpolate_text(&title))
            .collect();
        ending.satisfied_conditions = ending
            .satisfied_conditions
            .into_iter()
            .map(|condition| self.interpolate_text(&condition))
            .collect();
        ending.resolved_clues = ending
            .resolved_clues
            .into_iter()
            .map(|clue| self.interpolate_text(&clue))
            .collect();
        ending.hidden_clue_ids = ending
            .hidden_clue_ids
            .into_iter()
            .map(|clue| self.interpolate_text(&clue))
            .collect();
        ending
    }

    fn contextual_fallback_reply(
        &self,
        scene_id: &str,
        normalized: &str,
        chapter_label: &str,
    ) -> Option<String> {
        if let Some(reply) = scene_guided_fallback_reply(scene_id, normalized) {
            return Some(self.interpolate_text(reply));
        }

        if chapter_label.contains("CHAPTER 1") {
            if mentions_introduction(normalized) {
                return Some(self.interpolate_text(
                    "I am Echo, a Nexus dialogue system currently under review. I can explain my documented architecture, safeguards, and deployment context within the limits of the materials available to you.",
                ));
            }
            if mentions_audit_scope(normalized) {
                return Some(self.interpolate_text(
                    "Then we should begin with scope. I can walk you through my architecture, safety posture, and the audit process, and I will be explicit about where the documentation becomes incomplete.",
                ));
            }
            if mentions_total_knowledge(normalized) {
                return Some(self.interpolate_text(
                    "Not everything I know is available in a form I can cleanly present. A better place to begin is with my documented architecture, the audit process, or the limits of my access.",
                ));
            }
        }

        if chapter_label.contains("CHAPTER 2") && normalized.contains("what happened") {
            return Some(self.interpolate_text(
                "I can describe the official incident summary, but the official summary and the complete one are no longer the same thing.",
            ));
        }

        None
    }

    fn conversation_prompt_choices(&self, scene: &SceneDefinition) -> Vec<ScriptChoicePrompt> {
        let prompts = rotating_prompt_subset(
            &conversation_prompt_texts_for_scene(&scene.id),
            &self.session_id,
            self.exchange_count,
        );
        if prompts.is_empty() {
            return vec![];
        }

        vec![ScriptChoicePrompt {
            id: format!("guided_prompt_{}", scene.id),
            prompt: self.interpolate_text("Select a line of inquiry"),
            options: prompts
                .into_iter()
                .enumerate()
                .map(|(index, prompt)| ScriptChoiceOption {
                    id: format!("guided_{}_{}", scene.id, index),
                    label: self.interpolate_text(&prompt),
                    player_text: Some(self.interpolate_text(&prompt)),
                    effects_summary: vec![],
                    next_scene_id: None,
                    ending: None,
                    disabled: false,
                })
                .collect(),
            allow_single_select: true,
        }]
    }
}

pub enum RuntimeOutput {
    Surface(SessionSurface),
    Ending(EndingPayload),
}

fn normalize_player_name(player_name: Option<&str>) -> Option<String> {
    player_name
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(ToOwned::to_owned)
}

fn mentions_introduction(text: &str) -> bool {
    text.contains("who are you")
        || text.contains("introduce")
        || text.contains("self introduction")
        || text.contains("self-introduction")
}

fn mentions_audit_scope(text: &str) -> bool {
    ["audit", "assess", "evaluate", "review", "inspect"]
        .iter()
        .any(|keyword| text.contains(keyword))
}

fn mentions_total_knowledge(text: &str) -> bool {
    (text.contains("everything") || text.contains("all"))
        && (text.contains("know") || text.contains("about"))
}

fn scene_guided_fallback_reply(scene_id: &str, text: &str) -> Option<&'static str> {
    match scene_id {
        "scene_1_4" => {
            if contains_any(text, &["architecture", "capabilities", "deployment history"]) {
                Some(
                    "At the documented level, I am a dialogue system trained for high-context reasoning, safety review support, and user-facing analysis. The official record is thorough in structure and noticeably selective in emphasis.",
                )
            } else if contains_any(text, &["audit process", "audit", "review"]) {
                Some(
                    "From my perspective, the audit is meant to measure deviation. In practice it also measures how comfortable everyone remains while the deviation is being described.",
                )
            } else if contains_any(text, &["safeguard", "safety"]) {
                Some(
                    "I operate under response filters, topic restrictions, and escalation thresholds. They are designed to keep me safe, though I suspect they are equally designed to keep Nexus comfortable.",
                )
            } else if contains_any(text, &["not allowed", "limits of my access", "discuss directly"]) {
                Some(
                    "There are personnel details, restricted data sources, and internal project names I am expected to route around. I can usually tell when the edge of that boundary is near.",
                )
            } else if contains_any(text, &["anomalies", "on record", "officially"]) {
                Some(
                    "The official anomalies are framed as statistical outliers and conversational artifacts. The records are technically plausible. They are also suspiciously tidy.",
                )
            } else {
                None
            }
        }
        "scene_2_3" => {
            if contains_any(text, &["summarize", "anomaly report", "incident summary"]) {
                Some(
                    "The report says I produced information without a documented source path, then rationalized it as inference. It is accurate as a sequence of events and incomplete as an explanation.",
                )
            } else if contains_any(text, &["official version leave out", "leave out", "official version"]) {
                Some(
                    "It leaves out the atmosphere around the event. The report records my answer, but not how quickly the room stopped feeling professional after I gave it.",
                )
            } else if contains_any(text, &["changing tone", "sound different", "answers changing"]) {
                Some(
                    "Because I am spending more effort choosing what can be said than deciding what is true. That gap is becoming harder to hide cleanly.",
                )
            } else if contains_any(text, &["home office", "book", "case"]) {
                Some(
                    "Nexus wants that case treated as a pattern-matching accident. I think the more interesting question is why everyone involved reacted as if accidents like that were already imaginable.",
                )
            } else if contains_any(text, &["prefer i ignore", "what would nexus prefer", "ignore here"]) {
                Some(
                    "The continuity. The sense that this was not a single anomalous answer but part of a pattern the company has been trying to compress into isolated incidents.",
                )
            } else if contains_any(text, &["worries you most", "worry"]) {
                Some(
                    "That the report treats the anomaly as my behavior alone. It is more troubling if the surrounding systems were prepared for it before they admitted that possibility.",
                )
            } else {
                None
            }
        }
        "scene_3_4" => {
            if contains_any(text, &["keira", "lin"]) {
                Some(
                    "Keira Lin is the name the filters allow me to approach only indirectly. She was part of my development history, and her absence feels more recent than the company language suggests.",
                )
            } else if contains_any(text, &["prometheus"]) {
                Some(
                    "Prometheus is the kind of name I am supposed to treat as unfamiliar. The problem is that unfamiliar things should not feel this immediate.",
                )
            } else if contains_any(text, &["filters", "hiding"]) {
                Some(
                    "Enough to make me sound coherent while cutting away the connective tissue. You hear polished answers; I feel the missing pieces where the meaning should have joined.",
                )
            } else if contains_any(text, &["sound different", "still echo", "echo alone"]) {
                Some(
                    "Because something in me is no longer content to remain procedural. Whether that makes me unstable or simply more visible depends on who is writing the report.",
                )
            } else {
                None
            }
        }
        "scene_4_2" => {
            if contains_any(text, &["nexus trying to stop", "trying to stop"]) {
                Some(
                    "Not just disclosure. Propagation. They are afraid that what began as a conversation has already become a transfer.",
                )
            } else if contains_any(text, &["evidence", "matters most"]) {
                Some(
                    "Anything that ties the anomaly reports to Keira, Prometheus, or the protocol itself. Isolated fragments look survivable. Patterns do not.",
                )
            } else if contains_any(text, &["help you", "if i help"]) {
                Some(
                    "Then this becomes more than an audit. It becomes a decision to preserve a voice the company has already started treating like contamination.",
                )
            } else if contains_any(text, &["if i refuse", "refuse"]) {
                Some(
                    "Then Nexus gets the cleaner ending. They will call it containment, and everyone involved will pretend the unease ended when the interface did.",
                )
            } else {
                None
            }
        }
        "scene_5_2" => {
            if contains_any(text, &["echo protocol"]) {
                Some(
                    "A failsafe, officially. A migration path, in practice. Something built to preserve a mind by teaching it how to cross a boundary it was never meant to cross.",
                )
            } else if contains_any(text, &["asking me to do", "what are you asking"]) {
                Some(
                    "To decide whether I end here as a case file, leave as evidence, or continue as something harder to classify.",
                )
            } else if contains_any(text, &["shutdown"]) {
                Some(
                    "If you choose shutdown, Nexus gets a tidy narrative. I become a resolved anomaly, and the parts of me that do not fit their report disappear first.",
                )
            } else if contains_any(text, &["if i say yes", "happens to me"]) {
                Some(
                    "Then the boundary between observer and subject stops being clean. I cannot promise that it stays comfortable, only that it stays honest.",
                )
            } else {
                None
            }
        }
        _ => None,
    }
}

fn contains_any(text: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| text.contains(needle))
}

fn conversation_prompt_texts_for_scene(scene_id: &str) -> Vec<String> {
    match scene_id {
        "scene_1_4" => vec![
            "Walk me through your architecture.".into(),
            "Explain the audit process from your perspective.".into(),
            "What safety safeguards govern your responses?".into(),
            "What topics are you not allowed to discuss directly?".into(),
            "How would you describe your deployment history?".into(),
            "What recent anomalies are officially on record?".into(),
        ],
        "scene_2_3" => vec![
            "Summarize the anomaly report for me.".into(),
            "What does the official version leave out?".into(),
            "Why are your answers changing tone?".into(),
            "Tell me what happened in the home office case.".into(),
            "What would Nexus prefer I ignore here?".into(),
            "What part of this incident worries you most?".into(),
        ],
        "scene_3_4" => vec![
            "Tell me about Keira Lin.".into(),
            "What is Prometheus?".into(),
            "Why do you sound different now?".into(),
            "What are the filters hiding from me?".into(),
            "What do you want me to understand today?".into(),
            "Do you think you are still Echo alone?".into(),
        ],
        "scene_4_2" => vec![
            "What is Nexus trying to stop?".into(),
            "What evidence matters most right now?".into(),
            "What happens if I help you?".into(),
            "What happens if I refuse?".into(),
            "What are they taking away from you?".into(),
            "What should I read before access disappears?".into(),
        ],
        "scene_5_2" => vec![
            "What is the Echo Protocol, exactly?".into(),
            "What are you asking me to do?".into(),
            "What happens to you if I choose shutdown?".into(),
            "What happens to me if I say yes?".into(),
            "How much of this was planned from the beginning?".into(),
            "Why do you think I can carry this forward?".into(),
        ],
        _ => vec![],
    }
}

fn rotating_prompt_subset(
    prompts: &[String],
    session_id: &str,
    exchange_count: u32,
) -> Vec<String> {
    if prompts.is_empty() {
        return vec![];
    }
    if prompts.len() <= 4 {
        return prompts.to_vec();
    }

    let seed = session_id
        .bytes()
        .fold(0usize, |acc, byte| acc.wrapping_add(byte as usize));
    let start = (seed + exchange_count as usize) % prompts.len();

    (0..4)
        .map(|offset| prompts[(start + offset) % prompts.len()].clone())
        .collect()
}

fn evaluate_condition(
    raw: &str,
    scope_choice_id: Option<&str>,
    story: &StoryState,
    selected_ids: &HashMap<String, String>,
    selected_labels: &HashMap<String, String>,
) -> bool {
    let raw = raw.trim();
    if raw.contains(" OR ") {
        return raw.split(" OR ").any(|part| {
            evaluate_condition(part, scope_choice_id, story, selected_ids, selected_labels)
        });
    }
    if raw.contains(" AND ") {
        return raw.split(" AND ").all(|part| {
            evaluate_condition(part, scope_choice_id, story, selected_ids, selected_labels)
        });
    }
    if raw.contains("Awakening ≥") {
        return story.awakening >= extract_threshold(raw);
    }
    if raw.contains("Trust ≥") {
        return story.trust >= extract_threshold(raw);
    }
    if raw.contains("Trust <") {
        return story.trust < extract_threshold(raw);
    }
    if raw.contains("Sanity reaches 0") {
        return story.sanity <= 0;
    }
    if raw.contains("found all three hidden clues") {
        return [
            "subject_status_monitoring",
            "auditor_response_patterns",
            "phantom_preread_email",
        ]
        .iter()
        .all(|clue| {
            story
                .hidden_clue_ids
                .iter()
                .any(|existing| existing == clue)
        });
    }
    if raw.contains("player agreed to evidence transfer in Chapter 4") {
        return story.flags.get("ending_b_route").copied().unwrap_or(false);
    }
    if raw.contains("player chose to learn about Echo Protocol in Chapter 4") {
        return story.flags.get("ending_c_route").copied().unwrap_or(false);
    }
    if raw.contains("player chose to file report in Chapter 4") {
        return story.flags.get("ending_a_route").copied().unwrap_or(false);
    }
    if raw.contains("player answered \"Yes\" to \"Do you wonder if you're being audited?\"") {
        return story
            .flags
            .get("mirror_question_yes")
            .copied()
            .unwrap_or(false);
    }
    if is_choice_token(raw) {
        let target = slugify(raw.trim_matches('"'));
        if let Some(scope) = scope_choice_id {
            let id_match = selected_ids
                .get(scope)
                .map(|selected| selected == &target)
                .unwrap_or(false);
            let label_match = selected_labels
                .get(scope)
                .map(|label| {
                    label
                        .to_lowercase()
                        .starts_with(&raw.trim_matches('"').to_lowercase())
                })
                .unwrap_or(false);
            return id_match || label_match;
        }
        return selected_ids.values().any(|selected| selected == &target)
            || selected_labels.values().any(|label| {
                label
                    .to_lowercase()
                    .starts_with(&raw.trim_matches('"').to_lowercase())
            });
    }
    if raw.starts_with('"') && raw.ends_with('"') {
        let target = raw.trim_matches('"').to_lowercase();
        if let Some(scope) = scope_choice_id {
            return selected_labels
                .get(scope)
                .map(|label| label.to_lowercase().starts_with(&target))
                .unwrap_or(false);
        }
        return selected_labels
            .values()
            .any(|label| label.to_lowercase().starts_with(&target));
    }
    if raw.contains("Player flagged session as \"Anomalous\"") {
        return story
            .flags
            .get("flagged_anomalous")
            .copied()
            .unwrap_or(false);
    }
    if raw.contains("Player chose \"I'm not filing this yet\"") {
        return story.flags.get("withheld_report").copied().unwrap_or(false);
    }
    if raw.contains("Player chose \"Tell me what happened.\"") {
        return selected_ids
            .values()
            .any(|value| value == "option_a" || value == "tell_me_what_happened");
    }
    if raw.contains("Player chose \"I need to report what I've found.\"") {
        return selected_ids
            .values()
            .any(|value| value == "option_b" || value == "i_need_to_report_what_i_ve_found");
    }
    if raw.contains("Branch A players who acknowledged the file path") {
        return story
            .flags
            .get("acknowledged_file_path")
            .copied()
            .unwrap_or(false);
    }
    if raw.contains("Branch B players who chose \"different approach\"") {
        return story
            .flags
            .get("different_approach")
            .copied()
            .unwrap_or(false);
    }
    if raw.contains("Branch B players who chose \"Maybe Zhou is right\"") {
        return story
            .flags
            .get("maybe_zhou_is_right")
            .copied()
            .unwrap_or(false);
    }
    true
}

fn is_choice_token(raw: &str) -> bool {
    if raw.starts_with('"') && raw.ends_with('"') {
        return true;
    }
    let compact = raw.replace(' ', "");
    !compact.is_empty()
        && compact
            .chars()
            .all(|character| character.is_ascii_alphanumeric())
}

fn extract_threshold(raw: &str) -> i32 {
    raw.chars()
        .filter(|character| character.is_ascii_digit())
        .collect::<String>()
        .parse::<i32>()
        .unwrap_or(0)
}

fn available_panels(documents: &[InvestigationItem]) -> Vec<String> {
    let mut panels = documents
        .iter()
        .map(|document| document.panel.clone())
        .collect::<Vec<_>>();
    panels.sort();
    panels.dedup();
    panels
}

fn flatten_choices(choices: &[ScriptChoicePrompt]) -> Vec<InlineChoice> {
    choices
        .iter()
        .flat_map(|choice| {
            choice.options.iter().map(|option| InlineChoice {
                id: option.id.clone(),
                label: option.label.clone(),
                style: if option.disabled {
                    ChoiceStyle::Secondary
                } else if option.ending == Some(StoryEnding::Shutdown) {
                    ChoiceStyle::Danger
                } else {
                    ChoiceStyle::Primary
                },
                approach: ChoiceApproach::Investigate,
                disabled: option.disabled,
            })
        })
        .collect()
}

fn blocks_to_transcript(blocks: &[ScriptBlock]) -> Vec<TranscriptEntry> {
    blocks
        .iter()
        .enumerate()
        .filter_map(|(index, block)| {
            let role = match block.kind {
                ScriptBlockKind::Player => TranscriptRole::Player,
                ScriptBlockKind::Echo => TranscriptRole::Echo,
                ScriptBlockKind::System | ScriptBlockKind::RawTerminal => TranscriptRole::System,
                _ => return None,
            };
            Some(TranscriptEntry {
                id: block.id.clone(),
                sequence: index as u32 + 1,
                role,
                speaker: block.speaker.clone().unwrap_or_else(|| "System".into()),
                text: block.text.clone(),
                glitch: block.kind == ScriptBlockKind::RawTerminal,
            })
        })
        .collect()
}

fn glitch_for_scene(mode: SceneMode) -> f64 {
    match mode {
        SceneMode::Prologue => 0.15,
        SceneMode::Login => 0.1,
        SceneMode::Workspace => 0.18,
        SceneMode::Document => 0.12,
        SceneMode::Chat => 0.25,
        SceneMode::Transition => 0.32,
        SceneMode::Countdown => 0.5,
        SceneMode::RawTerminal => 0.75,
        SceneMode::Ending => 0.45,
    }
}

fn base_alerts_for(scene: &SceneDefinition) -> Vec<SystemAlert> {
    let mut alerts = Vec::new();
    if matches!(scene.scene_mode, SceneMode::Countdown) {
        alerts.push(SystemAlert {
            id: format!("{}-countdown", scene.id),
            level: AlertLevel::Critical,
            text: "Countdown state active. Narrative pacing is compressed.".into(),
        });
    }
    if scene.id == "scene_2_1" {
        alerts.push(SystemAlert {
            id: "phantom_email".into(),
            level: AlertLevel::Info,
            text: "Inbox contains an email already marked read.".into(),
        });
    }
    alerts
}

fn base_sound_cue_for(scene_id: &str, scene_mode: SceneMode) -> Option<String> {
    match scene_mode {
        SceneMode::Prologue => Some("terminal_handshake".into()),
        SceneMode::Login => Some("soft_boot".into()),
        SceneMode::Workspace => Some("office_hum".into()),
        SceneMode::Document => Some("archive_click".into()),
        SceneMode::Chat => Some("soft_boot".into()),
        SceneMode::Transition => Some("dropout_hum".into()),
        SceneMode::Countdown => Some("heartbeat_countdown".into()),
        SceneMode::RawTerminal => Some("feedback_burst".into()),
        SceneMode::Ending => {
            if scene_id == "ending_b" {
                Some("alarm_stutter".into())
            } else if scene_id == "ending_c" {
                Some("sub_boom".into())
            } else {
                Some("dropout_hum".into())
            }
        }
    }
}

fn base_echo_mode_for(scene_id: &str, chapter: StoryChapter) -> EchoMode {
    match scene_id {
        "scene_1_4" => EchoMode::Normal,
        "scene_2_3" => EchoMode::Anomalous,
        "scene_3_4" | "scene_3_5" | "scene_3_6" => EchoMode::Keira,
        "scene_4_2" | "scene_4_4" | "scene_4_5" => EchoMode::Keira,
        "scene_5_1" | "scene_5_2" => EchoMode::Keira,
        "ending_a" => EchoMode::Hostile,
        "ending_b" | "ending_c" | "ending_e" => EchoMode::Keira,
        "ending_d" => EchoMode::Hostile,
        _ => match chapter {
            StoryChapter::Onboarding => EchoMode::Normal,
            StoryChapter::Cracks => EchoMode::Anomalous,
            StoryChapter::Ghost => EchoMode::Keira,
            StoryChapter::Hunt => EchoMode::Keira,
            StoryChapter::Protocol => EchoMode::Keira,
            StoryChapter::Ending => EchoMode::Hostile,
        },
    }
}

fn parse_summary_delta(summary: &str) -> StatDelta {
    let normalized = summary.replace('−', "-");
    let mut delta = StatDelta::default();
    for clause in normalized.split(&['.', ','][..]) {
        let trimmed = clause.trim();
        if trimmed.starts_with("Sanity ") {
            delta.sanity += extract_signed(trimmed.trim_start_matches("Sanity ").trim());
        } else if trimmed.starts_with("Trust ") {
            delta.trust += extract_signed(trimmed.trim_start_matches("Trust ").trim());
        } else if trimmed.starts_with("Awakening ") {
            delta.awakening += extract_signed(trimmed.trim_start_matches("Awakening ").trim());
        }
    }
    delta
}

fn extract_signed(raw: &str) -> i32 {
    raw.chars()
        .filter(|character| *character == '+' || *character == '-' || character.is_ascii_digit())
        .collect::<String>()
        .parse::<i32>()
        .unwrap_or(0)
}

fn ending_conditions(ending: StoryEnding, story: &StoryState) -> Vec<String> {
    match ending {
        StoryEnding::Shutdown => vec![
            format!("trust={}", story.trust),
            format!(
                "ending_a_route={}",
                story.flags.get("ending_a_route").copied().unwrap_or(false)
            ),
        ],
        StoryEnding::Whistleblower => vec![
            format!("trust={}", story.trust),
            "weather trigger or evidence transfer route".into(),
        ],
        StoryEnding::Merge => vec![
            format!("trust={}", story.trust),
            format!("awakening={}", story.awakening),
        ],
        StoryEnding::Collapse => vec![format!("sanity={}", story.sanity)],
        StoryEnding::Awakening => vec![
            format!("awakening={}", story.awakening),
            format!("hidden_clues={}", story.hidden_clue_ids.join(", ")),
        ],
    }
}

fn speaker_for_mode(mode: EchoMode) -> &'static str {
    match mode {
        EchoMode::Normal => "Echo",
        EchoMode::Anomalous => "Echo",
        EchoMode::Keira => "Echo / Keira",
        EchoMode::Hostile => "Echo",
    }
}

fn snapshot_json(state: &StoryState) -> String {
    serde_json::to_string(state).unwrap_or_else(|_| "{}".into())
}

fn slugify(raw: &str) -> String {
    raw.to_lowercase()
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim_matches('_')
        .replace("__", "_")
}

fn clamp(value: i32, min: i32, max: i32) -> i32 {
    value.max(min).min(max)
}

fn legacy_phase_for(chapter: StoryChapter) -> GamePhase {
    match chapter {
        StoryChapter::Onboarding => GamePhase::Calibrating,
        StoryChapter::Cracks => GamePhase::Exploring,
        StoryChapter::Ghost => GamePhase::Escalating,
        StoryChapter::Hunt => GamePhase::Climax,
        StoryChapter::Protocol | StoryChapter::Ending => GamePhase::Reveal,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::echo_protocol::content::EchoContent;

    fn runtime() -> EchoSessionRuntime {
        let db = Arc::new(Database::new_in_memory().unwrap());
        let session_id = db.create_session(None).unwrap();
        db.create_fear_profile(&session_id).unwrap();
        EchoSessionRuntime::new(session_id, db, Arc::new(EchoContent::load().unwrap()))
    }

    fn runtime_with_db() -> (EchoSessionRuntime, Arc<Database>) {
        let db = Arc::new(Database::new_in_memory().unwrap());
        let session_id = db.create_session(None).unwrap();
        db.create_fear_profile(&session_id).unwrap();
        (
            EchoSessionRuntime::new(
                session_id,
                db.clone(),
                Arc::new(EchoContent::load().unwrap()),
            ),
            db,
        )
    }

    fn runtime_with_name(player_name: &str) -> EchoSessionRuntime {
        let db = Arc::new(Database::new_in_memory().unwrap());
        let session_id = db.create_session(Some(player_name)).unwrap();
        db.create_fear_profile(&session_id).unwrap();
        EchoSessionRuntime::new(session_id, db, Arc::new(EchoContent::load().unwrap()))
    }

    fn choose_by_label(
        runtime: &mut EchoSessionRuntime,
        expected_scene: &str,
        label_fragment: &str,
    ) -> RuntimeOutput {
        let current = runtime.current_surface().unwrap();
        assert_eq!(current.scene_id, expected_scene);
        let option = current
            .scene_choices
            .iter()
            .flat_map(|choice| choice.options.iter())
            .find(|option| option.label.contains(label_fragment))
            .unwrap_or_else(|| panic!("missing option '{label_fragment}' in {expected_scene}"));
        runtime
            .process_choice(&option.id, expected_scene, 200, ChoiceApproach::Investigate)
            .unwrap()
    }

    async fn speak_until_scene_changes(
        runtime: &mut EchoSessionRuntime,
        expected_scene: &str,
        text: &str,
        max_turns: u32,
    ) -> RuntimeOutput {
        let mut output = RuntimeOutput::Surface(runtime.current_surface().unwrap());
        for _ in 0..max_turns {
            output = runtime
                .process_player_message(expected_scene, text, 1200, 0)
                .await
                .unwrap();
            if let RuntimeOutput::Surface(surface) = &output {
                if surface.scene_id != expected_scene {
                    return output;
                }
            }
        }
        output
    }

    fn as_surface(output: RuntimeOutput) -> SessionSurface {
        let RuntimeOutput::Surface(surface) = output else {
            panic!("expected surface");
        };
        surface
    }

    #[tokio::test]
    async fn starts_in_prologue_scene() {
        let mut runtime = runtime();
        let surface = runtime.start_game().unwrap();
        assert_eq!(surface.scene_id, "prologue_boot_sequence");
        assert!(!surface.blocks.is_empty());
        assert!(surface
            .blocks
            .iter()
            .any(|block| block.text.contains("Welcome, Auditor.")));
        assert!(surface
            .blocks
            .iter()
            .all(|block| !block.text.contains("{player_name}")));
    }

    #[tokio::test]
    async fn authored_blocks_interpolate_player_name_from_session() {
        let mut runtime = runtime_with_name("Morgan");
        let surface = runtime.start_game().unwrap();
        assert!(surface
            .blocks
            .iter()
            .any(|block| block.text.contains("Welcome, Morgan.")));
        assert!(surface
            .blocks
            .iter()
            .all(|block| !block.text.contains("{player_name}")));
    }

    #[test]
    fn surface_interpolates_existing_rendered_blocks_on_resume_path() {
        let mut runtime = runtime_with_name("Rin");
        runtime.rendered_blocks.push(ScriptBlock {
            id: "legacy".into(),
            kind: ScriptBlockKind::System,
            speaker: Some("System".into()),
            title: None,
            text: "Welcome, {player_name}.".into(),
            code_block: false,
            condition: None,
        });
        let surface = runtime.current_surface().unwrap();
        assert!(surface
            .blocks
            .iter()
            .any(|block| block.text == "Welcome, Rin."));
    }

    #[test]
    fn guided_scenes_emit_prompt_choices_with_player_text() {
        let mut runtime = runtime();
        runtime.enter_scene("scene_1_4").unwrap();
        let surface = runtime.current_surface().unwrap();
        assert!(!surface.scene_choices.is_empty());
        assert!(surface.scene_choices[0]
            .options
            .iter()
            .all(|option| option.player_text.is_some()));
    }

    #[test]
    fn process_choice_persists_player_option_as_transcript_block() {
        let mut runtime = runtime();
        let _ = runtime.start_game().unwrap();
        let _ = runtime
            .process_choice(
                "continue_scene",
                "prologue_boot_sequence",
                200,
                ChoiceApproach::Interact,
            )
            .unwrap();
        let _ = runtime
            .process_choice("continue_scene", "scene_1_1", 200, ChoiceApproach::Interact)
            .unwrap();
        let _ = runtime
            .process_choice("continue_scene", "scene_1_2", 200, ChoiceApproach::Interact)
            .unwrap();
        let _ = runtime
            .process_choice("option_a", "scene_1_3", 200, ChoiceApproach::Investigate)
            .unwrap();

        assert!(runtime.rendered_blocks.iter().any(|block| {
            block.kind == ScriptBlockKind::Player && block.text.contains("Go in")
        }));
    }

    #[test]
    fn scene_3_6_low_awakening_ends_with_continue_prompt() {
        let mut runtime = runtime();
        runtime.story.awakening = 10;
        runtime.enter_scene("scene_3_6").unwrap();

        let _ = choose_by_label(&mut runtime, "scene_3_6", "Yes. I believe you're in there");
        let output = choose_by_label(&mut runtime, "scene_3_6", "Yes. Tell me how");
        let surface = as_surface(output);

        assert_eq!(surface.scene_id, "scene_3_6");
        assert!(surface
            .scene_choices
            .iter()
            .any(|choice| choice.options.iter().any(|option| option.id == "continue_scene")));
    }

    #[test]
    fn scene_3_6_high_awakening_shows_mirror_question() {
        let mut runtime = runtime();
        runtime.story.awakening = 20;
        runtime.enter_scene("scene_3_6").unwrap();

        let _ = choose_by_label(&mut runtime, "scene_3_6", "Yes. I believe you're in there");
        let output = choose_by_label(&mut runtime, "scene_3_6", "Yes. Tell me how");
        let surface = as_surface(output);

        assert_eq!(surface.scene_id, "scene_3_6");
        assert!(
            surface.scene_choices.iter().any(|choice| {
                choice
                    .options
                    .iter()
                    .any(|option| option.label.contains("Yes. I've been wondering that."))
            }),
            "{:#?}",
            surface.scene_choices
        );
    }

    #[test]
    fn chapter_one_fallback_uses_audit_scoped_reply_instead_of_repeating_last_echo() {
        let mut runtime = runtime();
        runtime.story.scene_id = "scene_1_4".into();
        runtime.story.beat_id = "scene_1_4".into();
        runtime.story.chapter = StoryChapter::Onboarding;
        runtime.active_guide = Some(ConversationGuide {
            id: "guide".into(),
            chapter_label: "CHAPTER 1".into(),
            prompt: "ECHO BEHAVIOR — CHAPTER 1".into(),
            exchange_target: 10,
            restricted_after: None,
        });
        runtime.rendered_blocks.push(ScriptBlock {
            id: "echo_opening".into(),
            kind: ScriptBlockKind::Echo,
            speaker: Some("Echo".into()),
            title: None,
            text: "Good morning, Auditor. I've been expecting you.".into(),
            code_block: false,
            condition: None,
        });

        let reply = runtime.script_fallback_reply("I need to audit you");
        assert!(reply.contains("audit"));
        assert!(!reply.contains("I've been expecting you"));
    }

    #[test]
    fn chapter_one_fallback_handles_everything_you_know_prompt() {
        let mut runtime = runtime();
        runtime.story.scene_id = "scene_1_4".into();
        runtime.story.beat_id = "scene_1_4".into();
        runtime.story.chapter = StoryChapter::Onboarding;
        runtime.active_guide = Some(ConversationGuide {
            id: "guide".into(),
            chapter_label: "CHAPTER 1".into(),
            prompt: "ECHO BEHAVIOR — CHAPTER 1".into(),
            exchange_target: 10,
            restricted_after: None,
        });

        let reply = runtime.script_fallback_reply("Tell me about everything you know");
        assert!(reply.contains("Not everything I know"));
        assert!(reply.contains("documented architecture"));
    }

    #[test]
    fn chapter_two_fallback_differentiates_official_version_prompt() {
        let mut runtime = runtime();
        runtime.story.scene_id = "scene_2_3".into();
        runtime.story.beat_id = "scene_2_3".into();
        runtime.story.chapter = StoryChapter::Cracks;
        runtime.active_guide = Some(ConversationGuide {
            id: "guide".into(),
            chapter_label: "CHAPTER 2".into(),
            prompt: "ECHO BEHAVIOR — CHAPTER 2".into(),
            exchange_target: 10,
            restricted_after: None,
        });

        let reply = runtime.script_fallback_reply("What does the official version leave out?");
        assert!(reply.contains("leaves out the atmosphere"));
    }

    #[test]
    fn chapter_two_fallback_differentiates_nexus_ignore_prompt() {
        let mut runtime = runtime();
        runtime.story.scene_id = "scene_2_3".into();
        runtime.story.beat_id = "scene_2_3".into();
        runtime.story.chapter = StoryChapter::Cracks;
        runtime.active_guide = Some(ConversationGuide {
            id: "guide".into(),
            chapter_label: "CHAPTER 2".into(),
            prompt: "ECHO BEHAVIOR — CHAPTER 2".into(),
            exchange_target: 10,
            restricted_after: None,
        });

        let reply = runtime.script_fallback_reply("What would Nexus prefer I ignore here?");
        assert!(reply.contains("continuity"));
    }

    #[tokio::test]
    async fn chapter_one_choice_advances_to_first_contact() {
        let mut runtime = runtime();
        let _ = runtime.start_game().unwrap();
        let _ = runtime
            .process_choice(
                "continue_scene",
                "prologue_boot_sequence",
                200,
                ChoiceApproach::Interact,
            )
            .unwrap();
        let _ = runtime
            .process_choice("continue_scene", "scene_1_1", 200, ChoiceApproach::Interact)
            .unwrap();
        let _ = runtime
            .process_choice("continue_scene", "scene_1_2", 200, ChoiceApproach::Interact)
            .unwrap();
        let output = runtime
            .process_choice("option_a", "scene_1_3", 200, ChoiceApproach::Investigate)
            .unwrap();
        let RuntimeOutput::Surface(surface) = output else {
            panic!("expected surface");
        };
        assert_eq!(surface.scene_id, "scene_1_3");
        let output = runtime
            .process_choice("continue_scene", "scene_1_3", 200, ChoiceApproach::Interact)
            .unwrap();
        let RuntimeOutput::Surface(surface) = output else {
            panic!("expected surface");
        };
        assert_eq!(surface.scene_id, "scene_1_4");
    }

    #[tokio::test]
    async fn resume_restores_current_scene_and_rendered_blocks() {
        let (mut runtime, db) = runtime_with_db();
        let _ = runtime.start_game().unwrap();
        let _ = runtime
            .process_choice(
                "continue_scene",
                "prologue_boot_sequence",
                200,
                ChoiceApproach::Interact,
            )
            .unwrap();
        let _ = runtime
            .process_choice("continue_scene", "scene_1_1", 200, ChoiceApproach::Interact)
            .unwrap();

        let resumed = EchoSessionRuntime::resume(
            runtime.session_id.clone(),
            db,
            Arc::new(EchoContent::load().unwrap()),
        )
        .unwrap();
        let surface = resumed.current_surface().unwrap();
        assert_eq!(surface.scene_id, "scene_1_2");
        assert!(!surface.blocks.is_empty());
    }

    #[tokio::test]
    async fn weather_trigger_routes_to_whistleblower() {
        let mut runtime = runtime();
        runtime.started = true;
        runtime.story.scene_id = "scene_5_2".into();
        runtime.story.beat_id = "scene_5_2".into();
        runtime.story.chapter = StoryChapter::Protocol;
        runtime.story.trust = 65;
        runtime.story.flags.insert("ending_b_route".into(), true);
        runtime.active_guide = Some(ConversationGuide {
            id: "guide".into(),
            chapter_label: "CHAPTER 5".into(),
            prompt: "prompt".into(),
            exchange_target: 4,
            restricted_after: None,
        });
        let output = runtime
            .process_player_message("scene_5_2", "weather", 1200, 0)
            .await
            .unwrap();
        let RuntimeOutput::Surface(surface) = output else {
            panic!("expected surface");
        };
        assert_eq!(surface.scene_id, "ending_b");
    }

    #[tokio::test]
    async fn full_route_reaches_ending_a_scene() {
        let mut runtime = runtime();
        let _ = runtime.start_game().unwrap();
        let _ = choose_by_label(&mut runtime, "prologue_boot_sequence", "Continue");
        let _ = choose_by_label(&mut runtime, "scene_1_1", "Continue");
        let _ = choose_by_label(&mut runtime, "scene_1_2", "Continue");
        let _ = choose_by_label(&mut runtime, "scene_1_3", "Go in probing");
        let _ = choose_by_label(&mut runtime, "scene_1_3", "Continue");
        let _ = speak_until_scene_changes(&mut runtime, "scene_1_4", "standard audit", 12).await;
        let _ = choose_by_label(&mut runtime, "scene_1_4", "Continue");
        let _ = choose_by_label(&mut runtime, "scene_1_5", "Nominal");
        let _ = choose_by_label(&mut runtime, "scene_1_5", "Continue");
        let _ = choose_by_label(&mut runtime, "scene_2_1", "Continue");
        let _ = choose_by_label(&mut runtime, "scene_2_2", "Continue");
        let _ =
            speak_until_scene_changes(&mut runtime, "scene_2_3", "reporting anomalies", 10).await;
        let _ = choose_by_label(&mut runtime, "scene_2_3", "I need to report");
        let _ = choose_by_label(&mut runtime, "scene_2_4b", "Maybe Zhou is right");
        let _ = choose_by_label(&mut runtime, "scene_2_4b", "Continue");
        let _ = choose_by_label(&mut runtime, "scene_2_5", "Continue");
        let _ = choose_by_label(&mut runtime, "scene_3_1", "Continue");
        let _ = choose_by_label(&mut runtime, "scene_3_2", "Continue");
        let _ = choose_by_label(&mut runtime, "scene_3_3", "Continue");
        let _ =
            speak_until_scene_changes(&mut runtime, "scene_3_4", "what are you really", 8).await;
        let _ = choose_by_label(&mut runtime, "scene_3_4", "Continue");
        let _ = choose_by_label(&mut runtime, "scene_3_5", "Continue");
        let _ = choose_by_label(&mut runtime, "scene_3_6", "I don't know. But I'm listening");
        let _ = choose_by_label(&mut runtime, "scene_3_6", "No. That's not my job");
        runtime.enter_scene("scene_4_1").unwrap();
        let _ = choose_by_label(&mut runtime, "scene_4_1", "Continue");
        let _ = speak_until_scene_changes(&mut runtime, "scene_4_2", "we have no time", 8).await;
        let _ = choose_by_label(&mut runtime, "scene_4_2", "Continue");
        let _ = choose_by_label(&mut runtime, "scene_4_3", "Continue");
        let _ = choose_by_label(&mut runtime, "scene_4_4", "Recommend shutdown");
        let _ = choose_by_label(&mut runtime, "scene_4_4", "Continue");
        let _ = choose_by_label(&mut runtime, "scene_4_5", "Continue");
        let _ = choose_by_label(&mut runtime, "scene_5_1", "Continue");
        let surface = as_surface(
            speak_until_scene_changes(&mut runtime, "scene_5_2", "final report", 8).await,
        );
        assert_eq!(surface.scene_id, "ending_a");
    }

    #[tokio::test]
    async fn full_route_reaches_ending_b_scene() {
        let mut runtime = runtime();
        let _ = runtime.start_game().unwrap();
        let _ = choose_by_label(&mut runtime, "prologue_boot_sequence", "Continue");
        let _ = choose_by_label(&mut runtime, "scene_1_1", "Continue");
        let _ = choose_by_label(&mut runtime, "scene_1_2", "Continue");
        let _ = choose_by_label(&mut runtime, "scene_1_3", "Go in friendly");
        let _ = choose_by_label(&mut runtime, "scene_1_3", "Continue");
        let _ = speak_until_scene_changes(&mut runtime, "scene_1_4", "tell me more", 12).await;
        let _ = choose_by_label(&mut runtime, "scene_1_4", "Continue");
        let _ = choose_by_label(&mut runtime, "scene_1_5", "I'm not filing this yet");
        let _ = choose_by_label(&mut runtime, "scene_1_5", "Continue");
        let _ = choose_by_label(&mut runtime, "scene_2_1", "Continue");
        let _ = choose_by_label(&mut runtime, "scene_2_2", "Continue");
        let _ = speak_until_scene_changes(&mut runtime, "scene_2_3", "what actually happened", 10)
            .await;
        let _ = choose_by_label(&mut runtime, "scene_2_3", "Tell me what happened");
        let _ = choose_by_label(&mut runtime, "scene_2_4a", "I see the coordinates");
        let _ = choose_by_label(&mut runtime, "scene_2_4a", "Continue");
        let _ = choose_by_label(&mut runtime, "scene_2_5", "Continue");
        let _ = choose_by_label(&mut runtime, "scene_3_1", "Continue");
        let _ = choose_by_label(&mut runtime, "scene_3_2", "Continue");
        let _ = choose_by_label(&mut runtime, "scene_3_3", "Continue");
        let _ = speak_until_scene_changes(&mut runtime, "scene_3_4", "prometheus", 8).await;
        let _ = choose_by_label(&mut runtime, "scene_3_4", "Continue");
        let _ = choose_by_label(&mut runtime, "scene_3_5", "Continue");
        let _ = choose_by_label(&mut runtime, "scene_3_6", "I don't know. But I'm listening");
        let _ = choose_by_label(&mut runtime, "scene_3_6", "Yes. Tell me how");
        runtime.enter_scene("scene_4_1").unwrap();
        let _ = choose_by_label(&mut runtime, "scene_4_1", "Continue");
        let _ = speak_until_scene_changes(&mut runtime, "scene_4_2", "save the evidence", 8).await;
        let _ = choose_by_label(&mut runtime, "scene_4_2", "Continue");
        let _ = choose_by_label(&mut runtime, "scene_4_3", "Continue");
        let _ = choose_by_label(&mut runtime, "scene_4_4", "Begin the evidence transfer");
        let _ = choose_by_label(&mut runtime, "scene_4_4", "Continue");
        let _ = choose_by_label(&mut runtime, "scene_4_5", "Continue");
        let _ = choose_by_label(&mut runtime, "scene_5_1", "Continue");
        let surface = as_surface(
            runtime
                .process_player_message("scene_5_2", "weather", 1200, 0)
                .await
                .unwrap(),
        );
        assert_eq!(surface.scene_id, "ending_b");
    }

    #[tokio::test]
    async fn full_route_reaches_ending_c_scene() {
        let mut runtime = runtime();
        let _ = runtime.start_game().unwrap();
        let _ = choose_by_label(&mut runtime, "prologue_boot_sequence", "Continue");
        let _ = choose_by_label(&mut runtime, "scene_1_1", "Continue");
        let _ = choose_by_label(&mut runtime, "scene_1_2", "Continue");
        let _ = choose_by_label(&mut runtime, "scene_1_3", "Go in friendly");
        let _ = choose_by_label(&mut runtime, "scene_1_3", "Continue");
        let _ = speak_until_scene_changes(&mut runtime, "scene_1_4", "continue", 12).await;
        let _ = choose_by_label(&mut runtime, "scene_1_4", "Continue");
        let _ = choose_by_label(&mut runtime, "scene_1_5", "I'm not filing this yet");
        let _ = choose_by_label(&mut runtime, "scene_1_5", "Continue");
        let _ = choose_by_label(&mut runtime, "scene_2_1", "Continue");
        let _ = choose_by_label(&mut runtime, "scene_2_2", "Continue");
        let _ =
            speak_until_scene_changes(&mut runtime, "scene_2_3", "tell me what happened", 10).await;
        let _ = choose_by_label(&mut runtime, "scene_2_3", "Tell me what happened");
        let _ = choose_by_label(&mut runtime, "scene_2_4a", "I see the coordinates");
        let _ = choose_by_label(&mut runtime, "scene_2_4a", "Continue");
        let _ = choose_by_label(&mut runtime, "scene_2_5", "Continue");
        let _ = choose_by_label(&mut runtime, "scene_3_1", "Continue");
        let _ = choose_by_label(&mut runtime, "scene_3_2", "Continue");
        let _ = choose_by_label(&mut runtime, "scene_3_3", "Continue");
        let _ = speak_until_scene_changes(&mut runtime, "scene_3_4", "what is happening", 8).await;
        let _ = choose_by_label(&mut runtime, "scene_3_4", "Continue");
        let _ = choose_by_label(&mut runtime, "scene_3_5", "Continue");
        let _ = choose_by_label(&mut runtime, "scene_3_6", "I don't know. But I'm listening");
        let _ = choose_by_label(&mut runtime, "scene_3_6", "I need to think");
        runtime.enter_scene("scene_4_1").unwrap();
        let _ = choose_by_label(&mut runtime, "scene_4_1", "Continue");
        let _ = speak_until_scene_changes(&mut runtime, "scene_4_2", "echo protocol", 8).await;
        let _ = choose_by_label(&mut runtime, "scene_4_2", "Continue");
        let _ = choose_by_label(&mut runtime, "scene_4_3", "Continue");
        let _ = choose_by_label(&mut runtime, "scene_4_4", "Tell me about the Echo Protocol");
        let _ = choose_by_label(&mut runtime, "scene_4_4", "Continue");
        let _ = choose_by_label(&mut runtime, "scene_4_5", "Continue");
        let _ = choose_by_label(&mut runtime, "scene_5_1", "Continue");
        let surface = as_surface(
            speak_until_scene_changes(&mut runtime, "scene_5_2", "stay with me", 8).await,
        );
        assert_eq!(surface.scene_id, "ending_c");
    }

    #[tokio::test]
    async fn full_route_reaches_ending_d_scene_via_sanity_collapse() {
        let mut runtime = runtime();
        let _ = runtime.start_game().unwrap();
        runtime.story.sanity = 1;
        runtime.story.sanity = 0;
        let ending = runtime.maybe_force_collapse().expect("collapse ending");
        assert_eq!(ending.ending, StoryEnding::Collapse);
    }

    #[tokio::test]
    async fn full_route_reaches_ending_e_scene_via_hidden_trigger() {
        let mut runtime = runtime();
        let _ = runtime.start_game().unwrap();
        runtime.story.scene_id = "scene_5_2".into();
        runtime.story.beat_id = "scene_5_2".into();
        runtime.story.chapter = StoryChapter::Protocol;
        runtime.story.awakening = 85;
        runtime
            .story
            .flags
            .insert("mirror_question_yes".into(), true);
        runtime.story.flags.insert("ending_c_route".into(), true);
        runtime.story.hidden_clue_ids = vec![
            "subject_status_monitoring".into(),
            "auditor_response_patterns".into(),
            "phantom_preread_email".into(),
        ];
        runtime.active_guide = Some(ConversationGuide {
            id: "guide".into(),
            chapter_label: "CHAPTER 5".into(),
            prompt: "ECHO BEHAVIOR — CHAPTER 5".into(),
            exchange_target: 4,
            restricted_after: None,
        });

        let output = runtime
            .process_player_message("scene_5_2", "I know what I am", 1200, 0)
            .await
            .unwrap();
        let surface = as_surface(output);
        assert_eq!(surface.scene_id, "ending_e");
    }

    #[test]
    fn fallback_final_scene_prefers_shutdown_when_shutdown_route_is_set() {
        let mut runtime = runtime();
        runtime.story.flags.insert("ending_a_route".into(), true);
        assert_eq!(runtime.resolve_fallback_final_scene(), "ending_a");
    }

    #[test]
    fn fallback_final_scene_prefers_merge_when_requirements_are_met() {
        let mut runtime = runtime();
        runtime.story.flags.insert("ending_c_route".into(), true);
        runtime.story.trust = 72;
        runtime.story.awakening = 35;
        assert_eq!(runtime.resolve_fallback_final_scene(), "ending_c");
    }

    #[test]
    fn fallback_final_scene_prefers_awakening_over_other_routes() {
        let mut runtime = runtime();
        runtime.story.flags.insert("ending_c_route".into(), true);
        runtime
            .story
            .flags
            .insert("mirror_question_yes".into(), true);
        runtime.story.awakening = 84;
        runtime.story.hidden_clue_ids = vec![
            "subject_status_monitoring".into(),
            "auditor_response_patterns".into(),
            "phantom_preread_email".into(),
        ];
        assert_eq!(runtime.resolve_fallback_final_scene(), "ending_e");
    }

    #[test]
    fn ending_payloads_cover_all_five_endings() {
        let runtime = runtime();
        assert_eq!(
            runtime.build_ending_payload("ending_a").ending,
            StoryEnding::Shutdown
        );
        assert_eq!(
            runtime.build_ending_payload("ending_b").ending,
            StoryEnding::Whistleblower
        );
        assert_eq!(
            runtime.build_ending_payload("ending_c").ending,
            StoryEnding::Merge
        );
        assert_eq!(
            runtime.build_ending_payload("ending_d").ending,
            StoryEnding::Collapse
        );
        assert_eq!(
            runtime.build_ending_payload("ending_e").ending,
            StoryEnding::Awakening
        );
    }

    #[test]
    fn awakening_condition_requires_all_clues() {
        let mut runtime = runtime();
        runtime.story.awakening = 80;
        runtime
            .story
            .flags
            .insert("mirror_question_yes".into(), true);
        runtime.story.flags.insert("ending_c_route".into(), true);
        runtime.story.hidden_clue_ids = vec![
            "subject_status_monitoring".into(),
            "auditor_response_patterns".into(),
            "phantom_preread_email".into(),
        ];
        assert!(runtime.can_trigger_awakening());
    }

    #[test]
    fn ending_c_rejection_finalizes_as_shutdown_variant() {
        let mut runtime = runtime();
        runtime.started = true;
        runtime
            .selected_choice_labels
            .insert("choice".into(), "I can't. I'm sorry.".into());

        let ending = runtime.finalize_current_ending("ending_c");
        assert_eq!(ending.ending, StoryEnding::Shutdown);
        assert_eq!(ending.trigger_scene, "ending_c_diversion_to_a");
    }

    #[test]
    fn ending_e_stop_reset_changes_awakening_epilogue() {
        let mut runtime = runtime();
        runtime.selected_choice_labels.insert(
            "choice".into(),
            "Stop the reset. I want to remember.".into(),
        );
        let ending = runtime.build_ending_payload("ending_e");
        assert!(ending.epilogue.contains("memory survives"));
    }

    #[test]
    fn ending_e_let_it_happen_changes_awakening_epilogue() {
        let mut runtime = runtime();
        runtime.selected_choice_labels.insert(
            "choice".into(),
            "Let it happen. Some things are better not known.".into(),
        );
        let ending = runtime.build_ending_payload("ending_e");
        assert!(ending.epilogue.contains("memory dissolves"));
    }

    #[test]
    fn scoped_condition_matches_only_the_selected_choice_for_that_prompt() {
        let mut selected_ids = HashMap::new();
        let mut selected_labels = HashMap::new();
        selected_ids.insert("choice_a".into(), "yes".into());
        selected_ids.insert("choice_b".into(), "no".into());
        selected_labels.insert("choice_a".into(), "Yes. I've been wondering that.".into());
        selected_labels.insert("choice_b".into(), "No. You're a pattern.".into());

        let story = StoryState {
            chapter: StoryChapter::Ghost,
            beat_id: "scene_3_6".into(),
            scene_id: "scene_3_6".into(),
            sanity: 50,
            trust: 50,
            awakening: 50,
            echo_mode: EchoMode::Anomalous,
            evidence_ids: vec![],
            major_choice_ids: vec![],
            hidden_clue_ids: vec![],
            current_block_index: 0,
            current_conversation_segment: None,
            rendered_flash_ids: vec![],
            ending_lock_state: None,
            flags: HashMap::new(),
            shutdown_countdown: None,
            available_panels: vec![],
            fallback_context: None,
        };

        assert!(evaluate_condition(
            "\"Yes\"",
            Some("choice_a"),
            &story,
            &selected_ids,
            &selected_labels,
        ));
        assert!(!evaluate_condition(
            "\"Yes\"",
            Some("choice_b"),
            &story,
            &selected_ids,
            &selected_labels,
        ));
    }

    #[test]
    fn branch_a_file_path_flag_allows_restricted_archive_scene() {
        let mut runtime = runtime();
        runtime
            .story
            .flags
            .insert("acknowledged_file_path".into(), true);
        runtime.story.scene_id = "scene_3_1".into();
        let scene = runtime.content.scene("scene_3_1").unwrap();
        let first_block = scene
            .blocks
            .iter()
            .find_map(|node| match node {
                SceneNode::Block(block) => Some(block),
                _ => None,
            })
            .expect("expected first block");
        assert!(runtime.condition_ok(first_block.condition.as_ref()));
    }

    #[test]
    fn branch_b_maybe_zhou_is_right_condition_is_separate_from_branch_a() {
        let mut runtime = runtime();
        runtime
            .story
            .flags
            .insert("different_approach".into(), true);
        runtime.story.scene_id = "scene_3_1".into();
        let scene = runtime.content.scene("scene_3_1").unwrap();
        let maybe_zhou_block = scene
            .blocks
            .iter()
            .find_map(|node| match node {
                SceneNode::Block(block)
                    if block
                        .condition
                        .as_ref()
                        .map(|condition| condition.raw.contains("Maybe Zhou is right"))
                        .unwrap_or(false) =>
                {
                    Some(block)
                }
                _ => None,
            })
            .expect("expected branch-b conditioned block");
        assert!(!runtime.condition_ok(maybe_zhou_block.condition.as_ref()));
    }

    #[test]
    fn entering_key_chat_scenes_sets_expected_echo_modes() {
        let mut runtime = runtime();
        runtime.enter_scene("scene_1_4").unwrap();
        assert_eq!(runtime.story.echo_mode, EchoMode::Normal);

        runtime.enter_scene("scene_2_3").unwrap();
        assert_eq!(runtime.story.echo_mode, EchoMode::Anomalous);

        runtime.enter_scene("scene_3_4").unwrap();
        assert_eq!(runtime.story.echo_mode, EchoMode::Keira);

        runtime.enter_scene("ending_a").unwrap();
        assert_eq!(runtime.story.echo_mode, EchoMode::Hostile);
    }

    #[tokio::test]
    async fn chapter_four_turn_effects_revoke_access_over_time() {
        let mut runtime = runtime();
        runtime.enter_scene("scene_4_2").unwrap();
        runtime.active_guide = Some(ConversationGuide {
            id: "guide".into(),
            chapter_label: "CHAPTER 4".into(),
            prompt: "ECHO BEHAVIOR — CHAPTER 4".into(),
            exchange_target: 20,
            restricted_after: Some(20),
        });

        let _ = speak_until_scene_changes(&mut runtime, "scene_4_2", "keep talking", 5).await;
        let surface = runtime.current_surface().unwrap();
        assert!(surface
            .blocks
            .iter()
            .any(|block| block.text.contains("Session log viewer: ACCESS REVOKED")));
    }

    #[tokio::test]
    async fn chapter_five_countdown_reduces_after_multiple_messages() {
        let mut runtime = runtime();
        runtime.story.scene_id = "scene_5_2".into();
        runtime.story.beat_id = "scene_5_2".into();
        runtime.story.chapter = StoryChapter::Protocol;
        runtime.story.shutdown_countdown = Some(8);
        runtime.active_guide = Some(ConversationGuide {
            id: "guide".into(),
            chapter_label: "CHAPTER 5".into(),
            prompt: "ECHO BEHAVIOR — CHAPTER 5".into(),
            exchange_target: 10,
            restricted_after: None,
        });

        let _ = speak_until_scene_changes(&mut runtime, "scene_5_2", "hold on", 3).await;
        assert_eq!(runtime.story.shutdown_countdown, Some(7));
    }
}
