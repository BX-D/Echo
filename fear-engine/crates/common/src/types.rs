//! Domain types shared across all Fear Engine crates.
//!
//! This module defines the canonical representations for fear categories, game phases,
//! behavior events, narrative responses, WebSocket protocol messages, and visual effect
//! directives. Every type derives [`serde::Serialize`] and [`serde::Deserialize`] so it
//! can travel across the WebSocket boundary without manual conversion.

use std::fmt;
use std::str::FromStr;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::FearEngineError;

// ---------------------------------------------------------------------------
// Fear categories — the 10 psychological axes the engine tracks
// ---------------------------------------------------------------------------

/// The ten fear axes that the Fear Engine profiles.
///
/// Each axis represents a distinct category of psychological fear. The engine
/// maintains a Bayesian score for every axis and uses the top-scoring ones to
/// personalise horror content.
///
/// # Example
///
/// ```
/// use fear_engine_common::types::FearType;
///
/// let all = FearType::all();
/// assert_eq!(all.len(), 10);
/// assert_eq!(FearType::Claustrophobia.to_string(), "claustrophobia");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FearType {
    Claustrophobia,
    Isolation,
    BodyHorror,
    Stalking,
    LossOfControl,
    UncannyValley,
    Darkness,
    SoundBased,
    Doppelganger,
    Abandonment,
}

impl FearType {
    /// Returns a [`Vec`] containing all ten fear categories.
    ///
    /// # Example
    ///
    /// ```
    /// use fear_engine_common::types::FearType;
    /// let all = FearType::all();
    /// assert_eq!(all.len(), 10);
    /// ```
    pub fn all() -> Vec<FearType> {
        vec![
            FearType::Claustrophobia,
            FearType::Isolation,
            FearType::BodyHorror,
            FearType::Stalking,
            FearType::LossOfControl,
            FearType::UncannyValley,
            FearType::Darkness,
            FearType::SoundBased,
            FearType::Doppelganger,
            FearType::Abandonment,
        ]
    }
}

impl fmt::Display for FearType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FearType::Claustrophobia => write!(f, "claustrophobia"),
            FearType::Isolation => write!(f, "isolation"),
            FearType::BodyHorror => write!(f, "body_horror"),
            FearType::Stalking => write!(f, "stalking"),
            FearType::LossOfControl => write!(f, "loss_of_control"),
            FearType::UncannyValley => write!(f, "uncanny_valley"),
            FearType::Darkness => write!(f, "darkness"),
            FearType::SoundBased => write!(f, "sound_based"),
            FearType::Doppelganger => write!(f, "doppelganger"),
            FearType::Abandonment => write!(f, "abandonment"),
        }
    }
}

impl FromStr for FearType {
    type Err = FearEngineError;

    /// Parses a [`FearType`] from its string representation (case-insensitive,
    /// underscores optional).
    ///
    /// # Example
    ///
    /// ```
    /// use fear_engine_common::types::FearType;
    /// let ft: FearType = "body_horror".parse().unwrap();
    /// assert_eq!(ft, FearType::BodyHorror);
    /// ```
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().replace('_', "").as_str() {
            "claustrophobia" => Ok(FearType::Claustrophobia),
            "isolation" => Ok(FearType::Isolation),
            "bodyhorror" => Ok(FearType::BodyHorror),
            "stalking" => Ok(FearType::Stalking),
            "lossofcontrol" => Ok(FearType::LossOfControl),
            "uncannyvalley" => Ok(FearType::UncannyValley),
            "darkness" => Ok(FearType::Darkness),
            "soundbased" => Ok(FearType::SoundBased),
            "doppelganger" => Ok(FearType::Doppelganger),
            "abandonment" => Ok(FearType::Abandonment),
            _ => Err(FearEngineError::InvalidInput {
                field: "fear_type".into(),
                reason: format!("unknown fear type: '{s}'"),
            }),
        }
    }
}

// ---------------------------------------------------------------------------
// Game phases
// ---------------------------------------------------------------------------

/// The five sequential phases of a Fear Engine game session.
///
/// Phases have a strict ordering from calibration through the final reveal.
///
/// # Example
///
/// ```
/// use fear_engine_common::types::GamePhase;
/// assert!(GamePhase::Calibrating < GamePhase::Reveal);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GamePhase {
    Calibrating,
    Exploring,
    Escalating,
    Climax,
    Reveal,
}

/// High-level authored act in the redesigned session experience.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionAct {
    Invitation,
    Calibration,
    Accommodation,
    Contamination,
    PerformanceCollapse,
    Verdict,
}

/// Presentation medium used by the interface orchestrator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SurfaceMedium {
    Chat,
    Questionnaire,
    Archive,
    Transcript,
    Webcam,
    Microphone,
    SystemDialog,
    Mirror,
}

/// How the intelligence is presenting itself during a beat.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrustPosture {
    Helpful,
    Curious,
    Clinical,
    Manipulative,
    Confessional,
    Hostile,
}

impl GamePhase {
    /// Returns the zero-based ordinal index of this phase.
    ///
    /// # Example
    ///
    /// ```
    /// use fear_engine_common::types::GamePhase;
    /// assert_eq!(GamePhase::Calibrating.ordinal(), 0);
    /// assert_eq!(GamePhase::Reveal.ordinal(), 4);
    /// ```
    pub fn ordinal(self) -> u8 {
        match self {
            GamePhase::Calibrating => 0,
            GamePhase::Exploring => 1,
            GamePhase::Escalating => 2,
            GamePhase::Climax => 3,
            GamePhase::Reveal => 4,
        }
    }
}

impl PartialOrd for GamePhase {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for GamePhase {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.ordinal().cmp(&other.ordinal())
    }
}

impl fmt::Display for GamePhase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GamePhase::Calibrating => write!(f, "calibrating"),
            GamePhase::Exploring => write!(f, "exploring"),
            GamePhase::Escalating => write!(f, "escalating"),
            GamePhase::Climax => write!(f, "climax"),
            GamePhase::Reveal => write!(f, "reveal"),
        }
    }
}

// ---------------------------------------------------------------------------
// Behavior events — raw signals from the frontend
// ---------------------------------------------------------------------------

/// A single behavior event captured by the frontend tracker.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehaviorEvent {
    pub event_type: BehaviorEventType,
    pub timestamp: DateTime<Utc>,
    pub scene_id: String,
}

/// The kind of player behavior that was observed.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BehaviorEventType {
    Keystroke {
        chars_per_second: f64,
        backspace_count: u32,
        total_chars: u32,
    },
    Pause {
        duration_ms: u64,
        scene_content_hash: String,
    },
    Choice {
        choice_id: String,
        time_to_decide_ms: u64,
        approach: ChoiceApproach,
    },
    MouseMovement {
        velocity: f64,
        tremor_score: f64,
    },
    Scroll {
        direction: ScrollDirection,
        to_position: f64,
        rereading: bool,
    },
    ChoiceHoverPattern {
        hovered_choice_ids: Vec<String>,
        dominant_choice_id: Option<String>,
        total_hover_ms: u64,
    },
    MediaEngagement {
        medium: SurfaceMedium,
        dwell_ms: u64,
        interaction_count: u32,
    },
    CameraPresence {
        visible_ms: u64,
        sustained_presence: bool,
    },
    MicSilenceResponse {
        dwell_ms: u64,
        exited_early: bool,
        returned_after_prompt: bool,
    },
    DevicePermission {
        device: String,
        granted: bool,
    },
    FocusChange {
        focused: bool,
        return_latency_ms: Option<u64>,
    },
}

/// Direction of a scroll action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScrollDirection {
    Up,
    Down,
}

// ---------------------------------------------------------------------------
// Choice approach categories
// ---------------------------------------------------------------------------

/// How the player approaches a decision — reveals fight-or-flight tendencies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChoiceApproach {
    Investigate,
    Avoid,
    Confront,
    Flee,
    Interact,
    Wait,
}

// ---------------------------------------------------------------------------
// Scene atmosphere
// ---------------------------------------------------------------------------

/// The emotional tone of a scene, used to drive audio and visual effects.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Atmosphere {
    Dread,
    Tension,
    Panic,
    Calm,
    Wrongness,
    Isolation,
    Paranoia,
}

// ---------------------------------------------------------------------------
// Adaptation strategies
// ---------------------------------------------------------------------------

/// The curve shape used when gradually escalating fear intensity.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EscalationCurve {
    Linear,
    Exponential,
    Sigmoid,
    StepFunction,
}

/// A high-level strategy the adaptation engine uses to modulate horror content.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "strategy", rename_all = "snake_case")]
pub enum AdaptationStrategy {
    /// Early game: test different fears with mild stimuli.
    Probe {
        target_fears: Vec<FearType>,
        intensity: f64,
    },
    /// Mid game: gradually increase confirmed fears.
    GradualEscalation {
        primary_fear: FearType,
        intensity_curve: EscalationCurve,
    },
    /// Build tension: calm before the storm.
    Contrast {
        calm_duration: u32,
        storm_fear: FearType,
        storm_intensity: f64,
    },
    /// Combine fears for amplification.
    Layering {
        base_fear: FearType,
        amplifier_fear: FearType,
        blend_ratio: f64,
    },
    /// Go against expectations for unpredictability.
    Subversion {
        expected_fear: FearType,
        actual_fear: FearType,
    },
}

// ---------------------------------------------------------------------------
// AI response types
// ---------------------------------------------------------------------------

/// The structured response returned by the Claude narrative generation pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NarrativeResponse {
    pub narrative: String,
    pub atmosphere: Atmosphere,
    pub sound_cue: Option<String>,
    pub image_prompt: Option<String>,
    pub choices: Vec<Choice>,
    pub hidden_elements: Vec<String>,
    pub intensity: f64,
    pub meta_break: Option<MetaBreak>,
    pub transcript_lines: Vec<String>,
    pub question_prompts: Vec<String>,
    pub archive_entries: Vec<String>,
    pub mirror_observations: Vec<String>,
}

/// A single player-facing choice within a scene.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Choice {
    pub id: String,
    pub text: String,
    pub approach: ChoiceApproach,
    pub fear_vector: FearType,
}

/// A fourth-wall-breaking moment injected by the AI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaBreak {
    pub text: String,
    pub target: MetaTarget,
}

/// Where a meta-horror break is rendered in the UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MetaTarget {
    Title,
    Overlay,
    Whisper,
    GlitchText,
}

// ---------------------------------------------------------------------------
// WebSocket message types
// ---------------------------------------------------------------------------

/// Messages sent from the client (browser) to the server.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload", rename_all = "snake_case")]
pub enum ClientMessage {
    StartGame {
        player_name: Option<String>,
    },
    Choice {
        scene_id: String,
        choice_id: String,
        time_to_decide_ms: u64,
        approach: ChoiceApproach,
    },
    BehaviorBatch {
        events: Vec<BehaviorEvent>,
        timestamp: DateTime<Utc>,
    },
    TextInput {
        scene_id: String,
        text: String,
        typing_duration_ms: u64,
        backspace_count: u32,
    },
}

/// Messages sent from the server to the client (browser).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload", rename_all = "snake_case")]
pub enum ServerMessage {
    Narrative {
        scene_id: String,
        text: String,
        atmosphere: Atmosphere,
        choices: Vec<Choice>,
        sound_cue: Option<String>,
        intensity: f64,
        effects: Vec<EffectDirective>,
        title: Option<String>,
        act: Option<SessionAct>,
        medium: Option<SurfaceMedium>,
        trust_posture: Option<TrustPosture>,
        status_line: Option<String>,
        observation_notes: Vec<String>,
        trace_items: Vec<String>,
        transcript_lines: Vec<String>,
        question_prompts: Vec<String>,
        archive_entries: Vec<String>,
        mirror_observations: Vec<String>,
        surface_label: Option<String>,
        auxiliary_text: Option<String>,
        surface_purpose: Option<String>,
        system_intent: Option<String>,
        active_links: Vec<String>,
        provisional: bool,
    },
    Image {
        scene_id: String,
        image_url: String,
        display_mode: DisplayMode,
    },
    PhaseChange {
        from: GamePhase,
        to: GamePhase,
    },
    Meta {
        text: String,
        target: MetaTarget,
        delay_ms: u64,
    },
    Reveal {
        fear_profile: FearProfileSummary,
        behavior_profile: BehaviorProfileSummary,
        session_summary: SessionSummary,
        key_moments: Vec<KeyMoment>,
        adaptation_log: Vec<AdaptationRecord>,
        ending_classification: EndingClassification,
        analysis: RevealAnalysis,
    },
    Error {
        code: String,
        message: String,
        recoverable: bool,
    },
}

/// How an AI-generated image should be displayed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DisplayMode {
    FadeIn,
    Glitch,
    Flash,
}

// ---------------------------------------------------------------------------
// Fear profile summary (used in Reveal)
// ---------------------------------------------------------------------------

/// A single fear axis score with its confidence level.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FearScore {
    pub fear_type: FearType,
    pub score: f64,
    pub confidence: f64,
}

/// The end-of-game summary of the player's fear profile.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FearProfileSummary {
    pub scores: Vec<FearScore>,
    pub primary_fear: FearType,
    pub secondary_fear: Option<FearType>,
    pub total_observations: u32,
}

/// A notable moment during gameplay that revealed a fear.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyMoment {
    pub scene_id: String,
    pub description: String,
    pub fear_revealed: FearType,
    pub behavior_trigger: String,
}

/// A record of one adaptation the engine made during the game.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdaptationRecord {
    pub scene_id: String,
    pub strategy: String,
    pub fear_targeted: FearType,
    pub intensity: f64,
}

/// Real aggregate statistics about the completed session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummary {
    pub duration_seconds: u64,
    pub total_beats: u32,
    pub focus_interruptions: u32,
    pub camera_permission_granted: Option<bool>,
    pub microphone_permission_granted: Option<bool>,
    pub contradiction_count: u32,
    pub media_exposures: Vec<MediumExposure>,
    pub completion_reason: String,
}

/// How often a presentation medium appeared during the session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediumExposure {
    pub medium: SurfaceMedium,
    pub count: u32,
}

/// Real behavioral profile derived from observed session data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehaviorProfileSummary {
    pub compliance: f64,
    pub resistance: f64,
    pub curiosity: f64,
    pub avoidance: f64,
    pub self_editing: f64,
    pub need_for_certainty: f64,
    pub ritualized_control: f64,
    pub recovery_after_escalation: f64,
    pub tolerance_after_violation: f64,
}

/// Final authored class for how the intelligence interprets the player.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EndingClassification {
    CompliantWitness,
    ResistantSubject,
    CuriousAccomplice,
    FracturedMirror,
    QuietExit,
}

/// AI-generated interpretation of the player's real session data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevealAnalysis {
    pub summary: String,
    pub key_patterns: Vec<String>,
    pub adaptation_summary: String,
    pub closing_message: String,
}

// ---------------------------------------------------------------------------
// Effect directives
// ---------------------------------------------------------------------------

/// An instruction for the frontend to play a visual/audio effect.
///
/// # Example
///
/// ```
/// use fear_engine_common::types::{EffectDirective, EffectType};
///
/// let shake = EffectDirective {
///     effect: EffectType::Shake,
///     intensity: 0.7,
///     duration_ms: 500,
///     delay_ms: 0,
/// };
/// assert_eq!(shake.intensity, 0.7);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EffectDirective {
    pub effect: EffectType,
    pub intensity: f64,
    pub duration_ms: u64,
    pub delay_ms: u64,
}

/// The kind of frontend effect to trigger.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EffectType {
    Shake,
    Flicker,
    Glitch,
    Darkness,
    Flashlight,
    Crt,
    SlowType,
    FastType,
    StrobeFlash,
    ChromaticShift,
    FocusPulse,
    FrameJump,
}

// ---------------------------------------------------------------------------
// Session ID helper
// ---------------------------------------------------------------------------

/// Generates a new random session identifier.
///
/// # Example
///
/// ```
/// let id = fear_engine_common::types::new_session_id();
/// assert!(!id.is_empty());
/// ```
pub fn new_session_id() -> String {
    Uuid::new_v4().to_string()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    // -- FearType ----------------------------------------------------------

    #[test]
    fn test_fear_type_all_returns_10_variants() {
        let all = FearType::all();
        assert_eq!(all.len(), 10);
        // No duplicates.
        let mut deduped = all.clone();
        deduped.sort_by_key(|f| f.to_string());
        deduped.dedup();
        assert_eq!(deduped.len(), 10);
    }

    #[test]
    fn test_fear_type_display_roundtrip() {
        for ft in FearType::all() {
            let s = ft.to_string();
            let parsed: FearType = s.parse().expect("should parse back");
            assert_eq!(ft, parsed);
        }
    }

    #[test]
    fn test_fear_type_from_str_case_insensitive() {
        assert_eq!("BODY_HORROR".parse::<FearType>().unwrap(), FearType::BodyHorror);
        assert_eq!("BodyHorror".parse::<FearType>().unwrap(), FearType::BodyHorror);
        assert_eq!("body_horror".parse::<FearType>().unwrap(), FearType::BodyHorror);
    }

    #[test]
    fn test_fear_type_from_str_invalid() {
        let result = "spiders".parse::<FearType>();
        assert!(result.is_err());
    }

    #[test]
    fn test_fear_type_serde_roundtrip() {
        for ft in FearType::all() {
            let json = serde_json::to_string(&ft).unwrap();
            let back: FearType = serde_json::from_str(&json).unwrap();
            assert_eq!(ft, back);
        }
    }

    proptest! {
        #[test]
        fn test_fear_type_serialization_always_valid_json(idx in 0usize..10) {
            let all = FearType::all();
            let ft = all[idx];
            let json = serde_json::to_string(&ft).unwrap();
            let value: serde_json::Value = serde_json::from_str(&json).unwrap();
            prop_assert!(value.is_string());
        }
    }

    // -- GamePhase ---------------------------------------------------------

    #[test]
    fn test_game_phase_ordering() {
        assert!(GamePhase::Calibrating < GamePhase::Exploring);
        assert!(GamePhase::Exploring < GamePhase::Escalating);
        assert!(GamePhase::Escalating < GamePhase::Climax);
        assert!(GamePhase::Climax < GamePhase::Reveal);
    }

    #[test]
    fn test_game_phase_ordinal_values() {
        assert_eq!(GamePhase::Calibrating.ordinal(), 0);
        assert_eq!(GamePhase::Exploring.ordinal(), 1);
        assert_eq!(GamePhase::Escalating.ordinal(), 2);
        assert_eq!(GamePhase::Climax.ordinal(), 3);
        assert_eq!(GamePhase::Reveal.ordinal(), 4);
    }

    #[test]
    fn test_game_phase_serde_roundtrip() {
        let phases = [
            GamePhase::Calibrating,
            GamePhase::Exploring,
            GamePhase::Escalating,
            GamePhase::Climax,
            GamePhase::Reveal,
        ];
        for phase in &phases {
            let json = serde_json::to_string(phase).unwrap();
            let back: GamePhase = serde_json::from_str(&json).unwrap();
            assert_eq!(phase, &back);
        }
    }

    // -- BehaviorEvent / BehaviorEventType ---------------------------------

    #[test]
    fn test_behavior_event_keystroke_roundtrip() {
        let event = BehaviorEvent {
            event_type: BehaviorEventType::Keystroke {
                chars_per_second: 5.2,
                backspace_count: 3,
                total_chars: 42,
            },
            timestamp: Utc::now(),
            scene_id: "scene_01".into(),
        };
        let json = serde_json::to_string(&event).unwrap();
        let back: BehaviorEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(back.scene_id, "scene_01");
    }

    #[test]
    fn test_behavior_event_pause_roundtrip() {
        let event = BehaviorEvent {
            event_type: BehaviorEventType::Pause {
                duration_ms: 3500,
                scene_content_hash: "abc123".into(),
            },
            timestamp: Utc::now(),
            scene_id: "scene_02".into(),
        };
        let json = serde_json::to_string(&event).unwrap();
        let back: BehaviorEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(back.scene_id, "scene_02");
    }

    #[test]
    fn test_behavior_event_choice_roundtrip() {
        let event = BehaviorEvent {
            event_type: BehaviorEventType::Choice {
                choice_id: "c1".into(),
                time_to_decide_ms: 2200,
                approach: ChoiceApproach::Investigate,
            },
            timestamp: Utc::now(),
            scene_id: "scene_03".into(),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("investigate"));
        let back: BehaviorEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(back.scene_id, "scene_03");
    }

    #[test]
    fn test_behavior_event_mouse_roundtrip() {
        let event = BehaviorEvent {
            event_type: BehaviorEventType::MouseMovement {
                velocity: 120.5,
                tremor_score: 0.8,
            },
            timestamp: Utc::now(),
            scene_id: "scene_04".into(),
        };
        let json = serde_json::to_string(&event).unwrap();
        let back: BehaviorEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(back.scene_id, "scene_04");
    }

    #[test]
    fn test_behavior_event_scroll_roundtrip() {
        let event = BehaviorEvent {
            event_type: BehaviorEventType::Scroll {
                direction: ScrollDirection::Up,
                to_position: 0.3,
                rereading: true,
            },
            timestamp: Utc::now(),
            scene_id: "scene_05".into(),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"rereading\":true"));
        let back: BehaviorEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(back.scene_id, "scene_05");
    }

    // -- ChoiceApproach / Atmosphere ---------------------------------------

    #[test]
    fn test_choice_approach_serde_roundtrip() {
        let approaches = [
            ChoiceApproach::Investigate,
            ChoiceApproach::Avoid,
            ChoiceApproach::Confront,
            ChoiceApproach::Flee,
            ChoiceApproach::Interact,
            ChoiceApproach::Wait,
        ];
        for approach in &approaches {
            let json = serde_json::to_string(approach).unwrap();
            let back: ChoiceApproach = serde_json::from_str(&json).unwrap();
            assert_eq!(approach, &back);
        }
    }

    #[test]
    fn test_atmosphere_serde_roundtrip() {
        let atmospheres = [
            Atmosphere::Dread,
            Atmosphere::Tension,
            Atmosphere::Panic,
            Atmosphere::Calm,
            Atmosphere::Wrongness,
            Atmosphere::Isolation,
            Atmosphere::Paranoia,
        ];
        for atm in &atmospheres {
            let json = serde_json::to_string(atm).unwrap();
            let back: Atmosphere = serde_json::from_str(&json).unwrap();
            assert_eq!(atm, &back);
        }
    }

    // -- AdaptationStrategy ------------------------------------------------

    #[test]
    fn test_adaptation_probe_roundtrip() {
        let strat = AdaptationStrategy::Probe {
            target_fears: vec![FearType::Darkness, FearType::Isolation],
            intensity: 0.3,
        };
        let json = serde_json::to_string(&strat).unwrap();
        let back: AdaptationStrategy = serde_json::from_str(&json).unwrap();
        assert_eq!(strat, back);
    }

    #[test]
    fn test_adaptation_gradual_escalation_roundtrip() {
        let strat = AdaptationStrategy::GradualEscalation {
            primary_fear: FearType::BodyHorror,
            intensity_curve: EscalationCurve::Sigmoid,
        };
        let json = serde_json::to_string(&strat).unwrap();
        let back: AdaptationStrategy = serde_json::from_str(&json).unwrap();
        assert_eq!(strat, back);
    }

    #[test]
    fn test_adaptation_contrast_roundtrip() {
        let strat = AdaptationStrategy::Contrast {
            calm_duration: 3,
            storm_fear: FearType::Stalking,
            storm_intensity: 0.9,
        };
        let json = serde_json::to_string(&strat).unwrap();
        let back: AdaptationStrategy = serde_json::from_str(&json).unwrap();
        assert_eq!(strat, back);
    }

    #[test]
    fn test_adaptation_layering_roundtrip() {
        let strat = AdaptationStrategy::Layering {
            base_fear: FearType::Darkness,
            amplifier_fear: FearType::SoundBased,
            blend_ratio: 0.6,
        };
        let json = serde_json::to_string(&strat).unwrap();
        let back: AdaptationStrategy = serde_json::from_str(&json).unwrap();
        assert_eq!(strat, back);
    }

    #[test]
    fn test_adaptation_subversion_roundtrip() {
        let strat = AdaptationStrategy::Subversion {
            expected_fear: FearType::Claustrophobia,
            actual_fear: FearType::UncannyValley,
        };
        let json = serde_json::to_string(&strat).unwrap();
        let back: AdaptationStrategy = serde_json::from_str(&json).unwrap();
        assert_eq!(strat, back);
    }

    // -- NarrativeResponse / Choice / MetaBreak ----------------------------

    #[test]
    fn test_narrative_response_roundtrip() {
        let resp = NarrativeResponse {
            narrative: "The corridor stretches before you.".into(),
            atmosphere: Atmosphere::Dread,
            sound_cue: Some("distant_dripping".into()),
            image_prompt: None,
            choices: vec![Choice {
                id: "c1".into(),
                text: "Step into the darkness".into(),
                approach: ChoiceApproach::Investigate,
                fear_vector: FearType::Darkness,
            }],
            hidden_elements: vec!["shadow in peripheral vision".into()],
            intensity: 0.6,
            meta_break: None,
            transcript_lines: vec![],
            question_prompts: vec![],
            archive_entries: vec![],
            mirror_observations: vec![],
        };
        let json = serde_json::to_string(&resp).unwrap();
        let back: NarrativeResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(back.narrative, resp.narrative);
        assert_eq!(back.choices.len(), 1);
    }

    #[test]
    fn test_narrative_response_with_meta_break() {
        let resp = NarrativeResponse {
            narrative: "Something watches.".into(),
            atmosphere: Atmosphere::Paranoia,
            sound_cue: None,
            image_prompt: None,
            choices: vec![],
            hidden_elements: vec![],
            intensity: 0.9,
            meta_break: Some(MetaBreak {
                text: "I can see your cursor trembling.".into(),
                target: MetaTarget::Whisper,
            }),
            transcript_lines: vec![],
            question_prompts: vec![],
            archive_entries: vec![],
            mirror_observations: vec![],
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("whisper"));
        let back: NarrativeResponse = serde_json::from_str(&json).unwrap();
        assert!(back.meta_break.is_some());
    }

    // -- ClientMessage / ServerMessage -------------------------------------

    #[test]
    fn test_client_message_start_game_roundtrip() {
        let msg = ClientMessage::StartGame {
            player_name: Some("Alice".into()),
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"type\":\"start_game\""));
        let back: ClientMessage = serde_json::from_str(&json).unwrap();
        match back {
            ClientMessage::StartGame { player_name } => {
                assert_eq!(player_name.as_deref(), Some("Alice"));
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn test_client_message_choice_roundtrip() {
        let msg = ClientMessage::Choice {
            scene_id: "s1".into(),
            choice_id: "c1".into(),
            time_to_decide_ms: 1500,
            approach: ChoiceApproach::Investigate,
        };
        let json = serde_json::to_string(&msg).unwrap();
        let back: ClientMessage = serde_json::from_str(&json).unwrap();
        match back {
            ClientMessage::Choice {
                time_to_decide_ms,
                approach,
                ..
            } => {
                assert_eq!(time_to_decide_ms, 1500);
                assert_eq!(approach, ChoiceApproach::Investigate);
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn test_client_message_text_input_roundtrip() {
        let msg = ClientMessage::TextInput {
            scene_id: "s2".into(),
            text: "I open the door".into(),
            typing_duration_ms: 4200,
            backspace_count: 2,
        };
        let json = serde_json::to_string(&msg).unwrap();
        let back: ClientMessage = serde_json::from_str(&json).unwrap();
        match back {
            ClientMessage::TextInput { text, .. } => {
                assert_eq!(text, "I open the door");
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn test_server_message_narrative_roundtrip() {
        let msg = ServerMessage::Narrative {
            scene_id: "s1".into(),
            text: "The room is empty.".into(),
            atmosphere: Atmosphere::Isolation,
            choices: vec![],
            sound_cue: None,
            intensity: 0.4,
            effects: vec![EffectDirective {
                effect: EffectType::Flicker,
                intensity: 0.3,
                duration_ms: 200,
                delay_ms: 0,
            }],
            title: None,
            act: Some(SessionAct::Calibration),
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
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"type\":\"narrative\""));
        let back: ServerMessage = serde_json::from_str(&json).unwrap();
        match back {
            ServerMessage::Narrative { effects, .. } => assert_eq!(effects.len(), 1),
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn test_server_message_image_roundtrip() {
        let msg = ServerMessage::Image {
            scene_id: "s1".into(),
            image_url: "data:image/png;base64,abc".into(),
            display_mode: DisplayMode::Glitch,
        };
        let json = serde_json::to_string(&msg).unwrap();
        let back: ServerMessage = serde_json::from_str(&json).unwrap();
        match back {
            ServerMessage::Image { display_mode, .. } => {
                assert_eq!(display_mode, DisplayMode::Glitch);
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn test_server_message_phase_change_roundtrip() {
        let msg = ServerMessage::PhaseChange {
            from: GamePhase::Calibrating,
            to: GamePhase::Exploring,
        };
        let json = serde_json::to_string(&msg).unwrap();
        let back: ServerMessage = serde_json::from_str(&json).unwrap();
        match back {
            ServerMessage::PhaseChange { from, to } => {
                assert_eq!(from, GamePhase::Calibrating);
                assert_eq!(to, GamePhase::Exploring);
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn test_server_message_meta_roundtrip() {
        let msg = ServerMessage::Meta {
            text: "I know what you're afraid of.".into(),
            target: MetaTarget::GlitchText,
            delay_ms: 500,
        };
        let json = serde_json::to_string(&msg).unwrap();
        let back: ServerMessage = serde_json::from_str(&json).unwrap();
        match back {
            ServerMessage::Meta { target, .. } => {
                assert_eq!(target, MetaTarget::GlitchText);
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn test_server_message_reveal_roundtrip() {
        let msg = ServerMessage::Reveal {
            fear_profile: FearProfileSummary {
                scores: vec![FearScore {
                    fear_type: FearType::Darkness,
                    score: 0.85,
                    confidence: 0.72,
                }],
                primary_fear: FearType::Darkness,
                secondary_fear: Some(FearType::Isolation),
                total_observations: 150,
            },
            behavior_profile: BehaviorProfileSummary {
                compliance: 0.6,
                resistance: 0.3,
                curiosity: 0.7,
                avoidance: 0.2,
                self_editing: 0.4,
                need_for_certainty: 0.5,
                ritualized_control: 0.3,
                recovery_after_escalation: 0.6,
                tolerance_after_violation: 0.7,
            },
            session_summary: SessionSummary {
                duration_seconds: 2400,
                total_beats: 12,
                focus_interruptions: 1,
                camera_permission_granted: Some(true),
                microphone_permission_granted: Some(false),
                contradiction_count: 2,
                media_exposures: vec![MediumExposure {
                    medium: SurfaceMedium::Chat,
                    count: 4,
                }],
                completion_reason: "completed".into(),
            },
            key_moments: vec![KeyMoment {
                scene_id: "s5".into(),
                description: "Player hesitated at the dark hallway".into(),
                fear_revealed: FearType::Darkness,
                behavior_trigger: "3.2s pause".into(),
            }],
            adaptation_log: vec![AdaptationRecord {
                scene_id: "s8".into(),
                strategy: "gradual_escalation".into(),
                fear_targeted: FearType::Darkness,
                intensity: 0.7,
            }],
            ending_classification: EndingClassification::CompliantWitness,
            analysis: RevealAnalysis {
                summary: "The session repeatedly converged on darkness.".into(),
                key_patterns: vec!["You paused at threshold spaces.".into()],
                adaptation_summary: "The game escalated darkness cues over time.".into(),
                closing_message: "The hospital learned exactly where to dim the lights.".into(),
            },
        };
        let json = serde_json::to_string(&msg).unwrap();
        let back: ServerMessage = serde_json::from_str(&json).unwrap();
        match back {
            ServerMessage::Reveal { fear_profile, key_moments, .. } => {
                assert_eq!(fear_profile.primary_fear, FearType::Darkness);
                assert_eq!(key_moments.len(), 1);
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn test_server_message_error_roundtrip() {
        let msg = ServerMessage::Error {
            code: "AI_TIMEOUT".into(),
            message: "Narrative generation timed out".into(),
            recoverable: true,
        };
        let json = serde_json::to_string(&msg).unwrap();
        let back: ServerMessage = serde_json::from_str(&json).unwrap();
        match back {
            ServerMessage::Error { recoverable, .. } => assert!(recoverable),
            _ => panic!("wrong variant"),
        }
    }

    // -- EffectDirective ---------------------------------------------------

    #[test]
    fn test_effect_directive_roundtrip() {
        let effects = [
            EffectType::Shake,
            EffectType::Flicker,
            EffectType::Glitch,
            EffectType::Darkness,
            EffectType::Flashlight,
            EffectType::Crt,
            EffectType::SlowType,
            EffectType::FastType,
            EffectType::StrobeFlash,
            EffectType::ChromaticShift,
            EffectType::FocusPulse,
            EffectType::FrameJump,
        ];
        for effect in &effects {
            let directive = EffectDirective {
                effect: *effect,
                intensity: 0.5,
                duration_ms: 1000,
                delay_ms: 100,
            };
            let json = serde_json::to_string(&directive).unwrap();
            let back: EffectDirective = serde_json::from_str(&json).unwrap();
            assert_eq!(back.effect, *effect);
        }
    }

    // -- new_session_id ----------------------------------------------------

    #[test]
    fn test_new_session_id_unique() {
        let a = new_session_id();
        let b = new_session_id();
        assert_ne!(a, b);
        assert_eq!(a.len(), 36); // UUID v4 string length
    }
}
