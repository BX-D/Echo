//! Bayesian fear scoring — the mathematical core of the Fear Engine.
//!
//! Uses the likelihood matrix from `ARCHITECTURE.md § 3.2` to compute
//! `P(fear | behavior)` via Bayes' theorem, then applies exponential
//! moving-average smoothing to prevent wild swings.

use std::collections::HashMap;

use fear_engine_common::types::FearType;
use fear_engine_common::{FearEngineError, Result};
use serde::{Deserialize, Serialize};

use crate::analyzer::BehaviorFeatures;

// ---------------------------------------------------------------------------
// Feature likelihoods (one row of the weight matrix)
// ---------------------------------------------------------------------------

/// Per-fear weights that map each behavior feature to a likelihood
/// contribution. Negative values (e.g. `engagement`) mean the feature
/// *reduces* the probability of that fear.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureLikelihoods {
    pub hesitation: f64,
    pub anxiety: f64,
    pub avoidance: f64,
    pub engagement: f64,
    pub indecision: f64,
    pub flight_bias: f64,
}

// ---------------------------------------------------------------------------
// Fear scorer
// ---------------------------------------------------------------------------

/// Performs Bayesian updates of fear scores using the likelihood matrix.
///
/// # Example
///
/// ```
/// use std::collections::HashMap;
/// use fear_engine_common::types::FearType;
/// use fear_engine_fear_profile::analyzer::BehaviorFeatures;
/// use fear_engine_fear_profile::scorer::FearScorer;
///
/// let scorer = FearScorer::new();
/// let priors: HashMap<FearType, f64> = FearType::all().into_iter().map(|f| (f, 0.5)).collect();
/// let features = BehaviorFeatures {
///     hesitation_score: 0.8, anxiety_score: 0.6, avoidance_score: 0.7,
///     engagement_score: 0.1, indecision_score: 0.5, fight_or_flight_ratio: 0.2,
/// };
/// let posteriors = scorer.update_scores(&priors, &features).unwrap();
/// assert!(posteriors[&FearType::Claustrophobia] > 0.5);
/// ```
pub struct FearScorer {
    likelihood_matrix: HashMap<FearType, FeatureLikelihoods>,
    smoothing_factor: f64,
}

impl FearScorer {
    /// Creates a scorer initialised with the weight matrix from
    /// `ARCHITECTURE.md § 3.2`.
    ///
    /// The EMA smoothing factor (`alpha`) defaults to `0.3`.
    ///
    /// # Example
    ///
    /// ```
    /// use fear_engine_fear_profile::scorer::FearScorer;
    /// let scorer = FearScorer::new();
    /// ```
    pub fn new() -> Self {
        Self::with_alpha(0.3)
    }

    /// Creates a scorer with a custom EMA alpha.
    ///
    /// # Example
    ///
    /// ```
    /// use fear_engine_fear_profile::scorer::FearScorer;
    /// let scorer = FearScorer::with_alpha(0.5);
    /// ```
    pub fn with_alpha(alpha: f64) -> Self {
        Self {
            likelihood_matrix: build_likelihood_matrix(),
            smoothing_factor: alpha,
        }
    }

    /// Computes `P(features | fear_type)` — the likelihood of observing the
    /// given behaviour features if this fear category is active.
    ///
    /// The likelihood is the dot-product of the feature vector and the
    /// per-fear weight row, clamped to `[0.01, 1.0]` (never zero, so
    /// Bayesian division is safe).
    ///
    /// # Example
    ///
    /// ```
    /// use fear_engine_common::types::FearType;
    /// use fear_engine_fear_profile::analyzer::BehaviorFeatures;
    /// use fear_engine_fear_profile::scorer::FearScorer;
    ///
    /// let scorer = FearScorer::new();
    /// let features = BehaviorFeatures {
    ///     hesitation_score: 0.9, anxiety_score: 0.8, avoidance_score: 0.7,
    ///     engagement_score: 0.1, indecision_score: 0.6, fight_or_flight_ratio: 0.1,
    /// };
    /// let l = scorer.likelihood(&FearType::Claustrophobia, &features);
    /// assert!(l > 0.0 && l <= 1.0);
    /// ```
    pub fn likelihood(&self, fear: &FearType, features: &BehaviorFeatures) -> f64 {
        let weights = match self.likelihood_matrix.get(fear) {
            Some(w) => w,
            None => return 0.5,
        };

        // flight_bias uses (1 - fight_or_flight_ratio) because low fight = high flight = fear
        let raw = weights.hesitation * features.hesitation_score
            + weights.anxiety * features.anxiety_score
            + weights.avoidance * features.avoidance_score
            + weights.engagement * features.engagement_score
            + weights.indecision * features.indecision_score
            + weights.flight_bias * (1.0 - features.fight_or_flight_ratio);

        // Map the weighted sum through a sigmoid centred at 0.5 so that
        // differences between fears are preserved.  The raw range for
        // maximal input is roughly [-0.3, 2.1] across fears.  We want:
        //   raw ≈ 0   → likelihood ≈ 0.3  (neutral-ish)
        //   raw ≈ 1.5 → likelihood ≈ 0.9
        //   raw < 0   → likelihood ≈ 0.1
        let k = 3.0; // steepness
        let midpoint = 0.7; // centre of sigmoid
        let sigmoid = 1.0 / (1.0 + (-k * (raw - midpoint)).exp());

        sigmoid.clamp(0.05, 0.95)
    }

    /// Computes the marginal evidence `P(features)` — the weighted sum of
    /// likelihoods across all fears.
    ///
    /// ```text
    /// P(features) = Σ P(features | fear_i) × P(fear_i)
    /// ```
    ///
    /// # Example
    ///
    /// ```
    /// use std::collections::HashMap;
    /// use fear_engine_common::types::FearType;
    /// use fear_engine_fear_profile::analyzer::BehaviorFeatures;
    /// use fear_engine_fear_profile::scorer::FearScorer;
    ///
    /// let scorer = FearScorer::new();
    /// let priors: HashMap<FearType, f64> = FearType::all().into_iter().map(|f| (f, 0.5)).collect();
    /// let features = BehaviorFeatures {
    ///     hesitation_score: 0.5, anxiety_score: 0.5, avoidance_score: 0.5,
    ///     engagement_score: 0.5, indecision_score: 0.5, fight_or_flight_ratio: 0.5,
    /// };
    /// let ev = scorer.evidence(&features, &priors);
    /// assert!(ev > 0.0);
    /// ```
    pub fn evidence(
        &self,
        features: &BehaviorFeatures,
        priors: &HashMap<FearType, f64>,
    ) -> f64 {
        let mut total = 0.0;
        for fear in FearType::all() {
            let prior = priors.get(&fear).copied().unwrap_or(0.5);
            total += self.likelihood(&fear, features) * prior;
        }
        total
    }

    /// Performs a full Bayesian update for every fear category.
    ///
    /// Each fear's score is updated independently (not renormalised across
    /// fears — a player can be afraid of multiple things at once). The
    /// update uses the likelihood as a directional signal:
    ///
    /// ```text
    /// direction  = likelihood - 0.5          (positive = evidence for fear)
    /// adjustment = alpha × direction
    /// new_score  = prior + adjustment
    /// ```
    ///
    /// This keeps scores in `[0, 1]` and allows multiple fears to rise
    /// simultaneously when the behaviour is generally fearful.
    ///
    /// # Errors
    ///
    /// Returns [`FearEngineError::InvalidInput`] if the computed evidence
    /// is zero (should not happen with the floor on likelihood).
    pub fn update_scores(
        &self,
        priors: &HashMap<FearType, f64>,
        features: &BehaviorFeatures,
    ) -> Result<HashMap<FearType, f64>> {
        let ev = self.evidence(features, priors);
        if ev <= 0.0 {
            return Err(FearEngineError::InvalidInput {
                field: "evidence".into(),
                reason: "evidence is zero — cannot divide".into(),
            });
        }

        let alpha = self.smoothing_factor;
        let mut posteriors = HashMap::new();

        for fear in FearType::all() {
            let prior = priors.get(&fear).copied().unwrap_or(0.5);
            let lh = self.likelihood(&fear, features);
            // Direction: positive when likelihood > 0.5 (fear signal),
            // negative when < 0.5 (calm signal).
            let direction = lh - 0.5;
            let new_score = prior + alpha * direction;
            posteriors.insert(fear, new_score.clamp(0.05, 0.95));
        }

        Ok(posteriors)
    }
}

impl Default for FearScorer {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Weight matrix (from ARCHITECTURE.md § 3.2)
// ---------------------------------------------------------------------------

fn build_likelihood_matrix() -> HashMap<FearType, FeatureLikelihoods> {
    let mut m = HashMap::new();
    // Columns: hesitation, anxiety, avoidance, engagement, indecision, flight_bias
    m.insert(FearType::Claustrophobia, FeatureLikelihoods {
        hesitation: 0.3, anxiety: 0.4, avoidance: 0.5, engagement: -0.2, indecision: 0.3, flight_bias: 0.4,
    });
    m.insert(FearType::Isolation, FeatureLikelihoods {
        hesitation: 0.2, anxiety: 0.3, avoidance: 0.2, engagement: -0.1, indecision: 0.2, flight_bias: 0.3,
    });
    m.insert(FearType::BodyHorror, FeatureLikelihoods {
        hesitation: 0.4, anxiety: 0.5, avoidance: 0.3, engagement: -0.3, indecision: 0.2, flight_bias: 0.2,
    });
    m.insert(FearType::Stalking, FeatureLikelihoods {
        hesitation: 0.3, anxiety: 0.5, avoidance: 0.4, engagement: -0.2, indecision: 0.2, flight_bias: 0.3,
    });
    m.insert(FearType::LossOfControl, FeatureLikelihoods {
        hesitation: 0.3, anxiety: 0.4, avoidance: 0.3, engagement: -0.2, indecision: 0.5, flight_bias: 0.4,
    });
    m.insert(FearType::UncannyValley, FeatureLikelihoods {
        hesitation: 0.5, anxiety: 0.3, avoidance: 0.4, engagement: -0.1, indecision: 0.3, flight_bias: 0.2,
    });
    m.insert(FearType::Darkness, FeatureLikelihoods {
        hesitation: 0.3, anxiety: 0.4, avoidance: 0.5, engagement: -0.2, indecision: 0.2, flight_bias: 0.4,
    });
    m.insert(FearType::SoundBased, FeatureLikelihoods {
        hesitation: 0.2, anxiety: 0.5, avoidance: 0.3, engagement: -0.1, indecision: 0.2, flight_bias: 0.3,
    });
    m.insert(FearType::Doppelganger, FeatureLikelihoods {
        hesitation: 0.5, anxiety: 0.3, avoidance: 0.4, engagement: -0.1, indecision: 0.3, flight_bias: 0.2,
    });
    m.insert(FearType::Abandonment, FeatureLikelihoods {
        hesitation: 0.2, anxiety: 0.3, avoidance: 0.2, engagement: -0.1, indecision: 0.4, flight_bias: 0.3,
    });
    m
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn uniform_priors() -> HashMap<FearType, f64> {
        FearType::all().into_iter().map(|f| (f, 0.5)).collect()
    }

    fn high_fear_features() -> BehaviorFeatures {
        BehaviorFeatures {
            hesitation_score: 0.9,
            anxiety_score: 0.8,
            avoidance_score: 0.85,
            engagement_score: 0.1,
            indecision_score: 0.7,
            fight_or_flight_ratio: 0.1,
        }
    }

    fn low_fear_features() -> BehaviorFeatures {
        BehaviorFeatures {
            hesitation_score: 0.1,
            anxiety_score: 0.1,
            avoidance_score: 0.05,
            engagement_score: 0.9,
            indecision_score: 0.05,
            fight_or_flight_ratio: 0.9,
        }
    }

    // -- Likelihood tests -------------------------------------------------

    #[test]
    fn test_high_likelihood_for_fearful_features() {
        let scorer = FearScorer::new();
        let l = scorer.likelihood(&FearType::Claustrophobia, &high_fear_features());
        assert!(l > 0.5, "got {l}");
    }

    #[test]
    fn test_low_likelihood_for_calm_features() {
        let scorer = FearScorer::new();
        let l = scorer.likelihood(&FearType::Claustrophobia, &low_fear_features());
        assert!(l < 0.6, "got {l}");
    }

    #[test]
    fn test_likelihood_always_positive() {
        let scorer = FearScorer::new();
        for fear in FearType::all() {
            let l = scorer.likelihood(&fear, &high_fear_features());
            assert!(l >= 0.01, "fear {fear}: likelihood = {l}");
        }
    }

    // -- Evidence tests ---------------------------------------------------

    #[test]
    fn test_evidence_is_positive() {
        let scorer = FearScorer::new();
        let ev = scorer.evidence(&high_fear_features(), &uniform_priors());
        assert!(ev > 0.0, "evidence = {ev}");
    }

    // -- Bayesian update tests -------------------------------------------

    #[test]
    fn test_high_likelihood_high_prior_increases_posterior() {
        let scorer = FearScorer::new();
        let mut priors = uniform_priors();
        priors.insert(FearType::Darkness, 0.7);

        let features = BehaviorFeatures {
            hesitation_score: 0.8,
            anxiety_score: 0.7,
            avoidance_score: 0.9,
            engagement_score: 0.1,
            indecision_score: 0.3,
            fight_or_flight_ratio: 0.1,
        };

        let posteriors = scorer.update_scores(&priors, &features).unwrap();
        assert!(
            posteriors[&FearType::Darkness] > 0.5,
            "darkness posterior = {}",
            posteriors[&FearType::Darkness]
        );
    }

    #[test]
    fn test_low_likelihood_high_prior_decreases_posterior() {
        let scorer = FearScorer::new();
        let mut priors = uniform_priors();
        priors.insert(FearType::Claustrophobia, 0.8);

        let posteriors = scorer.update_scores(&priors, &low_fear_features()).unwrap();
        // Smoothed, so it won't drop below prior dramatically, but should decrease.
        assert!(
            posteriors[&FearType::Claustrophobia] <= 0.8,
            "got {}",
            posteriors[&FearType::Claustrophobia]
        );
    }

    #[test]
    fn test_scores_always_in_unit_range() {
        let scorer = FearScorer::new();
        let posteriors = scorer
            .update_scores(&uniform_priors(), &high_fear_features())
            .unwrap();
        for (fear, score) in &posteriors {
            assert!(
                *score >= 0.0 && *score <= 1.0,
                "fear {fear}: score {score} out of range"
            );
        }
    }

    #[test]
    fn test_ema_smoothing_prevents_wild_swings() {
        let scorer = FearScorer::with_alpha(0.3);
        let priors = uniform_priors();

        let extreme = BehaviorFeatures {
            hesitation_score: 1.0,
            anxiety_score: 1.0,
            avoidance_score: 1.0,
            engagement_score: 0.0,
            indecision_score: 1.0,
            fight_or_flight_ratio: 0.0,
        };

        let posteriors = scorer.update_scores(&priors, &extreme).unwrap();
        for (fear, score) in &posteriors {
            // With alpha=0.3, the score should not jump from 0.5 to > 0.8
            // in a single update.
            assert!(
                *score < 0.85,
                "fear {fear}: score {score} swung too far from 0.5"
            );
        }
    }

    // -- End-to-end scoring scenarios ------------------------------------

    #[test]
    fn test_claustrophobic_player_raises_claustrophobia() {
        let scorer = FearScorer::new();
        let mut scores = uniform_priors();

        // Simulate several updates with claustrophobia-indicating features.
        let features = BehaviorFeatures {
            hesitation_score: 0.6,
            anxiety_score: 0.7,
            avoidance_score: 0.8,
            engagement_score: 0.15,
            indecision_score: 0.4,
            fight_or_flight_ratio: 0.15,
        };

        for _ in 0..10 {
            scores = scorer.update_scores(&scores, &features).unwrap();
        }

        let claustro = scores[&FearType::Claustrophobia];
        assert!(
            claustro > 0.55,
            "claustrophobia should have risen, got {claustro}"
        );
    }

    #[test]
    fn test_curious_explorer_keeps_fears_low() {
        let scorer = FearScorer::new();
        let mut scores = uniform_priors();

        let features = BehaviorFeatures {
            hesitation_score: 0.05,
            anxiety_score: 0.05,
            avoidance_score: 0.0,
            engagement_score: 0.95,
            indecision_score: 0.05,
            fight_or_flight_ratio: 0.95,
        };

        for _ in 0..10 {
            scores = scorer.update_scores(&scores, &features).unwrap();
        }

        for (fear, score) in &scores {
            assert!(
                *score < 0.55,
                "curious explorer: {fear} = {score} (should be low)"
            );
        }
    }

    #[test]
    fn test_anxious_avoider_raises_multiple_fears() {
        let scorer = FearScorer::new();
        let mut scores = uniform_priors();

        let features = BehaviorFeatures {
            hesitation_score: 0.7,
            anxiety_score: 0.9,
            avoidance_score: 0.85,
            engagement_score: 0.05,
            indecision_score: 0.6,
            fight_or_flight_ratio: 0.05,
        };

        for _ in 0..10 {
            scores = scorer.update_scores(&scores, &features).unwrap();
        }

        let above_baseline: Vec<_> = scores
            .iter()
            .filter(|(_, &s)| s > 0.52)
            .collect();
        assert!(
            above_baseline.len() >= 3,
            "expected multiple fears to rise, got {} above 0.52",
            above_baseline.len()
        );
    }

    #[test]
    fn test_deterministic_same_input_same_output() {
        let scorer = FearScorer::new();
        let priors = uniform_priors();
        let features = high_fear_features();
        let a = scorer.update_scores(&priors, &features).unwrap();
        let b = scorer.update_scores(&priors, &features).unwrap();
        for fear in FearType::all() {
            assert!(
                (a[&fear] - b[&fear]).abs() < f64::EPSILON,
                "non-deterministic for {fear}"
            );
        }
    }

    // -- Property tests ---------------------------------------------------

    proptest! {
        #[test]
        fn test_all_scores_always_in_unit_range(
            h in 0.0..=1.0f64,
            a in 0.0..=1.0f64,
            av in 0.0..=1.0f64,
            e in 0.0..=1.0f64,
            ind in 0.0..=1.0f64,
            ff in 0.0..=1.0f64,
        ) {
            let scorer = FearScorer::new();
            let features = BehaviorFeatures {
                hesitation_score: h,
                anxiety_score: a,
                avoidance_score: av,
                engagement_score: e,
                indecision_score: ind,
                fight_or_flight_ratio: ff,
            };
            let posteriors = scorer.update_scores(&uniform_priors(), &features).unwrap();
            for (_, score) in &posteriors {
                prop_assert!(*score >= 0.0 && *score <= 1.0);
            }
        }

        #[test]
        fn test_deterministic_for_any_input(
            h in 0.0..=1.0f64,
            a in 0.0..=1.0f64,
            av in 0.0..=1.0f64,
            e in 0.0..=1.0f64,
            ind in 0.0..=1.0f64,
            ff in 0.0..=1.0f64,
        ) {
            let scorer = FearScorer::new();
            let features = BehaviorFeatures {
                hesitation_score: h, anxiety_score: a, avoidance_score: av,
                engagement_score: e, indecision_score: ind, fight_or_flight_ratio: ff,
            };
            let priors = uniform_priors();
            let r1 = scorer.update_scores(&priors, &features).unwrap();
            let r2 = scorer.update_scores(&priors, &features).unwrap();
            for fear in FearType::all() {
                prop_assert!((r1[&fear] - r2[&fear]).abs() < 1e-12);
            }
        }
    }

    // -- Snapshot tests ---------------------------------------------------

    #[test]
    fn test_snapshot_high_claustrophobia() {
        let scorer = FearScorer::new();
        let mut scores = uniform_priors();
        let features = BehaviorFeatures {
            hesitation_score: 0.6, anxiety_score: 0.7, avoidance_score: 0.8,
            engagement_score: 0.15, indecision_score: 0.4, fight_or_flight_ratio: 0.15,
        };
        for _ in 0..5 {
            scores = scorer.update_scores(&scores, &features).unwrap();
        }
        let mut sorted: Vec<_> = scores.into_iter().map(|(f, s)| (f.to_string(), format!("{s:.4}"))).collect();
        sorted.sort();
        insta::assert_yaml_snapshot!("high_claustrophobia", sorted);
    }

    #[test]
    fn test_snapshot_curious_explorer() {
        let scorer = FearScorer::new();
        let mut scores = uniform_priors();
        let features = BehaviorFeatures {
            hesitation_score: 0.05, anxiety_score: 0.05, avoidance_score: 0.0,
            engagement_score: 0.95, indecision_score: 0.05, fight_or_flight_ratio: 0.95,
        };
        for _ in 0..5 {
            scores = scorer.update_scores(&scores, &features).unwrap();
        }
        let mut sorted: Vec<_> = scores.into_iter().map(|(f, s)| (f.to_string(), format!("{s:.4}"))).collect();
        sorted.sort();
        insta::assert_yaml_snapshot!("curious_explorer", sorted);
    }

    #[test]
    fn test_snapshot_anxious_avoider() {
        let scorer = FearScorer::new();
        let mut scores = uniform_priors();
        let features = BehaviorFeatures {
            hesitation_score: 0.7, anxiety_score: 0.9, avoidance_score: 0.85,
            engagement_score: 0.05, indecision_score: 0.6, fight_or_flight_ratio: 0.05,
        };
        for _ in 0..5 {
            scores = scorer.update_scores(&scores, &features).unwrap();
        }
        let mut sorted: Vec<_> = scores.into_iter().map(|(f, s)| (f.to_string(), format!("{s:.4}"))).collect();
        sorted.sort();
        insta::assert_yaml_snapshot!("anxious_avoider", sorted);
    }
}
