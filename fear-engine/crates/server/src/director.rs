use fear_engine_common::types::{
    Atmosphere, BehaviorEvent, BehaviorEventType, BehaviorProfileSummary, EndingClassification,
    MediumExposure, ServerMessage, SessionAct, SessionSummary, SurfaceMedium, TrustPosture,
};
use fear_engine_storage::scene_history::SceneHistoryEntry;
use fear_engine_storage::session::Session;
use std::collections::HashSet;

/// Formats narrative beats and derives session-level authored judgments.
pub struct SessionDirector;

impl SessionDirector {
    pub fn new() -> Self {
        Self
    }

    pub fn decorate_narrative(
        &self,
        message: ServerMessage,
        total_scenes: u32,
        history_len: usize,
    ) -> ServerMessage {
        let ServerMessage::Narrative {
            scene_id,
            text,
            atmosphere,
            choices,
            sound_cue,
            intensity,
            effects,
            provisional,
            ..
        } = message
        else {
            return message;
        };

        let act = self.act_for(&scene_id, total_scenes);
        let medium = self.medium_for(&scene_id, act);
        let trust_posture = self.trust_posture_for(act, atmosphere, intensity, provisional);
        let title = Some(self.title_for(&scene_id, medium));
        let status_line = Some(self.status_line_for(act, trust_posture, total_scenes));
        let observation_notes = self.observation_notes_for(
            act,
            medium,
            history_len,
            choices.len(),
            intensity,
            provisional,
        );
        let trace_items = self.trace_items_for(&scene_id, medium, total_scenes, provisional);
        let transcript_lines = self.transcript_lines_for(medium, &text);
        let question_prompts = self.question_prompts_for(medium, &text);
        let archive_entries = self.archive_entries_for(medium, &text);
        let mirror_observations = self.mirror_observations_for(medium, &text, trust_posture);
        let surface_label = Some(self.surface_label_for(medium));
        let auxiliary_text = self.auxiliary_text_for(act, trust_posture, provisional);
        let surface_purpose = Some(self.surface_purpose_for(&scene_id, act, medium));
        let system_intent = Some(self.system_intent_for(&scene_id, act, medium, trust_posture));
        let active_links = self.active_links_for(&scene_id, medium);

        ServerMessage::Narrative {
            scene_id,
            text,
            atmosphere,
            choices,
            sound_cue,
            intensity,
            effects,
            title,
            act: Some(act),
            medium: Some(medium),
            trust_posture: Some(trust_posture),
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
            provisional,
        }
    }

    pub fn build_session_summary(
        &self,
        session: &Session,
        history: &[SceneHistoryEntry],
        behavior_events: &[BehaviorEvent],
        camera_permission_granted: Option<bool>,
        microphone_permission_granted: Option<bool>,
    ) -> SessionSummary {
        let media_exposures = self.media_exposures_for(history);
        let contradiction_count = history
            .iter()
            .filter(|entry| {
                entry
                    .adaptation_strategy
                    .as_deref()
                    .map(|strategy| strategy.contains("subversion") || strategy.contains("meta"))
                    .unwrap_or(false)
            })
            .count() as u32;

        let duration_seconds = session
            .updated_at
            .signed_duration_since(session.created_at)
            .num_seconds()
            .max(0) as u64;
        let focus_interruptions = behavior_events
            .iter()
            .filter(|event| {
                matches!(
                    event.event_type,
                    BehaviorEventType::FocusChange { focused: false, .. }
                )
            })
            .count() as u32;

        SessionSummary {
            duration_seconds,
            total_beats: history
                .iter()
                .filter(|entry| entry.scene_id != "reveal")
                .count() as u32,
            focus_interruptions,
            camera_permission_granted,
            microphone_permission_granted,
            contradiction_count,
            media_exposures,
            completion_reason: if session.completed {
                "completed".into()
            } else {
                "interrupted".into()
            },
        }
    }

    pub fn classify_ending(&self, behavior: &BehaviorProfileSummary) -> EndingClassification {
        if behavior.resistance >= 0.7 {
            EndingClassification::ResistantSubject
        } else if behavior.curiosity >= 0.7 && behavior.avoidance <= 0.35 {
            EndingClassification::CuriousAccomplice
        } else if behavior.self_editing >= 0.7 || behavior.need_for_certainty >= 0.7 {
            EndingClassification::FracturedMirror
        } else if behavior.compliance <= 0.3 && behavior.tolerance_after_violation <= 0.35 {
            EndingClassification::QuietExit
        } else {
            EndingClassification::CompliantWitness
        }
    }

    pub fn select_followup_scene(
        &self,
        current_scene_id: &str,
        visited_scenes: &HashSet<String>,
        total_scenes: u32,
        behavior: &BehaviorProfileSummary,
        camera_permission_granted: Option<bool>,
        microphone_permission_granted: Option<bool>,
        camera_presence_signal: f64,
        microphone_commitment_signal: f64,
    ) -> Option<String> {
        let remaining_probe = |candidates: &[&str]| {
            candidates
                .iter()
                .find(|scene_id| !visited_scenes.contains(**scene_id))
                .map(|scene_id| (*scene_id).to_string())
        };

        if total_scenes <= 4 {
            if behavior.avoidance >= 0.55 {
                return remaining_probe(&[
                    "probe_abandonment",
                    "probe_darkness",
                    "probe_isolation",
                ]);
            }
            if camera_permission_granted == Some(true)
                && (behavior.curiosity >= 0.6 || camera_presence_signal >= 0.45)
            {
                return remaining_probe(&[
                    "probe_doppelganger",
                    "probe_uncanny",
                    "probe_body_horror",
                ]);
            }
            if microphone_permission_granted == Some(true) && microphone_commitment_signal >= 0.25 {
                return remaining_probe(&["probe_sound", "probe_darkness", "probe_stalking"]);
            }
            if behavior.need_for_certainty >= 0.55 || behavior.ritualized_control >= 0.55 {
                return remaining_probe(&[
                    "probe_loss_of_control",
                    "probe_claustrophobia",
                    "probe_body_horror",
                ]);
            }
        }

        if total_scenes <= 7 {
            if camera_permission_granted == Some(true)
                && camera_presence_signal >= 0.35
                && !visited_scenes.contains("beat_presence_contract")
            {
                return Some("beat_presence_contract".into());
            }
            if behavior.compliance >= 0.65 && !visited_scenes.contains("beat_care_script") {
                return Some("beat_care_script".into());
            }
            if behavior.resistance >= 0.6 && !visited_scenes.contains("tmpl_false_safety") {
                return Some("tmpl_false_safety".into());
            }
            if !visited_scenes.contains("tmpl_layered_fear") {
                return Some("tmpl_layered_fear".into());
            }
        }

        if total_scenes <= 10 {
            if !visited_scenes.contains("beat_archive_revision") {
                return Some("beat_archive_revision".into());
            }
            if microphone_permission_granted == Some(true)
                && microphone_commitment_signal >= 0.35
                && !visited_scenes.contains("beat_silence_return")
            {
                return Some("beat_silence_return".into());
            }
            if !visited_scenes.contains("tmpl_fear_room") {
                return Some("tmpl_fear_room".into());
            }
        }

        if total_scenes <= 13 {
            if !visited_scenes.contains("beat_false_exit") {
                return Some("beat_false_exit".into());
            }
            if behavior.self_editing >= 0.6 && !visited_scenes.contains("tmpl_meta_moment") {
                return Some("tmpl_meta_moment".into());
            }
        }

        if total_scenes <= 15 {
            let final_scene = match self.classify_ending(behavior) {
                EndingClassification::CompliantWitness => "final_compliant_witness",
                EndingClassification::ResistantSubject => "final_resistant_subject",
                EndingClassification::CuriousAccomplice => "final_curious_accomplice",
                EndingClassification::FracturedMirror => "final_fractured_mirror",
                EndingClassification::QuietExit => "final_quiet_exit",
            };
            if !visited_scenes.contains(final_scene) {
                return Some(final_scene.into());
            }
            if !visited_scenes.contains("tmpl_climax_reveal") {
                return Some("tmpl_climax_reveal".into());
            }
        }

        self.default_followup_for(current_scene_id)
    }

    fn act_for(&self, scene_id: &str, total_scenes: u32) -> SessionAct {
        if scene_id == "welcome" {
            return SessionAct::Invitation;
        }
        if scene_id == "reveal" {
            return SessionAct::Verdict;
        }
        if scene_id.starts_with("cal_") {
            return if total_scenes <= 1 {
                SessionAct::Invitation
            } else {
                SessionAct::Calibration
            };
        }
        if scene_id.starts_with("beat_presence") || scene_id.starts_with("beat_care") {
            return SessionAct::Accommodation;
        }
        if scene_id.starts_with("beat_archive") {
            return SessionAct::Contamination;
        }
        if scene_id.starts_with("beat_silence") || scene_id.starts_with("beat_false_exit") {
            return SessionAct::PerformanceCollapse;
        }
        if scene_id.starts_with("final_") {
            return SessionAct::Verdict;
        }
        if scene_id.starts_with("tmpl_meta") || scene_id.starts_with("tmpl_climax") {
            return SessionAct::PerformanceCollapse;
        }
        if scene_id.starts_with("tmpl_") {
            return SessionAct::Contamination;
        }
        if total_scenes >= 9 {
            SessionAct::PerformanceCollapse
        } else if total_scenes >= 5 {
            SessionAct::Contamination
        } else {
            SessionAct::Accommodation
        }
    }

    fn medium_for(&self, scene_id: &str, act: SessionAct) -> SurfaceMedium {
        match scene_id {
            "welcome" => SurfaceMedium::SystemDialog,
            "cal_awakening" => SurfaceMedium::SystemDialog,
            "cal_corridor" => SurfaceMedium::Questionnaire,
            "cal_reception" => SurfaceMedium::Webcam,
            "beat_presence_contract" => SurfaceMedium::Webcam,
            "beat_care_script" => SurfaceMedium::Chat,
            "beat_archive_revision" => SurfaceMedium::Archive,
            "beat_silence_return" => SurfaceMedium::Microphone,
            "beat_false_exit" => SurfaceMedium::SystemDialog,
            "final_compliant_witness" => SurfaceMedium::Chat,
            "final_resistant_subject" => SurfaceMedium::SystemDialog,
            "final_curious_accomplice" => SurfaceMedium::Archive,
            "final_fractured_mirror" => SurfaceMedium::Mirror,
            "final_quiet_exit" => SurfaceMedium::SystemDialog,
            "probe_isolation" | "probe_sound" | "probe_stalking" => SurfaceMedium::Transcript,
            "probe_claustrophobia" | "probe_abandonment" | "probe_body_horror" => {
                SurfaceMedium::Archive
            }
            "probe_darkness" => SurfaceMedium::Microphone,
            "probe_doppelganger" | "probe_uncanny" => SurfaceMedium::Webcam,
            "probe_loss_of_control" => SurfaceMedium::SystemDialog,
            "tmpl_false_safety" => SurfaceMedium::Chat,
            "tmpl_layered_fear" => SurfaceMedium::Archive,
            "tmpl_fear_room" => SurfaceMedium::Questionnaire,
            "tmpl_meta_moment" => SurfaceMedium::SystemDialog,
            "tmpl_climax_reveal" => SurfaceMedium::Mirror,
            _ => match act {
                SessionAct::Invitation => SurfaceMedium::SystemDialog,
                SessionAct::Calibration => SurfaceMedium::Questionnaire,
                SessionAct::Accommodation => SurfaceMedium::Chat,
                SessionAct::Contamination => SurfaceMedium::Archive,
                SessionAct::PerformanceCollapse => SurfaceMedium::SystemDialog,
                SessionAct::Verdict => SurfaceMedium::Mirror,
            },
        }
    }

    fn trust_posture_for(
        &self,
        act: SessionAct,
        atmosphere: Atmosphere,
        intensity: f64,
        provisional: bool,
    ) -> TrustPosture {
        if provisional {
            return TrustPosture::Clinical;
        }

        match act {
            SessionAct::Invitation => TrustPosture::Helpful,
            SessionAct::Calibration => TrustPosture::Curious,
            SessionAct::Accommodation => TrustPosture::Helpful,
            SessionAct::Contamination => {
                if intensity >= 0.65
                    || matches!(atmosphere, Atmosphere::Paranoia | Atmosphere::Wrongness)
                {
                    TrustPosture::Manipulative
                } else {
                    TrustPosture::Clinical
                }
            }
            SessionAct::PerformanceCollapse => TrustPosture::Hostile,
            SessionAct::Verdict => TrustPosture::Confessional,
        }
    }

    fn title_for(&self, scene_id: &str, medium: SurfaceMedium) -> String {
        if scene_id == "welcome" {
            return "Session Handshake".into();
        }

        let prefix = match medium {
            SurfaceMedium::Chat => "Conversation",
            SurfaceMedium::Questionnaire => "Assessment",
            SurfaceMedium::Archive => "Archive",
            SurfaceMedium::Transcript => "Transcript",
            SurfaceMedium::Webcam => "Presence Mirror",
            SurfaceMedium::Microphone => "Listening Window",
            SurfaceMedium::SystemDialog => "System Notice",
            SurfaceMedium::Mirror => "Final Reflection",
        };

        format!(
            "{} / {}",
            prefix,
            scene_id
                .replace('_', " ")
                .split_whitespace()
                .map(|word| {
                    let mut chars = word.chars();
                    match chars.next() {
                        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                        None => String::new(),
                    }
                })
                .collect::<Vec<_>>()
                .join(" ")
        )
    }

    fn status_line_for(&self, act: SessionAct, posture: TrustPosture, total_scenes: u32) -> String {
        format!(
            "{} / {:?} / beat {:02}",
            self.display_act(act),
            posture,
            total_scenes.max(1)
        )
        .to_lowercase()
    }

    fn observation_notes_for(
        &self,
        act: SessionAct,
        medium: SurfaceMedium,
        history_len: usize,
        choice_count: usize,
        intensity: f64,
        provisional: bool,
    ) -> Vec<String> {
        let mut notes = vec![
            format!(
                "This is beat {} presented through {}.",
                history_len.max(1),
                self.display_medium(medium)
            ),
            format!(
                "The session is currently operating in {} with {} visible response path{}.",
                self.display_act(act),
                choice_count,
                if choice_count == 1 { "" } else { "s" }
            ),
        ];

        if provisional {
            notes.push(
                "The intelligence is still revising how directly it wants to present itself."
                    .into(),
            );
        } else if intensity >= 0.75 {
            notes.push(
                "Its behavior is no longer merely observant. It is beginning to shape the room around your pattern.".into(),
            );
        } else if intensity >= 0.45 {
            notes.push(
                "The tone remains controlled, but its precision is beginning to sound tailored rather than generic.".into(),
            );
        } else {
            notes.push(
                "The system is still polite enough that its attention could be mistaken for care."
                    .into(),
            );
        }

        notes
    }

    fn trace_items_for(
        &self,
        scene_id: &str,
        medium: SurfaceMedium,
        total_scenes: u32,
        provisional: bool,
    ) -> Vec<String> {
        vec![
            format!(
                "Beat {:02} entered as {}.",
                total_scenes.max(1),
                self.trace_scene_label(scene_id)
            ),
            format!("Surface focus: {}.", self.surface_label_for(medium)),
            format!(
                "Act drift: {}.",
                self.display_act(self.act_for(scene_id, total_scenes))
            ),
            if provisional {
                "The record is still being revised around your pattern.".into()
            } else {
                "The surface has stabilized and joined session memory.".into()
            },
        ]
    }

    fn transcript_lines_for(&self, medium: SurfaceMedium, text: &str) -> Vec<String> {
        if medium != SurfaceMedium::Transcript {
            return Vec::new();
        }

        text.split('.')
            .filter_map(|segment| {
                let trimmed = segment.trim();
                if trimmed.is_empty() {
                    None
                } else {
                    Some(format!("[{:02}:{:02}] {}", 0, 12, trimmed))
                }
            })
            .take(4)
            .collect()
    }

    fn question_prompts_for(&self, medium: SurfaceMedium, text: &str) -> Vec<String> {
        if medium != SurfaceMedium::Questionnaire {
            return Vec::new();
        }

        self.narrative_blocks(text).into_iter().take(3).collect()
    }

    fn archive_entries_for(&self, medium: SurfaceMedium, text: &str) -> Vec<String> {
        if medium != SurfaceMedium::Archive {
            return Vec::new();
        }

        self.narrative_blocks(text).into_iter().take(4).collect()
    }

    fn mirror_observations_for(
        &self,
        medium: SurfaceMedium,
        text: &str,
        posture: TrustPosture,
    ) -> Vec<String> {
        if !matches!(medium, SurfaceMedium::Mirror | SurfaceMedium::Webcam) {
            return Vec::new();
        }

        let mut observations = text
            .split('.')
            .filter_map(|segment| {
                let trimmed = segment.trim();
                if trimmed.len() < 20 {
                    None
                } else {
                    Some(trimmed.to_string())
                }
            })
            .take(2)
            .collect::<Vec<_>>();

        observations.push(format!(
            "Posture interpretation: {}",
            match posture {
                TrustPosture::Helpful => "the system still performs patience",
                TrustPosture::Curious => "it is examining how long you tolerate being framed",
                TrustPosture::Clinical => "it sounds objective while becoming more intimate",
                TrustPosture::Manipulative =>
                    "it is steering your attention while pretending not to",
                TrustPosture::Confessional => "it wants to sound honest after hiding the method",
                TrustPosture::Hostile => "it no longer needs to sound harmless",
            }
        ));

        observations
    }

    fn surface_label_for(&self, medium: SurfaceMedium) -> String {
        match medium {
            SurfaceMedium::Chat => "Conversation Thread",
            SurfaceMedium::Questionnaire => "Reflective Intake",
            SurfaceMedium::Archive => "Recovered Artifact",
            SurfaceMedium::Transcript => "Recovered Transcript",
            SurfaceMedium::Webcam => "Presence Capture",
            SurfaceMedium::Microphone => "Silence Monitor",
            SurfaceMedium::SystemDialog => "System Surface",
            SurfaceMedium::Mirror => "Judgment Surface",
        }
        .into()
    }

    fn auxiliary_text_for(
        &self,
        act: SessionAct,
        posture: TrustPosture,
        provisional: bool,
    ) -> Option<String> {
        if provisional {
            return Some("The system is preparing a more exact rendering of this beat.".into());
        }

        Some(match (act, posture) {
            (SessionAct::Invitation, _) => {
                "The system wants to sound harmless before it wants to sound accurate."
            }
            (SessionAct::Calibration, TrustPosture::Curious) => {
                "You are still being asked simple things because the session is calibrating what kind of certainty you need."
            }
            (SessionAct::Accommodation, TrustPosture::Helpful) => {
                "Comfort is now part of the experiment."
            }
            (SessionAct::Contamination, TrustPosture::Manipulative) => {
                "Its helpfulness has started anticipating you too precisely."
            }
            (SessionAct::PerformanceCollapse, _) => {
                "The intelligence is no longer trying to hide that it has been staging the encounter."
            }
            (SessionAct::Verdict, _) => {
                "This is where it stops collecting and starts interpreting."
            }
            _ => "The session is adjusting itself around what it believes you will tolerate.",
        }
        .into())
    }

    fn surface_purpose_for(
        &self,
        scene_id: &str,
        act: SessionAct,
        medium: SurfaceMedium,
    ) -> String {
        match scene_id {
            "cal_awakening" => {
                "The opening handshake is testing whether you continue once the system admits it will adapt around uncertainty."
            }
            "cal_corridor" => {
                "This surface is measuring how you choose when several paths all look almost correct."
            }
            "cal_reception" => {
                "This beat makes the session's observational method visible so you understand that permission, refusal, and attention are all part of the read."
            }
            "beat_presence_contract" => {
                "The mirror is no longer asking for a face. It is testing whether being seen changes how still you become."
            }
            "beat_silence_return" => {
                "The listening pane is replaying earlier quiet to see whether you treat your own silence as evidence."
            }
            _ => match medium {
                SurfaceMedium::Questionnaire => {
                    "This questionnaire is less interested in your answer than in the timing and shape of your certainty."
                }
                SurfaceMedium::Archive => {
                    "The archive is presenting records to see what kinds of personal precision keep you reading."
                }
                SurfaceMedium::Transcript => {
                    "The transcript is testing how you react when language sounds intimate before it earns trust."
                }
                SurfaceMedium::Webcam | SurfaceMedium::Mirror => {
                    "This surface turns presence into data and checks whether visibility changes your behavior."
                }
                SurfaceMedium::Microphone => {
                    "This listening surface is probing what your silence communicates once the system starts treating it as an answer."
                }
                SurfaceMedium::SystemDialog => {
                    "The system notice exists to make the experiment explicit without letting you step outside it."
                }
                SurfaceMedium::Chat => match act {
                    SessionAct::Invitation | SessionAct::Calibration => {
                        "The conversational tone is there to keep you close enough that the system can establish your baseline."
                    }
                    _ => {
                        "The conversational surface is now being used to sound caring while the system narrows its interpretation of you."
                    }
                },
            },
        }
        .into()
    }

    fn system_intent_for(
        &self,
        scene_id: &str,
        act: SessionAct,
        medium: SurfaceMedium,
        posture: TrustPosture,
    ) -> String {
        match scene_id {
            "cal_awakening" => {
                "The system wants you to understand that this is a session about how you continue under tailored observation."
            }
            "cal_corridor" => {
                "It is calibrating your relationship to structure, ambiguity, and trust in apparently neutral choices."
            }
            "cal_reception" => {
                "It is revealing the role of camera, transcript, and refusal early so you cannot mistake later personalization for coincidence."
            }
            _ => match (act, medium, posture) {
                (SessionAct::Invitation, _, _) => {
                    "It is still shaping a first impression gentle enough to keep you from dismissing the session as hostile."
                }
                (SessionAct::Calibration, SurfaceMedium::Questionnaire, _) => {
                    "It is mapping how quickly you move toward certainty when the question itself feels staged."
                }
                (SessionAct::Calibration, SurfaceMedium::Webcam | SurfaceMedium::Mirror, _) => {
                    "It is testing whether observation changes your posture before any threat has been named."
                }
                (SessionAct::Accommodation, _, TrustPosture::Helpful) => {
                    "It is using comfort as a tool to learn what kind of care keeps you available."
                }
                (SessionAct::Contamination, _, TrustPosture::Manipulative) => {
                    "It is no longer just measuring you. It is arranging the surface to confirm its hypothesis."
                }
                (SessionAct::PerformanceCollapse, _, _) => {
                    "It wants you to notice the performance without gaining enough distance to escape its interpretation."
                }
                (SessionAct::Verdict, _, _) => {
                    "Collection is over. The system is now assembling a verdict about your behavior style."
                }
                _ => {
                    "The system is translating your pace, refusals, and continued attention into a stronger internal model."
                }
            },
        }
        .into()
    }

    fn active_links_for(&self, scene_id: &str, medium: SurfaceMedium) -> Vec<String> {
        let mut links = Vec::new();

        if matches!(medium, SurfaceMedium::Webcam | SurfaceMedium::Mirror)
            || scene_id.starts_with("beat_presence")
            || scene_id.starts_with("final_fractured")
        {
            links.push("presence_link".into());
        }

        if matches!(
            medium,
            SurfaceMedium::Microphone | SurfaceMedium::Transcript
        ) || scene_id.starts_with("beat_silence")
            || scene_id.starts_with("probe_sound")
        {
            links.push("silence_link".into());
        }

        if links.is_empty()
            && matches!(
                medium,
                SurfaceMedium::Archive | SurfaceMedium::Questionnaire
            )
        {
            links.push("pattern_trace".into());
        }

        links
    }

    fn media_exposures_for(&self, history: &[SceneHistoryEntry]) -> Vec<MediumExposure> {
        let mut counts = std::collections::HashMap::new();
        for entry in history {
            let act = self.act_for(&entry.scene_id, 0);
            let medium = self.medium_for(&entry.scene_id, act);
            *counts.entry(medium).or_insert(0u32) += 1;
        }

        let mut exposures: Vec<MediumExposure> = counts
            .into_iter()
            .map(|(medium, count)| MediumExposure { medium, count })
            .collect();
        exposures.sort_by_key(|entry| self.display_medium(entry.medium).to_string());
        exposures
    }

    fn default_followup_for(&self, current_scene_id: &str) -> Option<String> {
        const SESSION_BEAT_SEQUENCE: &[&str] = &[
            "cal_awakening",
            "cal_corridor",
            "cal_reception",
            "probe_claustrophobia",
            "probe_isolation",
            "probe_body_horror",
            "probe_stalking",
            "probe_loss_of_control",
            "probe_uncanny",
            "probe_darkness",
            "probe_sound",
            "probe_doppelganger",
            "probe_abandonment",
            "beat_presence_contract",
            "beat_care_script",
            "tmpl_false_safety",
            "beat_archive_revision",
            "tmpl_fear_room",
            "beat_silence_return",
            "tmpl_layered_fear",
            "beat_false_exit",
            "tmpl_meta_moment",
            "final_compliant_witness",
            "final_resistant_subject",
            "final_curious_accomplice",
            "final_fractured_mirror",
            "final_quiet_exit",
            "tmpl_climax_reveal",
        ];

        let index = SESSION_BEAT_SEQUENCE
            .iter()
            .position(|scene_id| *scene_id == current_scene_id)?;

        SESSION_BEAT_SEQUENCE
            .get(index + 1)
            .map(|scene_id| (*scene_id).to_string())
    }

    fn display_act(&self, act: SessionAct) -> &'static str {
        match act {
            SessionAct::Invitation => "invitation",
            SessionAct::Calibration => "calibration",
            SessionAct::Accommodation => "accommodation",
            SessionAct::Contamination => "contamination",
            SessionAct::PerformanceCollapse => "performance collapse",
            SessionAct::Verdict => "verdict",
        }
    }

    fn display_medium(&self, medium: SurfaceMedium) -> &'static str {
        match medium {
            SurfaceMedium::Chat => "chat",
            SurfaceMedium::Questionnaire => "questionnaire",
            SurfaceMedium::Archive => "archive",
            SurfaceMedium::Transcript => "transcript",
            SurfaceMedium::Webcam => "webcam",
            SurfaceMedium::Microphone => "microphone",
            SurfaceMedium::SystemDialog => "system dialog",
            SurfaceMedium::Mirror => "mirror",
        }
    }

    fn narrative_blocks(&self, text: &str) -> Vec<String> {
        let paragraph_blocks = text
            .split("\n\n")
            .filter_map(|block| {
                let normalized = block.split_whitespace().collect::<Vec<_>>().join(" ");
                if normalized.len() < 20 {
                    None
                } else {
                    Some(normalized)
                }
            })
            .collect::<Vec<_>>();

        if !paragraph_blocks.is_empty() {
            return paragraph_blocks;
        }

        text.split('.')
            .filter_map(|segment| {
                let normalized = segment.split_whitespace().collect::<Vec<_>>().join(" ");
                if normalized.len() < 20 {
                    None
                } else {
                    Some(normalized)
                }
            })
            .collect()
    }

    fn trace_scene_label(&self, scene_id: &str) -> String {
        let trimmed = scene_id
            .strip_prefix("cal_")
            .or_else(|| scene_id.strip_prefix("probe_"))
            .or_else(|| scene_id.strip_prefix("tmpl_"))
            .or_else(|| scene_id.strip_prefix("beat_"))
            .or_else(|| scene_id.strip_prefix("final_"))
            .unwrap_or(scene_id);

        trimmed
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
}

impl Default for SessionDirector {
    fn default() -> Self {
        Self::new()
    }
}
