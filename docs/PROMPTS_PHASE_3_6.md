# PROMPTS — Phase 3–6 Claude Code Execution Prompts

---

## Phase 3: Fear Profiling Engine

---

### Task 11 Prompt: Behavior Event Schema & Collector

```
# Task 11: Behavior Event Schema & Collector

Working in `crates/fear-profile/src/behavior.rs`. Implement behavior event types and the server-side collector.

## behavior.rs — Event Types

All BehaviorEventType variants from common::types need full implementation here with:
- Validation methods (reject impossible values: negative speeds, future timestamps)
- Feature extraction helpers
- Scene context association

```rust
pub struct BehaviorBatch {
    pub events: Vec<BehaviorEvent>,
    pub session_id: String,
    pub batch_timestamp: DateTime<Utc>,
}

impl BehaviorBatch {
    pub fn validate(&self) -> Result<()> {
        // Check: no future timestamps
        // Check: no negative durations/speeds
        // Check: batch not empty
        // Check: events are roughly ordered by timestamp
    }
}
```

## Collector

Create a `BehaviorCollector` that:
1. Receives validated `BehaviorBatch` from WebSocket handler
2. Stores raw events to database (via storage crate)
3. Emits `BehaviorRecorded` event on the event bus (when we integrate later)
4. Maintains a sliding window of recent events per session (last 60 seconds)

```rust
pub struct BehaviorCollector {
    db: Arc<Database>,
    recent_events: HashMap<String, VecDeque<BehaviorEvent>>, // session_id -> recent events
    window_duration: Duration, // 60 seconds
}

impl BehaviorCollector {
    pub fn new(db: Arc<Database>) -> Self;
    pub async fn process_batch(&mut self, batch: BehaviorBatch) -> Result<()>;
    pub fn get_recent_events(&self, session_id: &str) -> &[BehaviorEvent];
    pub fn clear_session(&mut self, session_id: &str);
}
```

## Testing
- All BehaviorEventType variants create and validate correctly
- Batch validation catches future timestamps
- Batch validation catches negative values
- Collector stores to database
- Collector maintains sliding window
- Sliding window evicts old events
- Empty batch handling
- Malformed event handling
- Property test: all valid events serialize/deserialize correctly

Run `cargo test -p fear-engine-profile` to verify.
```

---

### Task 12 Prompt: Frontend Behavior Tracker

```
# Task 12: Frontend Behavior Tracker

Working in `frontend/src/`. Implement invisible behavior tracking.

## src/systems/BehaviorCollector.ts

Core class that captures and batches behavior events:

```typescript
export class BehaviorCollector {
  private events: BehaviorEvent[] = [];
  private lastKeystrokeTime: number = 0;
  private maxScrollPosition: number = 0;
  private mousePositions: Array<{x: number, y: number, t: number}> = [];
  private choiceDisplayTime: number = 0;
  private currentSceneId: string = '';
  
  constructor(private sendBatch: (batch: BehaviorBatch) => void) {}
  
  // Start listening to DOM events
  attach(): void { ... }
  
  // Stop listening
  detach(): void { ... }
  
  // Capture handlers (all private)
  private handleKeydown(e: KeyboardEvent): void { ... }
  private handleMouseMove(e: MouseEvent): void { ... }
  private handleScroll(): void { ... }
  
  // Called by ChoicePanel when choices are displayed
  recordChoiceDisplayed(sceneId: string): void { ... }
  
  // Called by ChoicePanel when a choice is clicked
  recordChoiceSelected(choiceId: string): void { ... }
  
  // Send batch and clear
  private flush(): void { ... }
  
  // Scene context tracking
  setCurrentScene(sceneId: string): void { ... }
}
```

Key implementation details:
1. Keystroke speed: track time between keydown events, compute chars/sec
2. Mouse tremor: keep last 500ms of positions, compute direction variance
3. Pause detection: if no input for 3+ seconds after scene text finishes
4. Rereading: scroll position goes below 70% of max seen position
5. Batch every 2 seconds via setInterval
6. Use performance.now() for high-resolution timing

## src/hooks/useBehaviorTracker.ts

React hook wrapper:
```typescript
export function useBehaviorTracker(ws: WebSocket | null, sceneId: string) {
  // Creates BehaviorCollector
  // Attaches on mount, detaches on unmount
  // Sends batches via WebSocket
  // Updates scene context when sceneId changes
  // Returns: { recordChoiceDisplayed, recordChoiceSelected }
}
```

## IMPORTANT: The tracker must be COMPLETELY INVISIBLE to the player.
- No visual indicators
- No performance impact (< 1ms per event)
- Events are batched, not sent individually
- Use passive event listeners where possible

## Testing
Use vitest with JSDOM:
- Keystroke speed calculation accuracy
- Mouse tremor detection from simulated rapid movements
- Pause detection after simulated inactivity
- Rereading detection from scroll position
- Batch timing (fires every 2 seconds)
- Scene context updates correctly
- Hook attaches and detaches with component lifecycle
- No dropped events under rapid input

Run `cd frontend && npm run test` to verify.
```

---

### Task 13 Prompt: Fear Scoring Algorithm

```
# Task 13: Fear Scoring Algorithm

Working in `crates/fear-profile/src/`. This is the intellectual core of the project — the Bayesian fear scoring system.

## src/analyzer.rs — Feature Extraction

Extract BehaviorFeatures from raw events:

```rust
pub struct BehaviorFeatures {
    pub hesitation_score: f64,       // 0-1: typing slowdown + pauses relative to baseline
    pub anxiety_score: f64,          // 0-1: mouse tremor + short responses + rapid input
    pub avoidance_score: f64,        // 0-1: flee/avoid choice ratio
    pub engagement_score: f64,       // 0-1: response length + rereading + investigation choices
    pub indecision_score: f64,       // 0-1: backspace ratio + long choice deliberation
    pub fight_or_flight_ratio: f64,  // 0=pure flight, 1=pure fight
}

impl BehaviorFeatures {
    /// Extract features from a window of behavior events.
    /// `baseline` provides the player's normal behavior for comparison.
    pub fn extract(events: &[BehaviorEvent], baseline: &BehaviorBaseline) -> Self { ... }
}

pub struct BehaviorBaseline {
    pub avg_typing_speed: f64,
    pub avg_response_time_ms: f64,
    pub avg_choice_time_ms: f64,
    pub avg_mouse_velocity: f64,
}

impl BehaviorBaseline {
    pub fn compute(calibration_events: &[BehaviorEvent]) -> Self { ... }
}
```

Feature extraction logic:
- hesitation: (baseline_typing_speed - current_typing_speed) / baseline_typing_speed, clamped 0-1
- anxiety: weighted combination of (mouse_tremor > threshold) + (response_length < baseline * 0.5) + (typing_speed > baseline * 1.5)
- avoidance: count(flee|avoid choices) / total_choices
- engagement: count(investigate|interact choices) / total_choices + rereading_events / total_scenes
- indecision: backspace_ratio + normalized(choice_time / avg_choice_time)
- fight_or_flight: count(fight choices) / count(fight + flight choices)

## src/scorer.rs — Bayesian Scoring

The likelihood matrix maps behavior features to fear categories.

```rust
pub struct FearScorer {
    likelihood_matrix: HashMap<FearType, FeatureLikelihoods>,
    smoothing_factor: f64,  // Exponential moving average alpha (0.3)
}

pub struct FeatureLikelihoods {
    pub hesitation: f64,
    pub anxiety: f64,
    pub avoidance: f64,
    pub engagement: f64,    // negative = engagement reduces this fear
    pub indecision: f64,
    pub flight_bias: f64,   // how much flight behavior indicates this fear
}

impl FearScorer {
    pub fn new() -> Self { /* Initialize with the weight matrix from ARCHITECTURE.md */ }
    
    /// Compute the likelihood P(features | fear_type) for a specific fear
    pub fn likelihood(&self, fear: &FearType, features: &BehaviorFeatures) -> f64 { ... }
    
    /// Compute the evidence P(features) = sum over all fears of P(features|fear)*P(fear)
    pub fn evidence(&self, features: &BehaviorFeatures, priors: &HashMap<FearType, f64>) -> f64 { ... }
    
    /// Full Bayesian update: returns new posterior for each fear
    pub fn update_scores(
        &self,
        priors: &HashMap<FearType, f64>,
        features: &BehaviorFeatures,
    ) -> Result<HashMap<FearType, f64>> {
        // For each fear type:
        //   posterior = (likelihood * prior) / evidence
        //   Apply EMA smoothing: new = alpha * posterior + (1-alpha) * prior
        //   Clamp to [0.0, 1.0]
    }
}
```

IMPLEMENT THE ACTUAL MATH. No shortcuts. The Bayesian update must be mathematically correct.

## Testing

This module needs the most thorough testing:

Unit tests for feature extraction:
- Slow typing → high hesitation score
- Mouse tremor → high anxiety score  
- Flee choices → high avoidance score
- Long responses + rereading → high engagement
- Many backspaces → high indecision

Unit tests for Bayesian math:
- High likelihood + high prior → higher posterior
- Low likelihood + high prior → lower posterior
- Scores always in [0.0, 1.0]
- Zero evidence error handling
- EMA smoothing prevents wild swings

End-to-end scoring tests:
- Simulated "claustrophobic player" events → claustrophobia rises
- Simulated "curious explorer" events → engagement high, fears low
- Simulated "anxious avoider" events → multiple fears rise

Property tests (proptest):
- All scores always in [0, 1] for any input
- Confidence increases monotonically with observations
- Deterministic (same inputs → same outputs)

Snapshot tests (insta):
- Scoring output for "high claustrophobia" fixture
- Scoring output for "curious explorer" fixture  
- Scoring output for "anxious avoider" fixture

Run `cargo test -p fear-engine-profile` to verify.
```

---

### Task 14 Prompt: Fear Profile Builder

```
# Task 14: Fear Profile Builder

Working in `crates/fear-profile/src/profile.rs`. Build the FearProfile struct and update pipeline.

## Implementation

```rust
pub struct FearProfile {
    scores: HashMap<FearType, f64>,
    confidence: HashMap<FearType, FearConfidence>,
    meta: MetaPatterns,
    baseline: Option<BehaviorBaseline>,
    update_count: u32,
    history: Vec<FearProfileSnapshot>,
}

pub struct FearConfidence {
    pub observations: u32,
    pub recent_variance: f64,
    pub last_significant_change: Option<Instant>,
}

pub struct MetaPatterns {
    pub anxiety_threshold: f64,
    pub recovery_speed: f64,
    pub curiosity_vs_avoidance: f64,
}

pub struct FearProfileSnapshot {
    pub scores: HashMap<FearType, f64>,
    pub timestamp: Instant,
    pub trigger: String,
}

impl FearProfile {
    pub fn new() -> Self; // All scores at 0.5, all confidence at 0
    
    pub fn set_baseline(&mut self, baseline: BehaviorBaseline);
    
    pub fn update(&mut self, features: &BehaviorFeatures) -> Result<ProfileUpdateResult>;
    // Uses FearScorer internally
    // Updates confidence tracking
    // Takes snapshot if significant change
    // Returns which fears changed significantly
    
    pub fn top_fears(&self, n: usize, min_confidence: f64) -> Vec<(FearType, f64)>;
    
    pub fn primary_fear(&self) -> Option<(FearType, f64)>;
    // Highest score with confidence > 0.5
    
    pub fn snapshot(&self) -> FearProfileSnapshot;
    
    pub fn to_prompt_context(&self) -> String;
    // Formats profile for AI prompt Layer 2
    
    pub fn to_reveal_data(&self) -> RevealData;
    // For the end-game fear reveal screen
    
    pub fn confidence_level(&self, fear: &FearType) -> f64;
    // 0-1 confidence for a specific fear
    
    pub fn reset(&mut self);
}

pub struct ProfileUpdateResult {
    pub significant_changes: Vec<(FearType, f64, f64)>, // fear, old, new
    pub new_primary_fear: Option<FearType>,
    pub phase_transition_recommended: bool,
}
```

## Testing
- New profile has 0.5 for all scores
- Update changes scores based on features
- Multiple updates cause convergence
- top_fears returns correct ordering
- top_fears filters by confidence
- Snapshot captures current state independently
- to_prompt_context produces valid string
- Confidence increases with observations
- reset returns to initial state
- Persistence roundtrip (serialize → deserialize → equal)
- ProfileUpdateResult correctly identifies significant changes

Run `cargo test -p fear-engine-profile` to verify.
```

---

### Task 15 Prompt: Adaptation Strategy Engine

```
# Task 15: Adaptation Strategy Engine

Working in `crates/fear-profile/src/adaptation.rs`.

## Implementation

```rust
pub struct AdaptationEngine {
    current_strategy: Option<AdaptationStrategy>,
    intensity_curve: IntensityCurve,
    scene_count: u32,
    last_calm_scene: u32,
}

pub struct IntensityCurve {
    // Piecewise linear: scene_number → target intensity
    points: Vec<(u32, f64)>,
}

pub struct AdaptationDirective {
    pub strategy: AdaptationStrategy,
    pub intensity_target: f64,
    pub primary_fear: Option<FearType>,
    pub secondary_fears: Vec<FearType>,
    pub specific_instruction: String,  // Natural language for the AI
    pub forbidden_elements: Vec<String>,  // Things to avoid for pacing
}

impl AdaptationEngine {
    pub fn new() -> Self;
    
    pub fn compute_directive(
        &mut self,
        phase: GamePhase,
        profile: &FearProfile,
        scene_count: u32,
    ) -> AdaptationDirective;
    
    fn select_strategy(&self, phase: GamePhase, profile: &FearProfile) -> AdaptationStrategy;
    
    fn compute_intensity(&self, phase: GamePhase, scene_count: u32) -> f64;
    
    fn generate_instruction(&self, strategy: &AdaptationStrategy, profile: &FearProfile) -> String;
    // Generates natural language instructions like:
    // "The player shows strong fear of enclosed spaces. Describe the room getting smaller, 
    //  doors that won't open, walls closing in. But do it subtly — they noticed the last 
    //  direct approach."
    
    fn determine_forbidden(&self, phase: GamePhase, recent_fears_used: &[FearType]) -> Vec<String>;
    // Prevent overusing same fear type back-to-back
}

impl IntensityCurve {
    pub fn default_horror_curve() -> Self {
        // Scene 1-3: 0.2-0.3 (calibration, mild)
        // Scene 4-8: 0.3-0.5 (exploration, building)
        // Scene 9-12: 0.5-0.7 (escalation, rising)
        // Scene 13: 0.4 (contrast valley)
        // Scene 14-16: 0.7-0.9 (escalation peak)
        // Scene 17-18: 0.9-1.0 (climax)
    }
}
```

Strategy selection logic:
- Calibrating: Always Probe, cycling through fear categories
- Exploring + low confidence: Probe the lowest-confidence fears
- Exploring + some confidence: GradualEscalation for top fear, Probe for others
- Escalating: Layering (combine top 2 fears) + Contrast (periodic calm scenes)
- Climax: Maximum Layering + Subversion for final shock

## Testing
- Calibrating phase always returns Probe
- Exploring with no confident fears returns Probe
- Exploring with confirmed fears returns GradualEscalation
- Escalating uses Layering with top 2 fears
- Contrast inserts calm after N intense scenes
- Climax reaches maximum intensity
- Intensity curve has correct shape
- Directive instruction is non-empty and coherent
- Forbidden elements prevent repetition
- Snapshot tests for directive at each phase

Run `cargo test -p fear-engine-profile` to verify.
```

---

## Phase 4: AI Integration

---

### Task 16 Prompt: Claude API Client

```
# Task 16: Anthropic Claude API Client

Working in `crates/ai-integration/src/claude_client.rs`.

Build a production-grade async HTTP client for the Anthropic Messages API.

## Implementation

```rust
pub struct ClaudeClient {
    http: reqwest::Client,
    api_key: String,
    base_url: String,
    model: String,
    max_tokens: u32,
    rate_limiter: TokenBucketRateLimiter,
    config: ClientConfig,
}

pub struct ClientConfig {
    pub timeout: Duration,
    pub max_retries: u32,
    pub base_retry_delay: Duration,
    pub max_retry_delay: Duration,
}

pub struct GenerateRequest {
    pub system_prompt: String,
    pub messages: Vec<Message>,
    pub temperature: f64,
}

pub struct Message {
    pub role: Role,
    pub content: String,
}

pub enum Role { User, Assistant }

pub struct GenerateResponse {
    pub content: String,
    pub model: String,
    pub usage: TokenUsage,
    pub stop_reason: String,
}

pub struct TokenUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
}

struct TokenBucketRateLimiter {
    tokens: AtomicU32,
    max_tokens: u32,
    refill_rate: u32, // tokens per second
    last_refill: Mutex<Instant>,
}
```

Key requirements:
1. Retry logic: exponential backoff (1s, 2s, 4s) on 5xx and 429 errors
2. NO retry on 4xx errors (except 429)
3. Rate limiter: token bucket algorithm, configurable rate
4. Timeout: configurable, default 30s
5. Request logging at debug level
6. Proper error mapping to FearEngineError variants

API endpoint: POST https://api.anthropic.com/v1/messages
Headers: x-api-key, anthropic-version: 2023-06-01, content-type: application/json

## Testing
Use `wiremock` crate for HTTP mocking:
- Successful request/response
- Retry on 500 error
- Retry on 429 with backoff
- No retry on 400 error
- Timeout handling
- Rate limiter throttling
- Malformed response handling
- Auth error (401) handling
- Token usage parsing

Run `cargo test -p fear-engine-ai-integration` to verify.
```

---

### Task 17 Prompt: Prompt Engineering System

```
# Task 17: Prompt Engineering System

Working in `crates/ai-integration/src/prompt/`. Build the multi-layer prompt builder.

## Implementation

### src/prompt/system.rs
The constant system prompt. Hand-craft this — it's critical for output quality.
Store as a const &str. See ARCHITECTURE.md § AI Prompt Architecture for the full text.

### src/prompt/context.rs
Dynamic context builder:

```rust
pub struct PromptContext {
    pub fear_profile: FearProfileContext,
    pub game_state: GameStateContext,
    pub adaptation: AdaptationDirective,
}

pub struct FearProfileContext {
    pub top_fears: Vec<(FearType, f64, f64)>, // fear, score, confidence
    pub anxiety_threshold: f64,
    pub behavioral_pattern: String,
    pub estimated_emotional_state: String,
}

pub struct GameStateContext {
    pub location: String,
    pub phase: GamePhase,
    pub scene_number: u32,
    pub last_scene_summary: String,
    pub last_choice: String,
    pub active_threads: Vec<String>,
    pub inventory: Vec<String>,
    pub established_details: Vec<String>,
}

impl PromptContext {
    pub fn build_fear_layer(&self) -> String { ... }
    pub fn build_game_state_layer(&self) -> String { ... }
}
```

### src/prompt/output.rs
Output schema definition:
```rust
pub fn output_schema_prompt() -> &'static str {
    // The JSON schema instruction from ARCHITECTURE.md § Output Schema
}
```

### src/prompt/mod.rs
Full prompt assembler:
```rust
pub struct PromptBuilder;

impl PromptBuilder {
    pub fn build_system_prompt() -> String { ... }
    
    pub fn build_user_message(context: &PromptContext) -> String {
        // Combines: fear context + game state + output schema + "Generate the next scene"
    }
    
    pub fn estimate_tokens(prompt: &str) -> u32 {
        // Approximate: chars / 4
    }
}
```

## Testing
- System prompt is non-empty and contains key instructions
- Fear context includes top fears with scores
- Game state includes location and history
- Full prompt assembly combines all layers
- Token estimation is reasonable
- SNAPSHOT TESTS for prompts at each game phase (critical to catch unintended changes)

Run `cargo test -p fear-engine-ai-integration` to verify.
```

---

### Tasks 18-20 follow the same detailed prompt pattern.

For Tasks 18 (Narrative Pipeline), 19 (Image Gen), 20 (Safety & Cache), refer to the task descriptions in TASKS_PHASE_3_4.md and follow the same structure: concrete Rust types, specific implementation logic, comprehensive tests.

---

## Phase 5: Frontend Horror (Tasks 21-26)

### Task 21-26 Prompts

For each frontend task, the prompt structure is:

```
# Task N: [Component Name]

Working in `frontend/src/`.

## Component Specification
[Detailed props, state, behavior description]

## CSS/Animation Details
[Keyframes, transitions, effect parameters]

## Implementation Notes
[Specific code patterns, performance considerations]

## Testing
[Vitest + React Testing Library tests]

Run `cd frontend && npm run test` to verify.
```

Specific notes per task:

**Task 21 (Design System)**: Create globals.css with all CSS custom properties, import fonts, define keyframe animations. Output the full color system, spacing scale, and typography scale.

**Task 22 (Typewriter)**: Use requestAnimationFrame for smooth character reveal. Implement glitch by showing random chars for 50ms before correct char. Handle HTML entities. Performance critical — no layout thrashing.

**Task 23 (Scene Renderer)**: Use Framer Motion for transitions. AnimatePresence for enter/exit. scene history in a scrollable container with auto-scroll to bottom.

**Task 24 (Choice Panel)**: Stagger reveal with Framer Motion staggerChildren. Track hover time per choice. Send timing data to behavior tracker.

**Task 25 (Horror Image)**: Use CSS filters for glitch (hue-rotate, contrast). Fade-in via opacity transition. Flash mode: show for 200ms then fade to 30% opacity.

**Task 26 (Audio)**: Create AudioContext lazily (after user interaction). Use OscillatorNode for drone, GainNode for volume. Heartbeat via AudioBuffer with procedural generation.

---

## Phase 6: Integration & Polish (Tasks 27-30)

### Task 27 Prompt: End-to-End Integration

```
# Task 27: End-to-End Game Loop Integration

This is the integration task. Wire everything together.

## Steps:

1. **Server main.rs**: Initialize all subsystems:
   - Database + migrations
   - BehaviorCollector
   - FearScorer + FearProfile (per session)
   - AdaptationEngine (per session)
   - ClaudeClient
   - SceneGraph with hospital scenario
   - GameStateMachine (per session)

2. **WebSocket handler**: Route messages to correct subsystem:
   - StartGame → create session, send first scene
   - Choice → update scene, check phase transition, generate next scene
   - BehaviorBatch → process → update fear profile → potentially adapt
   - Send: Narrative, Image, PhaseChange, Meta, Error

3. **Game loop per session**:
   ```
   on_choice(choice_id):
     1. Record choice behavior
     2. Resolve next scene from graph
     3. If dynamic: build prompt → call Claude API → parse response
     4. If image_prompt: async generate image
     5. Check phase transition conditions
     6. Send narrative to client
     7. If meta_break: send meta message
   
   on_behavior_batch(events):
     1. Validate and store events
     2. If calibrating and enough data: compute baseline
     3. Extract features relative to baseline
     4. Update fear profile
     5. If significant change: log and potentially adjust strategy
   ```

4. **Frontend App.tsx**: Wire all components:
   - GameScreen uses NarrativeDisplay, ChoicePanel, HorrorImage
   - BehaviorTracker runs continuously
   - Audio responds to intensity changes
   - FearReveal shown when reveal phase reached

## Testing
- Simulated full game with mock Claude API
- Game handles API failures gracefully (fallback scenes)
- Disconnection and reconnection works
- Fear profile evolves over course of game
- Phase transitions happen at right times
- E2E Playwright test for complete playthrough

Run full test suite: `cargo test --workspace && cd frontend && npm run test`
```

### Tasks 28-30: Follow task descriptions from TASKS_PHASE_5_6.md with same prompt structure.
