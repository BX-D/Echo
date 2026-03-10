# CLAUDE.md — It Learns Your Fear (恐惧引擎)

## 🎯 Project Identity

**Name**: It Learns Your Fear (恐惧引擎 / Fear Engine)
**Type**: AI-powered adaptive horror text adventure game
**Core Concept**: A horror game that secretly analyzes player behavior (typing patterns, hesitation, choices) to build a real-time fear profile, then uses AI to generate personalized horror content that targets each player's specific fears.
**Tagline**: "The AI is learning what scares you."

---

## ⚠️ CRITICAL RULES FOR CODE GENERATION

### Zero Tolerance for Lazy Code

1. **NO placeholder implementations**. Every function must be fully implemented.
2. **NO `todo!()`, `unimplemented!()`, or `// TODO` comments** unless explicitly marked as Phase N dependency.
3. **NO simplified versions**. If the spec says Bayesian update, implement Bayesian update. Not a simple average.
4. **NO mock data in production code**. Test fixtures belong in `tests/` or `fixtures/`.
5. **ALL error handling must be explicit**. No `.unwrap()` in non-test code. Use `?` operator with custom error types.
6. **ALL public functions must have doc comments** with examples.
7. **ALL modules must have unit tests** in the same file (`#[cfg(test)] mod tests`).
8. **Integration tests go in `tests/` directory** of each crate.

### Rust Coding Standards

```rust
// ✅ CORRECT: Full error handling, doc comments, typed errors
/// Calculates the updated fear score using Bayesian inference.
///
/// # Arguments
/// * `prior` - Current fear score in range [0.0, 1.0]
/// * `likelihood` - P(behavior | fear_type) from behavior model
/// * `evidence` - P(behavior) marginal probability
///
/// # Returns
/// Updated fear score clamped to [0.0, 1.0]
///
/// # Example
/// ```
/// let updated = bayesian_update(0.5, 0.8, 0.3);
/// assert!(updated > 0.5); // Fear increased
/// ```
pub fn bayesian_update(prior: f64, likelihood: f64, evidence: f64) -> Result<f64> {
    if evidence == 0.0 {
        return Err(FearEngineError::InvalidEvidence("Evidence cannot be zero".into()));
    }
    let posterior = (likelihood * prior) / evidence;
    Ok(posterior.clamp(0.0, 1.0))
}

// ❌ WRONG: This will be rejected
pub fn bayesian_update(prior: f64, likelihood: f64, evidence: f64) -> f64 {
    todo!() // NEVER DO THIS
}

// ❌ WRONG: Unwrap in production code
pub fn update_fear(profile: &mut FearProfile, event: BehaviorEvent) {
    let score = calculate_score(&event).unwrap(); // NEVER
}
```

### Test Requirements Per Module

Every module MUST have:
- At least 3 unit tests covering happy path, edge cases, and error cases
- Property-based tests for any numeric computation (use `proptest` crate)
- Snapshot tests for any text/prompt generation (use `insta` crate)

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn test_bayesian_update_increases_score_for_high_likelihood() {
        let result = bayesian_update(0.5, 0.9, 0.3).unwrap();
        assert!(result > 0.5);
    }

    #[test]
    fn test_bayesian_update_decreases_score_for_low_likelihood() {
        let result = bayesian_update(0.5, 0.1, 0.3).unwrap();
        assert!(result < 0.5);
    }

    #[test]
    fn test_bayesian_update_zero_evidence_returns_error() {
        let result = bayesian_update(0.5, 0.9, 0.0);
        assert!(result.is_err());
    }

    proptest! {
        #[test]
        fn test_bayesian_update_always_in_range(
            prior in 0.0..=1.0f64,
            likelihood in 0.01..=1.0f64,
            evidence in 0.01..=1.0f64,
        ) {
            let result = bayesian_update(prior, likelihood, evidence).unwrap();
            prop_assert!(result >= 0.0 && result <= 1.0);
        }
    }
}
```

---

## 📁 Project Structure

```
fear-engine/
├── CLAUDE.md                    # THIS FILE - read first
├── PROGRESS.md                  # Task completion tracking
├── Cargo.toml                   # Workspace manifest
├── .env.example                 # Environment variables template
├── docker-compose.yml           # Local dev services
│
├── crates/
│   ├── common/                  # Shared types, errors, config
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── error.rs         # FearEngineError enum
│   │       ├── types.rs         # Shared domain types
│   │       └── config.rs        # App configuration
│   │
│   ├── storage/                 # SQLite persistence
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── migrations/      # SQL migration files
│   │       ├── session.rs       # Game session CRUD
│   │       ├── fear_profile.rs  # Fear profile persistence
│   │       └── behavior_log.rs  # Raw behavior event storage
│   │
│   ├── core/                    # Game engine
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── scene.rs         # Scene data model & graph
│   │       ├── state_machine.rs # Game state FSM
│   │       ├── event_bus.rs     # Pub/sub event system
│   │       ├── scene_manager.rs # Scene transitions
│   │       ├── inventory.rs     # Player inventory
│   │       └── narrative/       # Base narrative content
│   │           ├── mod.rs
│   │           ├── hospital.rs  # Hospital scenario
│   │           └── templates.rs # Scene templates
│   │
│   ├── fear-profile/            # Fear analysis engine
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── behavior.rs      # Behavior event types & parsing
│   │       ├── analyzer.rs      # Behavior → fear signal mapping
│   │       ├── scorer.rs        # Bayesian fear scoring
│   │       ├── profile.rs       # FearProfile struct & methods
│   │       ├── adaptation.rs    # Adaptation strategy engine
│   │       └── timeline.rs      # Temporal behavior analysis
│   │
│   ├── ai-integration/          # LLM & image generation
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── claude_client.rs # Anthropic API client
│   │       ├── prompt/          # Prompt engineering
│   │       │   ├── mod.rs
│   │       │   ├── system.rs    # System prompt templates
│   │       │   ├── context.rs   # Dynamic context builder
│   │       │   └── output.rs    # Output format schemas
│   │       ├── narrative.rs     # Narrative generation pipeline
│   │       ├── image.rs         # Image generation client
│   │       └── cache.rs         # Response caching
│   │
│   └── server/                  # HTTP & WebSocket server
│       ├── Cargo.toml
│       └── src/
│           ├── main.rs          # Entry point
│           ├── app.rs           # Axum app builder
│           ├── ws/              # WebSocket handling
│           │   ├── mod.rs
│           │   ├── handler.rs   # Connection handler
│           │   ├── messages.rs  # WS message types
│           │   └── session.rs   # Per-connection state
│           ├── routes/          # REST endpoints
│           │   ├── mod.rs
│           │   ├── game.rs      # Game session management
│           │   ├── health.rs    # Health check
│           │   └── debug.rs     # Debug endpoints (dev only)
│           └── middleware/
│               ├── mod.rs
│               └── cors.rs      # CORS configuration
│
├── frontend/                    # React application
│   ├── package.json
│   ├── tsconfig.json
│   ├── vite.config.ts
│   ├── tailwind.config.js
│   ├── index.html
│   └── src/
│       ├── main.tsx
│       ├── App.tsx
│       ├── hooks/
│       │   ├── useWebSocket.ts      # WS connection management
│       │   ├── useBehaviorTracker.ts # Behavior event collection
│       │   ├── useAudio.ts          # Audio engine hook
│       │   ├── useGameState.ts      # Game state management
│       │   └── useHorrorEffects.ts  # Visual effect triggers
│       ├── components/
│       │   ├── GameScreen.tsx        # Main game container
│       │   ├── NarrativeDisplay.tsx  # Text display with effects
│       │   ├── ChoicePanel.tsx       # Player choice buttons
│       │   ├── HorrorImage.tsx       # AI image display
│       │   ├── StatusBar.tsx         # Subtle game info
│       │   ├── LoadingScreen.tsx     # Themed loading
│       │   ├── FearReveal.tsx        # End-game fear profile reveal
│       │   ├── StartScreen.tsx       # Game start / title
│       │   └── effects/
│       │       ├── Typewriter.tsx    # Typewriter text effect
│       │       ├── GlitchText.tsx    # Text glitch effect
│       │       ├── ScreenShake.tsx   # Screen shake wrapper
│       │       ├── Vignette.tsx      # Darkness vignette
│       │       ├── Flicker.tsx       # Screen flicker
│       │       ├── CRTOverlay.tsx    # CRT scanline effect
│       │       └── Flashlight.tsx    # Cursor flashlight mode
│       ├── systems/
│       │   ├── BehaviorCollector.ts  # Behavior event aggregation
│       │   ├── AudioEngine.ts       # Web Audio API manager
│       │   └── EffectScheduler.ts   # Effect timing & sequencing
│       ├── types/
│       │   ├── game.ts              # Game state types
│       │   ├── ws.ts                # WebSocket message types
│       │   ├── behavior.ts          # Behavior event types
│       │   └── narrative.ts         # Narrative content types
│       ├── styles/
│       │   ├── globals.css          # Global styles + horror theme
│       │   ├── effects.css          # CSS animation keyframes
│       │   └── fonts.css            # Horror font imports
│       └── assets/
│           ├── fonts/               # Horror fonts
│           └── audio/               # Base ambient sounds
│
└── tests/                       # Workspace-level integration tests
    ├── full_game_loop.rs
    ├── fear_profile_accuracy.rs
    └── fixtures/
        ├── sample_behaviors.json
        └── expected_profiles.json
```

---

## 🔧 Tech Stack

| Component | Technology | Version | Purpose |
|-----------|-----------|---------|---------|
| Backend Runtime | Rust | 1.82+ | Performance, type safety |
| Web Framework | Axum | 0.8 | Async HTTP/WS server |
| Async Runtime | Tokio | 1.x | Async I/O |
| Serialization | Serde | 1.x | JSON serialization |
| Database | SQLite (rusqlite) | 0.32 | Lightweight persistence |
| HTTP Client | reqwest | 0.12 | API calls to Claude/image APIs |
| Testing | proptest, insta | latest | Property & snapshot tests |
| Frontend | React | 18 | UI framework |
| Frontend Lang | TypeScript | 5.x | Type safety |
| Build Tool | Vite | 6.x | Fast dev server |
| Styling | Tailwind CSS | 3.x | Utility-first CSS |
| Animation | Framer Motion | 11.x | Smooth animations |
| Audio | Web Audio API | native | Procedural audio |
| State | Zustand | 5.x | Lightweight state management |
| LLM | Anthropic Claude API | Messages API | Narrative generation |
| Images | Stability AI / Replicate | latest | Horror image generation |

---

## 🎮 Game Flow

```
START
  │
  ▼
[Title Screen] ─── "Press any key to begin..."
  │
  ▼
[Calibration Phase] ─── 2-3 "normal" scenes to establish behavioral baseline
  │                      (typing speed, response time, choice patterns)
  ▼
[Exploration Phase] ─── 5-8 scenes with diverse fear stimuli
  │                      AI is testing different fear categories
  │                      Fear profile confidence is building
  ▼
[Escalation Phase] ─── 5-8 scenes targeting confirmed fears
  │                      Content becomes increasingly personalized
  │                      Meta-horror elements begin appearing
  ▼
[Climax] ─── 2-3 scenes of maximum personalized horror
  │           AI uses everything it learned
  │           Fourth-wall breaks
  ▼
[Resolution] ─── Brief denouement
  │
  ▼
[Fear Reveal] ─── "Here's what the AI learned about you:"
  │                Show fear profile visualization
  │                Show key moments that revealed fears
  │                Show how content was adapted
  ▼
END
```

---

## 📊 Progress Tracking

After completing each task, update PROGRESS.md with:
```markdown
## Task N: [Task Name]
- Status: ✅ COMPLETE
- Date: YYYY-MM-DD
- Tests: X passed, 0 failed
- Notes: [Any deviations from spec]
```

**Current Phase**: Not Started
**Tasks Completed**: 0/30
**Next Task**: Task 1 — Project Scaffolding

---

## 🔑 Environment Variables

```env
# Required
ANTHROPIC_API_KEY=sk-ant-...
STABILITY_API_KEY=sk-...        # Or REPLICATE_API_TOKEN

# Optional
DATABASE_URL=sqlite://fear_engine.db
RUST_LOG=fear_engine=debug
SERVER_HOST=127.0.0.1
SERVER_PORT=3001
FRONTEND_URL=http://localhost:5173
```

---

## 🏗️ Build & Run Commands

```bash
# Backend
cargo build --workspace              # Build all crates
cargo test --workspace               # Run all tests
cargo run -p fear-engine-server      # Start backend server

# Frontend
cd frontend && npm install           # Install dependencies
cd frontend && npm run dev           # Start dev server
cd frontend && npm run build         # Production build
cd frontend && npm run test          # Run tests

# Full stack dev
cargo run -p fear-engine-server &    # Start backend
cd frontend && npm run dev           # Start frontend
```

---

## 📝 Commit Message Convention

```
type(scope): description

Types: feat, fix, test, refactor, docs, perf, chore
Scopes: core, fear-profile, ai, server, frontend, common, storage

Examples:
feat(fear-profile): implement Bayesian fear scoring algorithm
test(core): add property tests for state machine transitions
fix(server): handle WebSocket disconnection gracefully
perf(ai): add LRU cache for narrative generation responses
```
