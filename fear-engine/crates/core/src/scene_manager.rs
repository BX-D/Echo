//! Scene manager — resolves the next scene from a player choice, decides
//! whether to serve static or AI-generated content, manages transition
//! effects, and triggers game-phase advances when scene thresholds are met.
//!
//! The [`SceneManager`] sits between the WebSocket handler and the lower-level
//! [`SceneGraph`] / [`GameStateMachine`] types, orchestrating the flow.

use std::collections::{HashMap, HashSet};

use fear_engine_common::types::{
    Atmosphere, EffectDirective, EffectType, FearType, GamePhase,
};
use fear_engine_common::{FearEngineError, Result};
use serde::{Deserialize, Serialize};

use crate::event_bus::{EventBus, GameEvent};
use crate::scene::{ResolutionContext, Scene, SceneGraph, SceneTarget};
use crate::state_machine::{GameStateMachine, TransitionContext};

// ---------------------------------------------------------------------------
// Transition effects
// ---------------------------------------------------------------------------

/// The transition data returned when a scene change occurs.
#[derive(Debug, Clone)]
pub struct SceneTransition {
    /// The resolved target — either a concrete scene or a request for AI
    /// generation.
    pub target: ResolvedTarget,
    /// Visual / audio effects to play during the transition.
    pub transition_effects: Vec<EffectDirective>,
    /// Whether a game-phase change happened as part of this transition.
    pub phase_changed: Option<PhaseChange>,
}

/// What the scene manager resolved the next scene to be.
#[derive(Debug, Clone)]
pub enum ResolvedTarget {
    /// A scene that already exists in the graph.
    Static(Box<Scene>),
    /// The AI must generate a new scene with this context seed.
    Dynamic { context: String },
}

/// Records a phase change that occurred during scene resolution.
#[derive(Debug, Clone)]
pub struct PhaseChange {
    pub from: GamePhase,
    pub to: GamePhase,
}

/// Persisted runtime state for reconnect/resume.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SceneManagerState {
    pub scenes_in_phase: u32,
    pub visited_scenes: Vec<String>,
    pub inventory: Vec<String>,
}

// ---------------------------------------------------------------------------
// Scene manager
// ---------------------------------------------------------------------------

/// Orchestrates scene resolution, phase transitions, and transition effects.
///
/// # Example
///
/// ```
/// use fear_engine_core::scene_manager::SceneManager;
/// use fear_engine_core::scene::{SceneGraph, Scene, SceneType, SceneChoice, SceneTarget};
/// use fear_engine_core::state_machine::GameStateMachine;
/// use fear_engine_core::event_bus::EventBus;
/// use fear_engine_common::types::*;
///
/// let mut graph = SceneGraph::new("intro".into());
/// graph.add_scene(Scene {
///     id: "intro".into(), scene_type: SceneType::Static,
///     narrative: "You wake up.".into(), atmosphere: Atmosphere::Dread,
///     choices: vec![SceneChoice {
///         id: "go".into(), text: "Go".into(),
///         approach: ChoiceApproach::Investigate,
///         fear_vector: FearType::Darkness,
///         target_scene: SceneTarget::Static { scene_id: "hall".into() },
///     }],
///     effects: vec![], sound_cue: None, image_prompt: None,
///     fear_targets: vec![], intensity: 0.3, meta_break: None,
/// }).unwrap();
/// graph.add_scene(Scene {
///     id: "hall".into(), scene_type: SceneType::Static,
///     narrative: "A dark hallway.".into(), atmosphere: Atmosphere::Isolation,
///     choices: vec![], effects: vec![], sound_cue: None, image_prompt: None,
///     fear_targets: vec![], intensity: 0.5, meta_break: None,
/// }).unwrap();
///
/// let sm = GameStateMachine::new();
/// let bus = EventBus::new(32);
/// let mgr = SceneManager::new(graph, sm, bus);
/// assert_eq!(mgr.scenes_in_current_phase(), 0);
/// ```
pub struct SceneManager {
    graph: SceneGraph,
    state_machine: GameStateMachine,
    event_bus: EventBus,
    scenes_in_phase: u32,
    visited_scenes: HashSet<String>,
    inventory: Vec<String>,
    fear_scores: HashMap<FearType, f64>,
    fear_confidence: f64,
}

impl SceneManager {
    /// Creates a new scene manager from its constituent parts.
    pub fn new(graph: SceneGraph, state_machine: GameStateMachine, event_bus: EventBus) -> Self {
        Self {
            graph,
            state_machine,
            event_bus,
            scenes_in_phase: 0,
            visited_scenes: HashSet::new(),
            inventory: Vec::new(),
            fear_scores: HashMap::new(),
            fear_confidence: 0.0,
        }
    }

    /// Number of scenes completed in the current game phase.
    ///
    /// # Example
    ///
    /// ```
    /// use fear_engine_core::scene_manager::SceneManager;
    /// use fear_engine_core::scene::SceneGraph;
    /// use fear_engine_core::state_machine::GameStateMachine;
    /// use fear_engine_core::event_bus::EventBus;
    ///
    /// let mgr = SceneManager::new(
    ///     SceneGraph::new("s".into()),
    ///     GameStateMachine::new(),
    ///     EventBus::new(16),
    /// );
    /// assert_eq!(mgr.scenes_in_current_phase(), 0);
    /// ```
    pub fn scenes_in_current_phase(&self) -> u32 {
        self.scenes_in_phase
    }

    /// Current game phase (delegates to the inner state machine).
    pub fn current_phase(&self) -> GamePhase {
        self.state_machine.current_phase()
    }

    /// Set of all scene IDs the player has visited.
    pub fn visited_scenes(&self) -> &HashSet<String> {
        &self.visited_scenes
    }

    /// Updates the fear profile scores used for conditional resolution.
    ///
    /// # Example
    ///
    /// ```
    /// # use fear_engine_core::scene_manager::SceneManager;
    /// # use fear_engine_core::scene::SceneGraph;
    /// # use fear_engine_core::state_machine::GameStateMachine;
    /// # use fear_engine_core::event_bus::EventBus;
    /// use fear_engine_common::types::FearType;
    /// use std::collections::HashMap;
    ///
    /// let mut mgr = SceneManager::new(
    ///     SceneGraph::new("s".into()),
    ///     GameStateMachine::new(),
    ///     EventBus::new(16),
    /// );
    /// let mut scores = HashMap::new();
    /// scores.insert(FearType::Darkness, 0.85);
    /// mgr.update_fear_scores(scores, 0.7);
    /// ```
    pub fn update_fear_scores(&mut self, scores: HashMap<FearType, f64>, confidence: f64) {
        self.fear_scores = scores;
        self.fear_confidence = confidence;
    }

    /// Adds an item to the player's inventory.
    pub fn add_inventory_item(&mut self, item: String) {
        self.inventory.push(item);
    }

    /// Resolves the next scene based on the player's choice in the current
    /// scene and the game state.
    ///
    /// This is the main entry point for advancing the game.  It:
    ///
    /// 1. Asks the [`SceneGraph`] to resolve the target for the choice.
    /// 2. If the target is static, fetches the scene and builds transition
    ///    effects based on its atmosphere.
    /// 3. Increments the per-phase scene counter and marks the scene visited.
    /// 4. Checks whether a phase transition should happen and, if so,
    ///    advances the [`GameStateMachine`].
    /// 5. Publishes appropriate events on the [`EventBus`].
    ///
    /// # Errors
    ///
    /// Returns an error if the scene or choice cannot be found, or if a
    /// conditional branch has no matching condition.
    pub fn resolve_choice(
        &mut self,
        current_scene_id: &str,
        choice_id: &str,
        session_id: &str,
    ) -> Result<SceneTransition> {
        // 1. Resolve target via the graph.
        let ctx = self.build_resolution_context();
        let target = self
            .graph
            .resolve_next_scene(current_scene_id, choice_id, &ctx)?;

        // Publish choice event.
        self.event_bus.publish(GameEvent::ChoiceMade {
            scene_id: current_scene_id.into(),
            choice_id: choice_id.into(),
            session_id: session_id.into(),
        });

        // 2. Build the resolved target and transition effects.
        let (resolved, effects) = match &target {
            SceneTarget::Static { scene_id } => {
                let scene = self.graph.get_scene(scene_id)?;
                let effects = transition_effects_for_atmosphere(scene.atmosphere);
                (ResolvedTarget::Static(Box::new(scene.clone())), effects)
            }
            SceneTarget::Dynamic { context } => {
                let effects = vec![EffectDirective {
                    effect: EffectType::Glitch,
                    intensity: 0.4,
                    duration_ms: 1500,
                    delay_ms: 0,
                }];
                (
                    ResolvedTarget::Dynamic {
                        context: context.clone(),
                    },
                    effects,
                )
            }
            SceneTarget::Conditional { .. } => {
                // resolve_next_scene already resolved conditionals to Static.
                return Err(FearEngineError::InvalidState {
                    current: current_scene_id.into(),
                    attempted: "unexpected conditional after resolution".into(),
                });
            }
        };

        // 3. Track scene only once the next concrete scene is known.
        let phase_changed = if let ResolvedTarget::Static(ref s) = resolved {
            self.scenes_in_phase += 1;
            self.visited_scenes.insert(s.id.clone());
            self.event_bus.publish(GameEvent::SceneEntered {
                scene_id: s.id.clone(),
                session_id: session_id.into(),
            });
            self.try_phase_transition(session_id)
        } else {
            None
        };

        Ok(SceneTransition {
            target: resolved,
            transition_effects: effects,
            phase_changed,
        })
    }

    /// Manually enter a scene (e.g. the start scene) without a choice.
    ///
    /// Increments the scene counter, marks visited, publishes events.
    pub fn enter_scene(&mut self, scene_id: &str, session_id: &str) -> Result<SceneTransition> {
        let scene = self.graph.get_scene(scene_id)?.clone();
        let effects = transition_effects_for_atmosphere(scene.atmosphere);

        self.scenes_in_phase += 1;
        self.visited_scenes.insert(scene_id.into());
        self.event_bus.publish(GameEvent::SceneEntered {
            scene_id: scene_id.into(),
            session_id: session_id.into(),
        });

        let phase_changed = self.try_phase_transition(session_id);

        Ok(SceneTransition {
            target: ResolvedTarget::Static(Box::new(scene)),
            transition_effects: effects,
            phase_changed,
        })
    }

    /// Returns a reference to the inner event bus (for subscribing).
    pub fn event_bus(&self) -> &EventBus {
        &self.event_bus
    }

    /// Returns a persisted snapshot of the manager's runtime state.
    pub fn snapshot_state(&self) -> SceneManagerState {
        SceneManagerState {
            scenes_in_phase: self.scenes_in_phase,
            visited_scenes: self.visited_scenes.iter().cloned().collect(),
            inventory: self.inventory.clone(),
        }
    }

    /// Restores runtime state for an already-started session.
    pub fn restore_state(&mut self, state: SceneManagerState) {
        self.scenes_in_phase = state.scenes_in_phase;
        self.visited_scenes = state.visited_scenes.into_iter().collect();
        self.inventory = state.inventory;
    }

    /// Returns a scene from the underlying graph.
    pub fn get_scene(&self, scene_id: &str) -> Result<&Scene> {
        self.graph.get_scene(scene_id)
    }

    // -- private ----------------------------------------------------------

    fn build_resolution_context(&self) -> ResolutionContext {
        ResolutionContext {
            fear_scores: self.fear_scores.clone(),
            game_phase: self.state_machine.current_phase(),
            inventory: self.inventory.clone(),
            visited_scenes: self.visited_scenes.clone(),
        }
    }

    /// Attempts to advance the phase if the scene count and confidence meet
    /// the requirements.  Returns `Some` if a transition occurred.
    fn try_phase_transition(&mut self, session_id: &str) -> Option<PhaseChange> {
        let current = self.state_machine.current_phase();
        let next = match current {
            GamePhase::Calibrating => GamePhase::Exploring,
            GamePhase::Exploring => GamePhase::Escalating,
            GamePhase::Escalating => GamePhase::Climax,
            GamePhase::Climax => GamePhase::Reveal,
            GamePhase::Reveal => return None,
        };

        let ctx = TransitionContext {
            scenes_completed_in_phase: self.scenes_in_phase,
            fear_confidence: self.fear_confidence,
        };

        if self.state_machine.can_transition(next, &ctx)
            && self.state_machine.transition(next, &ctx).is_ok()
        {
            self.scenes_in_phase = 0;
            self.event_bus.publish(GameEvent::PhaseChanged {
                from: format!("{current}"),
                to: format!("{next}"),
            });
            let _ = session_id; // available for future DB writes
            return Some(PhaseChange {
                from: current,
                to: next,
            });
        }
        None
    }
}

// ---------------------------------------------------------------------------
// Transition effects
// ---------------------------------------------------------------------------

/// Produces a set of visual effects appropriate for entering a scene with the
/// given atmosphere.
///
/// # Example
///
/// ```
/// use fear_engine_core::scene_manager::transition_effects_for_atmosphere;
/// use fear_engine_common::types::Atmosphere;
///
/// let effects = transition_effects_for_atmosphere(Atmosphere::Panic);
/// assert!(!effects.is_empty());
/// ```
pub fn transition_effects_for_atmosphere(atmosphere: Atmosphere) -> Vec<EffectDirective> {
    match atmosphere {
        Atmosphere::Dread => vec![
            EffectDirective {
                effect: EffectType::Darkness,
                intensity: 0.3,
                duration_ms: 2000,
                delay_ms: 0,
            },
            EffectDirective {
                effect: EffectType::SlowType,
                intensity: 0.5,
                duration_ms: 4000,
                delay_ms: 0,
            },
        ],
        Atmosphere::Tension => vec![EffectDirective {
            effect: EffectType::Flicker,
            intensity: 0.2,
            duration_ms: 1500,
            delay_ms: 200,
        }],
        Atmosphere::Panic => vec![
            EffectDirective {
                effect: EffectType::Shake,
                intensity: 0.7,
                duration_ms: 800,
                delay_ms: 0,
            },
            EffectDirective {
                effect: EffectType::FastType,
                intensity: 0.8,
                duration_ms: 2000,
                delay_ms: 0,
            },
        ],
        Atmosphere::Calm => vec![],
        Atmosphere::Wrongness => vec![EffectDirective {
            effect: EffectType::Glitch,
            intensity: 0.3,
            duration_ms: 1000,
            delay_ms: 500,
        }],
        Atmosphere::Isolation => vec![EffectDirective {
            effect: EffectType::Darkness,
            intensity: 0.5,
            duration_ms: 3000,
            delay_ms: 0,
        }],
        Atmosphere::Paranoia => vec![
            EffectDirective {
                effect: EffectType::Flicker,
                intensity: 0.4,
                duration_ms: 2000,
                delay_ms: 0,
            },
            EffectDirective {
                effect: EffectType::Crt,
                intensity: 0.3,
                duration_ms: 5000,
                delay_ms: 300,
            },
        ],
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::{Scene, SceneChoice, SceneType};
    use crate::state_machine::TransitionRequirements;
    use fear_engine_common::types::ChoiceApproach;

    // -- helpers ----------------------------------------------------------

    fn simple_scene(id: &str, atm: Atmosphere) -> Scene {
        Scene {
            id: id.into(),
            scene_type: SceneType::Static,
            narrative: format!("Scene {id}"),
            atmosphere: atm,
            choices: vec![],
            effects: vec![],
            sound_cue: None,
            image_prompt: None,
            fear_targets: vec![],
            intensity: 0.3,
            meta_break: None,
        }
    }

    fn scene_with_target(id: &str, choice_id: &str, target: SceneTarget) -> Scene {
        Scene {
            choices: vec![SceneChoice {
                id: choice_id.into(),
                text: "Go".into(),
                approach: ChoiceApproach::Investigate,
                fear_vector: FearType::Darkness,
                target_scene: target,
            }],
            ..simple_scene(id, Atmosphere::Dread)
        }
    }

    fn relaxed_sm() -> GameStateMachine {
        let mut reqs = std::collections::HashMap::new();
        for phase in [
            GamePhase::Calibrating,
            GamePhase::Exploring,
            GamePhase::Escalating,
            GamePhase::Climax,
            GamePhase::Reveal,
        ] {
            reqs.insert(
                phase,
                TransitionRequirements {
                    min_scenes: 2,
                    min_confidence: None,
                },
            );
        }
        GameStateMachine::with_requirements(reqs)
    }

    fn build_linear_graph() -> SceneGraph {
        let mut g = SceneGraph::new("s1".into());
        g.add_scene(scene_with_target(
            "s1",
            "go",
            SceneTarget::Static {
                scene_id: "s2".into(),
            },
        ))
        .unwrap();
        g.add_scene(scene_with_target(
            "s2",
            "go",
            SceneTarget::Static {
                scene_id: "s3".into(),
            },
        ))
        .unwrap();
        g.add_scene(scene_with_target(
            "s3",
            "go",
            SceneTarget::Static {
                scene_id: "s4".into(),
            },
        ))
        .unwrap();
        g.add_scene(simple_scene("s4", Atmosphere::Calm)).unwrap();
        g
    }

    fn new_manager_relaxed() -> SceneManager {
        SceneManager::new(build_linear_graph(), relaxed_sm(), EventBus::new(64))
    }

    // -- Required tests ---------------------------------------------------

    #[test]
    fn test_resolve_next_scene_from_choice() {
        let mut mgr = new_manager_relaxed();
        let result = mgr.resolve_choice("s1", "go", "sess").unwrap();
        match &result.target {
            ResolvedTarget::Static(scene) => assert_eq!(scene.id, "s2"),
            ResolvedTarget::Dynamic { .. } => panic!("expected static"),
        }
    }

    #[test]
    fn test_resolve_dynamic_scene_when_ai_needed() {
        let mut g = SceneGraph::new("a".into());
        g.add_scene(scene_with_target(
            "a",
            "go",
            SceneTarget::Dynamic {
                context: "explore the basement".into(),
            },
        ))
        .unwrap();
        let mut mgr = SceneManager::new(g, relaxed_sm(), EventBus::new(16));

        let result = mgr.resolve_choice("a", "go", "sess").unwrap();
        match &result.target {
            ResolvedTarget::Dynamic { context } => {
                assert_eq!(context, "explore the basement");
            }
            ResolvedTarget::Static(_) => panic!("expected dynamic"),
        }
        // Dynamic targets should get a glitch effect.
        assert!(result
            .transition_effects
            .iter()
            .any(|e| e.effect == EffectType::Glitch));
    }

    #[test]
    fn test_scene_transition_effects_based_on_atmosphere() {
        // Panic → Shake + FastType
        let effects = transition_effects_for_atmosphere(Atmosphere::Panic);
        assert!(effects.iter().any(|e| e.effect == EffectType::Shake));
        assert!(effects.iter().any(|e| e.effect == EffectType::FastType));

        // Calm → empty
        let effects = transition_effects_for_atmosphere(Atmosphere::Calm);
        assert!(effects.is_empty());

        // Dread → Darkness + SlowType
        let effects = transition_effects_for_atmosphere(Atmosphere::Dread);
        assert!(effects.iter().any(|e| e.effect == EffectType::Darkness));
        assert!(effects.iter().any(|e| e.effect == EffectType::SlowType));

        // Paranoia → Flicker + CRT
        let effects = transition_effects_for_atmosphere(Atmosphere::Paranoia);
        assert!(effects.iter().any(|e| e.effect == EffectType::Flicker));
        assert!(effects.iter().any(|e| e.effect == EffectType::Crt));
    }

    #[test]
    fn test_phase_transition_triggered_at_scene_threshold() {
        // relaxed_sm requires 2 scenes per phase.
        let mut mgr = new_manager_relaxed();

        // Scene 1 — no phase change.
        let r1 = mgr.resolve_choice("s1", "go", "sess").unwrap();
        assert!(r1.phase_changed.is_none());
        assert_eq!(mgr.current_phase(), GamePhase::Calibrating);

        // Scene 2 — should trigger Calibrating → Exploring.
        let r2 = mgr.resolve_choice("s2", "go", "sess").unwrap();
        assert!(r2.phase_changed.is_some());
        let pc = r2.phase_changed.unwrap();
        assert_eq!(pc.from, GamePhase::Calibrating);
        assert_eq!(pc.to, GamePhase::Exploring);
        assert_eq!(mgr.current_phase(), GamePhase::Exploring);

        // Phase counter should have reset.
        assert_eq!(mgr.scenes_in_current_phase(), 0);
    }

    #[test]
    fn test_scene_count_tracking() {
        let mut mgr = new_manager_relaxed();
        assert_eq!(mgr.scenes_in_current_phase(), 0);

        mgr.enter_scene("s1", "sess").unwrap();
        assert_eq!(mgr.scenes_in_current_phase(), 1);

        mgr.enter_scene("s2", "sess").unwrap();
        // Phase transition resets counter.
        assert_eq!(mgr.scenes_in_current_phase(), 0);
    }

    #[test]
    fn test_scene_manager_with_full_game_flow() {
        // Build a graph with enough scenes for a full phase run.
        let mut g = SceneGraph::new("a".into());
        let ids: Vec<String> = (0..12).map(|i| format!("s{i}")).collect();
        for (i, id) in ids.iter().enumerate() {
            if i + 1 < ids.len() {
                g.add_scene(scene_with_target(
                    id,
                    "go",
                    SceneTarget::Static {
                        scene_id: ids[i + 1].clone(),
                    },
                ))
                .unwrap();
            } else {
                g.add_scene(simple_scene(id, Atmosphere::Calm)).unwrap();
            }
        }

        let mut mgr = SceneManager::new(g, relaxed_sm(), EventBus::new(128));

        let mut current = "s0".to_string();
        let mut phases_hit = vec![mgr.current_phase()];

        for _ in 0..11 {
            let result = mgr.resolve_choice(&current, "go", "sess");
            match result {
                Ok(transition) => {
                    if let Some(pc) = &transition.phase_changed {
                        phases_hit.push(pc.to);
                    }
                    if let ResolvedTarget::Static(s) = &transition.target {
                        current = s.id.clone();
                    }
                }
                Err(_) => break,
            }
        }

        // Should have progressed through multiple phases.
        assert!(phases_hit.len() >= 3);
        // At least Calibrating, Exploring, Escalating
        assert_eq!(phases_hit[0], GamePhase::Calibrating);
        assert_eq!(phases_hit[1], GamePhase::Exploring);
    }

    // -- Additional tests -------------------------------------------------

    #[test]
    fn test_visited_scenes_tracked() {
        let mut mgr = new_manager_relaxed();
        mgr.enter_scene("s1", "sess").unwrap();
        assert!(mgr.visited_scenes().contains("s1"));
    }

    #[test]
    fn test_enter_scene_publishes_event() {
        let mut mgr = new_manager_relaxed();
        let mut rx = mgr.event_bus().subscribe();
        mgr.enter_scene("s1", "sess").unwrap();

        let event = rx.try_recv().unwrap();
        assert!(matches!(
            event,
            GameEvent::SceneEntered { scene_id, .. } if scene_id == "s1"
        ));
    }

    #[test]
    fn test_resolve_choice_publishes_choice_event() {
        let mut mgr = new_manager_relaxed();
        let mut rx = mgr.event_bus().subscribe();
        mgr.resolve_choice("s1", "go", "sess").unwrap();

        // First event should be ChoiceMade.
        let event = rx.try_recv().unwrap();
        assert!(matches!(event, GameEvent::ChoiceMade { .. }));
    }

    #[test]
    fn test_update_fear_scores() {
        let mut mgr = new_manager_relaxed();
        let mut scores = HashMap::new();
        scores.insert(FearType::Darkness, 0.9);
        mgr.update_fear_scores(scores, 0.75);
        // Internal state is updated (verified indirectly through resolution).
        assert!((mgr.fear_confidence - 0.75).abs() < f64::EPSILON);
    }

    #[test]
    fn test_add_inventory_item() {
        let mut mgr = new_manager_relaxed();
        mgr.add_inventory_item("flashlight".into());
        assert_eq!(mgr.inventory.len(), 1);
    }

    #[test]
    fn test_resolve_choice_nonexistent_scene_errors() {
        let mut mgr = new_manager_relaxed();
        let result = mgr.resolve_choice("nonexistent", "go", "sess");
        assert!(result.is_err());
    }

    #[test]
    fn test_resolve_choice_nonexistent_choice_errors() {
        let mut mgr = new_manager_relaxed();
        let result = mgr.resolve_choice("s1", "bad_choice", "sess");
        assert!(result.is_err());
    }
}
