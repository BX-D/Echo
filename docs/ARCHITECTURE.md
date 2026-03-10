# ARCHITECTURE.md — System Architecture Deep Dive

## Overview

Fear Engine is a real-time adaptive horror game built on three pillars:

1. **Behavioral Intelligence** — Covert analysis of player input patterns
2. **AI Narrative Generation** — LLM-powered dynamic story creation
3. **Psychological Horror UX** — Frontend designed to amplify fear response

---

## 1. System Architecture Diagram

```
                        ┌──────────────────────────────────┐
                        │         PLAYER'S BROWSER          │
                        │                                    │
                        │  ┌────────────────────────────┐    │
                        │  │     React Application       │    │
                        │  │                              │    │
                        │  │  ┌──────────┐ ┌──────────┐  │    │
                        │  │  │ Horror   │ │ Behavior │  │    │
                        │  │  │ Renderer │ │ Tracker  │  │    │
                        │  │  └────┬─────┘ └────┬─────┘  │    │
                        │  │       │             │        │    │
                        │  │  ┌────┴─────────────┴────┐   │    │
                        │  │  │   WebSocket Client    │   │    │
                        │  │  └───────────┬───────────┘   │    │
                        │  └──────────────┼───────────────┘    │
                        └─────────────────┼────────────────────┘
                                          │
                              ┌───────────┴───────────┐
                              │  WebSocket (wss://)    │
                              └───────────┬───────────┘
                                          │
┌─────────────────────────────────────────┼────────────────────────────────────────┐
│                           RUST BACKEND  │                                        │
│                                         │                                        │
│  ┌──────────────────────────────────────┴──────────────────────────────────────┐  │
│  │                          Axum Server (crate: server)                        │  │
│  │  ┌─────────────────┐  ┌─────────────────┐  ┌───────────────────────────┐   │  │
│  │  │  WS Handler     │  │  REST Routes    │  │  Middleware               │   │  │
│  │  │  - Connection   │  │  - POST /game   │  │  - CORS                   │   │  │
│  │  │  - Messages     │  │  - GET /health  │  │  - Request logging        │   │  │
│  │  │  - Sessions     │  │  - GET /debug/* │  │  - Error recovery         │   │  │
│  │  └────────┬────────┘  └─────────────────┘  └───────────────────────────┘   │  │
│  └───────────┼────────────────────────────────────────────────────────────────┘  │
│              │                                                                    │
│  ┌───────────┴────────────────────────────────────────────────────────────────┐  │
│  │                       Game Engine (crate: core)                             │  │
│  │                                                                             │  │
│  │  ┌────────────────┐  ┌────────────────┐  ┌────────────────────────────┐    │  │
│  │  │  Scene Graph   │  │  State Machine │  │  Event Bus                 │    │  │
│  │  │                │  │                │  │                             │    │  │
│  │  │  - Scene nodes │  │  States:       │  │  Events:                   │    │  │
│  │  │  - Transitions │  │  - Calibrating │  │  - SceneEntered            │    │  │
│  │  │  - Conditions  │  │  - Exploring   │  │  - ChoiceMade              │    │  │
│  │  │  - Branches    │  │  - Escalating  │  │  - BehaviorRecorded        │    │  │
│  │  │               │  │  - Climax      │  │  - FearProfileUpdated      │    │  │
│  │  │               │  │  - Reveal      │  │  - NarrativeGenerated      │    │  │
│  │  └───────┬────────┘  └───────┬────────┘  └──────────────┬─────────────┘    │  │
│  │          │                   │                           │                  │  │
│  │  ┌───────┴───────────────────┴───────────────────────────┴──────────────┐   │  │
│  │  │                     Scene Manager                                    │   │  │
│  │  │  - Resolves next scene based on state + fear profile + player choice │   │  │
│  │  │  - Triggers AI generation when dynamic content needed                │   │  │
│  │  │  - Manages scene transition effects                                  │   │  │
│  │  └──────────────────────────────┬───────────────────────────────────────┘   │  │
│  └─────────────────────────────────┼──────────────────────────────────────────┘  │
│                                    │                                              │
│         ┌──────────────────────────┼──────────────────────────┐                  │
│         │                          │                          │                  │
│  ┌──────┴──────────────────┐ ┌─────┴────────────────┐ ┌──────┴───────────────┐  │
│  │ Fear Profile Engine     │ │ AI Integration       │ │ Storage              │  │
│  │ (crate: fear-profile)   │ │ (crate: ai-integ.)   │ │ (crate: storage)     │  │
│  │                         │ │                       │ │                       │  │
│  │ ┌───────────────────┐   │ │ ┌─────────────────┐   │ │ ┌─────────────────┐  │  │
│  │ │ Behavior Analyzer │   │ │ │ Claude Client   │   │ │ │ SQLite          │  │  │
│  │ │ - Signal extract  │   │ │ │ - Messages API  │   │ │ │ - Sessions      │  │  │
│  │ │ - Pattern detect  │   │ │ │ - Streaming     │   │ │ │ - Profiles      │  │  │
│  │ └────────┬──────────┘   │ │ └────────┬────────┘   │ │ │ - Behavior log  │  │  │
│  │          │              │ │          │            │ │ │ - Content cache │  │  │
│  │ ┌────────┴──────────┐   │ │ ┌────────┴────────┐   │ │ └─────────────────┘  │  │
│  │ │ Bayesian Scorer   │   │ │ │ Prompt Builder  │   │ │                       │  │
│  │ │ - Prior update    │   │ │ │ - System layer  │   │ │                       │  │
│  │ │ - Confidence calc │   │ │ │ - Fear context  │   │ │                       │  │
│  │ │ - Score clamping  │   │ │ │ - Game state    │   │ │                       │  │
│  │ └────────┬──────────┘   │ │ │ - Output format │   │ │                       │  │
│  │          │              │ │ └────────┬────────┘   │ │                       │  │
│  │ ┌────────┴──────────┐   │ │          │            │ │                       │  │
│  │ │ Adaptation Engine │   │ │ ┌────────┴────────┐   │ │                       │  │
│  │ │ - Strategy select │◄──┼─┤ │ Narrative Gen   │   │ │                       │  │
│  │ │ - Intensity ctrl  │───┼─►│ │ - Pipeline      │   │ │                       │  │
│  │ │ - Pacing mgmt     │   │ │ │ - Retry logic   │   │ │                       │  │
│  │ └──────────────────┘   │ │ │ - Validation    │   │ │                       │  │
│  │                         │ │ └────────┬────────┘   │ │                       │  │
│  │                         │ │          │            │ │                       │  │
│  │                         │ │ ┌────────┴────────┐   │ │                       │  │
│  │                         │ │ │ Image Gen       │   │ │                       │  │
│  │                         │ │ │ - Stability AI  │   │ │                       │  │
│  │                         │ │ │ - Prompt adapt  │   │ │                       │  │
│  │                         │ │ │ - Caching       │   │ │                       │  │
│  │                         │ │ └─────────────────┘   │ │                       │  │
│  └─────────────────────────┘ └───────────────────────┘ └───────────────────────┘  │
│                                                                                    │
└────────────────────────────────────────────────────────────────────────────────────┘
```

---

## 2. Data Flow

### 2.1 Player Action → Fear Update → Narrative Generation

```
[Player types/clicks]
       │
       ▼
[Frontend: BehaviorTracker]
  - Captures raw input events
  - Computes derived signals (typing speed, pause duration, etc.)
  - Batches events every 2 seconds
       │
       ▼ (WebSocket: BehaviorBatch message)
       │
[Server: WS Handler]
  - Deserializes batch
  - Routes to game session
       │
       ▼
[Fear Profile Engine: Analyzer]
  - Maps behavior signals to fear indicators
  - Extracts features: hesitation_score, avoidance_score, etc.
       │
       ▼
[Fear Profile Engine: Bayesian Scorer]
  - For each fear category:
    posterior = (likelihood × prior) / evidence
  - Updates confidence scores
  - Detects significant profile changes
       │
       ▼
[Fear Profile Engine: Adaptation Engine]
  - Selects adaptation strategy based on game phase + profile
  - Computes intensity target
  - Generates adaptation directive for AI
       │
       ▼
[AI Integration: Prompt Builder]
  - Assembles multi-layer prompt:
    Layer 1: System (constant horror writer persona)
    Layer 2: Fear context (dynamic from profile)
    Layer 3: Game state (scene history, inventory)
    Layer 4: Output schema (structured JSON)
       │
       ▼
[AI Integration: Claude Client]
  - Sends request to Anthropic Messages API
  - Handles streaming response
  - Validates response against schema
       │
       ▼
[AI Integration: Narrative Pipeline]
  - Parses structured response
  - Extracts: narrative, choices, image_prompt, sound_cue
  - Validates narrative continuity
  - Caches response
       │
       ▼ (WebSocket: NarrativeUpdate message)
       │
[Frontend: GameScreen]
  - Renders narrative with horror effects
  - Triggers audio cues
  - Displays choices
  - Optionally triggers image generation display
```

### 2.2 Image Generation Pipeline (Async)

```
[Narrative Pipeline outputs image_prompt]
       │
       ▼
[Image Generation: Prompt Builder]
  - Adds style prefix: "Dark atmospheric horror, high contrast, desaturated..."
  - Adds fear-specific modifiers from profile
  - Adds negative prompt: "cartoon, bright, happy, gore..."
       │
       ▼
[Image Generation: API Client]
  - Sends to Stability AI / Replicate
  - Polls for completion (async)
  - Downloads result
       │
       ▼
[Image Generation: Cache]
  - Stores generated image with hash key
  - Returns URL
       │
       ▼ (WebSocket: ImageReady message)
       │
[Frontend: HorrorImage]
  - Displays with fade-in effect
  - Optional glitch overlay
```

---

## 3. Fear Profiling Algorithm

### 3.1 Behavior Signal Extraction

```rust
/// Raw behavior events from the frontend
pub enum BehaviorEventType {
    Keystroke {
        chars_per_second: f64,
        backspace_count: u32,
        total_chars: u32,
    },
    Pause {
        duration_ms: u64,
        scene_content_hash: String,
    },
    Choice {
        choice_id: String,
        time_to_decide_ms: u64,
        choice_category: ChoiceCategory, // fight/flight/investigate/avoid
    },
    MouseMovement {
        velocity: f64,         // pixels per second
        tremor_score: f64,     // high-frequency movement component
    },
    Scroll {
        direction: ScrollDirection,
        to_position: f64,      // 0.0 = top, 1.0 = bottom
        rereading: bool,       // scrolled back to already-seen content
    },
}

/// Derived behavioral features
pub struct BehaviorFeatures {
    pub hesitation_score: f64,       // 0-1, based on typing slowdown + pauses
    pub anxiety_score: f64,          // 0-1, based on mouse tremor + short responses
    pub avoidance_score: f64,        // 0-1, based on choice patterns
    pub engagement_score: f64,       // 0-1, based on response length + rereading
    pub indecision_score: f64,       // 0-1, based on backspaces + choice time
    pub fight_or_flight_ratio: f64,  // 0=pure flight, 1=pure fight
}
```

### 3.2 Fear Category Mapping

Each behavior feature contributes differently to each fear category:

```
                        claustro  isolat  body_h  stalk  control  uncanny  dark  sound  doppel  abandon
hesitation_score          0.3      0.2     0.4    0.3     0.3      0.5    0.3   0.2     0.5     0.2
anxiety_score             0.4      0.3     0.5    0.5     0.4      0.3    0.4   0.5     0.3     0.3
avoidance_score           0.5      0.2     0.3    0.4     0.3      0.4    0.5   0.3     0.4     0.2
engagement_score         -0.2     -0.1    -0.3   -0.2    -0.2     -0.1   -0.2  -0.1    -0.1    -0.1
indecision_score          0.3      0.2     0.2    0.2     0.5      0.3    0.2   0.2     0.3     0.4
fight_flight (low=fear)   0.4      0.3     0.2    0.3     0.4      0.2    0.4   0.3     0.2     0.3
```

These weights are the **likelihood matrix** in our Bayesian model.

### 3.3 Bayesian Update

For each fear category `f` and new behavior observation `b`:

```
P(f | b) = P(b | f) × P(f) / P(b)

Where:
  P(f)     = current fear score (prior)
  P(b | f) = likelihood from weight matrix × behavior feature value
  P(b)     = Σ over all fears: P(b | f_i) × P(f_i)  (normalization)
```

Update happens after every behavior batch (every 2 seconds).

### 3.4 Confidence Tracking

```rust
pub struct FearConfidence {
    pub observations: u32,        // number of relevant behavior events
    pub variance: f64,            // score variance over last N updates
    pub last_significant_change: Instant,
}

impl FearConfidence {
    pub fn confidence_level(&self) -> f64 {
        let obs_factor = (self.observations as f64 / 20.0).min(1.0);
        let stability_factor = 1.0 - self.variance.min(1.0);
        obs_factor * 0.6 + stability_factor * 0.4
    }
}
```

### 3.5 Adaptation Strategies

```rust
pub enum AdaptationStrategy {
    /// Early game: test different fears with mild stimuli
    Probe {
        target_fears: Vec<FearType>,
        intensity: f64, // 0.2 - 0.4
    },
    
    /// Mid game: gradually increase confirmed fears
    GradualEscalation {
        primary_fear: FearType,
        intensity_curve: EscalationCurve,
    },
    
    /// Build tension: calm before the storm
    Contrast {
        calm_duration: u32, // scenes of relative calm
        storm_fear: FearType,
        storm_intensity: f64,
    },
    
    /// Combine fears for amplification
    Layering {
        base_fear: FearType,
        amplifier_fear: FearType,
        blend_ratio: f64,
    },
    
    /// Go against expectations for unpredictability
    Subversion {
        expected_fear: FearType,
        actual_fear: FearType,
    },
}
```

---

## 4. WebSocket Protocol

### 4.1 Message Types (Client → Server)

```typescript
// Player made a choice
type ChoiceMessage = {
  type: "choice";
  payload: {
    scene_id: string;
    choice_id: string;
    time_to_decide_ms: number;
  };
};

// Batch of behavior events
type BehaviorBatchMessage = {
  type: "behavior_batch";
  payload: {
    events: BehaviorEvent[];
    timestamp: number;
  };
};

// Player typed a free-text response
type TextInputMessage = {
  type: "text_input";
  payload: {
    scene_id: string;
    text: string;
    typing_duration_ms: number;
    backspace_count: number;
  };
};

// Player requests to start game
type StartGameMessage = {
  type: "start_game";
  payload: {
    player_name?: string;
  };
};
```

### 4.2 Message Types (Server → Client)

```typescript
// New scene narrative
type NarrativeMessage = {
  type: "narrative";
  payload: {
    scene_id: string;
    text: string;
    atmosphere: string;
    choices: Choice[];
    sound_cue?: string;
    intensity: number;
    effects: EffectDirective[];
  };
};

// AI-generated image is ready
type ImageMessage = {
  type: "image";
  payload: {
    scene_id: string;
    image_url: string;   // base64 data URL or CDN URL
    display_mode: "fade_in" | "glitch" | "flash";
  };
};

// Game phase transition
type PhaseMessage = {
  type: "phase_change";
  payload: {
    from: GamePhase;
    to: GamePhase;
  };
};

// Meta-horror: AI breaks fourth wall
type MetaMessage = {
  type: "meta";
  payload: {
    text: string;
    target: "title" | "overlay" | "whisper" | "glitch_text";
    delay_ms: number;
  };
};

// End game: fear profile reveal
type RevealMessage = {
  type: "reveal";
  payload: {
    fear_profile: FearProfileSummary;
    key_moments: KeyMoment[];
    adaptations_made: AdaptationRecord[];
  };
};

// Error message
type ErrorMessage = {
  type: "error";
  payload: {
    code: string;
    message: string;
    recoverable: boolean;
  };
};
```

### 4.3 Effect Directives

```typescript
type EffectDirective = {
  effect: "shake" | "flicker" | "glitch" | "darkness" | "flashlight" | "crt" | "slow_type" | "fast_type";
  intensity: number;  // 0.0 - 1.0
  duration_ms: number;
  delay_ms: number;   // delay before effect starts
};
```

---

## 5. Database Schema

```sql
-- Game sessions
CREATE TABLE sessions (
    id TEXT PRIMARY KEY,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    player_name TEXT,
    current_scene_id TEXT NOT NULL,
    game_phase TEXT NOT NULL DEFAULT 'calibrating',
    game_state_json TEXT NOT NULL DEFAULT '{}',
    completed BOOLEAN DEFAULT FALSE
);

-- Fear profiles (one per session)
CREATE TABLE fear_profiles (
    session_id TEXT PRIMARY KEY REFERENCES sessions(id),
    claustrophobia REAL DEFAULT 0.5,
    isolation REAL DEFAULT 0.5,
    body_horror REAL DEFAULT 0.5,
    stalking REAL DEFAULT 0.5,
    loss_of_control REAL DEFAULT 0.5,
    uncanny_valley REAL DEFAULT 0.5,
    darkness REAL DEFAULT 0.5,
    sound_based REAL DEFAULT 0.5,
    doppelganger REAL DEFAULT 0.5,
    abandonment REAL DEFAULT 0.5,
    anxiety_threshold REAL DEFAULT 0.5,
    recovery_speed REAL DEFAULT 0.5,
    curiosity_vs_avoidance REAL DEFAULT 0.5,
    confidence_json TEXT DEFAULT '{}',
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Raw behavior events (for analysis/debugging)
CREATE TABLE behavior_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT REFERENCES sessions(id),
    event_type TEXT NOT NULL,
    event_data_json TEXT NOT NULL,
    scene_id TEXT,
    timestamp TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Generated content cache
CREATE TABLE content_cache (
    cache_key TEXT PRIMARY KEY,
    content_type TEXT NOT NULL,  -- 'narrative' | 'image'
    content_json TEXT NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    ttl_seconds INTEGER DEFAULT 3600
);

-- Scene history per session
CREATE TABLE scene_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT REFERENCES sessions(id),
    scene_id TEXT NOT NULL,
    narrative_text TEXT,
    player_choice TEXT,
    fear_profile_snapshot_json TEXT,
    adaptation_strategy TEXT,
    timestamp TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Indexes for performance
CREATE INDEX idx_behavior_session ON behavior_events(session_id);
CREATE INDEX idx_behavior_timestamp ON behavior_events(timestamp);
CREATE INDEX idx_scene_history_session ON scene_history(session_id);
CREATE INDEX idx_content_cache_ttl ON content_cache(created_at, ttl_seconds);
```

---

## 6. AI Prompt Architecture

### 6.1 System Prompt (Layer 1 — Constant)

```
You are FEAR ENGINE, an AI horror narrative generator. Your purpose is to create 
deeply unsettling, psychologically disturbing text adventure scenes.

WRITING RULES:
1. Write in second person present tense ("You push open the door...")
2. Favor psychological horror over gore — dread, wrongness, uncanny details
3. Use sensory details: what the player sees, hears, smells, feels
4. Each scene is 2-4 paragraphs, 150-300 words
5. End with 2-4 choices that reveal player psychology
6. Maintain strict narrative continuity
7. Never break character or acknowledge being an AI (except during meta-horror moments when directed)

HORROR TECHNIQUES:
- Wrongness: describe familiar things that are subtly off
- Isolation: emphasize emptiness, silence, distance from help
- Escalation: each detail slightly more disturbing than the last
- Implication: suggest horrors worse than what you describe
- Routine disruption: normal actions that go wrong

OUTPUT FORMAT: Respond ONLY with valid JSON. No markdown, no explanation.
```

### 6.2 Fear Context (Layer 2 — Dynamic)

Template:
```
PLAYER PSYCHOLOGICAL PROFILE:
- Primary fear axis: {top_fear} (confidence: {confidence}%)
- Secondary fears: {fear_2} ({score_2}), {fear_3} ({score_3})
- Anxiety baseline: {anxiety_threshold}/1.0
- Behavioral pattern: {curiosity_vs_avoidance_description}
- Current emotional state: {estimated_state}

NARRATIVE DIRECTIVE:
Strategy: {adaptation_strategy_name}
Instruction: {specific_instruction}
Target intensity: {intensity}/1.0
Forbidden: {things_to_avoid_for_pacing}
```

### 6.3 Game State Context (Layer 3 — Dynamic)

Template:
```
GAME STATE:
Location: {current_location_description}
Phase: {game_phase} (scene {scene_number}/{total_estimated})
Previous scene summary: {last_scene_summary}
Player's last action: {last_choice_text}
Active narrative threads: {thread_list}
Player inventory: {inventory_list}
Established details: {world_details_player_has_learned}
```

### 6.4 Output Schema (Layer 4 — Constant)

```json
{
  "narrative": "string — the scene description text",
  "atmosphere": "string — one word: dread|tension|panic|calm|wrongness|isolation|paranoia",
  "sound_cue": "string|null — ambient sound description for audio engine",
  "image_prompt": "string|null — detailed image generation prompt, only for key moments",
  "choices": [
    {
      "id": "string — unique choice identifier",
      "text": "string — what the player sees",
      "approach": "investigate|avoid|confront|flee|interact|wait",
      "fear_vector": "string — which fear this choice tests"
    }
  ],
  "hidden_elements": ["string — subtle fear triggers in the narrative for logging"],
  "intensity": 0.0,
  "meta_break": null | {
    "text": "string — fourth wall break text",
    "target": "title|overlay|whisper|glitch_text"
  }
}
```

---

## 7. Performance Targets

| Metric | Target | Measurement |
|--------|--------|-------------|
| WebSocket latency (behavior batch) | < 50ms | Server-side timestamp delta |
| Fear profile update | < 10ms | Rust benchmark |
| AI narrative generation | < 5s | End-to-end including API call |
| Image generation | < 15s | Async, non-blocking |
| Frontend frame rate | 60fps | During effects |
| Memory usage (backend) | < 100MB | Per session |
| Concurrent sessions | 50+ | Load test |
