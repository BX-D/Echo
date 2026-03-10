# TASKS — Phase 5: Frontend Horror (Tasks 21–26) & Phase 6: Integration (Tasks 27–30)

---

## Phase 5: Frontend Horror Experience (Tasks 21–26)

---

### Task 21: Horror UI Design System

**Description**: Create the complete design system — color palette, typography, spacing, components, and CSS variables for the horror theme.

**Acceptance Criteria**:
- Color palette: deep blacks, blood reds, sickly greens, clinical whites
- Typography: "Special Elite" for narrative, monospace for UI, "Creepster" for titles
- CSS custom properties for all theme values
- Responsive design (works on 1024px+ screens)
- Vignette overlay component
- CRT scanline effect (optional toggle)
- Global ambient darkness (dark gradient from edges)
- Loading states with horror theme (pulsing, flickering)
- Smooth transitions between all UI states

**Required Tests**:
```typescript
// DesignSystem.test.tsx
test('all CSS variables are defined')
test('fonts load correctly')
test('vignette overlay renders without blocking interaction')
test('CRT effect can be toggled')
test('responsive breakpoints work correctly')
test('dark theme has sufficient contrast for readability')
```

**Color Palette**:
```css
:root {
  --color-void: #0a0a0a;           /* deepest background */
  --color-shadow: #1a1a1a;         /* elevated surfaces */
  --color-ash: #2a2a2a;            /* borders, dividers */
  --color-smoke: #666666;          /* secondary text */
  --color-bone: #d4d0c8;           /* primary text */
  --color-parchment: #e8e0d4;      /* highlighted text */
  --color-blood: #8b0000;          /* accent, danger */
  --color-rust: #a0522d;           /* warm accent */
  --color-bile: #556b2f;           /* sickness, unease */
  --color-clinical: #f0f8ff;       /* sterile white, flash */
  --color-bruise: #4a0e4e;         /* purple undertones */
  --color-gangrene: #2f4f4f;       /* dark teal, decay */

  /* Functional */
  --bg-primary: var(--color-void);
  --bg-secondary: var(--color-shadow);
  --text-primary: var(--color-bone);
  --text-secondary: var(--color-smoke);
  --text-highlight: var(--color-parchment);
  --accent-danger: var(--color-blood);
  --accent-unease: var(--color-bile);
}
```

---

### Task 22: Typewriter Text Effect with Glitch

**Description**: Implement the signature typewriter effect for narrative text, with configurable speed, glitch insertions, and dramatic pauses.

**Acceptance Criteria**:
- Character-by-character reveal with configurable speed
- Speed modes: slow (50ms, tension), normal (30ms), fast (15ms), instant (0ms, jump scare)
- **Glitch effect**: random characters briefly appear before correct character (1 in 20 chance)
- **Dramatic pause**: certain punctuation (. ... — !) triggers a longer delay
- **Sound sync**: optional typing sound per character
- Callback when typing completes
- Skip button (pressing any key finishes current text)
- Cursor blink at end of text

**Required Tests**:
```typescript
// Typewriter.test.tsx
test('renders empty initially')
test('reveals characters one at a time')
test('respects speed setting')
test('dramatic pause on period')
test('dramatic pause on ellipsis')
test('glitch effect shows wrong char briefly')
test('completes and calls onComplete callback')
test('skip reveals all text instantly')
test('cursor blinks after completion')
test('handles HTML entities correctly')
test('handles multiple paragraphs')

// Performance
test('maintains 60fps during typing animation')
```

---

### Task 23: Scene Renderer with Transitions

**Description**: Implement the main scene renderer that displays narrative text, manages transitions between scenes, and applies atmospheric effects.

**Acceptance Criteria**:
- Fade-in/fade-out transitions between scenes
- Atmosphere-based background effects (color tinting, vignette intensity)
- Narrative text display using Typewriter component
- Image display area (when AI generates images)
- Smooth scroll to new content
- Scene history preserved (can scroll up to see previous scenes)
- Loading state while waiting for AI generation

**Required Tests**:
```typescript
// GameScreen.test.tsx
test('displays narrative text')
test('shows choices after text completes')
test('transitions between scenes with fade')
test('applies atmosphere-based visual effects')
test('shows loading indicator during AI generation')
test('preserves previous scenes in scroll history')
test('image displays when provided')
test('handles missing image gracefully')
```

---

### Task 24: Choice Interface with Behavior Tracking

**Description**: Implement the player choice UI that also secretly times responses and tracks behavior.

**Acceptance Criteria**:
- Choices appear one by one (staggered reveal, 200ms between)
- Hover effects (subtle glow, slight movement)
- Click triggers choice submission + behavior data capture
- Time from choice display to click is measured
- Which choice was hovered longest is tracked
- Choice text uses appropriate horror typography
- Visual feedback on selection (brief flash, then scene transition)
- Keyboard shortcuts (1, 2, 3, 4 for choices)

**Required Tests**:
```typescript
// ChoicePanel.test.tsx
test('renders all choices')
test('staggers choice appearance')
test('hover effect applies')
test('click sends choice message')
test('choice timing is measured correctly')
test('keyboard shortcuts work')
test('selected choice shows feedback')
test('choices disabled after selection')
test('behavior data sent with choice')
```

---

### Task 25: Image Display with Horror Effects

**Description**: Implement the AI-generated image display component with horror-themed presentation effects.

**Acceptance Criteria**:
- Images fade in slowly from darkness
- Optional glitch effect on image (RGB channel split, scan lines)
- Image can "flash" briefly (subliminal horror)
- Loading state: dark shape slowly forming
- Error state: corrupted image aesthetic (intentional glitch)
- Responsive sizing
- Lazy loading (only when image enters viewport or is triggered)

**Required Tests**:
```typescript
// HorrorImage.test.tsx
test('fades in image from darkness')
test('applies glitch effect when directed')
test('handles loading state with placeholder')
test('handles error state with corrupted aesthetic')
test('responds to display_mode directive')
test('flash mode shows image briefly')
```

---

### Task 26: Audio Engine

**Description**: Implement the procedural audio engine using Web Audio API for ambient horror sounds and dynamic audio cues.

**Acceptance Criteria**:
- **Ambient drone**: low-frequency continuous sound, adjustable tone
- **Heartbeat**: procedural heartbeat sound, BPM adjustable (resting: 60, anxious: 120, panic: 160)
- **Sound cues**: triggered from narrative (door creaking, footsteps, whispers)
- **Binaural effects**: spatial audio for headphone users (sounds from left/right/behind)
- Volume management: ambient is quiet, cues are louder, jump scares are sharp
- User mute control
- Headphone recommendation on start

**Required Tests**:
```typescript
// AudioEngine.test.ts (using AudioContext mock)
test('creates audio context on init')
test('plays ambient drone')
test('adjusts heartbeat BPM')
test('triggers sound cue')
test('respects mute setting')
test('disposes audio resources on cleanup')
test('resumes context after user interaction')

// useAudio.test.ts
test('connects to game state')
test('adjusts audio based on intensity')
test('plays sound cues from narrative messages')
```

---

## Phase 6: Integration & Polish (Tasks 27–30)

---

### Task 27: End-to-End Game Loop Integration

**Description**: Wire everything together into a complete playable game loop. This is the big integration task.

**Acceptance Criteria**:
- Full game flow: Start → Calibration → Exploration → Escalation → Climax → Reveal
- Frontend sends behavior data continuously
- Backend updates fear profile in real-time
- AI generates adaptive content based on fear profile
- Images generated at key moments
- Scene transitions are smooth
- Audio responds to game state
- Meta-horror elements trigger at appropriate times
- Game completes within 15-25 minutes playtime
- No crashes, panics, or unhandled errors

**Required Tests**:
```rust
// Backend integration
#[tokio::test] async fn test_full_game_loop_with_simulated_player()
#[tokio::test] async fn test_game_loop_handles_disconnection_and_reconnection()
#[tokio::test] async fn test_game_loop_with_ai_api_failure_fallback()
#[tokio::test] async fn test_game_loop_fear_profile_evolves_correctly()
#[tokio::test] async fn test_game_loop_phase_transitions_at_correct_times()
```

```typescript
// Frontend E2E (Playwright)
test('complete game playthrough - curious player')
test('complete game playthrough - anxious player')
test('game continues after WebSocket reconnection')
test('game handles slow AI responses gracefully')
```

**Integration Checklist**:
- [ ] WebSocket connection established on page load
- [ ] Start screen → click → calibration begins
- [ ] Behavior tracker starts collecting immediately
- [ ] First 3 scenes use static calibration content
- [ ] Scene 4+ begin using AI-generated content
- [ ] Fear profile displayed in debug panel (dev mode only)
- [ ] Phase transitions happen automatically
- [ ] Images appear at least 3 times during a playthrough
- [ ] Audio changes with intensity
- [ ] At least 1 meta-horror moment occurs
- [ ] Game ends with fear reveal screen

---

### Task 28: Fear Reveal Screen

**Description**: Implement the end-game screen that shows the player what the AI learned about their fears. This is the "wow moment" of the demo.

**Acceptance Criteria**:
- **Animated fear radar chart**: shows all 10 fear scores as a radar/spider chart
- **Key moments timeline**: shows 3-5 moments where the AI detected strongest fear reactions
- **Adaptation reveals**: "Because you hesitated at the mirror, I gave you more doppelganger content"
- **Personal fear summary**: 2-3 sentences describing the player's fear profile in natural language
- **Comparison**: "You are more afraid of X than 73% of players" (mock data for demo)
- **Share button**: generate shareable image of fear profile
- Dramatic reveal animation (scores fill in one by one)

**Required Tests**:
```typescript
// FearReveal.test.tsx
test('renders radar chart with correct scores')
test('displays key moments with descriptions')
test('shows adaptation reveals')
test('generates natural language summary')
test('animation plays on mount')
test('share button generates image')
test('handles missing data gracefully')
```

**Radar Chart Implementation**: Use `recharts` RadarChart or custom SVG with Framer Motion animations.

---

### Task 29: Performance Optimization

**Description**: Profile and optimize for production performance targets.

**Acceptance Criteria**:
- Backend: < 10ms fear profile update latency
- Backend: supports 50 concurrent WebSocket connections
- Frontend: 60fps during all effects
- Frontend: < 3s initial load time
- AI response: < 5s average (with streaming partial display)
- Memory: < 100MB per backend session
- Bundle size: < 500KB gzipped for frontend

**Required Tests**:
```rust
// Benchmark tests (criterion crate)
#[bench] fn bench_fear_profile_update()
#[bench] fn bench_behavior_batch_processing()
#[bench] fn bench_prompt_building()
#[bench] fn bench_scene_graph_traversal()

// Load tests
#[tokio::test] async fn test_50_concurrent_websocket_connections()
#[tokio::test] async fn test_behavior_batch_throughput_1000_per_second()
```

```typescript
// Frontend performance
test('initial bundle size under 500KB gzipped')
test('typewriter effect maintains 60fps')
test('glitch effect maintains 60fps')
test('scene transition maintains 60fps')
```

---

### Task 30: Demo Mode & Presentation Prep

**Description**: Create a polished demo mode for presentation, including scripted paths, debug overlays, and presentation aids.

**Acceptance Criteria**:
- **Demo mode**: pre-configured game that showcases all features in 5 minutes
- **Speed controls**: 2x, 4x game speed for demo
- **Debug overlay**: real-time fear profile visualization (toggleable, only for presenter)
- **Scripted path**: auto-play with pre-recorded "player" behavior
- **Reset button**: instant game reset for multiple demo runs
- **Presentation notes**: markdown doc with talking points
- **Video fallback**: screen recording of ideal playthrough

**Required Tests**:
```typescript
test('demo mode starts with scripted behavior')
test('speed controls affect game pacing')
test('debug overlay shows real-time fear data')
test('reset returns to start screen')
test('demo completes within 5 minutes at 1x speed')
```
