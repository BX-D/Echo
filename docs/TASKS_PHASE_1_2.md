# TASKS.md — Complete Task Breakdown

> Each task includes: Description, Acceptance Criteria, Required Tests, and a Claude Code Prompt.
> Tasks must be completed in order within each phase. Cross-phase dependencies are noted.

---

## Phase 1: Foundation (Tasks 1–5)

---

### Task 1: Project Scaffolding

**Description**: Initialize the Rust workspace with all crates and the React frontend. Set up CI-ready tooling (clippy, rustfmt, eslint, prettier).

**Acceptance Criteria**:
- `cargo build --workspace` succeeds with zero warnings
- `cargo clippy --workspace` passes with zero warnings
- `cd frontend && npm install && npm run build` succeeds
- All crate `lib.rs` files exist with a module doc comment
- `.env.example` exists with all required variables
- `PROGRESS.md` exists with Task 1 marked in-progress

**Required Tests**:
- `cargo test --workspace` runs (even if no real tests yet, should compile)
- Frontend build produces `dist/` directory

**Files to Create**:
```
fear-engine/
├── Cargo.toml (workspace)
├── .env.example
├── .rustfmt.toml
├── clippy.toml
├── PROGRESS.md
├── crates/
│   ├── common/Cargo.toml + src/lib.rs
│   ├── storage/Cargo.toml + src/lib.rs
│   ├── core/Cargo.toml + src/lib.rs
│   ├── fear-profile/Cargo.toml + src/lib.rs
│   ├── ai-integration/Cargo.toml + src/lib.rs
│   └── server/Cargo.toml + src/main.rs
├── frontend/
│   ├── package.json
│   ├── tsconfig.json
│   ├── vite.config.ts
│   ├── tailwind.config.js
│   ├── postcss.config.js
│   ├── index.html
│   └── src/
│       ├── main.tsx
│       ├── App.tsx
│       └── vite-env.d.ts
```

---

### Task 2: Common Types & Error Handling

**Description**: Implement the shared types crate with domain types, custom error enum, and configuration.

**Acceptance Criteria**:
- `FearEngineError` enum covers all error categories with `thiserror`
- All domain types implemented with `Serialize`/`Deserialize`
- Config loads from environment variables with defaults
- All types have `Debug`, `Clone` where appropriate

**Required Tests**:
```rust
// tests in crates/common/src/error.rs
#[test] fn test_error_display_messages()
#[test] fn test_error_from_conversions()

// tests in crates/common/src/types.rs
#[test] fn test_fear_type_serialization_roundtrip()
#[test] fn test_game_phase_ordering()
#[test] fn test_behavior_event_type_variants()

// tests in crates/common/src/config.rs
#[test] fn test_config_default_values()
#[test] fn test_config_from_env()

// property tests
proptest! {
    #[test] fn test_fear_score_always_valid(score in 0.0..=1.0f64) { ... }
}
```

**Key Types to Implement**:
```rust
// Fear categories
pub enum FearType { Claustrophobia, Isolation, BodyHorror, Stalking, LossOfControl, UncannyValley, Darkness, SoundBased, Doppelganger, Abandonment }

// Game phases
pub enum GamePhase { Calibrating, Exploring, Escalating, Climax, Reveal }

// Behavior event types
pub enum BehaviorEventType { Keystroke{...}, Pause{...}, Choice{...}, MouseMovement{...}, Scroll{...} }

// Choice approach categories
pub enum ChoiceApproach { Investigate, Avoid, Confront, Flee, Interact, Wait }

// Scene atmosphere
pub enum Atmosphere { Dread, Tension, Panic, Calm, Wrongness, Isolation, Paranoia }
```

---

### Task 3: SQLite Storage Layer

**Description**: Implement the storage crate with SQLite database, migrations, and CRUD operations for all tables defined in ARCHITECTURE.md.

**Acceptance Criteria**:
- Database initializes with schema on first run
- All CRUD operations for: sessions, fear_profiles, behavior_events, content_cache, scene_history
- Transactions used for multi-table operations
- Connection pooling via `r2d2`

**Required Tests**:
```rust
// Integration tests with in-memory SQLite
#[test] fn test_create_and_get_session()
#[test] fn test_update_session_game_phase()
#[test] fn test_create_and_update_fear_profile()
#[test] fn test_fear_profile_all_scores_persisted()
#[test] fn test_insert_behavior_events_batch()
#[test] fn test_get_behavior_events_by_session()
#[test] fn test_content_cache_set_and_get()
#[test] fn test_content_cache_ttl_expiry()
#[test] fn test_insert_scene_history()
#[test] fn test_get_scene_history_ordered()
#[test] fn test_concurrent_session_access()  // multi-threaded test
```

---

### Task 4: Axum Server with WebSocket

**Description**: Implement the server crate with Axum, including REST endpoints, WebSocket handler, and per-connection session management.

**Acceptance Criteria**:
- Server starts and accepts HTTP + WebSocket connections
- Health check endpoint returns 200
- WebSocket connection lifecycle: connect → authenticate → game loop → disconnect
- Per-connection game session created and tracked
- Graceful shutdown handling
- CORS configured for frontend origin

**Required Tests**:
```rust
// Unit tests
#[test] fn test_ws_message_deserialization_all_types()
#[test] fn test_ws_message_serialization_all_types()
#[test] fn test_session_creation_on_connect()
#[test] fn test_session_cleanup_on_disconnect()

// Integration tests (using axum::test helpers)
#[tokio::test] async fn test_health_endpoint()
#[tokio::test] async fn test_websocket_connect_and_receive_welcome()
#[tokio::test] async fn test_websocket_start_game_flow()
#[tokio::test] async fn test_websocket_invalid_message_returns_error()
#[tokio::test] async fn test_cors_headers_present()
#[tokio::test] async fn test_multiple_concurrent_connections()
```

---

### Task 5: React Shell with WebSocket Client

**Description**: Set up the React frontend with routing, WebSocket connection, Zustand store, and basic horror-themed shell (dark background, font loading).

**Acceptance Criteria**:
- Vite dev server starts successfully
- WebSocket connects to backend on page load
- Zustand store manages game state
- Custom horror font loaded (use "Special Elite" or "Creepster" from Google Fonts)
- Dark theme applied globally
- TypeScript types for all WS messages
- Connection status indicator

**Required Tests**:
```typescript
// useWebSocket.test.ts
test('connects to WebSocket server on mount')
test('reconnects on disconnection')
test('parses incoming messages correctly')
test('sends messages in correct format')

// useGameState.test.ts
test('initial state is correct')
test('processes narrative message')
test('processes phase change')
test('tracks scene history')

// App.test.tsx
test('renders loading screen while connecting')
test('renders start screen when connected')
```

---

## Phase 2: Game Engine (Tasks 6–10)

---

### Task 6: Scene Data Model & Scene Graph

**Description**: Implement the scene data model — scenes are nodes in a directed graph with conditional edges. Scenes can be static (pre-written) or dynamic (AI-generated).

**Acceptance Criteria**:
- `Scene` struct with all fields (id, narrative, choices, atmosphere, effects, etc.)
- `SceneGraph` with add/remove/traverse operations
- Conditional transitions (based on fear profile, inventory, etc.)
- Support for both static and dynamic (AI-generated) scenes
- Scene templates for the hospital scenario skeleton

**Required Tests**:
```rust
#[test] fn test_scene_creation_with_all_fields()
#[test] fn test_scene_graph_add_and_traverse()
#[test] fn test_scene_graph_conditional_transition()
#[test] fn test_scene_graph_no_orphan_scenes()
#[test] fn test_scene_graph_detects_cycles()
#[test] fn test_dynamic_scene_placeholder()
#[test] fn test_scene_serialization()

proptest! {
    #[test] fn test_scene_graph_always_has_valid_transitions(...)
}
```

---

### Task 7: Game State Machine

**Description**: Implement a finite state machine for game phases with validated transitions.

**Acceptance Criteria**:
- States: `Calibrating → Exploring → Escalating → Climax → Reveal`
- Only valid transitions allowed (no skipping phases)
- Transition conditions (min scenes, confidence threshold)
- State entry/exit hooks for triggering side effects
- Full audit trail of transitions

**Required Tests**:
```rust
#[test] fn test_initial_state_is_calibrating()
#[test] fn test_valid_transition_calibrating_to_exploring()
#[test] fn test_invalid_transition_calibrating_to_climax()
#[test] fn test_transition_requires_min_scenes()
#[test] fn test_transition_requires_confidence_threshold()
#[test] fn test_all_valid_transition_paths()
#[test] fn test_transition_audit_trail()
#[test] fn test_cannot_transition_from_reveal()

proptest! {
    #[test] fn test_state_machine_never_reaches_invalid_state(...)
}
```

---

### Task 8: Event System (Pub/Sub)

**Description**: Implement an async event bus for decoupled communication between game subsystems.

**Acceptance Criteria**:
- Typed events: `SceneEntered`, `ChoiceMade`, `BehaviorRecorded`, `FearProfileUpdated`, `NarrativeGenerated`, `PhaseChanged`, `ImageGenerated`
- Subscribe with async handlers
- Publish with guaranteed delivery to all subscribers
- Event history for debugging
- Thread-safe (Send + Sync)

**Required Tests**:
```rust
#[tokio::test] async fn test_subscribe_and_receive_event()
#[tokio::test] async fn test_multiple_subscribers_all_receive()
#[tokio::test] async fn test_unsubscribe_stops_receiving()
#[tokio::test] async fn test_event_history_tracks_all_events()
#[tokio::test] async fn test_publish_to_no_subscribers_doesnt_panic()
#[tokio::test] async fn test_concurrent_publish_subscribe()
#[tokio::test] async fn test_event_ordering_preserved()
```

---

### Task 9: Scene Manager & Transitions

**Description**: Implement the scene manager that resolves next scenes based on player choices, game state, and fear profile.

**Acceptance Criteria**:
- Resolves next scene from choice + scene graph
- Decides when to use static vs AI-generated scenes
- Manages scene transition data (effects, delays)
- Integrates with state machine for phase transitions
- Tracks scene count per phase

**Required Tests**:
```rust
#[test] fn test_resolve_next_scene_from_choice()
#[test] fn test_resolve_dynamic_scene_when_ai_needed()
#[test] fn test_scene_transition_effects_based_on_atmosphere()
#[test] fn test_phase_transition_triggered_at_scene_threshold()
#[test] fn test_scene_count_tracking()
#[test] fn test_scene_manager_with_full_game_flow()
```

---

### Task 10: Base Narrative Content (Hospital Scenario)

**Description**: Create the skeleton narrative content for the abandoned hospital scenario. This includes calibration scenes (testing baseline behavior), probe scenes (testing different fears), and template scenes that AI will customize.

**Acceptance Criteria**:
- 3 calibration scenes (neutral-ish, establishing baseline)
- 10 fear-probe scenes (each targeting a different fear category)
- 5 scene templates for AI customization
- All scenes have proper choices with fear_vector tags
- Scene graph connects all scenes with conditional transitions
- Narrative quality is high (not placeholder text)

**Required Tests**:
```rust
#[test] fn test_all_calibration_scenes_exist()
#[test] fn test_all_fear_categories_have_probe_scenes()
#[test] fn test_all_scenes_have_valid_choices()
#[test] fn test_scene_graph_is_fully_connected()
#[test] fn test_no_dead_end_scenes()
#[test] fn test_scene_templates_have_placeholder_markers()
```

**Calibration Scene Examples** (these should be fully written, atmospheric, and genuinely good horror writing):

Scene 1 — "Awakening":
> You open your eyes. The ceiling above is stained with water damage, its patterns like spreading bruises across pale skin. Fluorescent lights flicker overhead — one working, two dead, the third buzzing in an irregular rhythm that sets your teeth on edge. You're lying on a gurney. The thin mattress beneath you is cold and slightly damp. The air smells of antiseptic and something else. Something organic and sweet, like fruit left to rot.

Scene 2 — "The Corridor":
> The corridor stretches in both directions, its linoleum floor scuffed with decades of foot traffic that ended long ago. Emergency exit signs cast a red glow that doesn't quite reach the floor. To your left, the corridor narrows toward a set of double doors with porthole windows. To your right, it opens into what appears to be a reception area. Behind you, the room you woke in. A clipboard rests on a chair by the wall. The name on it has been scratched out so violently the paper tore.

These set the tone and give the AI model for quality.
