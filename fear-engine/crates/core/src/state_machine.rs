//! Finite state machine for game phase transitions.
//!
//! The game progresses through five sequential phases:
//!
//! ```text
//! Calibrating → Exploring → Escalating → Climax → Reveal
//! ```
//!
//! Each transition requires minimum scene counts and (optionally) a fear-profile
//! confidence threshold.  Every transition is recorded in an audit trail so the
//! end-of-game reveal can show the player how the AI adapted.

use chrono::{DateTime, Utc};
use fear_engine_common::types::GamePhase;
use fear_engine_common::{FearEngineError, Result};
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Transition rules
// ---------------------------------------------------------------------------

/// Requirements that must be met before a phase transition is allowed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransitionRequirements {
    /// Minimum number of scenes completed in the current phase.
    pub min_scenes: u32,
    /// Minimum fear-profile confidence (0.0–1.0) before the engine moves on.
    /// `None` means no confidence gate.
    pub min_confidence: Option<f64>,
}

/// The default requirements for each phase boundary.
///
/// # Example
///
/// ```
/// use fear_engine_common::types::GamePhase;
/// use fear_engine_core::state_machine::default_requirements;
///
/// let req = default_requirements(GamePhase::Calibrating);
/// assert_eq!(req.min_scenes, 2);
/// ```
pub fn default_requirements(from: GamePhase) -> TransitionRequirements {
    match from {
        GamePhase::Calibrating => TransitionRequirements {
            min_scenes: 2,
            min_confidence: None,
        },
        GamePhase::Exploring => TransitionRequirements {
            min_scenes: 5,
            min_confidence: Some(0.4),
        },
        GamePhase::Escalating => TransitionRequirements {
            min_scenes: 5,
            min_confidence: Some(0.6),
        },
        GamePhase::Climax => TransitionRequirements {
            min_scenes: 2,
            min_confidence: None,
        },
        // Reveal is terminal — requirements are irrelevant.
        GamePhase::Reveal => TransitionRequirements {
            min_scenes: 0,
            min_confidence: None,
        },
    }
}

// ---------------------------------------------------------------------------
// Audit trail
// ---------------------------------------------------------------------------

/// One recorded phase transition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransitionRecord {
    /// Phase we transitioned *from*.
    pub from: GamePhase,
    /// Phase we transitioned *to*.
    pub to: GamePhase,
    /// When the transition occurred.
    pub timestamp: DateTime<Utc>,
    /// How many scenes were completed in the `from` phase.
    pub scenes_completed: u32,
    /// Fear-profile confidence at the moment of transition.
    pub confidence: f64,
}

// ---------------------------------------------------------------------------
// State machine
// ---------------------------------------------------------------------------

/// Context supplied by the caller when requesting a transition.
///
/// # Example
///
/// ```
/// use fear_engine_core::state_machine::TransitionContext;
///
/// let ctx = TransitionContext {
///     scenes_completed_in_phase: 3,
///     fear_confidence: 0.55,
/// };
/// assert_eq!(ctx.scenes_completed_in_phase, 3);
/// ```
#[derive(Debug, Clone)]
pub struct TransitionContext {
    /// Scenes the player has completed in the **current** phase.
    pub scenes_completed_in_phase: u32,
    /// Current overall fear-profile confidence (0.0–1.0).
    pub fear_confidence: f64,
}

/// A finite state machine that enforces the
/// `Calibrating → Exploring → Escalating → Climax → Reveal` progression.
///
/// # Example
///
/// ```
/// use fear_engine_core::state_machine::{GameStateMachine, TransitionContext};
/// use fear_engine_common::types::GamePhase;
///
/// let mut sm = GameStateMachine::new();
/// assert_eq!(sm.current_phase(), GamePhase::Calibrating);
///
/// let ctx = TransitionContext { scenes_completed_in_phase: 3, fear_confidence: 0.5 };
/// sm.transition(GamePhase::Exploring, &ctx).unwrap();
/// assert_eq!(sm.current_phase(), GamePhase::Exploring);
/// ```
pub struct GameStateMachine {
    phase: GamePhase,
    requirements: std::collections::HashMap<GamePhase, TransitionRequirements>,
    audit_trail: Vec<TransitionRecord>,
}

impl GameStateMachine {
    /// Creates a new state machine starting in [`GamePhase::Calibrating`].
    ///
    /// # Example
    ///
    /// ```
    /// use fear_engine_core::state_machine::GameStateMachine;
    /// use fear_engine_common::types::GamePhase;
    ///
    /// let sm = GameStateMachine::new();
    /// assert_eq!(sm.current_phase(), GamePhase::Calibrating);
    /// ```
    pub fn new() -> Self {
        let mut requirements = std::collections::HashMap::new();
        for phase in all_phases() {
            requirements.insert(phase, default_requirements(phase));
        }
        Self {
            phase: GamePhase::Calibrating,
            requirements,
            audit_trail: Vec::new(),
        }
    }

    /// Creates a state machine with custom transition requirements.
    ///
    /// # Example
    ///
    /// ```
    /// use std::collections::HashMap;
    /// use fear_engine_common::types::GamePhase;
    /// use fear_engine_core::state_machine::{GameStateMachine, TransitionRequirements};
    ///
    /// let mut reqs = HashMap::new();
    /// reqs.insert(GamePhase::Calibrating, TransitionRequirements { min_scenes: 1, min_confidence: None });
    /// let sm = GameStateMachine::with_requirements(reqs);
    /// assert_eq!(sm.current_phase(), GamePhase::Calibrating);
    /// ```
    pub fn with_requirements(
        requirements: std::collections::HashMap<GamePhase, TransitionRequirements>,
    ) -> Self {
        Self {
            phase: GamePhase::Calibrating,
            requirements,
            audit_trail: Vec::new(),
        }
    }

    /// Returns the current game phase.
    pub fn current_phase(&self) -> GamePhase {
        self.phase
    }

    /// Returns the full audit trail of completed transitions.
    pub fn audit_trail(&self) -> &[TransitionRecord] {
        &self.audit_trail
    }

    /// Attempts to transition to `target`.
    ///
    /// # Errors
    ///
    /// - [`FearEngineError::InvalidState`] if the transition is not the next
    ///   valid phase or if the current phase is [`GamePhase::Reveal`].
    /// - [`FearEngineError::InvalidInput`] if the transition requirements
    ///   (minimum scenes or confidence threshold) are not met.
    ///
    /// # Example
    ///
    /// ```
    /// use fear_engine_core::state_machine::{GameStateMachine, TransitionContext};
    /// use fear_engine_common::types::GamePhase;
    ///
    /// let mut sm = GameStateMachine::new();
    /// let ctx = TransitionContext { scenes_completed_in_phase: 3, fear_confidence: 0.5 };
    /// sm.transition(GamePhase::Exploring, &ctx).unwrap();
    /// assert_eq!(sm.current_phase(), GamePhase::Exploring);
    /// ```
    pub fn transition(&mut self, target: GamePhase, ctx: &TransitionContext) -> Result<()> {
        // Reveal is terminal.
        if self.phase == GamePhase::Reveal {
            return Err(FearEngineError::InvalidState {
                current: "reveal".into(),
                attempted: format!("{target}"),
            });
        }

        // Only the immediate successor is valid.
        let next = next_phase(self.phase).ok_or_else(|| FearEngineError::InvalidState {
            current: format!("{}", self.phase),
            attempted: format!("{target}"),
        })?;

        if target != next {
            return Err(FearEngineError::InvalidState {
                current: format!("{}", self.phase),
                attempted: format!("{target}"),
            });
        }

        // Check requirements.
        if let Some(req) = self.requirements.get(&self.phase) {
            if ctx.scenes_completed_in_phase < req.min_scenes {
                return Err(FearEngineError::InvalidInput {
                    field: "scenes_completed_in_phase".into(),
                    reason: format!(
                        "need at least {} scenes, have {}",
                        req.min_scenes, ctx.scenes_completed_in_phase
                    ),
                });
            }
            if let Some(min_conf) = req.min_confidence {
                if ctx.fear_confidence < min_conf {
                    return Err(FearEngineError::InvalidInput {
                        field: "fear_confidence".into(),
                        reason: format!(
                            "need confidence >= {min_conf}, have {}",
                            ctx.fear_confidence
                        ),
                    });
                }
            }
        }

        // Record and apply.
        self.audit_trail.push(TransitionRecord {
            from: self.phase,
            to: target,
            timestamp: Utc::now(),
            scenes_completed: ctx.scenes_completed_in_phase,
            confidence: ctx.fear_confidence,
        });
        self.phase = target;
        Ok(())
    }

    /// Returns `true` if transitioning to `target` would succeed with the
    /// given context, without actually performing the transition.
    ///
    /// # Example
    ///
    /// ```
    /// use fear_engine_core::state_machine::{GameStateMachine, TransitionContext};
    /// use fear_engine_common::types::GamePhase;
    ///
    /// let sm = GameStateMachine::new();
    /// let ctx = TransitionContext { scenes_completed_in_phase: 3, fear_confidence: 0.5 };
    /// assert!(sm.can_transition(GamePhase::Exploring, &ctx));
    /// assert!(!sm.can_transition(GamePhase::Climax, &ctx));
    /// ```
    pub fn can_transition(&self, target: GamePhase, ctx: &TransitionContext) -> bool {
        if self.phase == GamePhase::Reveal {
            return false;
        }
        let Some(next) = next_phase(self.phase) else {
            return false;
        };
        if target != next {
            return false;
        }
        if let Some(req) = self.requirements.get(&self.phase) {
            if ctx.scenes_completed_in_phase < req.min_scenes {
                return false;
            }
            if let Some(min_conf) = req.min_confidence {
                if ctx.fear_confidence < min_conf {
                    return false;
                }
            }
        }
        true
    }

    /// Restores the machine's current phase when resuming an existing session.
    pub fn set_phase_for_resume(&mut self, phase: GamePhase) {
        self.phase = phase;
    }
}

impl Default for GameStateMachine {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Returns the valid successor for a given phase, or `None` for Reveal.
fn next_phase(current: GamePhase) -> Option<GamePhase> {
    match current {
        GamePhase::Calibrating => Some(GamePhase::Exploring),
        GamePhase::Exploring => Some(GamePhase::Escalating),
        GamePhase::Escalating => Some(GamePhase::Climax),
        GamePhase::Climax => Some(GamePhase::Reveal),
        GamePhase::Reveal => None,
    }
}

/// All five phases in order.
fn all_phases() -> [GamePhase; 5] {
    [
        GamePhase::Calibrating,
        GamePhase::Exploring,
        GamePhase::Escalating,
        GamePhase::Climax,
        GamePhase::Reveal,
    ]
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn easy_ctx(scenes: u32, confidence: f64) -> TransitionContext {
        TransitionContext {
            scenes_completed_in_phase: scenes,
            fear_confidence: confidence,
        }
    }

    /// A state machine with all requirements set to 0 / None so every
    /// transition is trivially allowed.
    fn relaxed_sm() -> GameStateMachine {
        let mut reqs = std::collections::HashMap::new();
        for phase in all_phases() {
            reqs.insert(
                phase,
                TransitionRequirements {
                    min_scenes: 0,
                    min_confidence: None,
                },
            );
        }
        GameStateMachine::with_requirements(reqs)
    }

    // -- Required tests ---------------------------------------------------

    #[test]
    fn test_initial_state_is_calibrating() {
        let sm = GameStateMachine::new();
        assert_eq!(sm.current_phase(), GamePhase::Calibrating);
    }

    #[test]
    fn test_valid_transition_calibrating_to_exploring() {
        let mut sm = GameStateMachine::new();
        let ctx = easy_ctx(3, 0.5);
        sm.transition(GamePhase::Exploring, &ctx).unwrap();
        assert_eq!(sm.current_phase(), GamePhase::Exploring);
    }

    #[test]
    fn test_invalid_transition_calibrating_to_climax() {
        let mut sm = GameStateMachine::new();
        let ctx = easy_ctx(10, 1.0);
        let result = sm.transition(GamePhase::Climax, &ctx);
        assert!(result.is_err());
        assert_eq!(sm.current_phase(), GamePhase::Calibrating);
    }

    #[test]
    fn test_transition_requires_min_scenes() {
        let mut sm = GameStateMachine::new();
        // Default calibrating requires min_scenes = 2.
        let ctx = easy_ctx(1, 0.5);
        let result = sm.transition(GamePhase::Exploring, &ctx);
        assert!(result.is_err());
        match result.unwrap_err() {
            FearEngineError::InvalidInput { field, reason } => {
                assert_eq!(field, "scenes_completed_in_phase");
                assert!(reason.contains("2"));
            }
            other => panic!("expected InvalidInput, got {other:?}"),
        }
    }

    #[test]
    fn test_transition_requires_confidence_threshold() {
        let mut sm = GameStateMachine::new();
        // Move to Exploring first (needs 2 scenes, no confidence gate).
        sm.transition(GamePhase::Exploring, &easy_ctx(3, 0.0))
            .unwrap();
        // Exploring → Escalating needs min_confidence = 0.4.
        let result = sm.transition(GamePhase::Escalating, &easy_ctx(10, 0.2));
        assert!(result.is_err());
        match result.unwrap_err() {
            FearEngineError::InvalidInput { field, reason } => {
                assert_eq!(field, "fear_confidence");
                assert!(reason.contains("0.4"));
            }
            other => panic!("expected InvalidInput, got {other:?}"),
        }
    }

    #[test]
    fn test_all_valid_transition_paths() {
        let mut sm = relaxed_sm();
        let ctx = easy_ctx(0, 0.0);

        sm.transition(GamePhase::Exploring, &ctx).unwrap();
        assert_eq!(sm.current_phase(), GamePhase::Exploring);

        sm.transition(GamePhase::Escalating, &ctx).unwrap();
        assert_eq!(sm.current_phase(), GamePhase::Escalating);

        sm.transition(GamePhase::Climax, &ctx).unwrap();
        assert_eq!(sm.current_phase(), GamePhase::Climax);

        sm.transition(GamePhase::Reveal, &ctx).unwrap();
        assert_eq!(sm.current_phase(), GamePhase::Reveal);
    }

    #[test]
    fn test_transition_audit_trail() {
        let mut sm = relaxed_sm();
        let ctx = easy_ctx(5, 0.7);

        sm.transition(GamePhase::Exploring, &ctx).unwrap();
        sm.transition(GamePhase::Escalating, &ctx).unwrap();

        let trail = sm.audit_trail();
        assert_eq!(trail.len(), 2);
        assert_eq!(trail[0].from, GamePhase::Calibrating);
        assert_eq!(trail[0].to, GamePhase::Exploring);
        assert_eq!(trail[0].scenes_completed, 5);
        assert!((trail[0].confidence - 0.7).abs() < f64::EPSILON);
        assert_eq!(trail[1].from, GamePhase::Exploring);
        assert_eq!(trail[1].to, GamePhase::Escalating);
    }

    #[test]
    fn test_cannot_transition_from_reveal() {
        let mut sm = relaxed_sm();
        let ctx = easy_ctx(0, 0.0);
        sm.transition(GamePhase::Exploring, &ctx).unwrap();
        sm.transition(GamePhase::Escalating, &ctx).unwrap();
        sm.transition(GamePhase::Climax, &ctx).unwrap();
        sm.transition(GamePhase::Reveal, &ctx).unwrap();

        // Try every phase — all must fail.
        for phase in all_phases() {
            assert!(sm.transition(phase, &ctx).is_err());
        }
        assert_eq!(sm.current_phase(), GamePhase::Reveal);
    }

    // -- Additional unit tests -------------------------------------------

    #[test]
    fn test_can_transition_returns_true_for_valid() {
        let sm = GameStateMachine::new();
        let ctx = easy_ctx(5, 0.8);
        assert!(sm.can_transition(GamePhase::Exploring, &ctx));
    }

    #[test]
    fn test_can_transition_returns_false_for_invalid() {
        let sm = GameStateMachine::new();
        let ctx = easy_ctx(5, 0.8);
        assert!(!sm.can_transition(GamePhase::Climax, &ctx));
    }

    #[test]
    fn test_can_transition_false_from_reveal() {
        let mut sm = relaxed_sm();
        let ctx = easy_ctx(0, 0.0);
        sm.transition(GamePhase::Exploring, &ctx).unwrap();
        sm.transition(GamePhase::Escalating, &ctx).unwrap();
        sm.transition(GamePhase::Climax, &ctx).unwrap();
        sm.transition(GamePhase::Reveal, &ctx).unwrap();
        assert!(!sm.can_transition(GamePhase::Calibrating, &ctx));
    }

    #[test]
    fn test_default_requirements_values() {
        let r = default_requirements(GamePhase::Calibrating);
        assert_eq!(r.min_scenes, 2);
        assert!(r.min_confidence.is_none());

        let r = default_requirements(GamePhase::Exploring);
        assert_eq!(r.min_scenes, 5);
        assert!((r.min_confidence.unwrap() - 0.4).abs() < f64::EPSILON);
    }

    #[test]
    fn test_failed_transition_does_not_advance() {
        let mut sm = GameStateMachine::new();
        let _ = sm.transition(GamePhase::Exploring, &easy_ctx(0, 0.0)); // fails
        assert_eq!(sm.current_phase(), GamePhase::Calibrating);
        assert!(sm.audit_trail().is_empty());
    }

    #[test]
    fn test_backward_transition_rejected() {
        let mut sm = relaxed_sm();
        let ctx = easy_ctx(0, 0.0);
        sm.transition(GamePhase::Exploring, &ctx).unwrap();
        let result = sm.transition(GamePhase::Calibrating, &ctx);
        assert!(result.is_err());
    }

    // -- Property test ----------------------------------------------------

    proptest! {
        #[test]
        fn test_state_machine_never_reaches_invalid_state(
            s0 in 0u32..20,
            c0 in 0.0..1.0f64,
            s1 in 0u32..20,
            c1 in 0.0..1.0f64,
            s2 in 0u32..20,
            c2 in 0.0..1.0f64,
            s3 in 0u32..20,
            c3 in 0.0..1.0f64,
        ) {
            let mut sm = GameStateMachine::new();
            let phases = [
                (GamePhase::Exploring,  easy_ctx(s0, c0)),
                (GamePhase::Escalating, easy_ctx(s1, c1)),
                (GamePhase::Climax,     easy_ctx(s2, c2)),
                (GamePhase::Reveal,     easy_ctx(s3, c3)),
            ];

            let valid_phases: std::collections::HashSet<GamePhase> = [
                GamePhase::Calibrating,
                GamePhase::Exploring,
                GamePhase::Escalating,
                GamePhase::Climax,
                GamePhase::Reveal,
            ].into_iter().collect();

            for (target, ctx) in &phases {
                let _ = sm.transition(*target, ctx);
                // Regardless of success/failure, the machine must be in a valid phase.
                prop_assert!(valid_phases.contains(&sm.current_phase()));
            }

            // Audit trail length must equal the number of successful transitions.
            let successful_count = sm.audit_trail().len();
            // Each record must have from < to (ordinal-wise).
            for record in sm.audit_trail() {
                prop_assert!(record.from.ordinal() < record.to.ordinal());
            }
            // Trail length <= 4 (max transitions).
            prop_assert!(successful_count <= 4);
        }
    }
}
