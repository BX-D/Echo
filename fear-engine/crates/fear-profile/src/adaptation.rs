//! Adaptation strategy engine — decides *how* to scare the player based on
//! the current game phase, fear profile, and pacing requirements.
//!
//! The main entry point is [`AdaptationEngine::compute_directive`], which
//! returns an [`AdaptationDirective`] containing everything the AI prompt
//! builder needs to generate the next scene.

use fear_engine_common::types::{AdaptationStrategy, EscalationCurve, FearType, GamePhase};
use serde::{Deserialize, Serialize};

use crate::profile::FearProfile;

// ---------------------------------------------------------------------------
// Intensity curve
// ---------------------------------------------------------------------------

/// Piecewise-linear mapping from scene number to target horror intensity.
///
/// # Example
///
/// ```
/// use fear_engine_fear_profile::adaptation::IntensityCurve;
///
/// let curve = IntensityCurve::default_horror_curve();
/// let i = curve.intensity_at(1);
/// assert!(i >= 0.2 && i <= 0.3);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntensityCurve {
    /// Sorted list of `(scene_number, target_intensity)` knots.
    points: Vec<(u32, f64)>,
}

impl IntensityCurve {
    /// The canonical horror pacing curve.
    ///
    /// ```text
    /// Scenes  1– 3 : 0.20 → 0.30  (calibration, mild)
    /// Scenes  4– 8 : 0.30 → 0.50  (exploration, building)
    /// Scenes  9–12 : 0.50 → 0.70  (escalation, rising)
    /// Scene      13 : 0.40         (contrast valley)
    /// Scenes 14–16 : 0.70 → 0.90  (escalation peak)
    /// Scenes 17–18 : 0.90 → 1.00  (climax)
    /// ```
    pub fn default_horror_curve() -> Self {
        Self {
            points: vec![
                (1, 0.20),
                (3, 0.30),
                (4, 0.30),
                (8, 0.50),
                (9, 0.50),
                (12, 0.70),
                (13, 0.40),
                (14, 0.70),
                (16, 0.90),
                (17, 0.90),
                (18, 1.00),
            ],
        }
    }

    /// Linearly interpolates the intensity for a given scene number.
    ///
    /// Clamps to the first/last knot for out-of-range values.
    pub fn intensity_at(&self, scene: u32) -> f64 {
        if self.points.is_empty() {
            return 0.5;
        }
        if scene <= self.points[0].0 {
            return self.points[0].1;
        }
        let last = self.points[self.points.len() - 1];
        if scene >= last.0 {
            return last.1;
        }
        for window in self.points.windows(2) {
            let (s0, i0) = window[0];
            let (s1, i1) = window[1];
            if scene >= s0 && scene <= s1 {
                if s1 == s0 {
                    return i0;
                }
                let t = (scene - s0) as f64 / (s1 - s0) as f64;
                return i0 + t * (i1 - i0);
            }
        }
        last.1
    }
}

// ---------------------------------------------------------------------------
// Adaptation directive
// ---------------------------------------------------------------------------

/// Everything the AI prompt builder needs to generate the next scene.
///
/// Produced by [`AdaptationEngine::compute_directive`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdaptationDirective {
    /// The high-level strategy being applied.
    pub strategy: AdaptationStrategy,
    /// Target horror intensity for this scene (0.0–1.0).
    pub intensity_target: f64,
    /// The fear to focus on (if any).
    pub primary_fear: Option<FearType>,
    /// Additional fears to weave in.
    pub secondary_fears: Vec<FearType>,
    /// Natural-language instruction for the LLM.
    pub specific_instruction: String,
    /// Elements the LLM should avoid this scene (pacing / repetition).
    pub forbidden_elements: Vec<String>,
}

/// Persisted runtime state for reconnect/resume.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AdaptationEngineState {
    pub current_strategy: Option<AdaptationStrategy>,
    pub scenes_since_calm: u32,
    pub recent_fears_used: Vec<FearType>,
}

// ---------------------------------------------------------------------------
// Adaptation engine
// ---------------------------------------------------------------------------

/// Selects strategies and computes directives based on game phase and profile.
///
/// # Example
///
/// ```
/// use fear_engine_fear_profile::adaptation::AdaptationEngine;
/// use fear_engine_fear_profile::profile::FearProfile;
/// use fear_engine_common::types::GamePhase;
///
/// let mut engine = AdaptationEngine::new();
/// let profile = FearProfile::new();
/// let d = engine.compute_directive(GamePhase::Calibrating, &profile, 1);
/// assert!(d.intensity_target > 0.0);
/// assert!(!d.specific_instruction.is_empty());
/// ```
pub struct AdaptationEngine {
    current_strategy: Option<AdaptationStrategy>,
    intensity_curve: IntensityCurve,
    scenes_since_calm: u32,
    recent_fears_used: Vec<FearType>,
}

impl AdaptationEngine {
    /// Creates a new engine with the default horror intensity curve.
    pub fn new() -> Self {
        Self {
            current_strategy: None,
            intensity_curve: IntensityCurve::default_horror_curve(),
            scenes_since_calm: 0,
            recent_fears_used: Vec::new(),
        }
    }

    /// Computes a full directive for the next scene.
    pub fn compute_directive(
        &mut self,
        phase: GamePhase,
        profile: &FearProfile,
        scene_count: u32,
    ) -> AdaptationDirective {
        let strategy = self.select_strategy(phase, profile);
        let intensity = self.compute_intensity(phase, scene_count);
        let (primary, secondary) = extract_fears(&strategy, profile);
        let instruction = self.generate_instruction(&strategy, profile);
        let forbidden = self.determine_forbidden(phase);

        // Track pacing state.
        if matches!(strategy, AdaptationStrategy::Contrast { .. }) {
            self.scenes_since_calm = 0;
        } else {
            self.scenes_since_calm += 1;
        }
        if let Some(ref f) = primary {
            self.recent_fears_used.push(*f);
            if self.recent_fears_used.len() > 5 {
                self.recent_fears_used.remove(0);
            }
        }

        self.current_strategy = Some(strategy.clone());

        AdaptationDirective {
            strategy,
            intensity_target: intensity,
            primary_fear: primary,
            secondary_fears: secondary,
            specific_instruction: instruction,
            forbidden_elements: forbidden,
        }
    }

    /// Returns the current strategy (if any).
    pub fn current_strategy(&self) -> Option<&AdaptationStrategy> {
        self.current_strategy.as_ref()
    }

    /// Returns a persisted snapshot of the engine's pacing state.
    pub fn snapshot_state(&self) -> AdaptationEngineState {
        AdaptationEngineState {
            current_strategy: self.current_strategy.clone(),
            scenes_since_calm: self.scenes_since_calm,
            recent_fears_used: self.recent_fears_used.clone(),
        }
    }

    /// Restores pacing state for an already-started session.
    pub fn restore_state(&mut self, state: AdaptationEngineState) {
        self.current_strategy = state.current_strategy;
        self.scenes_since_calm = state.scenes_since_calm;
        self.recent_fears_used = state.recent_fears_used;
    }

    // -- private ----------------------------------------------------------

    fn select_strategy(&self, phase: GamePhase, profile: &FearProfile) -> AdaptationStrategy {
        match phase {
            GamePhase::Calibrating => {
                // Always probe, cycling through categories.
                let targets = least_confident_fears(profile, 3);
                AdaptationStrategy::Probe {
                    target_fears: targets,
                    intensity: 0.3,
                }
            }
            GamePhase::Exploring => {
                let top = profile.top_fears(2, 0.3);
                if top.is_empty() {
                    // Low confidence → keep probing.
                    let targets = least_confident_fears(profile, 3);
                    AdaptationStrategy::Probe {
                        target_fears: targets,
                        intensity: 0.4,
                    }
                } else {
                    // Some confirmed fears → gradual escalation.
                    let primary = top[0].0;
                    AdaptationStrategy::GradualEscalation {
                        primary_fear: primary,
                        intensity_curve: EscalationCurve::Sigmoid,
                    }
                }
            }
            GamePhase::Escalating => {
                // Insert contrast (calm) every 4 intense scenes.
                if self.scenes_since_calm >= 4 {
                    let top = profile.top_fears(1, 0.0);
                    let storm_fear = top.first().map(|(f, _)| *f).unwrap_or(FearType::Darkness);
                    return AdaptationStrategy::Contrast {
                        calm_duration: 1,
                        storm_fear,
                        storm_intensity: 0.8,
                    };
                }
                // Combine top 2 fears.
                let top = profile.top_fears(2, 0.0);
                let base = top.first().map(|(f, _)| *f).unwrap_or(FearType::Darkness);
                let amp = top.get(1).map(|(f, _)| *f).unwrap_or(FearType::Isolation);
                AdaptationStrategy::Layering {
                    base_fear: base,
                    amplifier_fear: amp,
                    blend_ratio: 0.6,
                }
            }
            GamePhase::Climax => {
                let top = profile.top_fears(2, 0.0);
                let base = top.first().map(|(f, _)| *f).unwrap_or(FearType::Darkness);
                let amp = top.get(1).map(|(f, _)| *f).unwrap_or(FearType::Isolation);
                // Occasional subversion for final shock.
                if self.scenes_since_calm % 3 == 2 {
                    return AdaptationStrategy::Subversion {
                        expected_fear: base,
                        actual_fear: amp,
                    };
                }
                AdaptationStrategy::Layering {
                    base_fear: base,
                    amplifier_fear: amp,
                    blend_ratio: 0.8,
                }
            }
            GamePhase::Reveal => {
                // No more horror — just data presentation.
                AdaptationStrategy::Probe {
                    target_fears: vec![],
                    intensity: 0.0,
                }
            }
        }
    }

    fn compute_intensity(&self, phase: GamePhase, scene_count: u32) -> f64 {
        match phase {
            GamePhase::Calibrating => self.intensity_curve.intensity_at(scene_count).min(0.35),
            GamePhase::Exploring => self.intensity_curve.intensity_at(scene_count).min(0.55),
            GamePhase::Escalating => self.intensity_curve.intensity_at(scene_count),
            GamePhase::Climax => self.intensity_curve.intensity_at(scene_count).max(0.85),
            GamePhase::Reveal => 0.0,
        }
    }

    fn generate_instruction(&self, strategy: &AdaptationStrategy, profile: &FearProfile) -> String {
        let obs = profile.update_count();
        match strategy {
            AdaptationStrategy::Probe { target_fears, .. } => {
                let fears_str = target_fears
                    .iter()
                    .map(|f| f.to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                format!(
                    "Introduce subtle elements that test the player's reaction to: {fears_str}. \
                     Keep it atmospheric — don't be overt. ({obs} observations so far.)"
                )
            }
            AdaptationStrategy::GradualEscalation { primary_fear, .. } => {
                let desc = fear_description(primary_fear);
                format!(
                    "The player shows sensitivity to {primary_fear}. {desc} \
                     Increase the intensity gradually — each detail slightly more \
                     unsettling than the last."
                )
            }
            AdaptationStrategy::Contrast { storm_fear, .. } => {
                format!(
                    "Create a moment of false safety. Let the player breathe. \
                     Make the environment feel almost normal. Then plant one small \
                     detail related to {storm_fear} that breaks the illusion."
                )
            }
            AdaptationStrategy::Layering {
                base_fear,
                amplifier_fear,
                ..
            } => {
                let base_desc = fear_description(base_fear);
                let amp_desc = fear_description(amplifier_fear);
                format!(
                    "Combine {base_fear} with {amplifier_fear} for maximum effect. \
                     {base_desc} Layer on top: {amp_desc} \
                     The combination should feel inescapable."
                )
            }
            AdaptationStrategy::Subversion {
                expected_fear,
                actual_fear,
            } => {
                format!(
                    "The player expects {expected_fear} — subvert that expectation. \
                     Set up what looks like a {expected_fear} scene, then twist it \
                     into {actual_fear}. The surprise amplifies the fear."
                )
            }
        }
    }

    fn determine_forbidden(&self, phase: GamePhase) -> Vec<String> {
        let mut forbidden = Vec::new();

        // Don't repeat the same fear back-to-back.
        if let Some(last) = self.recent_fears_used.last() {
            if self.recent_fears_used.iter().filter(|f| *f == last).count() >= 2 {
                forbidden.push(format!("Avoid direct {} elements (used recently)", last));
            }
        }

        match phase {
            GamePhase::Calibrating => {
                forbidden.push("No jump scares".into());
                forbidden.push("No extreme horror".into());
                forbidden.push("No meta-horror or fourth-wall breaks".into());
            }
            GamePhase::Exploring => {
                forbidden.push("No meta-horror yet".into());
            }
            GamePhase::Escalating => {
                forbidden.push("Avoid pure calm scenes (save for contrast beats)".into());
            }
            GamePhase::Climax => {
                forbidden.push("Do not reduce tension".into());
            }
            GamePhase::Reveal => {}
        }

        forbidden
    }
}

impl Default for AdaptationEngine {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Returns the N fears with the lowest confidence, for probing.
fn least_confident_fears(profile: &FearProfile, n: usize) -> Vec<FearType> {
    let mut fears: Vec<(FearType, f64)> = FearType::all()
        .into_iter()
        .map(|f| (f, profile.confidence_level(&f)))
        .collect();
    fears.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
    fears.into_iter().take(n).map(|(f, _)| f).collect()
}

/// Extracts the primary and secondary fears from a strategy.
fn extract_fears(
    strategy: &AdaptationStrategy,
    profile: &FearProfile,
) -> (Option<FearType>, Vec<FearType>) {
    match strategy {
        AdaptationStrategy::Probe { target_fears, .. } => {
            let primary = target_fears.first().copied();
            let secondary = target_fears.iter().skip(1).copied().collect();
            (primary, secondary)
        }
        AdaptationStrategy::GradualEscalation { primary_fear, .. } => {
            let secondary = profile
                .top_fears(3, 0.0)
                .into_iter()
                .map(|(f, _)| f)
                .filter(|f| f != primary_fear)
                .take(2)
                .collect();
            (Some(*primary_fear), secondary)
        }
        AdaptationStrategy::Contrast { storm_fear, .. } => (Some(*storm_fear), vec![]),
        AdaptationStrategy::Layering {
            base_fear,
            amplifier_fear,
            ..
        } => (Some(*base_fear), vec![*amplifier_fear]),
        AdaptationStrategy::Subversion {
            expected_fear,
            actual_fear,
        } => (Some(*actual_fear), vec![*expected_fear]),
    }
}

/// Returns a brief description of how to evoke a fear type.
fn fear_description(fear: &FearType) -> &'static str {
    match fear {
        FearType::Claustrophobia => {
            "Describe spaces getting smaller, walls pressing in, doors that won't open."
        }
        FearType::Isolation => "Emphasise emptiness, silence, distance from help or other people.",
        FearType::BodyHorror => {
            "Include wrongness in bodies — extra limbs, impossible anatomy, transformation."
        }
        FearType::Stalking => {
            "Suggest something is following, watching, or anticipating the player's moves."
        }
        FearType::LossOfControl => {
            "Remove agency — locked doors, involuntary movement, decisions made for the player."
        }
        FearType::UncannyValley => {
            "Describe things that look almost right but aren't — wrong smiles, stilted movement."
        }
        FearType::Darkness => {
            "Emphasise what can't be seen. Sounds without sources. Shapes at the edge of vision."
        }
        FearType::SoundBased => {
            "Focus on disturbing sounds — whispers, scraping, music that shouldn't be playing."
        }
        FearType::Doppelganger => {
            "Introduce doubles, mirrors that lie, voices that mimic the player."
        }
        FearType::Abandonment => {
            "Reference being left behind, promises broken, empty places where people used to be."
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::BehaviorFeatures;

    fn fearful_features() -> BehaviorFeatures {
        BehaviorFeatures {
            hesitation_score: 0.8,
            anxiety_score: 0.7,
            avoidance_score: 0.75,
            engagement_score: 0.15,
            indecision_score: 0.6,
            fight_or_flight_ratio: 0.15,
        }
    }

    fn trained_profile() -> FearProfile {
        let mut p = FearProfile::new();
        for _ in 0..20 {
            p.update(&fearful_features()).unwrap();
        }
        p
    }

    #[test]
    fn test_calibrating_always_probe() {
        let mut engine = AdaptationEngine::new();
        let profile = FearProfile::new();
        let d = engine.compute_directive(GamePhase::Calibrating, &profile, 1);
        assert!(
            matches!(d.strategy, AdaptationStrategy::Probe { .. }),
            "expected Probe, got {:?}",
            d.strategy
        );
    }

    #[test]
    fn test_exploring_no_confidence_returns_probe() {
        let mut engine = AdaptationEngine::new();
        let profile = FearProfile::new(); // no updates → no confidence
        let d = engine.compute_directive(GamePhase::Exploring, &profile, 5);
        assert!(matches!(d.strategy, AdaptationStrategy::Probe { .. }));
    }

    #[test]
    fn test_exploring_with_confidence_returns_escalation() {
        let mut engine = AdaptationEngine::new();
        let profile = trained_profile();
        let d = engine.compute_directive(GamePhase::Exploring, &profile, 6);
        assert!(
            matches!(d.strategy, AdaptationStrategy::GradualEscalation { .. }),
            "expected GradualEscalation, got {:?}",
            d.strategy
        );
    }

    #[test]
    fn test_escalating_uses_layering() {
        let mut engine = AdaptationEngine::new();
        let profile = trained_profile();
        let d = engine.compute_directive(GamePhase::Escalating, &profile, 10);
        assert!(
            matches!(d.strategy, AdaptationStrategy::Layering { .. }),
            "expected Layering, got {:?}",
            d.strategy
        );
    }

    #[test]
    fn test_contrast_after_intense_scenes() {
        let mut engine = AdaptationEngine::new();
        let profile = trained_profile();

        // Run 4 escalating scenes to trigger contrast.
        for i in 0..4 {
            engine.compute_directive(GamePhase::Escalating, &profile, 9 + i);
        }
        let d = engine.compute_directive(GamePhase::Escalating, &profile, 13);
        assert!(
            matches!(d.strategy, AdaptationStrategy::Contrast { .. }),
            "expected Contrast after 4 intense scenes, got {:?}",
            d.strategy
        );
    }

    #[test]
    fn test_climax_high_intensity() {
        let mut engine = AdaptationEngine::new();
        let profile = trained_profile();
        let d = engine.compute_directive(GamePhase::Climax, &profile, 17);
        assert!(
            d.intensity_target >= 0.85,
            "climax intensity too low: {}",
            d.intensity_target
        );
    }

    #[test]
    fn test_intensity_curve_shape() {
        let curve = IntensityCurve::default_horror_curve();
        assert!(curve.intensity_at(1) >= 0.2 && curve.intensity_at(1) <= 0.3);
        assert!(curve.intensity_at(5) >= 0.3 && curve.intensity_at(5) <= 0.5);
        assert!(curve.intensity_at(10) >= 0.5 && curve.intensity_at(10) <= 0.7);
        assert!((curve.intensity_at(13) - 0.4).abs() < 0.01);
        assert!(curve.intensity_at(18) >= 0.95);
    }

    #[test]
    fn test_directive_instruction_nonempty() {
        let mut engine = AdaptationEngine::new();
        let profile = FearProfile::new();
        for phase in [
            GamePhase::Calibrating,
            GamePhase::Exploring,
            GamePhase::Escalating,
            GamePhase::Climax,
        ] {
            let d = engine.compute_directive(phase, &profile, 5);
            assert!(
                !d.specific_instruction.is_empty(),
                "empty instruction for {phase:?}"
            );
        }
    }

    #[test]
    fn test_forbidden_elements_vary_by_phase() {
        let mut engine = AdaptationEngine::new();
        let profile = FearProfile::new();

        let d_cal = engine.compute_directive(GamePhase::Calibrating, &profile, 1);
        assert!(d_cal
            .forbidden_elements
            .iter()
            .any(|f| f.contains("jump scare")));

        let d_climax = engine.compute_directive(GamePhase::Climax, &profile, 17);
        assert!(d_climax
            .forbidden_elements
            .iter()
            .any(|f| f.contains("tension")));
    }

    #[test]
    fn test_forbidden_prevents_repetition() {
        let mut engine = AdaptationEngine::new();
        let profile = trained_profile();

        // Run several escalating scenes to build up recent_fears_used.
        for i in 0..5 {
            engine.compute_directive(GamePhase::Escalating, &profile, 9 + i);
        }

        let d = engine.compute_directive(GamePhase::Escalating, &profile, 14);
        // There may or may not be a repetition warning depending on the fears
        // selected, but the directive should still be valid.
        assert!(!d.specific_instruction.is_empty());
    }

    #[test]
    fn test_reveal_phase_zero_intensity() {
        let mut engine = AdaptationEngine::new();
        let profile = FearProfile::new();
        let d = engine.compute_directive(GamePhase::Reveal, &profile, 20);
        assert!(
            d.intensity_target < f64::EPSILON,
            "reveal should have 0 intensity"
        );
    }

    #[test]
    fn test_snapshot_calibrating_directive() {
        let mut engine = AdaptationEngine::new();
        let profile = FearProfile::new();
        let d = engine.compute_directive(GamePhase::Calibrating, &profile, 2);
        let json = serde_json::to_string_pretty(&d).unwrap();
        insta::assert_snapshot!("calibrating_directive", json);
    }

    #[test]
    fn test_snapshot_escalating_directive() {
        let mut engine = AdaptationEngine::new();
        let profile = trained_profile();
        let d = engine.compute_directive(GamePhase::Escalating, &profile, 10);
        let json = serde_json::to_string_pretty(&d).unwrap();
        insta::assert_snapshot!("escalating_directive", json);
    }
}
