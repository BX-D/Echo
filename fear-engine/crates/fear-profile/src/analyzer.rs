//! Feature extraction — maps raw [`BehaviorEvent`]s into a compact
//! [`BehaviorFeatures`] vector suitable for Bayesian scoring.

use fear_engine_common::types::{BehaviorEvent, BehaviorEventType, ChoiceApproach};
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Behavior baseline
// ---------------------------------------------------------------------------

/// Normal-state metrics established during the calibration phase.
///
/// # Example
///
/// ```
/// use fear_engine_fear_profile::analyzer::BehaviorBaseline;
/// let b = BehaviorBaseline::default();
/// assert!(b.avg_typing_speed > 0.0);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehaviorBaseline {
    pub avg_typing_speed: f64,
    pub avg_response_time_ms: f64,
    pub avg_choice_time_ms: f64,
    pub avg_mouse_velocity: f64,
}

impl Default for BehaviorBaseline {
    fn default() -> Self {
        Self {
            avg_typing_speed: 5.0,
            avg_response_time_ms: 3000.0,
            avg_choice_time_ms: 4000.0,
            avg_mouse_velocity: 200.0,
        }
    }
}

impl BehaviorBaseline {
    /// Computes a baseline from the events gathered during calibration.
    ///
    /// Falls back to sensible defaults when there are too few events of a
    /// given type.
    ///
    /// # Example
    ///
    /// ```
    /// use chrono::Utc;
    /// use fear_engine_common::types::{BehaviorEvent, BehaviorEventType};
    /// use fear_engine_fear_profile::analyzer::BehaviorBaseline;
    ///
    /// let events = vec![BehaviorEvent {
    ///     event_type: BehaviorEventType::Keystroke {
    ///         chars_per_second: 6.0, backspace_count: 1, total_chars: 30,
    ///     },
    ///     timestamp: Utc::now(),
    ///     scene_id: "cal".into(),
    /// }];
    /// let baseline = BehaviorBaseline::compute(&events);
    /// assert!((baseline.avg_typing_speed - 6.0).abs() < f64::EPSILON);
    /// ```
    pub fn compute(calibration_events: &[BehaviorEvent]) -> Self {
        let mut typing_speeds = Vec::new();
        let mut choice_times = Vec::new();
        let mut mouse_velocities = Vec::new();

        for event in calibration_events {
            match &event.event_type {
                BehaviorEventType::Keystroke {
                    chars_per_second, ..
                } => {
                    typing_speeds.push(*chars_per_second);
                }
                BehaviorEventType::Choice {
                    time_to_decide_ms, ..
                } => {
                    choice_times.push(*time_to_decide_ms as f64);
                }
                BehaviorEventType::MouseMovement { velocity, .. } => {
                    mouse_velocities.push(*velocity);
                }
                _ => {}
            }
        }

        let defaults = Self::default();
        Self {
            avg_typing_speed: mean_or(&typing_speeds, defaults.avg_typing_speed),
            avg_response_time_ms: defaults.avg_response_time_ms,
            avg_choice_time_ms: mean_or(&choice_times, defaults.avg_choice_time_ms),
            avg_mouse_velocity: mean_or(&mouse_velocities, defaults.avg_mouse_velocity),
        }
    }
}

// ---------------------------------------------------------------------------
// Behavior features
// ---------------------------------------------------------------------------

/// Compact feature vector derived from a window of raw events.
///
/// All scores are in the range `[0.0, 1.0]` except `fight_or_flight_ratio`
/// which is `0.0` (pure flight) to `1.0` (pure fight).
///
/// # Example
///
/// ```
/// use fear_engine_fear_profile::analyzer::{BehaviorFeatures, BehaviorBaseline};
///
/// let features = BehaviorFeatures::extract(&[], &BehaviorBaseline::default());
/// assert!((features.hesitation_score - 0.0).abs() < f64::EPSILON);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehaviorFeatures {
    pub hesitation_score: f64,
    pub anxiety_score: f64,
    pub avoidance_score: f64,
    pub engagement_score: f64,
    pub indecision_score: f64,
    pub fight_or_flight_ratio: f64,
}

impl BehaviorFeatures {
    /// Extracts features from a window of behavior events, comparing
    /// against the player's calibration [`BehaviorBaseline`].
    pub fn extract(events: &[BehaviorEvent], baseline: &BehaviorBaseline) -> Self {
        if events.is_empty() {
            return Self {
                hesitation_score: 0.0,
                anxiety_score: 0.0,
                avoidance_score: 0.0,
                engagement_score: 0.0,
                indecision_score: 0.0,
                fight_or_flight_ratio: 0.5,
            };
        }

        let hesitation_score = compute_hesitation(events, baseline);
        let anxiety_score = compute_anxiety(events, baseline);
        let avoidance_score = compute_avoidance(events);
        let engagement_score = compute_engagement(events);
        let indecision_score = compute_indecision(events, baseline);
        let fight_or_flight_ratio = compute_fight_flight(events);

        Self {
            hesitation_score,
            anxiety_score,
            avoidance_score,
            engagement_score,
            indecision_score,
            fight_or_flight_ratio,
        }
    }
}

// ---------------------------------------------------------------------------
// Individual feature computations
// ---------------------------------------------------------------------------

/// Hesitation: how much the player's typing has slowed relative to baseline.
fn compute_hesitation(events: &[BehaviorEvent], baseline: &BehaviorBaseline) -> f64 {
    let mut typing_speeds = Vec::new();
    let mut pause_count = 0u32;
    let mut total_relevant = 0u32;

    for e in events {
        match &e.event_type {
            BehaviorEventType::Keystroke {
                chars_per_second, ..
            } => {
                typing_speeds.push(*chars_per_second);
                total_relevant += 1;
            }
            BehaviorEventType::Pause { .. } => {
                pause_count += 1;
                total_relevant += 1;
            }
            _ => {}
        }
    }

    if total_relevant == 0 {
        return 0.0;
    }

    let avg_speed = mean_or(&typing_speeds, baseline.avg_typing_speed);
    let speed_slowdown = if baseline.avg_typing_speed > 0.0 {
        ((baseline.avg_typing_speed - avg_speed) / baseline.avg_typing_speed).clamp(0.0, 1.0)
    } else {
        0.0
    };

    let pause_ratio = (pause_count as f64 / total_relevant as f64).clamp(0.0, 1.0);

    (speed_slowdown * 0.6 + pause_ratio * 0.4).clamp(0.0, 1.0)
}

/// Anxiety: mouse tremor + short responses + rapid typing.
fn compute_anxiety(events: &[BehaviorEvent], baseline: &BehaviorBaseline) -> f64 {
    let mut tremor_scores = Vec::new();
    let mut fast_typing_count = 0u32;
    let mut typing_count = 0u32;
    let mut short_response_count = 0u32;
    let mut keystroke_count = 0u32;

    for e in events {
        match &e.event_type {
            BehaviorEventType::MouseMovement { tremor_score, .. } => {
                tremor_scores.push(*tremor_score);
            }
            BehaviorEventType::Keystroke {
                chars_per_second,
                total_chars,
                ..
            } => {
                typing_count += 1;
                if *chars_per_second > baseline.avg_typing_speed * 1.5 {
                    fast_typing_count += 1;
                }
                keystroke_count += 1;
                if *total_chars < 10 {
                    short_response_count += 1;
                }
            }
            _ => {}
        }
    }

    let avg_tremor = mean_or(&tremor_scores, 0.0);
    let tremor_component = (avg_tremor * 2.0).clamp(0.0, 1.0);

    let fast_ratio = if typing_count > 0 {
        fast_typing_count as f64 / typing_count as f64
    } else {
        0.0
    };

    let short_ratio = if keystroke_count > 0 {
        short_response_count as f64 / keystroke_count as f64
    } else {
        0.0
    };

    (tremor_component * 0.4 + fast_ratio * 0.3 + short_ratio * 0.3).clamp(0.0, 1.0)
}

/// Avoidance: ratio of flee/avoid choices.
fn compute_avoidance(events: &[BehaviorEvent]) -> f64 {
    let (avoid, total) = choice_counts(events, &[ChoiceApproach::Flee, ChoiceApproach::Avoid]);
    if total == 0 {
        return 0.0;
    }
    (avoid as f64 / total as f64).clamp(0.0, 1.0)
}

/// Engagement: ratio of investigate/interact choices + rereading.
fn compute_engagement(events: &[BehaviorEvent]) -> f64 {
    let (engage, total_choices) = choice_counts(
        events,
        &[ChoiceApproach::Investigate, ChoiceApproach::Interact],
    );

    let rereading_count = events
        .iter()
        .filter(|e| {
            matches!(
                &e.event_type,
                BehaviorEventType::Scroll {
                    rereading: true,
                    ..
                }
            )
        })
        .count();

    let choice_ratio = if total_choices > 0 {
        engage as f64 / total_choices as f64
    } else {
        0.0
    };

    let rereading_bonus = (rereading_count as f64 * 0.1).clamp(0.0, 0.3);

    (choice_ratio + rereading_bonus).clamp(0.0, 1.0)
}

/// Indecision: backspace ratio + normalised choice deliberation time.
fn compute_indecision(events: &[BehaviorEvent], baseline: &BehaviorBaseline) -> f64 {
    let mut total_chars = 0u32;
    let mut total_backspaces = 0u32;
    let mut choice_times = Vec::new();

    for e in events {
        match &e.event_type {
            BehaviorEventType::Keystroke {
                backspace_count,
                total_chars: tc,
                ..
            } => {
                total_backspaces += backspace_count;
                total_chars += tc;
            }
            BehaviorEventType::Choice {
                time_to_decide_ms, ..
            } => {
                choice_times.push(*time_to_decide_ms as f64);
            }
            _ => {}
        }
    }

    let backspace_ratio = if total_chars > 0 {
        (total_backspaces as f64 / total_chars as f64).clamp(0.0, 1.0)
    } else {
        0.0
    };

    let avg_choice_time = mean_or(&choice_times, baseline.avg_choice_time_ms);
    let deliberation = if baseline.avg_choice_time_ms > 0.0 {
        (avg_choice_time / baseline.avg_choice_time_ms - 1.0).clamp(0.0, 1.0)
    } else {
        0.0
    };

    (backspace_ratio * 0.5 + deliberation * 0.5).clamp(0.0, 1.0)
}

/// Fight-or-flight ratio: 1.0 = pure fight, 0.0 = pure flight.
fn compute_fight_flight(events: &[BehaviorEvent]) -> f64 {
    let fight_approaches = [ChoiceApproach::Confront, ChoiceApproach::Investigate];
    let flight_approaches = [ChoiceApproach::Flee, ChoiceApproach::Avoid];

    let mut fight = 0u32;
    let mut flight = 0u32;

    for e in events {
        if let BehaviorEventType::Choice { approach, .. } = &e.event_type {
            if fight_approaches.contains(approach) {
                fight += 1;
            }
            if flight_approaches.contains(approach) {
                flight += 1;
            }
        }
    }

    let total = fight + flight;
    if total == 0 {
        return 0.5;
    }
    (fight as f64 / total as f64).clamp(0.0, 1.0)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn mean_or(values: &[f64], default: f64) -> f64 {
    if values.is_empty() {
        return default;
    }
    values.iter().sum::<f64>() / values.len() as f64
}

fn choice_counts(events: &[BehaviorEvent], approaches: &[ChoiceApproach]) -> (u32, u32) {
    let mut matching = 0u32;
    let mut total = 0u32;
    for e in events {
        if let BehaviorEventType::Choice { approach, .. } = &e.event_type {
            total += 1;
            if approaches.contains(approach) {
                matching += 1;
            }
        }
    }
    (matching, total)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn ks(cps: f64, backspaces: u32, total: u32) -> BehaviorEvent {
        BehaviorEvent {
            event_type: BehaviorEventType::Keystroke {
                chars_per_second: cps,
                backspace_count: backspaces,
                total_chars: total,
            },
            timestamp: Utc::now(),
            scene_id: "s1".into(),
        }
    }

    fn choice(approach: ChoiceApproach, time_ms: u64) -> BehaviorEvent {
        BehaviorEvent {
            event_type: BehaviorEventType::Choice {
                choice_id: "c".into(),
                time_to_decide_ms: time_ms,
                approach,
            },
            timestamp: Utc::now(),
            scene_id: "s1".into(),
        }
    }

    fn mouse(vel: f64, tremor: f64) -> BehaviorEvent {
        BehaviorEvent {
            event_type: BehaviorEventType::MouseMovement {
                velocity: vel,
                tremor_score: tremor,
            },
            timestamp: Utc::now(),
            scene_id: "s1".into(),
        }
    }

    fn pause(ms: u64) -> BehaviorEvent {
        BehaviorEvent {
            event_type: BehaviorEventType::Pause {
                duration_ms: ms,
                scene_content_hash: "h".into(),
            },
            timestamp: Utc::now(),
            scene_id: "s1".into(),
        }
    }

    fn scroll_reread() -> BehaviorEvent {
        BehaviorEvent {
            event_type: BehaviorEventType::Scroll {
                direction: fear_engine_common::types::ScrollDirection::Up,
                to_position: 0.2,
                rereading: true,
            },
            timestamp: Utc::now(),
            scene_id: "s1".into(),
        }
    }

    fn baseline() -> BehaviorBaseline {
        BehaviorBaseline {
            avg_typing_speed: 5.0,
            avg_response_time_ms: 3000.0,
            avg_choice_time_ms: 4000.0,
            avg_mouse_velocity: 200.0,
        }
    }

    #[test]
    fn test_slow_typing_high_hesitation() {
        let events = vec![ks(1.0, 0, 20), pause(5000)];
        let f = BehaviorFeatures::extract(&events, &baseline());
        assert!(f.hesitation_score > 0.4, "got {}", f.hesitation_score);
    }

    #[test]
    fn test_fast_typing_low_hesitation() {
        let events = vec![ks(7.0, 0, 40)];
        let f = BehaviorFeatures::extract(&events, &baseline());
        assert!(f.hesitation_score < 0.2, "got {}", f.hesitation_score);
    }

    #[test]
    fn test_mouse_tremor_high_anxiety() {
        let events = vec![mouse(300.0, 0.8), mouse(350.0, 0.9)];
        let f = BehaviorFeatures::extract(&events, &baseline());
        assert!(f.anxiety_score > 0.3, "got {}", f.anxiety_score);
    }

    #[test]
    fn test_flee_choices_high_avoidance() {
        let events = vec![
            choice(ChoiceApproach::Flee, 2000),
            choice(ChoiceApproach::Avoid, 1500),
            choice(ChoiceApproach::Investigate, 3000),
        ];
        let f = BehaviorFeatures::extract(&events, &baseline());
        assert!(
            f.avoidance_score > 0.5,
            "expected >0.5, got {}",
            f.avoidance_score
        );
    }

    #[test]
    fn test_investigate_choices_high_engagement() {
        let events = vec![
            choice(ChoiceApproach::Investigate, 2000),
            choice(ChoiceApproach::Interact, 2500),
            scroll_reread(),
        ];
        let f = BehaviorFeatures::extract(&events, &baseline());
        assert!(
            f.engagement_score > 0.5,
            "expected >0.5, got {}",
            f.engagement_score
        );
    }

    #[test]
    fn test_backspaces_high_indecision() {
        let events = vec![ks(5.0, 15, 30), choice(ChoiceApproach::Wait, 8000)];
        let f = BehaviorFeatures::extract(&events, &baseline());
        assert!(
            f.indecision_score > 0.3,
            "expected >0.3, got {}",
            f.indecision_score
        );
    }

    #[test]
    fn test_fight_flight_ratio_all_fight() {
        let events = vec![
            choice(ChoiceApproach::Confront, 1000),
            choice(ChoiceApproach::Investigate, 1000),
        ];
        let f = BehaviorFeatures::extract(&events, &baseline());
        assert!(
            (f.fight_or_flight_ratio - 1.0).abs() < f64::EPSILON,
            "got {}",
            f.fight_or_flight_ratio
        );
    }

    #[test]
    fn test_fight_flight_ratio_all_flight() {
        let events = vec![
            choice(ChoiceApproach::Flee, 1000),
            choice(ChoiceApproach::Avoid, 1000),
        ];
        let f = BehaviorFeatures::extract(&events, &baseline());
        assert!(
            f.fight_or_flight_ratio < f64::EPSILON,
            "got {}",
            f.fight_or_flight_ratio
        );
    }

    #[test]
    fn test_empty_events_returns_neutral() {
        let f = BehaviorFeatures::extract(&[], &baseline());
        assert!((f.hesitation_score).abs() < f64::EPSILON);
        assert!((f.fight_or_flight_ratio - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_baseline_compute() {
        let events = vec![ks(8.0, 0, 40), ks(6.0, 0, 30), mouse(300.0, 0.1)];
        let b = BehaviorBaseline::compute(&events);
        assert!((b.avg_typing_speed - 7.0).abs() < f64::EPSILON);
        assert!((b.avg_mouse_velocity - 300.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_baseline_defaults_on_empty() {
        let b = BehaviorBaseline::compute(&[]);
        assert!((b.avg_typing_speed - 5.0).abs() < f64::EPSILON);
    }
}
