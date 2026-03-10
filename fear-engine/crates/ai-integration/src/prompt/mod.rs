//! Multi-layer prompt assembly for the Fear Engine narrative generator.
//!
//! The prompt has four layers:
//!
//! | Layer | Source | Content |
//! |-------|--------|---------|
//! | 1 | [`system::SYSTEM_PROMPT`] | Constant persona & rules |
//! | 2 | [`context::PromptContext::build_fear_layer`] | Dynamic fear profile |
//! | 3 | [`context::PromptContext::build_game_state_layer`] | Dynamic game state |
//! | 4 | [`output::OUTPUT_SCHEMA`] | Constant JSON schema |

pub mod context;
pub mod output;
pub mod system;

use context::PromptContext;
use output::OUTPUT_SCHEMA;
use system::SYSTEM_PROMPT;

/// Assembles the complete prompt from all four layers.
///
/// # Example
///
/// ```
/// use fear_engine_ai_integration::prompt::PromptBuilder;
///
/// let sys = PromptBuilder::build_system_prompt();
/// assert!(sys.contains("FEAR ENGINE"));
/// ```
pub struct PromptBuilder;

impl PromptBuilder {
    /// Returns the full system prompt (Layer 1).
    ///
    /// # Example
    ///
    /// ```
    /// use fear_engine_ai_integration::prompt::PromptBuilder;
    /// let p = PromptBuilder::build_system_prompt();
    /// assert!(!p.is_empty());
    /// ```
    pub fn build_system_prompt() -> String {
        SYSTEM_PROMPT.to_string()
    }

    /// Assembles Layers 2 + 3 + 4 into the user message.
    ///
    /// # Example
    ///
    /// ```
    /// use fear_engine_ai_integration::prompt::PromptBuilder;
    /// use fear_engine_ai_integration::prompt::context::*;
    /// use fear_engine_common::types::*;
    /// use fear_engine_fear_profile::adaptation::AdaptationEngine;
    /// use fear_engine_fear_profile::profile::FearProfile;
    ///
    /// let mut engine = AdaptationEngine::new();
    /// let profile = FearProfile::new();
    /// let adaptation = engine.compute_directive(GamePhase::Calibrating, &profile, 1);
    /// let ctx = PromptContext {
    ///     fear_profile: FearProfileContext {
    ///         top_fears: vec![], anxiety_threshold: 0.5,
    ///         behavioral_pattern: "neutral".into(),
    ///         estimated_emotional_state: "calm".into(),
    ///     },
    ///     game_state: GameStateContext {
    ///         location: "Lobby".into(), phase: GamePhase::Calibrating,
    ///         medium: Some(SurfaceMedium::SystemDialog),
    ///         trust_posture: Some(TrustPosture::Helpful),
    ///         scene_number: 1, last_scene_summary: "N/A".into(),
    ///         last_choice: "N/A".into(), active_threads: vec![],
    ///         inventory: vec![], established_details: vec![],
    ///     },
    ///     adaptation,
    /// };
    /// let msg = PromptBuilder::build_user_message(&ctx);
    /// assert!(msg.contains("PLAYER PSYCHOLOGICAL PROFILE"));
    /// assert!(msg.contains("GAME STATE"));
    /// assert!(msg.contains("narrative"));
    /// ```
    pub fn build_user_message(context: &PromptContext) -> String {
        let fear_layer = context.build_fear_layer();
        let game_layer = context.build_game_state_layer();

        format!(
            "{fear_layer}\n\n\
             {game_layer}\n\n\
             {OUTPUT_SCHEMA}\n\n\
             Generate the next scene."
        )
    }

    /// Rough token estimate: ~4 characters per token.
    ///
    /// # Example
    ///
    /// ```
    /// use fear_engine_ai_integration::prompt::PromptBuilder;
    /// assert_eq!(PromptBuilder::estimate_tokens("Hello world!"), 3);
    /// ```
    pub fn estimate_tokens(prompt: &str) -> u32 {
        (prompt.len() as f64 / 4.0).ceil() as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use context::*;
    use fear_engine_common::types::*;
    use fear_engine_fear_profile::adaptation::AdaptationEngine;
    use fear_engine_fear_profile::profile::FearProfile;

    fn make_context(phase: GamePhase, scene: u32) -> PromptContext {
        let mut engine = AdaptationEngine::new();
        let profile = FearProfile::new();
        let adaptation = engine.compute_directive(phase, &profile, scene);

        PromptContext {
            fear_profile: FearProfileContext {
                top_fears: vec![
                    (FearType::Darkness, 0.78, 0.65),
                    (FearType::Stalking, 0.62, 0.50),
                ],
                anxiety_threshold: 0.65,
                behavioral_pattern: "cautious explorer".into(),
                estimated_emotional_state: "tense".into(),
            },
            game_state: GameStateContext {
                location: "Hospital basement".into(),
                phase,
                medium: Some(SurfaceMedium::Archive),
                trust_posture: Some(TrustPosture::Clinical),
                scene_number: scene,
                last_scene_summary: "Discovered a locked ward.".into(),
                last_choice: "Tried the door handle".into(),
                active_threads: vec!["power outage".into()],
                inventory: vec!["flashlight".into()],
                established_details: vec!["Ward B is sealed".into()],
            },
            adaptation,
        }
    }

    #[test]
    fn test_system_prompt_nonempty() {
        let p = PromptBuilder::build_system_prompt();
        assert!(p.len() > 100);
    }

    #[test]
    fn test_full_prompt_combines_all_layers() {
        let ctx = make_context(GamePhase::Exploring, 5);
        let msg = PromptBuilder::build_user_message(&ctx);

        // Layer 2
        assert!(msg.contains("PLAYER PSYCHOLOGICAL PROFILE"));
        assert!(msg.contains("darkness"));
        assert!(msg.contains("NARRATIVE DIRECTIVE"));

        // Layer 3
        assert!(msg.contains("GAME STATE"));
        assert!(msg.contains("Hospital basement"));
        assert!(msg.contains("flashlight"));

        // Layer 4
        assert!(msg.contains("narrative"));
        assert!(msg.contains("choices"));

        // Final instruction
        assert!(msg.contains("Generate the next scene"));
    }

    #[test]
    fn test_token_estimation() {
        assert_eq!(PromptBuilder::estimate_tokens("Hello world!"), 3);
        assert_eq!(PromptBuilder::estimate_tokens(""), 0);
        assert_eq!(PromptBuilder::estimate_tokens("abcd"), 1);
    }

    #[test]
    fn test_snapshot_calibrating_prompt() {
        let ctx = make_context(GamePhase::Calibrating, 2);
        let msg = PromptBuilder::build_user_message(&ctx);
        insta::assert_snapshot!("prompt_calibrating", msg);
    }

    #[test]
    fn test_snapshot_exploring_prompt() {
        let ctx = make_context(GamePhase::Exploring, 6);
        let msg = PromptBuilder::build_user_message(&ctx);
        insta::assert_snapshot!("prompt_exploring", msg);
    }

    #[test]
    fn test_snapshot_escalating_prompt() {
        let ctx = make_context(GamePhase::Escalating, 12);
        let msg = PromptBuilder::build_user_message(&ctx);
        insta::assert_snapshot!("prompt_escalating", msg);
    }

    #[test]
    fn test_snapshot_climax_prompt() {
        let ctx = make_context(GamePhase::Climax, 17);
        let msg = PromptBuilder::build_user_message(&ctx);
        insta::assert_snapshot!("prompt_climax", msg);
    }
}
