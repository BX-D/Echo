# FRONTEND_DESIGN.md — Horror UX Specification

## Design Philosophy

The frontend doesn't just display the game — it IS part of the horror. Every animation, transition, and visual effect exists to amplify psychological unease. The design should feel like the interface itself is breaking down, like something is wrong with your computer.

---

## 1. Typography

### Font Stack
```css
/* Narrative text — old typewriter feel */
@import url('https://fonts.googleapis.com/css2?family=Special+Elite&display=swap');

/* UI elements — sterile, clinical */
@import url('https://fonts.googleapis.com/css2?family=IBM+Plex+Mono:wght@300;400;500&display=swap');

/* Title / horror moments — creepy display */
@import url('https://fonts.googleapis.com/css2?family=Creepster&display=swap');

:root {
  --font-narrative: 'Special Elite', cursive;
  --font-ui: 'IBM Plex Mono', monospace;
  --font-horror: 'Creepster', cursive;
  --font-system: -apple-system, system-ui, sans-serif;
}
```

### Scale
```css
--text-xs: 0.75rem;    /* 12px - whisper text */
--text-sm: 0.875rem;   /* 14px - UI elements */
--text-base: 1rem;     /* 16px - choices */
--text-lg: 1.125rem;   /* 18px - narrative body */
--text-xl: 1.25rem;    /* 20px - scene headers */
--text-2xl: 1.5rem;    /* 24px - phase titles */
--text-4xl: 2.25rem;   /* 36px - game title */
--text-6xl: 3.75rem;   /* 60px - fear reveal numbers */
```

---

## 2. Effect Catalog

### 2.1 Typewriter Effect

```typescript
interface TypewriterConfig {
  speed: 'slow' | 'normal' | 'fast' | 'instant';
  glitchChance: number;      // 0.0 - 1.0, default 0.05
  dramaticPauseMs: number;   // pause after . ... — !, default 300
  soundEnabled: boolean;
  cursor: boolean;
}
```

**CSS for cursor blink:**
```css
@keyframes cursor-blink {
  0%, 50% { opacity: 1; }
  51%, 100% { opacity: 0; }
}

.typewriter-cursor {
  display: inline-block;
  width: 2px;
  height: 1.2em;
  background: var(--color-bone);
  animation: cursor-blink 1s infinite;
  margin-left: 2px;
}
```

**Glitch character effect:**
```css
@keyframes glitch-char {
  0% { content: attr(data-glitch); opacity: 0.8; }
  50% { content: attr(data-glitch); opacity: 0.6; color: var(--color-blood); }
  100% { content: attr(data-char); opacity: 1; }
}
```

### 2.2 Screen Shake

```css
@keyframes screen-shake {
  0% { transform: translate(0, 0) rotate(0deg); }
  10% { transform: translate(-2px, -1px) rotate(-0.5deg); }
  20% { transform: translate(3px, 1px) rotate(0.5deg); }
  30% { transform: translate(-1px, 2px) rotate(0deg); }
  40% { transform: translate(2px, -2px) rotate(0.5deg); }
  50% { transform: translate(-3px, 1px) rotate(-0.5deg); }
  60% { transform: translate(1px, -1px) rotate(0deg); }
  70% { transform: translate(-2px, 2px) rotate(-0.5deg); }
  80% { transform: translate(3px, -1px) rotate(0.5deg); }
  90% { transform: translate(-1px, -2px) rotate(0deg); }
  100% { transform: translate(0, 0) rotate(0deg); }
}

.screen-shake {
  animation: screen-shake 0.5s ease-in-out;
}

.screen-shake-intense {
  animation: screen-shake 0.3s ease-in-out;
  /* Override with larger translate values via CSS custom properties */
  --shake-intensity: 5px;
}
```

### 2.3 Screen Flicker

```css
@keyframes flicker {
  0% { opacity: 1; }
  3% { opacity: 0.4; }
  6% { opacity: 0.9; }
  7% { opacity: 0.3; }
  8% { opacity: 0.8; }
  9% { opacity: 1; }
  50% { opacity: 1; }
  52% { opacity: 0.6; }
  53% { opacity: 1; }
  100% { opacity: 1; }
}

.screen-flicker {
  animation: flicker 3s infinite;
}
```

### 2.4 CRT Scanline Overlay

```css
.crt-overlay {
  position: fixed;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  pointer-events: none;
  z-index: 9999;
  background: repeating-linear-gradient(
    0deg,
    rgba(0, 0, 0, 0.15) 0px,
    rgba(0, 0, 0, 0.15) 1px,
    transparent 1px,
    transparent 2px
  );
}

.crt-overlay::after {
  content: '';
  position: absolute;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  background: radial-gradient(
    ellipse at center,
    transparent 60%,
    rgba(0, 0, 0, 0.4) 100%
  );
}
```

### 2.5 Vignette (Dynamic Darkness)

```css
.vignette {
  position: fixed;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  pointer-events: none;
  z-index: 100;
  box-shadow: inset 0 0 150px rgba(0, 0, 0, var(--vignette-intensity, 0.8));
  transition: box-shadow 2s ease;
}
```

Intensity controlled by game state: calm=0.5, tense=0.7, panic=0.9

### 2.6 Text Glitch (RGB Split)

```css
@keyframes text-glitch {
  0% { text-shadow: 2px 0 #ff0000, -2px 0 #00ff00; transform: translate(0); }
  20% { text-shadow: -2px 0 #ff0000, 2px 0 #00ff00; transform: translate(-2px, 1px); }
  40% { text-shadow: 2px 0 #ff0000, -2px 0 #00ff00; transform: translate(1px, -1px); }
  60% { text-shadow: -2px 0 #ff0000, 2px 0 #00ff00; transform: translate(2px, 0); }
  80% { text-shadow: 2px 0 #ff0000, -2px 0 #00ff00; transform: translate(-1px, 1px); }
  100% { text-shadow: none; transform: translate(0); }
}

.glitch-text {
  animation: text-glitch 0.3s ease-in-out;
}
```

### 2.7 Flashlight Mode (Cursor Spotlight)

```typescript
// Only area around cursor is visible
const FlashlightMode: React.FC = () => {
  const [pos, setPos] = useState({ x: 0, y: 0 });
  
  useEffect(() => {
    const handler = (e: MouseEvent) => setPos({ x: e.clientX, y: e.clientY });
    window.addEventListener('mousemove', handler);
    return () => window.removeEventListener('mousemove', handler);
  }, []);
  
  return (
    <div style={{
      position: 'fixed',
      inset: 0,
      zIndex: 1000,
      pointerEvents: 'none',
      background: `radial-gradient(circle 120px at ${pos.x}px ${pos.y}px, transparent 0%, rgba(0,0,0,0.95) 100%)`
    }} />
  );
};
```

---

## 3. Component Architecture

```
App
├── LoadingScreen
├── StartScreen
│   └── TitleAnimation
├── GameScreen
│   ├── Vignette (overlay)
│   ├── CRTOverlay (overlay, optional)
│   ├── Flashlight (overlay, triggered)
│   ├── ScreenShake (wrapper)
│   │   └── GameContent
│   │       ├── SceneHistory (scrollable past scenes)
│   │       ├── NarrativeDisplay
│   │       │   ├── Typewriter
│   │       │   └── GlitchText (for meta moments)
│   │       ├── HorrorImage (when available)
│   │       └── ChoicePanel
│   ├── StatusBar (minimal, bottom)
│   ├── AudioControls (mute button, top-right)
│   └── MetaOverlay (for meta-horror text)
├── FearReveal
│   ├── RadarChart (animated)
│   ├── KeyMoments (timeline)
│   ├── AdaptationReveals
│   ├── FearSummary (natural language)
│   └── ShareButton
└── DebugPanel (dev mode only)
    ├── FearProfileLive (real-time scores)
    ├── BehaviorStreamVisualization
    ├── GameStateInfo
    └── SpeedControls
```

---

## 4. Audio Design

### 4.1 Ambient Drone

```typescript
class AmbientDrone {
  private ctx: AudioContext;
  private oscillators: OscillatorNode[];
  private gainNode: GainNode;
  
  constructor(ctx: AudioContext) {
    // Low-frequency drone: two oscillators with slight detune
    // Creates a "beating" effect that feels unsettling
    const osc1 = ctx.createOscillator();
    osc1.type = 'sine';
    osc1.frequency.value = 55; // A1, very low
    
    const osc2 = ctx.createOscillator();
    osc2.type = 'sine';
    osc2.frequency.value = 57; // Slightly detuned = beating
    
    // Add subtle sawtooth for texture
    const osc3 = ctx.createOscillator();
    osc3.type = 'sawtooth';
    osc3.frequency.value = 110;
    
    // Very quiet
    this.gainNode = ctx.createGain();
    this.gainNode.gain.value = 0.05;
  }
  
  setTension(level: number) {
    // Higher tension = higher frequency, more dissonance
    // level: 0.0 (calm) to 1.0 (panic)
  }
}
```

### 4.2 Procedural Heartbeat

```typescript
class Heartbeat {
  private bpm: number = 60;
  
  setBPM(bpm: number) {
    // Resting: 60, Nervous: 90, Anxious: 120, Panic: 160
    this.bpm = Math.min(Math.max(bpm, 40), 180);
  }
  
  generateBeat(): AudioBuffer {
    // Two-part heartbeat: "lub-dub"
    // First beat: lower, softer (closure of AV valves)
    // Second beat: higher, sharper (closure of semilunar valves)
    // Gap between beats varies with BPM
  }
}
```

### 4.3 Sound Cues (from AI narrative)

Map AI-provided sound_cue strings to audio:
```typescript
const SOUND_CUE_MAP: Record<string, () => void> = {
  'door_creak': () => playFrequencySweep(200, 800, 0.5),
  'footsteps_distant': () => playRhythmic(400, 0.02, 6, 0.3),
  'whisper': () => playNoise('bandpass', 2000, 0.01, 2),
  'metal_scrape': () => playFrequencySweep(1000, 4000, 0.3),
  'heartbeat_loud': () => heartbeat.setBPM(140),
  'silence': () => ambientDrone.setTension(0),
  'static': () => playNoise('white', null, 0.1, 1),
  'breathing': () => playModulated('sine', 200, 0.02, 0.3),
};
```

---

## 5. Fear Reveal Screen Design

The reveal screen is the climax of the demo — it needs to be visually stunning.

### Layout
```
┌──────────────────────────────────────────────────────────────┐
│                                                              │
│                    YOUR FEAR PROFILE                          │
│                                                              │
│         ┌─────────────────────────────┐                      │
│         │                             │                      │
│         │      Radar Chart            │     KEY MOMENTS      │
│         │      (animated fill)        │                      │
│         │                             │     1. Scene 5:      │
│         │                             │     "You hesitated   │
│         │                             │      at the mirror"  │
│         └─────────────────────────────┘                      │
│                                                              │
│  ┌────────────────────────────────────────────────────────┐  │
│  │  "You are most afraid of being trapped in small spaces  │  │
│  │   with something watching you. Your instinct is to run, │  │
│  │   but your curiosity sometimes overrides your fear."    │  │
│  └────────────────────────────────────────────────────────┘  │
│                                                              │
│  HOW THE AI ADAPTED:                                         │
│  • Because you avoided the basement, I made the next room    │
│    feel smaller                                              │
│  • Your typing slowed when mirrors were mentioned, so I      │
│    added more reflective surfaces                            │
│  • You kept checking behind you, so I put something there    │
│                                                              │
│                    [ Share Your Fear Profile ]                │
│                                                              │
└──────────────────────────────────────────────────────────────┘
```

### Radar Chart Animation
- Scores start at 0 and animate outward to final values
- Each axis animates sequentially (0.3s gap between axes)
- Primary fear axis pulses red after reveal
- Total animation: ~5 seconds

### Color Coding
- Low fear (0-0.3): dim gray
- Medium fear (0.3-0.6): amber
- High fear (0.6-0.8): orange-red
- Maximum fear (0.8-1.0): deep red, pulsing

---

## 6. Responsive Design

### Breakpoints
```css
/* Mobile: not supported, show warning */
@media (max-width: 767px) {
  .mobile-warning { display: flex; }
  .game-container { display: none; }
}

/* Tablet: simplified layout */
@media (min-width: 768px) and (max-width: 1023px) {
  .game-container { padding: 1rem; }
  .narrative-text { font-size: var(--text-base); }
}

/* Desktop: full experience */
@media (min-width: 1024px) {
  .game-container { max-width: 900px; margin: 0 auto; padding: 2rem; }
  .narrative-text { font-size: var(--text-lg); }
}

/* Large: cinematic */
@media (min-width: 1440px) {
  .game-container { max-width: 1100px; }
}
```

The game is designed for desktop first. Mobile gets a message: "For the full experience, use a desktop with headphones."
