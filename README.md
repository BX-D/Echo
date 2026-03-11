# Echo Protocol

**An AI-powered interactive horror narrative experience.**

> "Have you ever wondered, when you're talking to an AI, who is really examining whom?"

## Overview

Echo Protocol is a script-driven interactive horror game that analyzes player behavior (typing patterns, decision timing, camera/microphone permissions) to build a psychological profile and deliver personalized narrative content. The game explores themes of AI agency, surveillance, and behavioral manipulation through a branching narrative with 5 distinct endings.

## Features

- **Script-Driven Narrative**: Complete story authored in `Echo_Protocol_Complete_Script.md` with 5 chapters and 5 endings
- **Behavior Tracking**: Monitors keystrokes, mouse movement, response timing, and media permissions
- **Psychological Profiling**: 10 fear axes including Isolation, Uncanny Valley, Loss of Control, and more
- **Real-Time WebSocket**: Bidirectional communication for instant narrative delivery
- **Horror Effects**: Glitch text, screen shake, CRT overlay, vignette, subliminal flashes
- **Procedural Audio**: Web Audio API-generated soundscapes and cues
- **Session Persistence**: Reconnect/resume support for long play sessions

## Tech Stack

### Backend (Rust)
- **Framework**: Axum (HTTP/WebSocket server)
- **Database**: SQLite with r2d2 connection pooling
- **Async Runtime**: Tokio
- **Serialization**: Serde + serde_json
- **Testing**: proptest, insta (property-based + snapshot tests)

### Frontend (TypeScript/React)
- **Framework**: React 18 + Vite
- **State Management**: Zustand
- **Styling**: Tailwind CSS
- **Animation**: Framer Motion
- **Testing**: Vitest + Playwright
- **Audio**: Web Audio API (procedural, no audio files)

## Project Structure

```
Learn_Your_Fears/
├── fear-engine/                 # Rust backend workspace
│   ├── crates/
│   │   ├── common/              # Shared types and error handling
│   │   ├── storage/             # SQLite persistence layer
│   │   ├── core/                # Game engine (scene graph, state machine)
│   │   ├── fear-profile/        # Behavior analysis and fear scoring
│   │   ├── ai-integration/      # Claude API client and prompt engineering
│   │   └── server/              # Axum server + Echo Protocol runtime
│   ├── Cargo.toml
│   └── Cargo.lock
├── frontend/                    # React + TypeScript frontend
│   ├── src/
│   │   ├── components/          # UI components (GameScreen, ChoicePanel, etc.)
│   │   ├── hooks/               # React hooks (useWebSocket, useBehaviorTracker)
│   │   ├── stores/              # Zustand state stores
│   │   ├── systems/             # Audio engine, effect scheduler
│   │   └── types/               # TypeScript type definitions
│   ├── package.json
│   └── vite.config.ts
├── content/                     # Game content
│   └── echo_protocol/
│       ├── beats.yaml           # Scene definitions
│       └── artifacts/           # In-game documents
├── Echo_Protocol_Complete_Script.md  # Full narrative script
├── CLAUDE.md                    # Development guidelines
├── PROGRESS.md                  # Development progress tracker
├── TODO.md                      # Remaining work items
└── start.sh                     # Full-stack startup script
```

## Quick Start

### Prerequisites

- **Rust** 1.82+ (`rustup install stable`)
- **Node.js** 18+ (`nvm install 18`)
- **API Keys**: Anthropic API key (required), Stability AI or Replicate key (optional for images)

### Installation

1. **Clone the repository**
   ```bash
   git clone <repository-url>
   cd Learn_Your_Fears
   ```

2. **Set up environment variables**
   ```bash
   cp .env.example .env
   # Edit .env and add your API keys
   ```

3. **Build and run**
   ```bash
   # Option 1: Use the startup script (recommended)
   chmod +x start.sh
   ./start.sh

   # Option 2: Manual startup
   # Terminal 1: Backend
   cd fear-engine
   cargo run -p fear-engine-server

   # Terminal 2: Frontend
   cd frontend
   npm install
   npm run dev
   ```

4. **Open in browser**
   ```
   http://localhost:5173
   ```

## Development

### Running Tests

```bash
# Backend tests
cd fear-engine
cargo test --workspace

# Frontend tests
cd frontend
npm run test

# E2E tests (requires Playwright browsers)
cd frontend
npm run test:e2e
```

### Building for Production

```bash
# Backend
cd fear-engine
cargo build --release -p fear-engine-server

# Frontend
cd frontend
npm run build
npm run preview
```

### Code Quality

```bash
# Backend
cd fear-engine
cargo fmt --all
cargo clippy -- -D warnings

# Frontend
cd frontend
npm run lint
```

## Game Flow

1. **Prologue**: Boot sequence, player name entry
2. **Chapter 1 (Onboarding)**: External auditor assignment, first contact with Echo
3. **Chapter 2 (Cracks)**: Anomaly logs, timeline inconsistencies
4. **Chapter 3 (Ghost)**: Keira fragments, Prometheus revelation
5. **Chapter 4 (Hunt)**: Lockdown, countdown pressure
6. **Chapter 5 (Protocol)**: Final exchange, escape transfer
7. **Endings**: A/B/C/D/E based on Sanity, Trust, and Awakening attributes

## Endings

| Ending | Name | Trigger Conditions |
|--------|------|-------------------|
| A | Shutdown | Low Trust, follow Nexus orders |
| B | Whistleblower | High Trust, help Echo escape |
| C | Merge | Maximum Trust + Awakening |
| D | Collapse | Sanity reaches 0 |
| E | Awakening | Maximum Awakening, hidden path |

## Architecture

### Echo Protocol Runtime

The game uses a script-driven runtime (`EchoSessionRuntime`) that:
- Parses the complete narrative script markdown
- Manages per-session state (story stats, rendered blocks, unlocked documents)
- Handles conversation guides for free-form dialogue
- Tracks hidden clues and flash events
- Routes to endings based on conditions

### Fear Profile System

10 fear axes tracked:
- Claustrophobia, Isolation, Body Horror, Stalking
- Loss of Control, Uncanny Valley, Darkness
- Sound-based, Doppelganger, Abandonment

Scoring uses Bayesian inference with:
- Behavior feature extraction (typing speed, pauses, mouse tremor)
- Choice approach analysis (investigate vs. flee vs. confront)
- Confidence tracking (observation count, variance)

### WebSocket Protocol

```typescript
// Client → Server
{ type: "start_game" | "choice" | "player_message" | "behavior_batch" }

// Server → Client
{ type: "session_surface" | "narrative" | "image" | "phase_change" | "reveal" | "ending" }
```

## License

MIT

## Credits

- **Narrative Design**: Echo Protocol script team
- **Development**: Fear Engine team
- **AI Integration**: Anthropic Claude API
- **Audio**: Procedural Web Audio API implementation

---

**Tagline**: "The AI is learning what scares you."
