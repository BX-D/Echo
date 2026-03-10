# PROMPTS.md — Claude Code Execution Prompts

> Copy-paste these prompts into Claude Code for each task.
> Each prompt includes context, specific instructions, and verification steps.
> After each task, run the verification prompt to confirm completion.

---

## 🔄 Pre-Session Context Prompt

**Run this FIRST every time you start a new Claude Code session:**

```
Read the CLAUDE.md file in the project root. This is the master context document for the "It Learns Your Fear" project — an AI-powered adaptive horror game built with Rust (Axum) backend and React (TypeScript) frontend.

Key rules:
1. NO placeholder code, NO todo!(), NO unimplemented!() — every function must be fully implemented
2. ALL public functions need doc comments with examples
3. ALL modules need unit tests in #[cfg(test)] mod tests {}
4. NO .unwrap() in non-test code — use ? operator with custom errors
5. Use proptest for numeric computations, insta for snapshot tests
6. Follow the exact file structure defined in CLAUDE.md

Read PROGRESS.md to see which tasks are completed and what task we're on next.
```

---

## 📊 Post-Task Verification Prompt

**Run this AFTER every task to verify and update progress:**

```
Verify Task N is complete:
1. Run `cargo build --workspace` — must succeed with zero warnings
2. Run `cargo clippy --workspace -- -D warnings` — must pass
3. Run `cargo test --workspace` — all tests must pass
4. For frontend tasks: run `cd frontend && npm run build && npm run test`
5. Count the tests added in this task and report: "X tests added, Y total passing"

If all checks pass, update PROGRESS.md:
- Mark Task N as ✅ COMPLETE
- Record the date and test count
- Set "Next Task" to Task N+1

If any checks fail, fix the issues before marking complete.
```

---

## Phase 1: Foundation

---

### Task 1 Prompt: Project Scaffolding

```
# Task 1: Project Scaffolding

Create the Rust workspace and React frontend for "It Learns Your Fear" — an AI horror game.

## Rust Workspace Setup

Create `fear-engine/Cargo.toml` as a workspace with these member crates:
- crates/common — shared types and errors
- crates/storage — SQLite persistence  
- crates/core — game engine
- crates/fear-profile — fear analysis engine
- crates/ai-integration — LLM and image generation
- crates/server — Axum HTTP/WebSocket server (this is the binary crate)

Each crate's Cargo.toml should have appropriate dependencies:
- common: serde, serde_json, thiserror, chrono, uuid
- storage: rusqlite (with bundled feature), r2d2, r2d2_sqlite + common
- core: tokio, uuid + common
- fear-profile: + common (proptest as dev-dependency)
- ai-integration: reqwest (with json feature), tokio + common
- server: axum (with ws feature), tokio (full), tower, tower-http (cors) + all other crates

Every crate lib.rs should have:
- Module doc comment explaining the crate's purpose
- A basic smoke test

Create `.rustfmt.toml`:
```toml
edition = "2021"
max_width = 100
tab_spaces = 4
use_field_init_shorthand = true
```

Create `.env.example` with:

```
ANTHROPIC_API_KEY=sk-ant-your-key-here
STABILITY_API_KEY=sk-your-key-here
DATABASE_URL=sqlite://fear_engine.db
RUST_LOG=fear_engine=debug
SERVER_HOST=127.0.0.1
SERVER_PORT=3001
FRONTEND_URL=http://localhost:5173
```

## React Frontend Setup

Create `frontend/` with Vite + React + TypeScript:

- package.json with: react, react-dom, typescript, vite, @vitejs/plugin-react, tailwindcss, postcss, autoprefixer, framer-motion, zustand, recharts
- Dev dependencies: vitest, @testing-library/react, @testing-library/jest-dom, jsdom, playwright
- tsconfig.json with strict mode
- vite.config.ts with proxy to backend WebSocket
- tailwind.config.js with custom horror theme colors
- Basic App.tsx that renders "Fear Engine — Loading..."
- Basic main.tsx entry point

## Project Files

Create PROGRESS.md:

```markdown
# Fear Engine — Progress Tracker

## Current Status
- Phase: 1 — Foundation
- Current Task: 1 — Project Scaffolding
- Tasks Completed: 0/30

## Task History
(will be filled as tasks complete)
```

## Verification

After creating everything:

1. Run `cargo build --workspace` — must compile
2. Run `cargo test --workspace` — smoke tests pass
3. Run `cd frontend && npm install && npm run build` — must succeed
4. Run `cargo clippy --workspace` — no warnings

```

---

### Task 2 Prompt: Common Types & Error Handling

```

# Task 2: Common Types & Error Handling

Working in `crates/common/src/`. Implement the shared foundation types for Fear Engine.

## Files to create/modify:

### src/error.rs

Create `FearEngineError` using thiserror:

Variants needed:

- Database(String) — SQLite errors
- WebSocket(String) — connection/message errors
- Serialization(String) — JSON parse/format errors
- AiGeneration(String) — LLM API errors
- ImageGeneration(String) — Image API errors
- InvalidState { current: String, attempted: String } — state machine violations
- InvalidInput { field: String, reason: String } — validation failures
- NotFound { entity: String, id: String } — resource lookup failures
- RateLimit { retry_after_ms: u64 } — API rate limiting
- Timeout { operation: String, duration_ms: u64 } — operation timeouts
- Configuration(String) — missing/invalid config

Implement From conversions for: rusqlite::Error, serde_json::Error, reqwest::Error
Create a type alias: `pub type Result<T> = std::result::Result<T, FearEngineError>;`

### src/types.rs

Implement these types with Serialize, Deserialize, Debug, Clone:

```rust
// Fear categories — the 10 fear axes
pub enum FearType { ... } // all 10 from ARCHITECTURE.md
// Implement Display, FromStr, and a method all() -> Vec<FearType>

// Game phases
pub enum GamePhase { Calibrating, Exploring, Escalating, Climax, Reveal }
// Implement Ord (phases have a natural order)

// Behavior event types — the raw signals from frontend
pub struct BehaviorEvent { event_type: BehaviorEventType, timestamp: chrono::DateTime<Utc>, scene_id: String }
pub enum BehaviorEventType { Keystroke{...}, Pause{...}, Choice{...}, MouseMovement{...}, Scroll{...} }

// Choice approach categories
pub enum ChoiceApproach { Investigate, Avoid, Confront, Flee, Interact, Wait }

// Scene atmosphere
pub enum Atmosphere { Dread, Tension, Panic, Calm, Wrongness, Isolation, Paranoia }

// Adaptation strategies
pub enum AdaptationStrategy { Probe{...}, GradualEscalation{...}, Contrast{...}, Layering{...}, Subversion{...} }

// AI response types
pub struct NarrativeResponse { narrative: String, atmosphere: Atmosphere, sound_cue: Option<String>, image_prompt: Option<String>, choices: Vec<Choice>, hidden_elements: Vec<String>, intensity: f64, meta_break: Option<MetaBreak> }
pub struct Choice { id: String, text: String, approach: ChoiceApproach, fear_vector: FearType }
pub struct MetaBreak { text: String, target: MetaTarget }
pub enum MetaTarget { Title, Overlay, Whisper, GlitchText }

// WebSocket message types
pub enum ClientMessage { StartGame{...}, Choice{...}, BehaviorBatch{...}, TextInput{...} }
pub enum ServerMessage { Narrative{...}, Image{...}, PhaseChange{...}, Meta{...}, Reveal{...}, Error{...} }

// Effect directives
pub struct EffectDirective { effect: EffectType, intensity: f64, duration_ms: u64, delay_ms: u64 }
pub enum EffectType { Shake, Flicker, Glitch, Darkness, Flashlight, Crt, SlowType, FastType }
```

### src/config.rs

App configuration struct that reads from env:

```rust
pub struct AppConfig {
    pub anthropic_api_key: String,
    pub stability_api_key: Option<String>,
    pub database_url: String,
    pub server_host: String,
    pub server_port: u16,
    pub frontend_url: String,
    pub log_level: String,
}
```

Implement `AppConfig::from_env()` with sensible defaults. Use std::env, NOT a config crate.

### src/lib.rs

Re-export everything:

```rust
pub mod error;
pub mod types;
pub mod config;
pub use error::{FearEngineError, Result};
```

## Testing Requirements

Write comprehensive tests for ALL types:

- Serialization roundtrip for every enum and struct
- Error display messages are human-readable
- Config falls back to defaults when env vars missing
- FearType::all() returns all 10 variants
- GamePhase ordering is correct
- Property test: FearType serialization is always valid

Run `cargo test -p fear-engine-common` to verify all tests pass.

```

---

### Task 3 Prompt: SQLite Storage Layer

```

# Task 3: SQLite Storage Layer

Working in `crates/storage/src/`. Implement the persistence layer with SQLite.

## Database Design

The schema is defined in ARCHITECTURE.md § Database Schema. Implement it exactly.

## Files to create:

### src/lib.rs

Database connection pool and initialization:

```rust
pub struct Database {
    pool: r2d2::Pool<r2d2_sqlite::SqliteConnectionManager>,
}

impl Database {
    pub fn new(database_url: &str) -> Result<Self> { ... }
    pub fn new_in_memory() -> Result<Self> { ... }  // For testing
    pub fn initialize(&self) -> Result<()> { ... }  // Run migrations
    fn get_conn(&self) -> Result<PooledConnection> { ... }
}
```

### src/migrations/ directory

Create `001_initial.sql` with all CREATE TABLE and CREATE INDEX statements from ARCHITECTURE.md.

### src/session.rs

Session CRUD:

- create_session(player_name: Option<&str>) -> Result`<String>` // returns session_id
- get_session(id: &str) -> Result`<Session>`
- update_session_phase(id: &str, phase: GamePhase) -> Result<()>
- update_session_state(id: &str, scene_id: &str, state_json: &str) -> Result<()>
- complete_session(id: &str) -> Result<()>
- list_active_sessions() -> Result<Vec`<Session>`>

### src/fear_profile.rs

Fear profile CRUD:

- create_fear_profile(session_id: &str) -> Result<()>
- get_fear_profile(session_id: &str) -> Result`<FearProfileRow>`
- update_fear_profile(session_id: &str, profile: &FearProfileRow) -> Result<()>
- All 10 fear scores + 3 meta patterns stored as REAL columns

### src/behavior_log.rs

Behavior event storage:

- insert_behavior_events(session_id: &str, events: &[BehaviorEvent]) -> Result<()>  // batch insert
- get_behavior_events(session_id: &str, since: Option`<DateTime>`) -> Result<Vec`<BehaviorEvent>`>
- count_behavior_events(session_id: &str) -> Result`<u64>`

### src/cache.rs (content cache)

- cache_set(key: &str, content_type: &str, content_json: &str, ttl: u32) -> Result<()>
- cache_get(key: &str) -> Result<Option`<CacheEntry>`>
- cache_cleanup_expired() -> Result`<u64>`  // delete expired entries

### src/scene_history.rs

- insert_scene_history(entry: &SceneHistoryEntry) -> Result<()>
- get_scene_history(session_id: &str) -> Result<Vec`<SceneHistoryEntry>`>
- get_latest_scene(session_id: &str) -> Result<Option`<SceneHistoryEntry>`>

## IMPORTANT: All tests use Database::new_in_memory() for isolation.

## Testing Requirements

Every CRUD function needs at least:

1. Happy path test
2. Not-found test (where applicable)
3. Duplicate/conflict test (where applicable)

Plus:

- test_concurrent_session_access (multi-threaded)
- test_content_cache_ttl_expiry (with time manipulation)
- test_batch_insert_performance (100+ events)

Run `cargo test -p fear-engine-storage` to verify.

```

---

### Task 4 Prompt: Axum Server with WebSocket

```

# Task 4: Axum Server with WebSocket

Working in `crates/server/src/`. Build the HTTP and WebSocket server.

## Architecture

The server uses Axum with:

- REST endpoints for health check and game management
- WebSocket endpoint for real-time game communication
- Shared state via Arc`<AppState>`

## Files to create:

### src/main.rs

Entry point:

- Load config from environment
- Initialize database
- Build Axum app
- Start server with graceful shutdown (tokio signal handling)

### src/app.rs

Axum app builder:

```rust
pub struct AppState {
    pub db: Database,
    pub sessions: DashMap<String, GameSession>, // Active sessions
}

pub fn build_app(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/health", get(health_check))
        .route("/ws", get(ws_upgrade))
        .route("/api/game", post(create_game))
        .layer(CorsLayer::new()
            .allow_origin(config.frontend_url.parse().unwrap())
            .allow_methods([Method::GET, Method::POST])
            .allow_headers([CONTENT_TYPE]))
        .with_state(state)
}
```

### src/ws/handler.rs

WebSocket connection handler:

```rust
pub async fn ws_upgrade(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: Arc<AppState>) {
    // 1. Split into sender/receiver
    // 2. Create game session
    // 3. Send welcome message
    // 4. Enter message loop:
    //    - Receive: parse ClientMessage, route to handler
    //    - Send: forward ServerMessage from game engine
    // 5. Cleanup on disconnect
}
```

### src/ws/messages.rs

WebSocket message serialization/deserialization helpers.
All message types from common::types, with JSON encoding.

### src/ws/session.rs

Per-connection game session:

```rust
pub struct GameSession {
    pub session_id: String,
    pub game_phase: GamePhase,
    pub sender: mpsc::Sender<ServerMessage>,
    pub created_at: Instant,
}
```

### src/routes/health.rs

Simple health check: returns {"status": "ok", "version": "0.1.0"}

### src/routes/game.rs

POST /api/game — creates a new game session, returns session_id
(alternative to WebSocket-based creation for testing)

### src/middleware/cors.rs

CORS configuration module.

## Testing Requirements

Use axum::test helpers and tokio_tungstenite for WebSocket tests.

Key tests:

1. Health endpoint returns 200 with correct body
2. WebSocket connects successfully
3. WebSocket receives welcome message on connect
4. Sending StartGame creates a session
5. Sending invalid JSON returns error message
6. Sending BehaviorBatch is acknowledged
7. Disconnect cleans up session
8. CORS headers present on responses
9. Multiple concurrent WebSocket connections work

Run `cargo test -p fear-engine-server` to verify.

```

---

### Task 5 Prompt: React Shell with WebSocket Client

```

# Task 5: React Frontend Shell with WebSocket

Working in `frontend/src/`. Build the React application shell.

## State Management (Zustand)

### src/stores/gameStore.ts

```typescript
interface GameState {
  // Connection
  connectionStatus: 'connecting' | 'connected' | 'disconnected' | 'error';
  
  // Game
  sessionId: string | null;
  gamePhase: GamePhase | null;
  currentScene: NarrativeMessage | null;
  sceneHistory: NarrativeMessage[];
  
  // Actions
  setConnectionStatus: (status: ConnectionStatus) => void;
  setSessionId: (id: string) => void;
  processNarrative: (msg: NarrativeMessage) => void;
  processPhaseChange: (msg: PhaseChangeMessage) => void;
  processMeta: (msg: MetaMessage) => void;
  processImage: (msg: ImageMessage) => void;
  processReveal: (msg: RevealMessage) => void;
  reset: () => void;
}
```

## WebSocket Hook

### src/hooks/useWebSocket.ts

Custom hook for WebSocket management:

- Connect to ws://localhost:3001/ws on mount
- Auto-reconnect with exponential backoff (1s, 2s, 4s, max 30s)
- Parse incoming messages and dispatch to Zustand store
- Expose send() function for outgoing messages
- Connection status tracking
- Heartbeat ping every 30s

## TypeScript Types

### src/types/game.ts, ws.ts, behavior.ts, narrative.ts

Mirror all the Rust types from common crate. Use TypeScript enums/types.

## Components

### src/App.tsx

Root component:

- Initializes WebSocket connection
- Routes to correct screen based on game state:
  - Connecting → LoadingScreen
  - Connected, no game → StartScreen
  - In game → GameScreen
  - Reveal phase → FearReveal

### src/components/StartScreen.tsx

Title screen:

- "It Learns Your Fear" title (horror font)
- Subtle flickering animation
- "Press Enter to begin..." pulsing text
- On enter: sends StartGame message

### src/components/LoadingScreen.tsx

Themed loading:

- Dark screen with pulsing dot
- "Connecting..." text
- Horror aesthetic (not a boring spinner)

## Styling

### src/styles/globals.css

- Import horror fonts (Special Elite, Creepster)
- Set dark background (#0a0a0a)
- Smooth scroll behavior
- Custom scrollbar styling (dark theme)
- Base text color (bone/cream)

### tailwind.config.js

Extend with horror color palette:

```javascript
colors: {
  void: '#0a0a0a',
  shadow: '#1a1a1a', 
  ash: '#2a2a2a',
  smoke: '#666666',
  bone: '#d4d0c8',
  parchment: '#e8e0d4',
  blood: '#8b0000',
  rust: '#a0522d',
  bile: '#556b2f',
  clinical: '#f0f8ff',
  bruise: '#4a0e4e',
  gangrene: '#2f4f4f',
}
```

## Testing

Write tests with vitest + @testing-library/react:

- useWebSocket: mock WebSocket, test connect/reconnect/message parsing
- gameStore: test all state transitions
- App: test screen routing based on state
- StartScreen: test enter key triggers game start
- LoadingScreen: renders correctly

Run `cd frontend && npm run test` to verify.

```

---

## Phase 2: Game Engine

---

### Task 6 Prompt: Scene Data Model

```

# Task 6: Scene Data Model & Scene Graph

Working in `crates/core/src/`. Implement the scene system.

## src/scene.rs — Scene Data Model

```rust
/// A single scene in the horror game.
/// Scenes can be static (pre-written) or dynamic (AI-generated).
pub struct Scene {
    pub id: String,
    pub scene_type: SceneType,
    pub narrative: String,
    pub atmosphere: Atmosphere,
    pub choices: Vec<SceneChoice>,
    pub effects: Vec<EffectDirective>,
    pub sound_cue: Option<String>,
    pub image_prompt: Option<String>,
    pub fear_targets: Vec<FearType>,  // which fears this scene is testing/targeting
    pub intensity: f64,               // 0.0 - 1.0
    pub meta_break: Option<MetaBreak>,
}

pub enum SceneType {
    Static,                          // Pre-written, always same content
    Template { placeholders: Vec<String> },  // Partially written with AI fill-in
    Dynamic,                         // Fully AI-generated
}

pub struct SceneChoice {
    pub id: String,
    pub text: String,
    pub approach: ChoiceApproach,
    pub fear_vector: FearType,
    pub target_scene: SceneTarget,
}

pub enum SceneTarget {
    Static(String),                  // Go to a specific scene ID
    Dynamic { context: String },     // Generate a new scene with this context
    Conditional(Vec<ConditionalTarget>),  // Branch based on conditions
}

pub struct ConditionalTarget {
    pub condition: TransitionCondition,
    pub target: String,  // scene ID
}

pub enum TransitionCondition {
    FearAboveThreshold { fear: FearType, threshold: f64 },
    FearBelowThreshold { fear: FearType, threshold: f64 },
    PhaseIs(GamePhase),
    HasItem(String),
    SceneVisited(String),
    Random { probability: f64 },
}
```

## Scene Graph

```rust
pub struct SceneGraph {
    scenes: HashMap<String, Scene>,
    start_scene_id: String,
}

impl SceneGraph {
    pub fn new(start_scene_id: String) -> Self;
    pub fn add_scene(&mut self, scene: Scene) -> Result<()>;
    pub fn get_scene(&self, id: &str) -> Result<&Scene>;
    pub fn resolve_next_scene(&self, current: &str, choice_id: &str, context: &ResolutionContext) -> Result<SceneTarget>;
    pub fn validate(&self) -> Result<Vec<ValidationWarning>>; // checks for orphans, dead ends, cycles
    pub fn all_scene_ids(&self) -> Vec<&str>;
}

pub struct ResolutionContext {
    pub fear_profile: FearProfile,  // We'll use a simplified version for now
    pub game_phase: GamePhase,
    pub inventory: Vec<String>,
    pub visited_scenes: HashSet<String>,
}
```

Implement full validation:

- No orphan scenes (unreachable from start)
- No dead-end scenes (scenes with no choices and no end marker)
- Detect cycles (warn, don't error — cycles can be intentional)
- All conditional targets reference existing scenes

## Testing

- Scene creation with all field types
- SceneGraph CRUD operations
- Scene graph traversal from start
- Conditional transition resolution
- Validation catches orphans
- Validation catches dead ends
- Validation detects cycles
- Property test: scene graph always has valid transitions after add

Run `cargo test -p fear-engine-core` to verify.

```

---

### Task 7 Prompt: Game State Machine

```

# Task 7: Game State Machine

Working in `crates/core/src/state_machine.rs`. Implement a strict finite state machine.

## State Machine Design

States: Calibrating → Exploring → Escalating → Climax → Reveal

Valid transitions:

- Calibrating → Exploring (after 3+ calibration scenes AND behavior baseline established)
- Exploring → Escalating (after 5+ exploration scenes AND at least 2 fears with confidence > 0.6)
- Escalating → Climax (after 5+ escalation scenes AND primary fear confidence > 0.8)
- Climax → Reveal (after 2+ climax scenes)

```rust
pub struct GameStateMachine {
    current_state: GamePhase,
    scene_counts: HashMap<GamePhase, u32>,
    transition_history: Vec<StateTransition>,
    transition_conditions: HashMap<(GamePhase, GamePhase), Box<dyn TransitionCheck>>,
}

pub struct StateTransition {
    pub from: GamePhase,
    pub to: GamePhase,
    pub timestamp: Instant,
    pub reason: String,
}

pub trait TransitionCheck: Send + Sync {
    fn can_transition(&self, context: &TransitionContext) -> bool;
    fn describe(&self) -> String;
}

pub struct TransitionContext {
    pub scenes_in_current_phase: u32,
    pub total_scenes: u32,
    pub fear_confidences: HashMap<FearType, f64>,
    pub primary_fear_confidence: f64,
    pub behavior_baseline_established: bool,
}

impl GameStateMachine {
    pub fn new() -> Self;
    pub fn current_phase(&self) -> GamePhase;
    pub fn can_advance(&self, context: &TransitionContext) -> bool;
    pub fn advance(&mut self, context: &TransitionContext) -> Result<GamePhase>;
    pub fn force_advance(&mut self) -> Result<GamePhase>;  // For demo mode
    pub fn record_scene(&mut self);
    pub fn scenes_in_phase(&self) -> u32;
    pub fn transition_history(&self) -> &[StateTransition];
}
```

Implement all transition conditions as concrete types implementing TransitionCheck.
The state machine MUST be deterministic and MUST reject invalid transitions with clear errors.

## Testing

Write thorough tests:

- Initial state is Calibrating
- Valid forward transitions work
- Invalid skip transitions are rejected (Calibrating → Climax)
- Backward transitions are rejected (Exploring → Calibrating)
- Transition requires meeting conditions
- Transition records in history
- Scene counting per phase
- force_advance works for demo mode
- Property: state machine never enters invalid state under random inputs

Run `cargo test -p fear-engine-core` to verify.

```

---

### Task 8-10 Prompts follow the same pattern. Use the task descriptions from TASKS_PHASE_1_2.md.
For Task 10, emphasize: write REAL horror narrative content, not placeholder text.
```
