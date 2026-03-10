//! The [`FearProfile`] — the living model of a player's fears.
//!
//! Combines the [`FearScorer`] with confidence tracking, meta-pattern
//! detection, snapshot history, and output formatting for the AI prompt
//! layer and the end-game reveal screen.

use std::collections::HashMap;
use std::time::Instant;

use fear_engine_common::types::{ChoiceApproach, FearType};
use fear_engine_common::Result;
use serde::{Deserialize, Serialize};

use crate::analyzer::{BehaviorBaseline, BehaviorFeatures};
use crate::scorer::FearScorer;

/// Threshold above which a score change is considered "significant".
const SIGNIFICANT_CHANGE: f64 = 0.05;

// ---------------------------------------------------------------------------
// Supporting types
// ---------------------------------------------------------------------------

/// Tracks how confident the engine is in a particular fear score.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FearConfidence {
    /// Number of behavior observations that influenced this score.
    pub observations: u32,
    /// Rolling variance of the score over the last several updates.
    pub recent_variance: f64,
    /// Monotonic instant of the last significant change (not serialised).
    #[serde(skip)]
    pub last_significant_change: Option<Instant>,
}

impl FearConfidence {
    fn new() -> Self {
        Self {
            observations: 0,
            recent_variance: 1.0,
            last_significant_change: None,
        }
    }

    /// Confidence level in `[0.0, 1.0]`.
    ///
    /// ```text
    /// confidence = 0.6 × min(observations/20, 1) + 0.4 × (1 − min(variance, 1))
    /// ```
    pub fn level(&self) -> f64 {
        let obs_factor = (self.observations as f64 / 20.0).min(1.0);
        let stability_factor = 1.0 - self.recent_variance.min(1.0);
        obs_factor * 0.6 + stability_factor * 0.4
    }
}

/// Meta-patterns that transcend individual fear categories.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaPatterns {
    /// How anxious the player generally is (0 = calm, 1 = very anxious).
    pub anxiety_threshold: f64,
    /// How quickly the player recovers after a scare (0 = slow, 1 = fast).
    pub recovery_speed: f64,
    /// Whether the player explores or avoids (0 = avoids, 1 = explores).
    pub curiosity_vs_avoidance: f64,
}

impl Default for MetaPatterns {
    fn default() -> Self {
        Self {
            anxiety_threshold: 0.5,
            recovery_speed: 0.5,
            curiosity_vs_avoidance: 0.5,
        }
    }
}

/// A point-in-time snapshot of the fear scores.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FearProfileSnapshot {
    pub scores: HashMap<FearType, f64>,
    #[serde(skip)]
    pub instant: Option<Instant>,
    pub trigger: String,
}

/// Result of a single profile update.
#[derive(Debug, Clone)]
pub struct ProfileUpdateResult {
    /// Fears whose score changed by more than [`SIGNIFICANT_CHANGE`].
    /// Each entry: `(fear, old_score, new_score)`.
    pub significant_changes: Vec<(FearType, f64, f64)>,
    /// Set if the primary fear changed as a result of this update.
    pub new_primary_fear: Option<FearType>,
    /// `true` when confidence is high enough to recommend a phase transition.
    pub phase_transition_recommended: bool,
}

/// Data prepared for the end-of-game reveal screen.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevealData {
    pub scores: Vec<(String, f64)>,
    pub primary_fear: Option<String>,
    pub secondary_fear: Option<String>,
    pub meta: MetaPatterns,
    pub total_observations: u32,
}

/// Persisted runtime state for reconnect/resume.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FearProfileState {
    pub scores: Vec<(FearType, f64)>,
    pub confidence: Vec<(FearType, FearConfidence)>,
    pub meta: MetaPatterns,
    pub baseline: Option<BehaviorBaseline>,
    pub update_count: u32,
    pub recent_scores: Vec<(FearType, Vec<f64>)>,
}

// ---------------------------------------------------------------------------
// FearProfile
// ---------------------------------------------------------------------------

/// The living model of a player's psychological fear profile.
///
/// # Example
///
/// ```
/// use fear_engine_fear_profile::profile::FearProfile;
/// use fear_engine_fear_profile::analyzer::BehaviorFeatures;
/// use fear_engine_common::types::FearType;
///
/// let mut profile = FearProfile::new();
/// assert!((profile.score(&FearType::Darkness) - 0.5).abs() < f64::EPSILON);
/// ```
pub struct FearProfile {
    scores: HashMap<FearType, f64>,
    confidence: HashMap<FearType, FearConfidence>,
    meta: MetaPatterns,
    baseline: Option<BehaviorBaseline>,
    scorer: FearScorer,
    update_count: u32,
    history: Vec<FearProfileSnapshot>,
    /// Rolling window of recent score values for variance calculation.
    recent_scores: HashMap<FearType, Vec<f64>>,
}

impl FearProfile {
    /// Creates a new profile with all fear scores at 0.5 and zero confidence.
    ///
    /// # Example
    ///
    /// ```
    /// use fear_engine_fear_profile::profile::FearProfile;
    /// use fear_engine_common::types::FearType;
    ///
    /// let p = FearProfile::new();
    /// assert_eq!(p.update_count(), 0);
    /// for f in FearType::all() {
    ///     assert!((p.score(&f) - 0.5).abs() < f64::EPSILON);
    /// }
    /// ```
    pub fn new() -> Self {
        let scores: HashMap<FearType, f64> = FearType::all().into_iter().map(|f| (f, 0.5)).collect();
        let confidence: HashMap<FearType, FearConfidence> =
            FearType::all().into_iter().map(|f| (f, FearConfidence::new())).collect();
        let recent_scores: HashMap<FearType, Vec<f64>> =
            FearType::all().into_iter().map(|f| (f, vec![0.5])).collect();

        Self {
            scores,
            confidence,
            meta: MetaPatterns::default(),
            baseline: None,
            scorer: FearScorer::new(),
            update_count: 0,
            history: Vec::new(),
            recent_scores,
        }
    }

    /// Sets the calibration baseline for feature extraction.
    pub fn set_baseline(&mut self, baseline: BehaviorBaseline) {
        self.baseline = Some(baseline);
    }

    /// Returns the calibration baseline if one has been established.
    pub fn baseline(&self) -> Option<&BehaviorBaseline> {
        self.baseline.as_ref()
    }

    /// Returns the current score for a fear type.
    pub fn score(&self, fear: &FearType) -> f64 {
        self.scores.get(fear).copied().unwrap_or(0.5)
    }

    /// Returns the full scores map.
    pub fn scores(&self) -> &HashMap<FearType, f64> {
        &self.scores
    }

    /// Number of updates applied so far.
    pub fn update_count(&self) -> u32 {
        self.update_count
    }

    /// Confidence level for a specific fear (0.0–1.0).
    ///
    /// # Example
    ///
    /// ```
    /// use fear_engine_fear_profile::profile::FearProfile;
    /// use fear_engine_common::types::FearType;
    ///
    /// let p = FearProfile::new();
    /// assert!(p.confidence_level(&FearType::Darkness) < 0.5);
    /// ```
    pub fn confidence_level(&self, fear: &FearType) -> f64 {
        self.confidence
            .get(fear)
            .map(|c| c.level())
            .unwrap_or(0.0)
    }

    /// Overall confidence (mean across all fears).
    pub fn overall_confidence(&self) -> f64 {
        let sum: f64 = FearType::all().iter().map(|f| self.confidence_level(f)).sum();
        sum / FearType::all().len() as f64
    }

    /// Returns the current meta-pattern summary.
    pub fn meta_patterns(&self) -> &MetaPatterns {
        &self.meta
    }

    /// Updates the profile with new behaviour features.
    ///
    /// # Example
    ///
    /// ```
    /// use fear_engine_fear_profile::profile::FearProfile;
    /// use fear_engine_fear_profile::analyzer::BehaviorFeatures;
    ///
    /// let mut p = FearProfile::new();
    /// let features = BehaviorFeatures {
    ///     hesitation_score: 0.8, anxiety_score: 0.7, avoidance_score: 0.6,
    ///     engagement_score: 0.2, indecision_score: 0.5, fight_or_flight_ratio: 0.2,
    /// };
    /// let result = p.update(&features).unwrap();
    /// assert!(p.update_count() == 1);
    /// ```
    pub fn update(&mut self, features: &BehaviorFeatures) -> Result<ProfileUpdateResult> {
        let old_scores = self.scores.clone();
        let old_primary = self.primary_fear().map(|(f, _)| f);

        // Bayesian update.
        let new_scores = self.scorer.update_scores(&self.scores, features)?;
        self.scores = new_scores;
        self.update_count += 1;

        // Update confidence and variance tracking.
        for fear in FearType::all() {
            let old = old_scores.get(&fear).copied().unwrap_or(0.5);
            let new = self.score(&fear);
            let delta = (new - old).abs();

            if let Some(conf) = self.confidence.get_mut(&fear) {
                conf.observations += 1;

                // Track recent scores for variance.
                let window = self.recent_scores.entry(fear).or_default();
                window.push(new);
                if window.len() > 10 {
                    window.remove(0);
                }
                conf.recent_variance = variance(window);

                if delta > SIGNIFICANT_CHANGE {
                    conf.last_significant_change = Some(Instant::now());
                }
            }
        }

        // Update meta-patterns.
        self.meta.anxiety_threshold =
            0.7 * self.meta.anxiety_threshold + 0.3 * features.anxiety_score;
        self.meta.curiosity_vs_avoidance =
            0.7 * self.meta.curiosity_vs_avoidance + 0.3 * features.engagement_score;
        self.meta.recovery_speed =
            0.7 * self.meta.recovery_speed + 0.3 * (1.0 - features.hesitation_score);

        // Identify significant changes.
        let significant_changes: Vec<(FearType, f64, f64)> = FearType::all()
            .into_iter()
            .filter_map(|f| {
                let old = old_scores.get(&f).copied().unwrap_or(0.5);
                let new = self.score(&f);
                if (new - old).abs() > SIGNIFICANT_CHANGE {
                    Some((f, old, new))
                } else {
                    None
                }
            })
            .collect();

        // Snapshot on significant change.
        if !significant_changes.is_empty() {
            self.history.push(FearProfileSnapshot {
                scores: self.scores.clone(),
                instant: Some(Instant::now()),
                trigger: format!("update #{}", self.update_count),
            });
        }

        let new_primary = self.primary_fear().map(|(f, _)| f);
        let new_primary_fear = if new_primary != old_primary {
            new_primary
        } else {
            None
        };

        let phase_transition_recommended = self.overall_confidence() > 0.4;

        Ok(ProfileUpdateResult {
            significant_changes,
            new_primary_fear,
            phase_transition_recommended,
        })
    }

    /// Applies a direct signal from an explicit player choice.
    ///
    /// This prevents all fear axes from collapsing together by treating the
    /// chosen fear vector as strong evidence about what kind of horror the
    /// player is engaging with or avoiding.
    pub fn apply_choice_signal(
        &mut self,
        fear: FearType,
        approach: ChoiceApproach,
        time_to_decide_ms: u64,
    ) {
        let old_primary = self.primary_fear().map(|(current, _)| current);
        let baseline_choice_time = self
            .baseline
            .as_ref()
            .map(|baseline| baseline.avg_choice_time_ms)
            .unwrap_or(4000.0);
        let hesitation_bonus = ((time_to_decide_ms as f64 / baseline_choice_time) - 1.0)
            .clamp(0.0, 1.0)
            * 0.04;
        let approach_weight = match approach {
            ChoiceApproach::Investigate => 0.10,
            ChoiceApproach::Interact => 0.09,
            ChoiceApproach::Wait => 0.08,
            ChoiceApproach::Avoid => 0.11,
            ChoiceApproach::Flee => 0.12,
            ChoiceApproach::Confront => 0.07,
        };
        let boost = approach_weight + hesitation_bonus;

        for candidate in FearType::all() {
            let current = self.score(&candidate);
            let updated = if candidate == fear {
                (current + boost).clamp(0.05, 0.95)
            } else {
                (current - boost * 0.12).clamp(0.05, 0.95)
            };
            self.scores.insert(candidate, updated);
        }

        self.update_count += 1;

        for candidate in FearType::all() {
            let score = self.score(&candidate);
            if let Some(conf) = self.confidence.get_mut(&candidate) {
                conf.observations += if candidate == fear { 2 } else { 1 };
                let window = self.recent_scores.entry(candidate).or_default();
                window.push(score);
                if window.len() > 10 {
                    window.remove(0);
                }
                conf.recent_variance = variance(window);
                if candidate == fear {
                    conf.last_significant_change = Some(Instant::now());
                }
            }
        }

        self.meta.curiosity_vs_avoidance = 0.8 * self.meta.curiosity_vs_avoidance
            + 0.2
                * match approach {
                    ChoiceApproach::Investigate | ChoiceApproach::Interact => 1.0,
                    ChoiceApproach::Confront => 0.7,
                    ChoiceApproach::Wait => 0.5,
                    ChoiceApproach::Avoid | ChoiceApproach::Flee => 0.1,
                };

        if old_primary != self.primary_fear().map(|(current, _)| current) {
            self.history.push(FearProfileSnapshot {
                scores: self.scores.clone(),
                instant: Some(Instant::now()),
                trigger: format!("choice signal: {}", fear),
            });
        }
    }

    /// Returns the top `n` fears sorted by score, filtered by minimum confidence.
    ///
    /// # Example
    ///
    /// ```
    /// use fear_engine_fear_profile::profile::FearProfile;
    ///
    /// let p = FearProfile::new();
    /// let top = p.top_fears(3, 0.0);
    /// assert!(top.len() <= 3);
    /// ```
    pub fn top_fears(&self, n: usize, min_confidence: f64) -> Vec<(FearType, f64)> {
        let mut ranked: Vec<(FearType, f64)> = FearType::all()
            .into_iter()
            .filter(|f| self.confidence_level(f) >= min_confidence)
            .map(|f| (f, self.score(&f)))
            .collect();
        ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        ranked.truncate(n);
        ranked
    }

    /// Returns the highest-scoring fear with confidence > 0.5.
    ///
    /// # Example
    ///
    /// ```
    /// use fear_engine_fear_profile::profile::FearProfile;
    ///
    /// let p = FearProfile::new();
    /// // No fear has confidence > 0.5 yet.
    /// assert!(p.primary_fear().is_none());
    /// ```
    pub fn primary_fear(&self) -> Option<(FearType, f64)> {
        self.top_fears(1, 0.5).into_iter().next()
    }

    /// Creates an independent snapshot of the current state.
    pub fn snapshot(&self) -> FearProfileSnapshot {
        FearProfileSnapshot {
            scores: self.scores.clone(),
            instant: Some(Instant::now()),
            trigger: "manual".into(),
        }
    }

    /// Returns the snapshot history.
    pub fn history(&self) -> &[FearProfileSnapshot] {
        &self.history
    }

    /// Formats the profile for use in the AI prompt's fear-context layer.
    ///
    /// # Example
    ///
    /// ```
    /// use fear_engine_fear_profile::profile::FearProfile;
    ///
    /// let p = FearProfile::new();
    /// let ctx = p.to_prompt_context();
    /// assert!(ctx.contains("PLAYER PSYCHOLOGICAL PROFILE"));
    /// ```
    pub fn to_prompt_context(&self) -> String {
        let top = self.top_fears(3, 0.0);
        let primary_line = match top.first() {
            Some((f, s)) => format!(
                "Primary fear axis: {} (score: {:.0}%, confidence: {:.0}%)",
                f,
                s * 100.0,
                self.confidence_level(f) * 100.0
            ),
            None => "Primary fear axis: undetermined".into(),
        };
        let secondary_lines: String = top
            .iter()
            .skip(1)
            .map(|(f, s)| format!("  {} ({:.0}%)", f, s * 100.0))
            .collect::<Vec<_>>()
            .join("\n");

        format!(
            "PLAYER PSYCHOLOGICAL PROFILE:\n\
             - {primary_line}\n\
             - Secondary fears:\n{secondary_lines}\n\
             - Anxiety baseline: {:.2}/1.0\n\
             - Recovery speed: {:.2}/1.0\n\
             - Curiosity vs avoidance: {:.2}/1.0\n\
             - Total observations: {}",
            self.meta.anxiety_threshold,
            self.meta.recovery_speed,
            self.meta.curiosity_vs_avoidance,
            self.update_count,
        )
    }

    /// Produces data for the end-of-game fear reveal screen.
    ///
    /// # Example
    ///
    /// ```
    /// use fear_engine_fear_profile::profile::FearProfile;
    ///
    /// let p = FearProfile::new();
    /// let reveal = p.to_reveal_data();
    /// assert_eq!(reveal.scores.len(), 10);
    /// ```
    pub fn to_reveal_data(&self) -> RevealData {
        let mut scores: Vec<(String, f64)> = FearType::all()
            .into_iter()
            .map(|f| (f.to_string(), self.score(&f)))
            .collect();
        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let primary = self.primary_fear().map(|(f, _)| f.to_string());
        let secondary = self
            .top_fears(2, 0.5)
            .into_iter()
            .nth(1)
            .map(|(f, _)| f.to_string());

        RevealData {
            scores,
            primary_fear: primary,
            secondary_fear: secondary,
            meta: self.meta.clone(),
            total_observations: self.update_count,
        }
    }

    /// Resets the profile to its initial state.
    pub fn reset(&mut self) {
        *self = Self::new();
    }

    /// Returns a persisted snapshot of the fear profile.
    pub fn to_persisted_state(&self) -> FearProfileState {
        FearProfileState {
            scores: FearType::all()
                .into_iter()
                .map(|fear| (fear, self.score(&fear)))
                .collect(),
            confidence: FearType::all()
                .into_iter()
                .map(|fear| {
                    (
                        fear,
                        self.confidence
                            .get(&fear)
                            .cloned()
                            .unwrap_or_else(FearConfidence::new),
                    )
                })
                .collect(),
            meta: self.meta.clone(),
            baseline: self.baseline.clone(),
            update_count: self.update_count,
            recent_scores: FearType::all()
                .into_iter()
                .map(|fear| {
                    (
                        fear,
                        self.recent_scores
                            .get(&fear)
                            .cloned()
                            .unwrap_or_else(|| vec![self.score(&fear)]),
                    )
                })
                .collect(),
        }
    }

    /// Restores a fear profile from persisted session state.
    pub fn from_persisted_state(state: FearProfileState) -> Self {
        let default_scores: HashMap<FearType, f64> =
            FearType::all().into_iter().map(|fear| (fear, 0.5)).collect();
        let default_confidence: HashMap<FearType, FearConfidence> = FearType::all()
            .into_iter()
            .map(|fear| (fear, FearConfidence::new()))
            .collect();
        let mut scores = default_scores;
        for (fear, score) in state.scores {
            scores.insert(fear, score);
        }

        let mut confidence = default_confidence;
        for (fear, value) in state.confidence {
            confidence.insert(fear, value);
        }

        let mut recent_scores: HashMap<FearType, Vec<f64>> = FearType::all()
            .into_iter()
            .map(|fear| {
                let score = scores.get(&fear).copied().unwrap_or(0.5);
                (fear, vec![score])
            })
            .collect();
        for (fear, window) in state.recent_scores {
            recent_scores.insert(fear, window);
        }

        Self {
            scores,
            confidence,
            meta: state.meta,
            baseline: state.baseline,
            scorer: FearScorer::new(),
            update_count: state.update_count,
            history: Vec::new(),
            recent_scores,
        }
    }
}

impl Default for FearProfile {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn variance(values: &[f64]) -> f64 {
    if values.len() < 2 {
        return 1.0;
    }
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    let var = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / values.len() as f64;
    var
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

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

    fn calm_features() -> BehaviorFeatures {
        BehaviorFeatures {
            hesitation_score: 0.1,
            anxiety_score: 0.05,
            avoidance_score: 0.0,
            engagement_score: 0.9,
            indecision_score: 0.05,
            fight_or_flight_ratio: 0.9,
        }
    }

    #[test]
    fn test_new_profile_has_half_scores() {
        let p = FearProfile::new();
        for f in FearType::all() {
            assert!((p.score(&f) - 0.5).abs() < f64::EPSILON, "{f}");
        }
    }

    #[test]
    fn test_update_changes_scores() {
        let mut p = FearProfile::new();
        let before = p.score(&FearType::Darkness);
        p.update(&fearful_features()).unwrap();
        let after = p.score(&FearType::Darkness);
        assert!((after - before).abs() > 0.001, "score didn't change");
    }

    #[test]
    fn test_multiple_updates_cause_convergence() {
        let mut p = FearProfile::new();
        for _ in 0..20 {
            p.update(&fearful_features()).unwrap();
        }
        let final_score = p.score(&FearType::Darkness);
        // After many fearful updates, darkness should have risen.
        assert!(final_score > 0.55, "got {final_score}");
    }

    #[test]
    fn test_top_fears_returns_correct_ordering() {
        let mut p = FearProfile::new();
        for _ in 0..10 {
            p.update(&fearful_features()).unwrap();
        }
        let top = p.top_fears(3, 0.0);
        assert!(top.len() <= 3);
        // Ordered descending.
        for w in top.windows(2) {
            assert!(w[0].1 >= w[1].1);
        }
    }

    #[test]
    fn test_top_fears_filters_by_confidence() {
        let p = FearProfile::new();
        // No observations → very low confidence.
        let top = p.top_fears(3, 0.8);
        assert!(top.is_empty());
    }

    #[test]
    fn test_primary_fear_none_when_no_confidence() {
        let p = FearProfile::new();
        assert!(p.primary_fear().is_none());
    }

    #[test]
    fn test_primary_fear_after_many_updates() {
        let mut p = FearProfile::new();
        for _ in 0..25 {
            p.update(&fearful_features()).unwrap();
        }
        // After 25 observations, confidence should be high enough.
        let primary = p.primary_fear();
        assert!(primary.is_some(), "expected a primary fear after 25 updates");
    }

    #[test]
    fn test_snapshot_independent_copy() {
        let mut p = FearProfile::new();
        let snap = p.snapshot();
        p.update(&fearful_features()).unwrap();
        // The snapshot should not reflect the update.
        let snap_dark = snap.scores.get(&FearType::Darkness).copied().unwrap_or(0.0);
        let current_dark = p.score(&FearType::Darkness);
        assert!((snap_dark - current_dark).abs() > 0.001);
    }

    #[test]
    fn test_to_prompt_context_valid_string() {
        let p = FearProfile::new();
        let ctx = p.to_prompt_context();
        assert!(ctx.contains("PLAYER PSYCHOLOGICAL PROFILE"));
        assert!(ctx.contains("Anxiety baseline"));
        assert!(ctx.contains("Recovery speed"));
    }

    #[test]
    fn test_to_reveal_data() {
        let p = FearProfile::new();
        let reveal = p.to_reveal_data();
        assert_eq!(reveal.scores.len(), 10);
        assert_eq!(reveal.total_observations, 0);
    }

    #[test]
    fn test_confidence_increases_with_observations() {
        let mut p = FearProfile::new();
        let c0 = p.confidence_level(&FearType::Darkness);
        for _ in 0..15 {
            p.update(&fearful_features()).unwrap();
        }
        let c15 = p.confidence_level(&FearType::Darkness);
        assert!(c15 > c0, "confidence didn't increase: {c0} → {c15}");
    }

    #[test]
    fn test_reset_returns_to_initial() {
        let mut p = FearProfile::new();
        for _ in 0..5 {
            p.update(&fearful_features()).unwrap();
        }
        p.reset();
        assert_eq!(p.update_count(), 0);
        assert!((p.score(&FearType::Darkness) - 0.5).abs() < f64::EPSILON);
        assert!(p.history().is_empty());
    }

    #[test]
    fn test_update_result_identifies_significant_changes() {
        let mut p = FearProfile::new();
        let result = p.update(&fearful_features()).unwrap();
        // At least some fears should change significantly on first update.
        assert!(
            !result.significant_changes.is_empty(),
            "expected significant changes"
        );
    }

    #[test]
    fn test_meta_patterns_update() {
        let mut p = FearProfile::new();
        for _ in 0..5 {
            p.update(&fearful_features()).unwrap();
        }
        // Fearful features → anxiety should rise, curiosity should drop.
        assert!(p.meta.anxiety_threshold > 0.5, "anxiety: {}", p.meta.anxiety_threshold);
        assert!(
            p.meta.curiosity_vs_avoidance < 0.5,
            "curiosity: {}",
            p.meta.curiosity_vs_avoidance
        );
    }

    #[test]
    fn test_calm_features_lower_scores() {
        let mut p = FearProfile::new();
        for _ in 0..10 {
            p.update(&calm_features()).unwrap();
        }
        for f in FearType::all() {
            assert!(
                p.score(&f) < 0.5,
                "{f} should have decreased, got {}",
                p.score(&f)
            );
        }
    }

    #[test]
    fn test_serialize_deserialize_snapshot() {
        let p = FearProfile::new();
        let snap = p.snapshot();
        let json = serde_json::to_string(&snap).unwrap();
        let back: FearProfileSnapshot = serde_json::from_str(&json).unwrap();
        assert_eq!(back.scores.len(), 10);
        assert_eq!(back.trigger, "manual");
    }

    #[test]
    fn test_history_grows_on_significant_changes() {
        let mut p = FearProfile::new();
        assert!(p.history().is_empty());
        p.update(&fearful_features()).unwrap();
        // First update from 0.5 with strong features should produce significant changes.
        assert!(!p.history().is_empty());
    }

    #[test]
    fn test_set_baseline() {
        let mut p = FearProfile::new();
        let bl = BehaviorBaseline {
            avg_typing_speed: 8.0,
            avg_response_time_ms: 2000.0,
            avg_choice_time_ms: 3000.0,
            avg_mouse_velocity: 250.0,
        };
        p.set_baseline(bl);
        assert!(p.baseline.is_some());
    }

    #[test]
    fn test_overall_confidence() {
        let mut p = FearProfile::new();
        let c0 = p.overall_confidence();
        for _ in 0..10 {
            p.update(&fearful_features()).unwrap();
        }
        let c10 = p.overall_confidence();
        assert!(c10 > c0);
    }

    #[test]
    fn test_apply_choice_signal_creates_score_separation() {
        let mut p = FearProfile::new();
        p.apply_choice_signal(
            FearType::Claustrophobia,
            ChoiceApproach::Flee,
            6_000,
        );

        assert!(p.score(&FearType::Claustrophobia) > 0.5);
        assert!(p.score(&FearType::Claustrophobia) > p.score(&FearType::Isolation));
        assert!(p.update_count() > 0);
    }
}
