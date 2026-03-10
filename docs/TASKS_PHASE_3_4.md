# TASKS — Phase 3: Fear Profiling (Tasks 11–15) & Phase 4: AI Integration (Tasks 16–20)

---

## Phase 3: Fear Profiling Engine (Tasks 11–15)

---

### Task 11: Behavior Event Schema & Collector

**Description**: Implement the behavior event types, serialization, and server-side collector that receives batched events from the frontend and stores them.

**Acceptance Criteria**:
- All `BehaviorEventType` variants implemented per ARCHITECTURE.md spec
- `BehaviorBatch` struct for batched WebSocket messages
- Server-side collector receives batches, validates, and stores to SQLite
- Event timestamps are validated (no future timestamps, reasonable ordering)
- Collector emits `BehaviorRecorded` events on the event bus

**Required Tests**:
```rust
// Unit tests
#[test] fn test_keystroke_event_creation()
#[test] fn test_pause_event_creation()
#[test] fn test_choice_event_creation()
#[test] fn test_mouse_movement_event_creation()
#[test] fn test_scroll_event_creation()
#[test] fn test_behavior_batch_serialization_roundtrip()
#[test] fn test_reject_future_timestamps()
#[test] fn test_reject_negative_durations()

// Integration tests
#[tokio::test] async fn test_collector_stores_batch_to_db()
#[tokio::test] async fn test_collector_emits_event_bus_event()
#[tokio::test] async fn test_collector_handles_empty_batch()
#[tokio::test] async fn test_collector_handles_malformed_events()

// Property tests
proptest! {
    #[test] fn test_all_behavior_events_serialize_deserialize(event in arb_behavior_event())
}
```

---

### Task 12: Frontend Behavior Tracker

**Description**: Implement the frontend behavior tracking system that monitors player input patterns without being visible to the player.

**Acceptance Criteria**:
- **Keystroke tracking**: chars/sec, backspace frequency, total chars
- **Pause detection**: time between last keystroke/click and next action
- **Choice timing**: ms from choice presentation to selection
- **Mouse tracking**: velocity, tremor (high-frequency jitter), position
- **Scroll tracking**: direction, rereading detection
- Events batched every 2 seconds and sent via WebSocket
- Zero visual indicator of tracking (completely invisible)
- Performance: < 1ms overhead per event capture

**Required Tests**:
```typescript
// BehaviorCollector.test.ts
test('captures keystroke speed correctly')
test('detects pause after scene text displayed')
test('measures choice decision time accurately')
test('calculates mouse velocity')
test('detects mouse tremor from rapid small movements')
test('detects scroll-back rereading pattern')
test('batches events at 2-second intervals')
test('clears batch after sending')
test('handles rapid input without dropping events')
test('does not interfere with normal UI events')

// useBehaviorTracker.test.ts
test('starts tracking on mount')
test('stops tracking on unmount')
test('sends batches via WebSocket')
test('respects scene context for events')
```

**Implementation Notes**:
```typescript
// Key implementation patterns:

// 1. Keystroke speed via key timing
let lastKeystrokeTime = 0;
const handleKeydown = (e: KeyboardEvent) => {
  const now = performance.now();
  const delta = now - lastKeystrokeTime;
  const charsPerSecond = delta > 0 ? 1000 / delta : 0;
  lastKeystrokeTime = now;
  collector.record({ type: 'keystroke', data: { charsPerSecond, isBackspace: e.key === 'Backspace' }});
};

// 2. Mouse tremor via high-pass filter on movement
const mousePositions: Array<{x: number, y: number, t: number}> = [];
const TREMOR_WINDOW_MS = 500;
const handleMouseMove = (e: MouseEvent) => {
  const now = performance.now();
  mousePositions.push({ x: e.clientX, y: e.clientY, t: now });
  // Remove old positions outside window
  while (mousePositions.length > 0 && mousePositions[0].t < now - TREMOR_WINDOW_MS) {
    mousePositions.shift();
  }
  // Tremor = variance of velocity direction changes
  if (mousePositions.length >= 3) {
    const tremor = calculateDirectionVariance(mousePositions);
    collector.record({ type: 'mouse', data: { tremor, velocity: calculateVelocity(mousePositions) }});
  }
};

// 3. Rereading detection
const handleScroll = (e: Event) => {
  const scrollTop = document.documentElement.scrollTop;
  const maxSeen = collector.getMaxScrollPosition();
  if (scrollTop < maxSeen * 0.7) {
    collector.record({ type: 'scroll', data: { rereading: true, position: scrollTop / maxSeen }});
  }
};
```

---

### Task 13: Fear Scoring Algorithm

**Description**: Implement the Bayesian fear scoring system that maps behavior features to fear category scores.

**Acceptance Criteria**:
- `BehaviorFeatures` extraction from raw events (hesitation, anxiety, avoidance, engagement, indecision, fight/flight)
- Likelihood matrix (behavior features × fear categories) as per ARCHITECTURE.md
- Bayesian update: `posterior = (likelihood × prior) / evidence`
- Score clamping to [0.0, 1.0] with smoothing
- Confidence tracking per fear category
- Exponential moving average to prevent wild swings

**Required Tests**:
```rust
// Unit tests — Bayesian math
#[test] fn test_bayesian_update_basic()
#[test] fn test_bayesian_update_high_likelihood_increases_score()
#[test] fn test_bayesian_update_low_likelihood_decreases_score()
#[test] fn test_bayesian_update_zero_evidence_error()
#[test] fn test_bayesian_update_clamped_to_unit_range()
#[test] fn test_exponential_moving_average_smoothing()

// Unit tests — Feature extraction
#[test] fn test_extract_hesitation_from_slow_typing()
#[test] fn test_extract_anxiety_from_mouse_tremor()
#[test] fn test_extract_avoidance_from_flee_choices()
#[test] fn test_extract_engagement_from_long_responses()
#[test] fn test_extract_indecision_from_backspaces()
#[test] fn test_extract_fight_flight_ratio()

// Unit tests — Full scoring pipeline
#[test] fn test_scoring_pipeline_end_to_end()
#[test] fn test_scoring_updates_confidence()
#[test] fn test_scoring_multiple_updates_converge()
#[test] fn test_scoring_different_behaviors_affect_different_fears()

// Property tests
proptest! {
    #[test] fn test_all_fear_scores_always_in_range(events in vec(arb_behavior_event(), 1..100))
    #[test] fn test_confidence_monotonically_increases_with_observations(n in 1..50usize)
    #[test] fn test_bayesian_update_is_deterministic(prior in 0.0..=1.0f64, likelihood in 0.01..=1.0f64)
}

// Snapshot tests
#[test] fn test_scoring_snapshot_high_claustrophobia_player()
#[test] fn test_scoring_snapshot_curious_explorer_player()
#[test] fn test_scoring_snapshot_anxious_avoider_player()
```

---

### Task 14: Fear Profile Builder & Updater

**Description**: Implement the `FearProfile` struct and the profile builder that maintains the full player psychological model.

**Acceptance Criteria**:
- `FearProfile` with all 10 fear scores + 3 meta-patterns + confidence map
- Profile initialized with neutral priors (0.5 for all)
- `update(&mut self, features: &BehaviorFeatures)` applies Bayesian updates
- Profile snapshots for history tracking
- Profile serialization to JSON for storage and AI prompt context
- Top-N fears extraction with confidence filtering
- Profile comparison (for A/B testing different strategies)

**Required Tests**:
```rust
#[test] fn test_new_profile_has_neutral_priors()
#[test] fn test_update_changes_scores()
#[test] fn test_multiple_updates_accumulate()
#[test] fn test_top_fears_returns_highest_scores()
#[test] fn test_top_fears_filters_low_confidence()
#[test] fn test_profile_snapshot_is_independent_copy()
#[test] fn test_profile_serialization_roundtrip()
#[test] fn test_profile_comparison_diff()
#[test] fn test_profile_reset()
#[test] fn test_profile_persistence_to_storage()

#[tokio::test] async fn test_profile_update_emits_event()
```

---

### Task 15: Adaptation Strategy Engine

**Description**: Implement the engine that decides HOW to use the fear profile to generate content — selecting between probe, escalation, contrast, layering, and subversion strategies.

**Acceptance Criteria**:
- Strategy selection based on game phase + fear profile + scene count
- Calibrating phase → always `Probe` strategy
- Exploring phase → `Probe` for low-confidence fears, `GradualEscalation` for confirmed ones
- Escalating phase → `GradualEscalation` + `Layering` + `Contrast`
- Climax phase → maximum intensity `Layering` + `Subversion` for shock
- Intensity curve follows game pacing (ramp up, brief valleys, spike at climax)
- Strategy produces an `AdaptationDirective` used by the prompt builder

**Required Tests**:
```rust
#[test] fn test_calibrating_phase_always_probes()
#[test] fn test_exploring_phase_probes_low_confidence()
#[test] fn test_exploring_phase_escalates_confirmed_fears()
#[test] fn test_escalating_phase_uses_layering()
#[test] fn test_contrast_strategy_inserts_calm_scenes()
#[test] fn test_climax_phase_maximum_intensity()
#[test] fn test_subversion_goes_against_expectation()
#[test] fn test_intensity_curve_shape()
#[test] fn test_adaptation_directive_format()
#[test] fn test_strategy_with_real_fear_profile()

// Snapshot tests for directive generation
#[test] fn test_directive_snapshot_early_game_neutral_profile()
#[test] fn test_directive_snapshot_mid_game_claustrophobic_player()
#[test] fn test_directive_snapshot_late_game_multi_fear_player()
```

---

## Phase 4: AI Integration (Tasks 16–20)

---

### Task 16: Anthropic Claude API Client

**Description**: Implement a robust, production-grade client for the Anthropic Messages API with streaming support, retry logic, and rate limiting.

**Acceptance Criteria**:
- Async client using `reqwest`
- Support for Messages API (non-streaming and streaming)
- Exponential backoff retry (3 attempts, 1s/2s/4s)
- Rate limiting (respect API rate limits with token bucket)
- Request/response logging (debug level)
- Timeout configuration (30s default)
- Structured error handling (rate limit, auth, network, API errors)

**Required Tests**:
```rust
// Unit tests with mock HTTP server (use wiremock crate)
#[tokio::test] async fn test_send_message_success()
#[tokio::test] async fn test_send_message_with_system_prompt()
#[tokio::test] async fn test_retry_on_500_error()
#[tokio::test] async fn test_no_retry_on_400_error()
#[tokio::test] async fn test_retry_on_rate_limit_with_backoff()
#[tokio::test] async fn test_timeout_after_configured_duration()
#[tokio::test] async fn test_streaming_response_assembly()
#[tokio::test] async fn test_auth_error_handling()
#[tokio::test] async fn test_malformed_response_handling()
#[tokio::test] async fn test_rate_limiter_throttles_requests()
```

**Key Implementation Pattern**:
```rust
pub struct ClaudeClient {
    http: reqwest::Client,
    api_key: String,
    base_url: String,
    rate_limiter: RateLimiter,
    config: ClientConfig,
}

pub struct ClientConfig {
    pub model: String,           // "claude-sonnet-4-20250514"
    pub max_tokens: u32,         // 1024
    pub timeout: Duration,       // 30s
    pub max_retries: u32,        // 3
    pub base_retry_delay: Duration, // 1s
}

impl ClaudeClient {
    pub async fn generate(&self, request: GenerateRequest) -> Result<GenerateResponse> {
        let mut attempt = 0;
        loop {
            self.rate_limiter.acquire().await;
            match self.send_request(&request).await {
                Ok(response) => return Ok(response),
                Err(e) if e.is_retryable() && attempt < self.config.max_retries => {
                    let delay = self.config.base_retry_delay * 2u32.pow(attempt);
                    tokio::time::sleep(delay).await;
                    attempt += 1;
                }
                Err(e) => return Err(e),
            }
        }
    }
}
```

---

### Task 17: Prompt Engineering System

**Description**: Implement the multi-layer prompt builder that constructs prompts from templates + dynamic context.

**Acceptance Criteria**:
- 4-layer prompt architecture (System, Fear Context, Game State, Output Schema)
- System prompt is constant and hand-crafted
- Fear context layer dynamically built from `FearProfile`
- Game state context built from `SceneManager` state
- Output schema enforces structured JSON response
- Template engine with variable substitution (use `tera` or `handlebars` crate, or simple custom impl)
- Prompt token counting (approximate, for budget management)

**Required Tests**:
```rust
#[test] fn test_system_prompt_is_valid()
#[test] fn test_fear_context_includes_top_fears()
#[test] fn test_fear_context_includes_adaptation_directive()
#[test] fn test_game_state_context_includes_location()
#[test] fn test_game_state_context_includes_history()
#[test] fn test_full_prompt_assembly()
#[test] fn test_prompt_token_count_approximation()
#[test] fn test_prompt_stays_under_token_budget()

// Snapshot tests (CRITICAL — these catch unintended prompt changes)
#[test] fn test_prompt_snapshot_early_game()
#[test] fn test_prompt_snapshot_mid_game_claustrophobic()
#[test] fn test_prompt_snapshot_climax_multi_fear()
#[test] fn test_prompt_snapshot_with_meta_horror_directive()
```

---

### Task 18: Narrative Generation Pipeline

**Description**: Implement the full pipeline: prompt building → API call → response parsing → validation → scene creation.

**Acceptance Criteria**:
- Builds prompt from current game state + fear profile
- Calls Claude API
- Parses structured JSON response
- Validates response (has narrative, valid choices, intensity in range)
- Falls back to template scene if API fails
- Extracts image_prompt for async image generation
- Caches responses for similar contexts

**Required Tests**:
```rust
// Integration tests with mock Claude API
#[tokio::test] async fn test_pipeline_generates_valid_scene()
#[tokio::test] async fn test_pipeline_parses_choices_correctly()
#[tokio::test] async fn test_pipeline_extracts_image_prompt()
#[tokio::test] async fn test_pipeline_falls_back_on_api_error()
#[tokio::test] async fn test_pipeline_falls_back_on_invalid_json()
#[tokio::test] async fn test_pipeline_validates_intensity_range()
#[tokio::test] async fn test_pipeline_caches_similar_requests()
#[tokio::test] async fn test_pipeline_respects_token_budget()
#[tokio::test] async fn test_pipeline_handles_streaming_response()
#[tokio::test] async fn test_pipeline_meta_break_extraction()
```

---

### Task 19: Image Generation Integration

**Description**: Implement the image generation client (Stability AI or Replicate) for generating horror images at key narrative moments.

**Acceptance Criteria**:
- Async image generation (non-blocking game flow)
- Horror-specific prompt builder (base style + scene context + fear modifiers)
- Negative prompt to avoid inappropriate content
- Image caching (same prompt → same image)
- Graceful degradation if image API fails (game continues without images)
- Image served as base64 data URL via WebSocket

**Required Tests**:
```rust
// Unit tests
#[test] fn test_image_prompt_builder_includes_style_prefix()
#[test] fn test_image_prompt_builder_includes_fear_modifiers()
#[test] fn test_image_prompt_builder_includes_negative_prompt()
#[test] fn test_image_prompt_builder_scene_specific()

// Integration tests with mock API
#[tokio::test] async fn test_image_generation_success()
#[tokio::test] async fn test_image_generation_caching()
#[tokio::test] async fn test_image_generation_timeout_graceful()
#[tokio::test] async fn test_image_generation_api_error_graceful()
#[tokio::test] async fn test_image_delivered_via_websocket()
```

---

### Task 20: Content Safety & Caching Layer

**Description**: Implement content safety validation and LRU caching for both narrative and image generation.

**Acceptance Criteria**:
- Content safety filter: no real violence instructions, no self-harm content, keep horror fictional
- Narrative length validation (150-300 words)
- LRU cache for narrative responses (configurable size, default 100)
- LRU cache for generated images (configurable size, default 50)
- Cache hit/miss metrics
- Cache key computation (hash of relevant prompt components)
- TTL-based expiry

**Required Tests**:
```rust
#[test] fn test_safety_filter_allows_horror_content()
#[test] fn test_safety_filter_blocks_harmful_content()
#[test] fn test_narrative_length_validation()
#[test] fn test_lru_cache_set_and_get()
#[test] fn test_lru_cache_eviction_on_capacity()
#[test] fn test_lru_cache_ttl_expiry()
#[test] fn test_cache_key_computation_deterministic()
#[test] fn test_cache_key_different_for_different_contexts()
#[test] fn test_cache_metrics_tracking()
```
