# FEAR_SYSTEM.md — Fear Profiling Deep Dive

## Overview

The fear profiling system is the core innovation of this project. It transforms raw player behavior (typing speed, hesitation, choices) into a psychological model that the AI uses to generate personalized horror content.

---

## 1. Signal Taxonomy

### 1.1 Direct Signals (High Confidence)

| Signal | Measurement | Interpretation |
|--------|------------|----------------|
| Choice selection | Which option selected | Direct fear preference |
| Choice avoidance | Options NOT selected | What player is afraid to face |
| Explicit text input | Content of typed responses | Verbal expression of state |

### 1.2 Indirect Signals (Medium Confidence)

| Signal | Measurement | Interpretation |
|--------|------------|----------------|
| Typing speed change | chars/sec relative to baseline | Speed drop = processing disturbing content |
| Response latency | Time from scene display to first input | Long delay = fear/thinking |
| Choice deliberation time | Time from choices shown to selection | Long = difficult emotional decision |
| Backspace frequency | backspaces / total keystrokes | High = indecision, self-censoring |
| Response length change | Words per response relative to baseline | Short = terse from anxiety, Long = seeking control |

### 1.3 Physiological Proxy Signals (Low Confidence)

| Signal | Measurement | Interpretation |
|--------|------------|----------------|
| Mouse tremor | Direction variance in 500ms window | Physical anxiety response |
| Mouse velocity | Pixels per second | Erratic = anxious, Slow = frozen |
| Scroll rereading | Scrolling back to already-seen content | Trying to make sense of something disturbing |

---

## 2. Baseline Calibration

The first 3 scenes serve a dual purpose:
1. **Story function**: Introduce the setting and establish atmosphere
2. **Data function**: Establish the player's behavioral baseline

### Baseline Metrics Collected:
```rust
pub struct BehaviorBaseline {
    // Typing
    pub avg_typing_speed: f64,        // chars per second
    pub typing_speed_stddev: f64,     // natural variance
    
    // Response
    pub avg_response_time_ms: f64,    // scene display → first input
    pub avg_response_length: f64,     // words per response
    
    // Choice
    pub avg_choice_time_ms: f64,      // choices displayed → selection
    
    // Mouse
    pub avg_mouse_velocity: f64,      // pixels per second
    pub avg_mouse_tremor: f64,        // baseline jitter
    
    // Derived
    pub backspace_ratio: f64,         // backspaces / total keystrokes
    pub reading_speed: f64,           // estimated from scroll behavior
}
```

### Calibration Requirements:
- Minimum 30 keystroke events
- At least 3 choice selections
- At least 2 minutes of play time
- Baseline is "locked" after calibration phase

---

## 3. Feature Extraction Pipeline

```
Raw Events (per 2-second batch)
    │
    ├── Filter: Remove events outside current scene context
    │
    ├── Aggregate: Compute per-batch statistics
    │   ├── avg_typing_speed (this batch)
    │   ├── pause_count (pauses > 3s)
    │   ├── backspace_ratio
    │   ├── mouse_tremor (current window)
    │   └── choice_data (if a choice was made)
    │
    ├── Normalize: Compare to baseline
    │   ├── typing_speed_ratio = batch_speed / baseline_speed
    │   ├── response_time_ratio = batch_response_time / baseline_response_time
    │   └── tremor_ratio = batch_tremor / baseline_tremor
    │
    └── Extract Features:
        ├── hesitation_score = sigmoid(1 - typing_speed_ratio) × pause_factor
        ├── anxiety_score = weighted_sum(tremor_ratio, short_response_flag, rapid_input_flag)
        ├── avoidance_score = avoid_choice_ratio (rolling window of last 5 choices)
        ├── engagement_score = weighted_sum(investigate_ratio, rereading_flag, long_response_flag)
        ├── indecision_score = weighted_sum(backspace_ratio, choice_time_ratio)
        └── fight_flight_ratio = fight_choices / (fight_choices + flight_choices)
```

### Sigmoid Normalization:
```rust
fn sigmoid_normalize(value: f64, midpoint: f64, steepness: f64) -> f64 {
    1.0 / (1.0 + (-steepness * (value - midpoint)).exp())
}
```

---

## 4. Bayesian Fear Scoring — Mathematical Detail

### 4.1 The Model

We model 10 fear categories as latent variables. Observed behavior provides evidence for updating our beliefs about which fears the player has.

For fear category `f_i` and behavior feature vector `B`:

```
P(f_i | B) = P(B | f_i) × P(f_i) / P(B)
```

Where:
- `P(f_i)` = prior belief (current fear score)
- `P(B | f_i)` = likelihood (from weight matrix)
- `P(B)` = evidence (normalization constant)

### 4.2 Likelihood Computation

The likelihood matrix `W` maps features to fears:

```
W[fear_type][feature] = weight ∈ [-0.5, 0.5]
```

For a feature vector `B = [h, a, v, e, i, ff]`:

```
P(B | f_i) = σ(Σ_j W[f_i][j] × B[j])
```

Where `σ` is the sigmoid function to keep likelihood in (0, 1).

### 4.3 Evidence (Normalization)

```
P(B) = Σ_i P(B | f_i) × P(f_i)
```

### 4.4 Smoothing

To prevent single observations from causing wild swings:

```
score_new = α × posterior + (1 - α) × score_old
```

Where `α = 0.3` (smoothing factor).

### 4.5 Confidence Update

```
confidence.observations += 1
confidence.variance = running_variance(last_10_scores)
confidence_level = min(observations/20, 1.0) × 0.6 + (1 - variance) × 0.4
```

---

## 5. Adaptation Strategy Algorithms

### 5.1 Strategy Selection Decision Tree

```
if phase == Calibrating:
    return Probe(all_fears, intensity=0.25)

if phase == Exploring:
    high_conf = fears where confidence > 0.6
    low_conf = fears where confidence < 0.4
    
    if len(high_conf) >= 2:
        return GradualEscalation(primary=top_fear)
    else:
        return Probe(low_conf, intensity=0.35)

if phase == Escalating:
    top2 = top_2_fears_by_score
    scenes_since_calm = current_scene - last_calm_scene
    
    if scenes_since_calm >= 3:
        return Contrast(calm_duration=1, storm=top2[0], intensity=0.8)
    else:
        return Layering(base=top2[0], amplifier=top2[1], intensity=compute_from_curve())

if phase == Climax:
    // Maximum impact: combine top fear with subversion
    if random() < 0.3:
        return Subversion(expected=top_fear, actual=second_fear)
    else:
        return Layering(base=top_fear, amplifier=second_fear, intensity=0.95)
```

### 5.2 Intensity Curve

```
Intensity
1.0 ─┤                                              ╱──●
     │                                             ╱
0.8 ─┤                                     ╱──────╱
     │                                   ╱
0.6 ─┤                            ╱─────╱
     │                          ╱
0.4 ─┤             ╱────╲──── ╱   (contrast valley)
     │           ╱        ╲╱
0.2 ─┤  ╱──────╱
     │╱
0.0 ─┼──┬──┬──┬──┬──┬──┬──┬──┬──┬──┬──┬──┬──┬──┬──┬──┬──┤
     1  2  3  4  5  6  7  8  9 10 11 12 13 14 15 16 17 18
     |Calibrate|  Exploring    | Escalating        |Climax|
```

### 5.3 Instruction Generation

Each strategy produces natural-language instructions for the AI:

**Probe** (Claustrophobia):
> "Subtly introduce enclosed spaces. Describe a room with low ceilings, a narrow corridor, or a closet. Don't make it obvious — weave it naturally into the scene. Observe whether the player chooses to enter or avoid confined spaces."

**GradualEscalation** (Stalking, intensity=0.6):
> "The player fears being followed. Escalate this fear moderately. Include subtle signs of being watched: a shadow moving at the edge of vision, footsteps that stop when the player stops, a feeling of breath on the neck. The observer should feel closer than before but still not directly visible."

**Layering** (Claustrophobia + Darkness, intensity=0.8):
> "Combine the player's two primary fears. Create a scene where the space is getting smaller AND the light is failing. A crawlspace where the flashlight flickers. A basement where the door slowly closes behind them and the single bulb goes out. Layer both fears to amplify the effect."

**Contrast** (calm before storm):
> "Give the player a moment of false safety. A well-lit room, an open space, maybe a comforting detail. The calm should feel genuine but slightly wrong — just enough to make them nervous about what comes next."

**Subversion** (expected: Doppelganger, actual: Isolation):
> "The player expects to see another version of themselves (they've been primed for doppelganger content). Instead, subvert this: the mirror is empty. There's no reflection at all. The realization that they might not exist, that they're alone in a way that goes beyond physical — hit them with existential isolation instead."

---

## 6. Meta-Horror Integration

The meta-horror layer is what makes this game memorable. The AI breaks the fourth wall to reference things only it could know.

### 6.1 Meta Triggers

| Condition | Meta Event |
|-----------|-----------|
| Player pauses > 10s | "Take your time. I can wait." (whisper overlay) |
| Player tries to go back | "You can't unsee what you've seen." (title change) |
| Fear profile confidence > 0.8 | "I know what scares you, [pattern description]." (glitch text) |
| Player makes 3 avoidance choices | "You keep running. But the hospital goes on forever." |
| Climax phase reached | "Did you think this was random? Every scene was chosen for you." |

### 6.2 Implementation

Meta events are sent as separate WebSocket messages with a target display mode:
- **title**: Change the browser tab title temporarily
- **overlay**: Semi-transparent text appears over the scene
- **whisper**: Small text that appears at the bottom and fades
- **glitch_text**: Text that appears scrambled and slowly resolves

### 6.3 Pacing

Meta events should be rare (max 3-4 per playthrough) to maintain impact. Overuse kills the effect.
