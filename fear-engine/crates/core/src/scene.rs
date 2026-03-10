//! Scene data model and scene graph for the Fear Engine.
//!
//! A [`Scene`] is one narrative beat — text the player reads plus the choices
//! they can make.  Scenes live inside a [`SceneGraph`] that wires them together
//! via [`SceneTarget`] links and supports conditional branching based on the
//! player's fear profile, inventory, and game phase.

use std::collections::{HashMap, HashSet, VecDeque};

use fear_engine_common::types::{
    Atmosphere, ChoiceApproach, EffectDirective, FearType, GamePhase, MetaBreak,
};
use fear_engine_common::{FearEngineError, Result};
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Scene
// ---------------------------------------------------------------------------

/// A single scene in the horror game.
///
/// Scenes can be [`SceneType::Static`] (pre-written), [`SceneType::Template`]
/// (partial AI fill-in), or [`SceneType::Dynamic`] (fully AI-generated).
///
/// # Example
///
/// ```
/// use fear_engine_core::scene::{Scene, SceneType};
/// use fear_engine_common::types::Atmosphere;
///
/// let scene = Scene {
///     id: "lobby".into(),
///     scene_type: SceneType::Static,
///     narrative: "The lobby is empty.".into(),
///     atmosphere: Atmosphere::Isolation,
///     choices: vec![],
///     effects: vec![],
///     sound_cue: None,
///     image_prompt: None,
///     fear_targets: vec![],
///     intensity: 0.3,
///     meta_break: None,
/// };
/// assert_eq!(scene.id, "lobby");
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scene {
    /// Unique identifier for this scene.
    pub id: String,
    /// Whether the content is fixed, templated, or fully dynamic.
    pub scene_type: SceneType,
    /// The narrative text shown to the player.
    pub narrative: String,
    /// Emotional tone of the scene.
    pub atmosphere: Atmosphere,
    /// Choices the player can make.
    pub choices: Vec<SceneChoice>,
    /// Visual/audio effects to trigger.
    pub effects: Vec<EffectDirective>,
    /// Ambient sound cue name.
    pub sound_cue: Option<String>,
    /// Prompt for AI image generation (key moments only).
    pub image_prompt: Option<String>,
    /// Which fears this scene is testing or targeting.
    pub fear_targets: Vec<FearType>,
    /// Horror intensity from 0.0 (calm) to 1.0 (maximum).
    pub intensity: f64,
    /// Optional fourth-wall-breaking moment.
    pub meta_break: Option<MetaBreak>,
}

/// How the scene's content is produced.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SceneType {
    /// Pre-written narrative; always the same.
    Static,
    /// Partially written with placeholder tokens for AI fill-in.
    Template { placeholders: Vec<String> },
    /// Narrative is generated entirely by the LLM.
    Dynamic,
}

// ---------------------------------------------------------------------------
// Choices & transitions
// ---------------------------------------------------------------------------

/// A choice the player can make inside a scene.
///
/// Each choice carries metadata about the player's psychological approach and
/// a [`SceneTarget`] that determines where the game goes next.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneChoice {
    /// Unique choice identifier within the scene.
    pub id: String,
    /// Player-visible text.
    pub text: String,
    /// Psychological approach this choice represents.
    pub approach: ChoiceApproach,
    /// Which fear axis this choice tests.
    pub fear_vector: FearType,
    /// Where this choice leads.
    pub target_scene: SceneTarget,
}

/// Determines which scene follows when a choice is made.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SceneTarget {
    /// Go to a specific scene by ID.
    Static { scene_id: String },
    /// Ask the AI to generate a new scene, seeded with this context.
    Dynamic { context: String },
    /// Branch based on run-time conditions.
    Conditional { branches: Vec<ConditionalTarget> },
}

/// One branch inside a [`SceneTarget::Conditional`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConditionalTarget {
    /// The condition that must be satisfied.
    pub condition: TransitionCondition,
    /// Scene ID to transition to when the condition is met.
    pub target: String,
}

/// Run-time conditions evaluated during scene resolution.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TransitionCondition {
    /// Player's score for `fear` is above `threshold`.
    FearAboveThreshold { fear: FearType, threshold: f64 },
    /// Player's score for `fear` is below `threshold`.
    FearBelowThreshold { fear: FearType, threshold: f64 },
    /// Game is in a specific phase.
    PhaseIs { phase: GamePhase },
    /// Player has a specific item.
    HasItem { item: String },
    /// Player has visited a specific scene.
    SceneVisited { scene_id: String },
    /// Passes with the given probability (0.0 – 1.0).
    Random { probability: f64 },
}

// ---------------------------------------------------------------------------
// Scene graph
// ---------------------------------------------------------------------------

/// Context used when resolving conditional scene transitions.
///
/// # Example
///
/// ```
/// use std::collections::HashSet;
/// use fear_engine_core::scene::ResolutionContext;
/// use fear_engine_common::types::GamePhase;
///
/// let ctx = ResolutionContext {
///     fear_scores: std::collections::HashMap::new(),
///     game_phase: GamePhase::Calibrating,
///     inventory: vec![],
///     visited_scenes: HashSet::new(),
/// };
/// assert_eq!(ctx.game_phase, GamePhase::Calibrating);
/// ```
#[derive(Debug, Clone)]
pub struct ResolutionContext {
    /// Current fear profile scores, keyed by fear type.
    pub fear_scores: HashMap<FearType, f64>,
    /// Current game phase.
    pub game_phase: GamePhase,
    /// Items the player holds.
    pub inventory: Vec<String>,
    /// Scene IDs the player has already visited.
    pub visited_scenes: HashSet<String>,
}

/// A warning produced by [`SceneGraph::validate`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationWarning {
    /// Scene is not reachable from the start node.
    OrphanScene { scene_id: String },
    /// Scene has no choices and is not marked as an end scene.
    DeadEnd { scene_id: String },
    /// A cycle was detected involving these scene IDs.
    Cycle { scene_ids: Vec<String> },
    /// A conditional target references a scene that does not exist.
    MissingTarget { from_scene: String, target_id: String },
}

/// Directed graph of [`Scene`] nodes connected by choices.
///
/// # Example
///
/// ```
/// use fear_engine_core::scene::{SceneGraph, Scene, SceneType, SceneChoice, SceneTarget};
/// use fear_engine_common::types::{Atmosphere, ChoiceApproach, FearType};
///
/// let mut graph = SceneGraph::new("start".into());
/// graph.add_scene(Scene {
///     id: "start".into(),
///     scene_type: SceneType::Static,
///     narrative: "Begin.".into(),
///     atmosphere: Atmosphere::Calm,
///     choices: vec![SceneChoice {
///         id: "go".into(),
///         text: "Continue".into(),
///         approach: ChoiceApproach::Investigate,
///         fear_vector: FearType::Darkness,
///         target_scene: SceneTarget::Static { scene_id: "end".into() },
///     }],
///     effects: vec![],
///     sound_cue: None,
///     image_prompt: None,
///     fear_targets: vec![],
///     intensity: 0.1,
///     meta_break: None,
/// }).unwrap();
/// assert_eq!(graph.all_scene_ids().len(), 1);
/// ```
pub struct SceneGraph {
    scenes: HashMap<String, Scene>,
    start_scene_id: String,
}

impl SceneGraph {
    /// Creates a new, empty graph with the given start scene ID.
    ///
    /// The start scene itself must be added separately via [`Self::add_scene`].
    ///
    /// # Example
    ///
    /// ```
    /// use fear_engine_core::scene::SceneGraph;
    /// let g = SceneGraph::new("intro".into());
    /// assert!(g.all_scene_ids().is_empty());
    /// ```
    pub fn new(start_scene_id: String) -> Self {
        Self {
            scenes: HashMap::new(),
            start_scene_id,
        }
    }

    /// Adds a scene to the graph.
    ///
    /// Returns an error if a scene with the same ID already exists.
    ///
    /// # Example
    ///
    /// ```
    /// use fear_engine_core::scene::{SceneGraph, Scene, SceneType};
    /// use fear_engine_common::types::Atmosphere;
    ///
    /// let mut g = SceneGraph::new("s1".into());
    /// g.add_scene(Scene {
    ///     id: "s1".into(),
    ///     scene_type: SceneType::Static,
    ///     narrative: "Hi.".into(),
    ///     atmosphere: Atmosphere::Calm,
    ///     choices: vec![],
    ///     effects: vec![],
    ///     sound_cue: None,
    ///     image_prompt: None,
    ///     fear_targets: vec![],
    ///     intensity: 0.0,
    ///     meta_break: None,
    /// }).unwrap();
    /// assert_eq!(g.all_scene_ids().len(), 1);
    /// ```
    pub fn add_scene(&mut self, scene: Scene) -> Result<()> {
        if self.scenes.contains_key(&scene.id) {
            return Err(FearEngineError::InvalidInput {
                field: "scene.id".into(),
                reason: format!("scene '{}' already exists in graph", scene.id),
            });
        }
        self.scenes.insert(scene.id.clone(), scene);
        Ok(())
    }

    /// Returns a reference to the scene with the given ID.
    ///
    /// # Errors
    ///
    /// Returns [`FearEngineError::NotFound`] if the scene does not exist.
    ///
    /// # Example
    ///
    /// ```
    /// use fear_engine_core::scene::{SceneGraph, Scene, SceneType};
    /// use fear_engine_common::types::Atmosphere;
    ///
    /// let mut g = SceneGraph::new("s1".into());
    /// g.add_scene(Scene {
    ///     id: "s1".into(), scene_type: SceneType::Static,
    ///     narrative: "N.".into(), atmosphere: Atmosphere::Calm,
    ///     choices: vec![], effects: vec![], sound_cue: None,
    ///     image_prompt: None, fear_targets: vec![], intensity: 0.0,
    ///     meta_break: None,
    /// }).unwrap();
    /// assert!(g.get_scene("s1").is_ok());
    /// assert!(g.get_scene("nope").is_err());
    /// ```
    pub fn get_scene(&self, id: &str) -> Result<&Scene> {
        self.scenes.get(id).ok_or_else(|| FearEngineError::NotFound {
            entity: "Scene".into(),
            id: id.into(),
        })
    }

    /// Returns the IDs of every scene in the graph.
    pub fn all_scene_ids(&self) -> Vec<&str> {
        self.scenes.keys().map(|s| s.as_str()).collect()
    }

    /// Determines the [`SceneTarget`] for a given choice in the current scene.
    ///
    /// For [`SceneTarget::Conditional`] targets the first matching branch wins.
    /// If no branch matches the function returns an error.
    ///
    /// # Example
    ///
    /// ```
    /// use std::collections::{HashMap, HashSet};
    /// use fear_engine_core::scene::*;
    /// use fear_engine_common::types::*;
    ///
    /// let mut g = SceneGraph::new("a".into());
    /// g.add_scene(Scene {
    ///     id: "a".into(), scene_type: SceneType::Static,
    ///     narrative: "A.".into(), atmosphere: Atmosphere::Calm,
    ///     choices: vec![SceneChoice {
    ///         id: "go".into(), text: "Go".into(),
    ///         approach: ChoiceApproach::Investigate,
    ///         fear_vector: FearType::Darkness,
    ///         target_scene: SceneTarget::Static { scene_id: "b".into() },
    ///     }],
    ///     effects: vec![], sound_cue: None, image_prompt: None,
    ///     fear_targets: vec![], intensity: 0.0, meta_break: None,
    /// }).unwrap();
    ///
    /// let ctx = ResolutionContext {
    ///     fear_scores: HashMap::new(),
    ///     game_phase: GamePhase::Calibrating,
    ///     inventory: vec![],
    ///     visited_scenes: HashSet::new(),
    /// };
    /// let target = g.resolve_next_scene("a", "go", &ctx).unwrap();
    /// match target {
    ///     SceneTarget::Static { scene_id } => assert_eq!(scene_id, "b"),
    ///     _ => panic!("expected static target"),
    /// }
    /// ```
    pub fn resolve_next_scene(
        &self,
        current: &str,
        choice_id: &str,
        context: &ResolutionContext,
    ) -> Result<SceneTarget> {
        let scene = self.get_scene(current)?;
        let choice = scene
            .choices
            .iter()
            .find(|c| c.id == choice_id)
            .ok_or_else(|| FearEngineError::NotFound {
                entity: "Choice".into(),
                id: choice_id.into(),
            })?;

        match &choice.target_scene {
            SceneTarget::Conditional { branches } => {
                for branch in branches {
                    if evaluate_condition(&branch.condition, context) {
                        return Ok(SceneTarget::Static {
                            scene_id: branch.target.clone(),
                        });
                    }
                }
                Err(FearEngineError::InvalidState {
                    current: current.into(),
                    attempted: format!("conditional from choice '{choice_id}': no branch matched"),
                })
            }
            other => Ok(other.clone()),
        }
    }

    /// Validates the graph, returning a list of warnings.
    ///
    /// Checks performed:
    /// - **Orphan scenes**: not reachable from the start scene.
    /// - **Dead ends**: scenes with no outgoing choices (and not the only scene).
    /// - **Cycles**: strongly connected components of length > 1.
    /// - **Missing targets**: conditional targets referencing nonexistent scenes.
    ///
    /// # Example
    ///
    /// ```
    /// use fear_engine_core::scene::{SceneGraph, Scene, SceneType, ValidationWarning};
    /// use fear_engine_common::types::Atmosphere;
    ///
    /// let mut g = SceneGraph::new("a".into());
    /// g.add_scene(Scene {
    ///     id: "a".into(), scene_type: SceneType::Static,
    ///     narrative: "A.".into(), atmosphere: Atmosphere::Calm,
    ///     choices: vec![], effects: vec![], sound_cue: None,
    ///     image_prompt: None, fear_targets: vec![], intensity: 0.0,
    ///     meta_break: None,
    /// }).unwrap();
    /// // Orphan scene:
    /// g.add_scene(Scene {
    ///     id: "orphan".into(), scene_type: SceneType::Static,
    ///     narrative: "O.".into(), atmosphere: Atmosphere::Calm,
    ///     choices: vec![], effects: vec![], sound_cue: None,
    ///     image_prompt: None, fear_targets: vec![], intensity: 0.0,
    ///     meta_break: None,
    /// }).unwrap();
    /// let w = g.validate().unwrap();
    /// assert!(w.iter().any(|w| matches!(w, ValidationWarning::OrphanScene { .. })));
    /// ```
    pub fn validate(&self) -> Result<Vec<ValidationWarning>> {
        let mut warnings = Vec::new();

        // --- Reachability (orphan detection) ---
        let reachable = self.reachable_from_start();
        for id in self.scenes.keys() {
            if !reachable.contains(id) {
                warnings.push(ValidationWarning::OrphanScene {
                    scene_id: id.clone(),
                });
            }
        }

        // --- Dead ends ---
        if self.scenes.len() > 1 {
            for scene in self.scenes.values() {
                if scene.choices.is_empty() {
                    warnings.push(ValidationWarning::DeadEnd {
                        scene_id: scene.id.clone(),
                    });
                }
            }
        }

        // --- Missing targets ---
        for scene in self.scenes.values() {
            for choice in &scene.choices {
                self.check_target_exists(scene, choice, &mut warnings);
            }
        }

        // --- Cycle detection ---
        self.detect_cycles(&mut warnings);

        Ok(warnings)
    }

    // ---- private helpers ------------------------------------------------

    /// BFS from the start scene; returns the set of reachable scene IDs.
    fn reachable_from_start(&self) -> HashSet<String> {
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();

        if self.scenes.contains_key(&self.start_scene_id) {
            queue.push_back(self.start_scene_id.clone());
            visited.insert(self.start_scene_id.clone());
        }

        while let Some(id) = queue.pop_front() {
            if let Some(scene) = self.scenes.get(&id) {
                for choice in &scene.choices {
                    for neighbor in static_target_ids(&choice.target_scene) {
                        if !visited.contains(&neighbor) {
                            visited.insert(neighbor.clone());
                            queue.push_back(neighbor);
                        }
                    }
                }
            }
        }
        visited
    }

    /// Warns if any static/conditional target references a nonexistent scene.
    fn check_target_exists(
        &self,
        scene: &Scene,
        choice: &SceneChoice,
        warnings: &mut Vec<ValidationWarning>,
    ) {
        for target_id in static_target_ids(&choice.target_scene) {
            if !self.scenes.contains_key(&target_id) {
                warnings.push(ValidationWarning::MissingTarget {
                    from_scene: scene.id.clone(),
                    target_id,
                });
            }
        }
    }

    /// DFS-based cycle detection. Reports each cycle as a warning.
    fn detect_cycles(&self, warnings: &mut Vec<ValidationWarning>) {
        let mut visited = HashSet::new();
        let mut on_stack = HashSet::new();
        let mut path = Vec::new();

        for id in self.scenes.keys() {
            if !visited.contains(id) {
                self.dfs_cycle(id, &mut visited, &mut on_stack, &mut path, warnings);
            }
        }
    }

    fn dfs_cycle(
        &self,
        node: &str,
        visited: &mut HashSet<String>,
        on_stack: &mut HashSet<String>,
        path: &mut Vec<String>,
        warnings: &mut Vec<ValidationWarning>,
    ) {
        visited.insert(node.to_string());
        on_stack.insert(node.to_string());
        path.push(node.to_string());

        if let Some(scene) = self.scenes.get(node) {
            for choice in &scene.choices {
                for neighbor in static_target_ids(&choice.target_scene) {
                    if !self.scenes.contains_key(&neighbor) {
                        continue;
                    }
                    if !visited.contains(&neighbor) {
                        self.dfs_cycle(&neighbor, visited, on_stack, path, warnings);
                    } else if on_stack.contains(&neighbor) {
                        if let Some(pos) = path.iter().position(|p| *p == neighbor) {
                            let cycle: Vec<String> = path[pos..].to_vec();
                            if cycle.len() > 1 {
                                warnings.push(ValidationWarning::Cycle {
                                    scene_ids: cycle,
                                });
                            }
                        }
                    }
                }
            }
        }

        path.pop();
        on_stack.remove(node);
    }
}

// ---------------------------------------------------------------------------
// Condition evaluation
// ---------------------------------------------------------------------------

/// Evaluates a [`TransitionCondition`] against the current game context.
///
/// [`TransitionCondition::Random`] is evaluated deterministically here
/// (always `true` if probability ≥ 0.5) because real randomness would make
/// tests non-deterministic.  A proper RNG will be injected in the game loop.
///
/// # Example
///
/// ```
/// use std::collections::{HashMap, HashSet};
/// use fear_engine_core::scene::{TransitionCondition, ResolutionContext, evaluate_condition};
/// use fear_engine_common::types::{FearType, GamePhase};
///
/// let mut scores = HashMap::new();
/// scores.insert(FearType::Darkness, 0.8);
/// let ctx = ResolutionContext {
///     fear_scores: scores,
///     game_phase: GamePhase::Exploring,
///     inventory: vec!["flashlight".into()],
///     visited_scenes: HashSet::from(["lobby".into()]),
/// };
/// assert!(evaluate_condition(
///     &TransitionCondition::FearAboveThreshold { fear: FearType::Darkness, threshold: 0.5 },
///     &ctx,
/// ));
/// ```
pub fn evaluate_condition(condition: &TransitionCondition, ctx: &ResolutionContext) -> bool {
    match condition {
        TransitionCondition::FearAboveThreshold { fear, threshold } => {
            ctx.fear_scores.get(fear).copied().unwrap_or(0.5) > *threshold
        }
        TransitionCondition::FearBelowThreshold { fear, threshold } => {
            ctx.fear_scores.get(fear).copied().unwrap_or(0.5) < *threshold
        }
        TransitionCondition::PhaseIs { phase } => ctx.game_phase == *phase,
        TransitionCondition::HasItem { item } => ctx.inventory.contains(item),
        TransitionCondition::SceneVisited { scene_id } => ctx.visited_scenes.contains(scene_id),
        TransitionCondition::Random { probability } => *probability >= 0.5,
    }
}

/// Extracts all statically-known scene-ID references from a [`SceneTarget`].
fn static_target_ids(target: &SceneTarget) -> Vec<String> {
    match target {
        SceneTarget::Static { scene_id } => vec![scene_id.clone()],
        SceneTarget::Dynamic { .. } => vec![],
        SceneTarget::Conditional { branches } => {
            branches.iter().map(|b| b.target.clone()).collect()
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use std::collections::HashSet;

    // -- helpers ----------------------------------------------------------

    fn simple_scene(id: &str) -> Scene {
        Scene {
            id: id.into(),
            scene_type: SceneType::Static,
            narrative: format!("Narrative for {id}"),
            atmosphere: Atmosphere::Calm,
            choices: vec![],
            effects: vec![],
            sound_cue: None,
            image_prompt: None,
            fear_targets: vec![],
            intensity: 0.0,
            meta_break: None,
        }
    }

    fn scene_with_choice(id: &str, choice_target: SceneTarget) -> Scene {
        Scene {
            choices: vec![SceneChoice {
                id: "go".into(),
                text: "Go".into(),
                approach: ChoiceApproach::Investigate,
                fear_vector: FearType::Darkness,
                target_scene: choice_target,
            }],
            ..simple_scene(id)
        }
    }

    fn default_context() -> ResolutionContext {
        ResolutionContext {
            fear_scores: HashMap::new(),
            game_phase: GamePhase::Calibrating,
            inventory: vec![],
            visited_scenes: HashSet::new(),
        }
    }

    // -- Scene creation ---------------------------------------------------

    #[test]
    fn test_scene_creation_static() {
        let s = simple_scene("lobby");
        assert_eq!(s.id, "lobby");
        assert!(matches!(s.scene_type, SceneType::Static));
    }

    #[test]
    fn test_scene_creation_template() {
        let s = Scene {
            scene_type: SceneType::Template {
                placeholders: vec!["{{FEAR}}".into(), "{{NAME}}".into()],
            },
            ..simple_scene("tmpl")
        };
        if let SceneType::Template { placeholders } = &s.scene_type {
            assert_eq!(placeholders.len(), 2);
        } else {
            panic!("expected Template");
        }
    }

    #[test]
    fn test_scene_creation_dynamic() {
        let s = Scene {
            scene_type: SceneType::Dynamic,
            ..simple_scene("dyn")
        };
        assert!(matches!(s.scene_type, SceneType::Dynamic));
    }

    #[test]
    fn test_scene_all_fields() {
        let s = Scene {
            id: "x".into(),
            scene_type: SceneType::Static,
            narrative: "N".into(),
            atmosphere: Atmosphere::Dread,
            choices: vec![SceneChoice {
                id: "c".into(),
                text: "T".into(),
                approach: ChoiceApproach::Flee,
                fear_vector: FearType::Stalking,
                target_scene: SceneTarget::Dynamic {
                    context: "ctx".into(),
                },
            }],
            effects: vec![EffectDirective {
                effect: fear_engine_common::types::EffectType::Shake,
                intensity: 0.5,
                duration_ms: 1000,
                delay_ms: 0,
            }],
            sound_cue: Some("drip".into()),
            image_prompt: Some("dark hallway".into()),
            fear_targets: vec![FearType::Darkness, FearType::Isolation],
            intensity: 0.7,
            meta_break: Some(MetaBreak {
                text: "hi".into(),
                target: fear_engine_common::types::MetaTarget::Whisper,
            }),
        };
        assert_eq!(s.fear_targets.len(), 2);
        assert!(s.meta_break.is_some());
        assert_eq!(s.effects.len(), 1);
    }

    // -- SceneGraph CRUD --------------------------------------------------

    #[test]
    fn test_graph_add_and_get() {
        let mut g = SceneGraph::new("a".into());
        g.add_scene(simple_scene("a")).unwrap();
        let s = g.get_scene("a").unwrap();
        assert_eq!(s.id, "a");
    }

    #[test]
    fn test_graph_get_nonexistent() {
        let g = SceneGraph::new("a".into());
        assert!(g.get_scene("nope").is_err());
    }

    #[test]
    fn test_graph_duplicate_scene_rejected() {
        let mut g = SceneGraph::new("a".into());
        g.add_scene(simple_scene("a")).unwrap();
        assert!(g.add_scene(simple_scene("a")).is_err());
    }

    #[test]
    fn test_graph_all_scene_ids() {
        let mut g = SceneGraph::new("a".into());
        g.add_scene(simple_scene("a")).unwrap();
        g.add_scene(simple_scene("b")).unwrap();
        let ids: HashSet<&str> = g.all_scene_ids().into_iter().collect();
        assert_eq!(ids.len(), 2);
        assert!(ids.contains("a"));
        assert!(ids.contains("b"));
    }

    // -- Traversal / resolution -------------------------------------------

    #[test]
    fn test_resolve_static_target() {
        let mut g = SceneGraph::new("a".into());
        g.add_scene(scene_with_choice(
            "a",
            SceneTarget::Static {
                scene_id: "b".into(),
            },
        ))
        .unwrap();
        g.add_scene(simple_scene("b")).unwrap();

        let ctx = default_context();
        let target = g.resolve_next_scene("a", "go", &ctx).unwrap();
        match target {
            SceneTarget::Static { scene_id } => assert_eq!(scene_id, "b"),
            _ => panic!("expected Static"),
        }
    }

    #[test]
    fn test_resolve_dynamic_target() {
        let mut g = SceneGraph::new("a".into());
        g.add_scene(scene_with_choice(
            "a",
            SceneTarget::Dynamic {
                context: "explore".into(),
            },
        ))
        .unwrap();

        let target = g
            .resolve_next_scene("a", "go", &default_context())
            .unwrap();
        match target {
            SceneTarget::Dynamic { context } => assert_eq!(context, "explore"),
            _ => panic!("expected Dynamic"),
        }
    }

    #[test]
    fn test_resolve_conditional_target() {
        let mut g = SceneGraph::new("a".into());
        let mut scores = HashMap::new();
        scores.insert(FearType::Darkness, 0.9);

        g.add_scene(scene_with_choice(
            "a",
            SceneTarget::Conditional {
                branches: vec![
                    ConditionalTarget {
                        condition: TransitionCondition::FearAboveThreshold {
                            fear: FearType::Darkness,
                            threshold: 0.7,
                        },
                        target: "dark_path".into(),
                    },
                    ConditionalTarget {
                        condition: TransitionCondition::FearBelowThreshold {
                            fear: FearType::Darkness,
                            threshold: 0.3,
                        },
                        target: "light_path".into(),
                    },
                ],
            },
        ))
        .unwrap();

        let ctx = ResolutionContext {
            fear_scores: scores,
            ..default_context()
        };
        let target = g.resolve_next_scene("a", "go", &ctx).unwrap();
        match target {
            SceneTarget::Static { scene_id } => assert_eq!(scene_id, "dark_path"),
            _ => panic!("expected Static from conditional"),
        }
    }

    #[test]
    fn test_resolve_conditional_no_match_errors() {
        let mut g = SceneGraph::new("a".into());
        g.add_scene(scene_with_choice(
            "a",
            SceneTarget::Conditional {
                branches: vec![ConditionalTarget {
                    condition: TransitionCondition::HasItem {
                        item: "key".into(),
                    },
                    target: "locked".into(),
                }],
            },
        ))
        .unwrap();

        let result = g.resolve_next_scene("a", "go", &default_context());
        assert!(result.is_err());
    }

    #[test]
    fn test_resolve_missing_choice_errors() {
        let mut g = SceneGraph::new("a".into());
        g.add_scene(simple_scene("a")).unwrap();
        assert!(g
            .resolve_next_scene("a", "nonexistent", &default_context())
            .is_err());
    }

    // -- Condition evaluation ---------------------------------------------

    #[test]
    fn test_condition_fear_above() {
        let mut scores = HashMap::new();
        scores.insert(FearType::Isolation, 0.8);
        let ctx = ResolutionContext {
            fear_scores: scores,
            ..default_context()
        };
        assert!(evaluate_condition(
            &TransitionCondition::FearAboveThreshold {
                fear: FearType::Isolation,
                threshold: 0.5,
            },
            &ctx,
        ));
        assert!(!evaluate_condition(
            &TransitionCondition::FearAboveThreshold {
                fear: FearType::Isolation,
                threshold: 0.9,
            },
            &ctx,
        ));
    }

    #[test]
    fn test_condition_fear_below() {
        let ctx = default_context(); // defaults to 0.5
        assert!(evaluate_condition(
            &TransitionCondition::FearBelowThreshold {
                fear: FearType::Darkness,
                threshold: 0.6,
            },
            &ctx,
        ));
    }

    #[test]
    fn test_condition_phase_is() {
        let ctx = ResolutionContext {
            game_phase: GamePhase::Escalating,
            ..default_context()
        };
        assert!(evaluate_condition(
            &TransitionCondition::PhaseIs {
                phase: GamePhase::Escalating,
            },
            &ctx,
        ));
        assert!(!evaluate_condition(
            &TransitionCondition::PhaseIs {
                phase: GamePhase::Reveal,
            },
            &ctx,
        ));
    }

    #[test]
    fn test_condition_has_item() {
        let ctx = ResolutionContext {
            inventory: vec!["flashlight".into()],
            ..default_context()
        };
        assert!(evaluate_condition(
            &TransitionCondition::HasItem {
                item: "flashlight".into(),
            },
            &ctx,
        ));
        assert!(!evaluate_condition(
            &TransitionCondition::HasItem {
                item: "key".into(),
            },
            &ctx,
        ));
    }

    #[test]
    fn test_condition_scene_visited() {
        let ctx = ResolutionContext {
            visited_scenes: HashSet::from(["lobby".into()]),
            ..default_context()
        };
        assert!(evaluate_condition(
            &TransitionCondition::SceneVisited {
                scene_id: "lobby".into(),
            },
            &ctx,
        ));
    }

    #[test]
    fn test_condition_random() {
        let ctx = default_context();
        assert!(evaluate_condition(
            &TransitionCondition::Random { probability: 0.8 },
            &ctx,
        ));
        assert!(!evaluate_condition(
            &TransitionCondition::Random { probability: 0.3 },
            &ctx,
        ));
    }

    // -- Validation -------------------------------------------------------

    #[test]
    fn test_validate_catches_orphans() {
        let mut g = SceneGraph::new("a".into());
        g.add_scene(scene_with_choice(
            "a",
            SceneTarget::Static {
                scene_id: "b".into(),
            },
        ))
        .unwrap();
        g.add_scene(simple_scene("b")).unwrap();
        g.add_scene(simple_scene("orphan")).unwrap();

        let warnings = g.validate().unwrap();
        assert!(warnings
            .iter()
            .any(|w| matches!(w, ValidationWarning::OrphanScene { scene_id } if scene_id == "orphan")));
    }

    #[test]
    fn test_validate_catches_dead_ends() {
        let mut g = SceneGraph::new("a".into());
        g.add_scene(scene_with_choice(
            "a",
            SceneTarget::Static {
                scene_id: "b".into(),
            },
        ))
        .unwrap();
        g.add_scene(simple_scene("b")).unwrap(); // no choices = dead end

        let warnings = g.validate().unwrap();
        assert!(warnings
            .iter()
            .any(|w| matches!(w, ValidationWarning::DeadEnd { scene_id } if scene_id == "b")));
    }

    #[test]
    fn test_validate_detects_cycles() {
        let mut g = SceneGraph::new("a".into());
        g.add_scene(scene_with_choice(
            "a",
            SceneTarget::Static {
                scene_id: "b".into(),
            },
        ))
        .unwrap();
        g.add_scene(scene_with_choice(
            "b",
            SceneTarget::Static {
                scene_id: "a".into(),
            },
        ))
        .unwrap();

        let warnings = g.validate().unwrap();
        assert!(warnings
            .iter()
            .any(|w| matches!(w, ValidationWarning::Cycle { .. })));
    }

    #[test]
    fn test_validate_catches_missing_target() {
        let mut g = SceneGraph::new("a".into());
        g.add_scene(scene_with_choice(
            "a",
            SceneTarget::Static {
                scene_id: "nowhere".into(),
            },
        ))
        .unwrap();

        let warnings = g.validate().unwrap();
        assert!(warnings
            .iter()
            .any(|w| matches!(w, ValidationWarning::MissingTarget { target_id, .. } if target_id == "nowhere")));
    }

    #[test]
    fn test_validate_single_scene_no_dead_end_warning() {
        let mut g = SceneGraph::new("a".into());
        g.add_scene(simple_scene("a")).unwrap();
        let warnings = g.validate().unwrap();
        // A single scene with no choices is fine (end-of-game).
        assert!(!warnings
            .iter()
            .any(|w| matches!(w, ValidationWarning::DeadEnd { .. })));
    }

    #[test]
    fn test_validate_empty_graph() {
        let g = SceneGraph::new("a".into());
        let warnings = g.validate().unwrap();
        assert!(warnings.is_empty());
    }

    // -- Property test ----------------------------------------------------

    proptest! {
        #[test]
        fn test_scene_graph_add_is_consistent(count in 1usize..20) {
            let mut g = SceneGraph::new("s_0".into());
            for i in 0..count {
                let id = format!("s_{i}");
                let target_id = format!("s_{}", (i + 1) % count);
                let scene = scene_with_choice(
                    &id,
                    SceneTarget::Static { scene_id: target_id },
                );
                g.add_scene(scene).unwrap();
            }
            prop_assert_eq!(g.all_scene_ids().len(), count);
            // Every scene should be gettable
            for i in 0..count {
                let id = format!("s_{i}");
                prop_assert!(g.get_scene(&id).is_ok());
            }
        }
    }
}
