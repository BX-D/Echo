//! Dynamic context builders for Layers 2 (fear profile) and 3 (game state).

use fear_engine_common::types::{FearType, GamePhase, SurfaceMedium, TrustPosture};
use fear_engine_fear_profile::adaptation::AdaptationDirective;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Fear profile context (Layer 2)
// ---------------------------------------------------------------------------

/// Summarised fear-profile data ready for prompt insertion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FearProfileContext {
    /// Top fears: `(fear_type, score, confidence)`.
    pub top_fears: Vec<(FearType, f64, f64)>,
    /// Overall anxiety level (0–1).
    pub anxiety_threshold: f64,
    /// Human-readable summary of the player's behavioral pattern.
    pub behavioral_pattern: String,
    /// Estimated current emotional state.
    pub estimated_emotional_state: String,
}

// ---------------------------------------------------------------------------
// Game state context (Layer 3)
// ---------------------------------------------------------------------------

/// Current game-world state for prompt insertion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameStateContext {
    /// Description of the player's current location.
    pub location: String,
    /// Current game phase.
    pub phase: GamePhase,
    /// Current interface surface.
    pub medium: Option<SurfaceMedium>,
    /// Current system posture.
    pub trust_posture: Option<TrustPosture>,
    /// Scene number within the session.
    pub scene_number: u32,
    /// One-line summary of the previous scene.
    pub last_scene_summary: String,
    /// The text of the player's last choice.
    pub last_choice: String,
    /// Ongoing narrative threads.
    pub active_threads: Vec<String>,
    /// Items the player is carrying.
    pub inventory: Vec<String>,
    /// Facts the player has learned about the world.
    pub established_details: Vec<String>,
}

// ---------------------------------------------------------------------------
// Combined prompt context
// ---------------------------------------------------------------------------

/// All dynamic data needed to build Layers 2–3 of the prompt.
///
/// # Example
///
/// ```
/// use fear_engine_ai_integration::prompt::context::*;
/// use fear_engine_common::types::*;
/// use fear_engine_fear_profile::adaptation::{AdaptationDirective, AdaptationEngine};
/// use fear_engine_fear_profile::profile::FearProfile;
///
/// let fear = FearProfileContext {
///     top_fears: vec![(FearType::Darkness, 0.8, 0.6)],
///     anxiety_threshold: 0.7,
///     behavioral_pattern: "cautious explorer".into(),
///     estimated_emotional_state: "anxious".into(),
/// };
/// let game = GameStateContext {
///     location: "Hospital corridor".into(),
///     phase: GamePhase::Exploring,
///     medium: Some(SurfaceMedium::Chat),
///     trust_posture: Some(TrustPosture::Helpful),
///     scene_number: 5,
///     last_scene_summary: "Found a locked door.".into(),
///     last_choice: "Tried the handle".into(),
///     active_threads: vec!["missing patient file".into()],
///     inventory: vec!["flashlight".into()],
///     established_details: vec!["power is out in ward B".into()],
/// };
/// let mut engine = AdaptationEngine::new();
/// let profile = FearProfile::new();
/// let adaptation = engine.compute_directive(GamePhase::Exploring, &profile, 5);
///
/// let ctx = PromptContext { fear_profile: fear, game_state: game, adaptation };
/// let layer2 = ctx.build_fear_layer();
/// assert!(layer2.contains("darkness"));
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptContext {
    pub fear_profile: FearProfileContext,
    pub game_state: GameStateContext,
    pub adaptation: AdaptationDirective,
}

impl PromptContext {
    /// Builds the Layer 2 fear-profile section of the user message.
    ///
    /// # Example
    ///
    /// ```
    /// # use fear_engine_ai_integration::prompt::context::*;
    /// # use fear_engine_common::types::*;
    /// # use fear_engine_fear_profile::adaptation::{AdaptationDirective, AdaptationEngine};
    /// # use fear_engine_fear_profile::profile::FearProfile;
    /// # let fear = FearProfileContext { top_fears: vec![(FearType::Darkness, 0.8, 0.6)], anxiety_threshold: 0.7, behavioral_pattern: "cautious".into(), estimated_emotional_state: "tense".into() };
    /// # let game = GameStateContext { location: "Hall".into(), phase: GamePhase::Exploring, medium: Some(SurfaceMedium::Chat), trust_posture: Some(TrustPosture::Helpful), scene_number: 3, last_scene_summary: "s".into(), last_choice: "c".into(), active_threads: vec![], inventory: vec![], established_details: vec![] };
    /// # let mut e = AdaptationEngine::new(); let p = FearProfile::new(); let a = e.compute_directive(GamePhase::Exploring, &p, 3);
    /// # let ctx = PromptContext { fear_profile: fear, game_state: game, adaptation: a };
    /// let layer2 = ctx.build_fear_layer();
    /// assert!(layer2.contains("PLAYER PSYCHOLOGICAL PROFILE"));
    /// ```
    pub fn build_fear_layer(&self) -> String {
        let fp = &self.fear_profile;
        let ad = &self.adaptation;

        let primary_line = match fp.top_fears.first() {
            Some((fear, score, confidence)) => format!(
                "Primary fear axis: {} (score: {:.0}%, confidence: {:.0}%)",
                fear,
                score * 100.0,
                confidence * 100.0,
            ),
            None => "Primary fear axis: undetermined".into(),
        };

        let secondary_lines: String = fp
            .top_fears
            .iter()
            .skip(1)
            .map(|(f, s, _)| format!("{} ({:.0}%)", f, s * 100.0))
            .collect::<Vec<_>>()
            .join(", ");

        let strategy_name = strategy_label(&ad.strategy);
        let forbidden = if ad.forbidden_elements.is_empty() {
            "none".into()
        } else {
            ad.forbidden_elements.join("; ")
        };

        format!(
            "PLAYER PSYCHOLOGICAL PROFILE:\n\
             - {primary_line}\n\
             - Secondary fears: {secondary_lines}\n\
             - Anxiety baseline: {:.2}/1.0\n\
             - Behavioral pattern: {}\n\
             - Current emotional state: {}\n\
             \n\
             NARRATIVE DIRECTIVE:\n\
             Strategy: {strategy_name}\n\
             Instruction: {}\n\
             Target intensity: {:.2}/1.0\n\
             Forbidden: {forbidden}",
            fp.anxiety_threshold,
            fp.behavioral_pattern,
            fp.estimated_emotional_state,
            ad.specific_instruction,
            ad.intensity_target,
        )
    }

    /// Builds the Layer 3 game-state section of the user message.
    ///
    /// # Example
    ///
    /// ```
    /// # use fear_engine_ai_integration::prompt::context::*;
    /// # use fear_engine_common::types::*;
    /// # use fear_engine_fear_profile::adaptation::{AdaptationDirective, AdaptationEngine};
    /// # use fear_engine_fear_profile::profile::FearProfile;
    /// # let fear = FearProfileContext { top_fears: vec![], anxiety_threshold: 0.5, behavioral_pattern: "n".into(), estimated_emotional_state: "n".into() };
    /// # let game = GameStateContext { location: "Ward A".into(), phase: GamePhase::Exploring, medium: Some(SurfaceMedium::Archive), trust_posture: Some(TrustPosture::Clinical), scene_number: 4, last_scene_summary: "s".into(), last_choice: "c".into(), active_threads: vec!["mystery".into()], inventory: vec!["key".into()], established_details: vec!["power is out".into()] };
    /// # let mut e = AdaptationEngine::new(); let p = FearProfile::new(); let a = e.compute_directive(GamePhase::Exploring, &p, 4);
    /// # let ctx = PromptContext { fear_profile: fear, game_state: game, adaptation: a };
    /// let layer3 = ctx.build_game_state_layer();
    /// assert!(layer3.contains("Ward A"));
    /// ```
    pub fn build_game_state_layer(&self) -> String {
        let gs = &self.game_state;
        let threads = if gs.active_threads.is_empty() {
            "none".into()
        } else {
            gs.active_threads.join(", ")
        };
        let inventory = if gs.inventory.is_empty() {
            "nothing".into()
        } else {
            gs.inventory.join(", ")
        };
        let details = if gs.established_details.is_empty() {
            "none yet".into()
        } else {
            gs.established_details.join("; ")
        };

        format!(
            "GAME STATE:\n\
             Location: {}\n\
             Surface medium: {}\n\
             Trust posture: {}\n\
             Phase: {} (scene {}/~18)\n\
             Previous scene summary: {}\n\
             Player's last action: {}\n\
             Active narrative threads: {threads}\n\
             Player inventory: {inventory}\n\
             Established details: {details}\n\
             Surface directive: {}",
            gs.location,
            gs.medium
                .map(|medium| format!("{medium:?}").to_lowercase())
                .unwrap_or_else(|| "chat".into()),
            gs.trust_posture
                .map(|posture| format!("{posture:?}").to_lowercase())
                .unwrap_or_else(|| "helpful".into()),
            gs.phase,
            gs.scene_number,
            gs.last_scene_summary,
            gs.last_choice,
            surface_directive(gs.medium),
        )
    }
}

fn surface_directive(medium: Option<SurfaceMedium>) -> &'static str {
    match medium.unwrap_or(SurfaceMedium::Chat) {
        SurfaceMedium::Chat => {
            "Write flowing conversational prose. Keep all surface arrays empty."
        }
        SurfaceMedium::Questionnaire => {
            "Return 2-3 concise question_prompts that feel like tailored intake prompts. The main narrative should frame the questionnaire rather than duplicate it."
        }
        SurfaceMedium::Archive => {
            "Return 2-4 archive_entries formatted like recovered records or revised notes. The main narrative should contextualize the archive, not repeat every entry."
        }
        SurfaceMedium::Transcript => {
            "Return 2-4 transcript_lines that feel like clipped recovered dialogue or machine transcripts. The main narrative should describe why the transcript matters now."
        }
        SurfaceMedium::Webcam => {
            "Return 2-3 mirror_observations describing what the intelligence thinks it sees in the player. The narrative should describe the presence frame and the emotional pressure of being observed."
        }
        SurfaceMedium::Microphone => {
            "Use the main narrative to focus on listening, room tone, and silence. Keep transcript_lines empty unless a synthetic replay is part of the beat."
        }
        SurfaceMedium::SystemDialog => {
            "Write with procedural precision, as if the system is interrupting the session with a notice or correction."
        }
        SurfaceMedium::Mirror => {
            "Return 2-3 mirror_observations that read like cold interpretive judgments. The narrative should feel final and diagnostic."
        }
    }
}

/// Returns a human-readable label for a strategy variant.
fn strategy_label(strategy: &fear_engine_common::types::AdaptationStrategy) -> &'static str {
    use fear_engine_common::types::AdaptationStrategy;
    match strategy {
        AdaptationStrategy::Probe { .. } => "Probe",
        AdaptationStrategy::GradualEscalation { .. } => "Gradual Escalation",
        AdaptationStrategy::Contrast { .. } => "Contrast",
        AdaptationStrategy::Layering { .. } => "Layering",
        AdaptationStrategy::Subversion { .. } => "Subversion",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fear_engine_fear_profile::adaptation::AdaptationEngine;
    use fear_engine_fear_profile::profile::FearProfile;

    fn sample_context(phase: GamePhase, scene: u32) -> PromptContext {
        let mut engine = AdaptationEngine::new();
        let profile = FearProfile::new();
        let adaptation = engine.compute_directive(phase, &profile, scene);

        PromptContext {
            fear_profile: FearProfileContext {
                top_fears: vec![
                    (FearType::Darkness, 0.78, 0.65),
                    (FearType::Isolation, 0.62, 0.50),
                ],
                anxiety_threshold: 0.7,
                behavioral_pattern: "cautious but curious".into(),
                estimated_emotional_state: "anxious".into(),
            },
            game_state: GameStateContext {
                location: "Hospital basement".into(),
                phase,
                medium: Some(SurfaceMedium::Archive),
                trust_posture: Some(TrustPosture::Clinical),
                scene_number: scene,
                last_scene_summary: "Player found a locked medicine cabinet.".into(),
                last_choice: "Tried to force the lock open".into(),
                active_threads: vec!["missing patient file".into(), "power outage".into()],
                inventory: vec!["flashlight".into(), "bent key".into()],
                established_details: vec![
                    "Ward B has no power".into(),
                    "Something scratches inside the walls".into(),
                ],
            },
            adaptation,
        }
    }

    #[test]
    fn test_fear_layer_contains_top_fears() {
        let ctx = sample_context(GamePhase::Exploring, 5);
        let layer = ctx.build_fear_layer();
        assert!(layer.contains("darkness"));
        assert!(layer.contains("isolation"));
        assert!(layer.contains("PLAYER PSYCHOLOGICAL PROFILE"));
    }

    #[test]
    fn test_fear_layer_contains_directive() {
        let ctx = sample_context(GamePhase::Exploring, 5);
        let layer = ctx.build_fear_layer();
        assert!(layer.contains("NARRATIVE DIRECTIVE"));
        assert!(layer.contains("Strategy:"));
        assert!(layer.contains("Target intensity:"));
    }

    #[test]
    fn test_game_state_layer_contains_location() {
        let ctx = sample_context(GamePhase::Exploring, 5);
        let layer = ctx.build_game_state_layer();
        assert!(layer.contains("Hospital basement"));
        assert!(layer.contains("GAME STATE"));
    }

    #[test]
    fn test_game_state_layer_contains_inventory() {
        let ctx = sample_context(GamePhase::Exploring, 5);
        let layer = ctx.build_game_state_layer();
        assert!(layer.contains("flashlight"));
        assert!(layer.contains("bent key"));
    }

    #[test]
    fn test_game_state_layer_contains_threads() {
        let ctx = sample_context(GamePhase::Exploring, 5);
        let layer = ctx.build_game_state_layer();
        assert!(layer.contains("missing patient file"));
    }

    #[test]
    fn test_empty_inventory_shows_nothing() {
        let mut ctx = sample_context(GamePhase::Calibrating, 1);
        ctx.game_state.inventory.clear();
        let layer = ctx.build_game_state_layer();
        assert!(layer.contains("nothing"));
    }
}
