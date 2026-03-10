import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, screen } from "@testing-library/react";
import { useGameStore } from "./stores/gameStore";

// Mock useWebSocket so App doesn't open a real connection
vi.mock("./hooks/useWebSocket", () => ({
  useWebSocket: () => ({ send: vi.fn() }),
}));

// Import after mocking
import App from "./App";

beforeEach(() => {
  useGameStore.getState().reset();
  vi.stubGlobal(
    "fetch",
    vi.fn().mockResolvedValue({
      ok: true,
      json: async () => ({ session_id: "test-session" }),
    }),
  );
});

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("App routing", () => {
  it("shows LoadingScreen when disconnected", () => {
    useGameStore.setState({
      connectionStatus: "disconnected",
      sessionId: "test-session",
    });
    render(<App />);
    expect(screen.getByText(/connecting/i)).toBeInTheDocument();
  });

  it("shows LoadingScreen when connecting", () => {
    useGameStore.setState({
      connectionStatus: "connecting",
      sessionId: "test-session",
    });
    render(<App />);
    expect(screen.getAllByText(/connecting/i).length).toBeGreaterThan(0);
  });

  it("shows StartScreen when connected with no active scene", () => {
    useGameStore.setState({
      connectionStatus: "connected",
      sessionId: "test-session",
    });
    render(<App />);
    expect(screen.getByText(/it learns your fear/i)).toBeInTheDocument();
    expect(screen.getByText(/press enter to begin/i)).toBeInTheDocument();
  });

  it("shows StartScreen after the camera step is completed", () => {
    useGameStore.setState({
      connectionStatus: "connected",
      sessionId: "test-session",
      cameraStepDone: true,
    });
    render(<App />);
    expect(screen.getByText(/it learns your fear/i)).toBeInTheDocument();
    expect(screen.getByText(/press enter to begin/i)).toBeInTheDocument();
  });

  it("shows game scene when currentScene is set beyond welcome", () => {
    useGameStore.setState({
      connectionStatus: "connected",
      sessionId: "test-session",
      currentScene: {
        scene_id: "intro",
        text: "The corridor stretches before you.",
        atmosphere: "dread",
        choices: [
          {
            id: "c1",
            text: "Go left",
            approach: "investigate",
            fear_vector: "darkness",
          },
        ],
        sound_cue: null,
        intensity: 0.3,
        effects: [],
        title: null,
        act: "calibration",
        medium: "chat",
        trust_posture: "helpful",
        status_line: null,
        observation_notes: [],
        trace_items: [],
        transcript_lines: [],
        question_prompts: [],
        archive_entries: [],
        mirror_observations: [],
        surface_label: null,
        auxiliary_text: null,
        provisional: false,
      },
    });
    render(<App />);
    // GameScreen renders with a Typewriter — check the container exists.
    expect(screen.getByTestId("game-screen")).toBeInTheDocument();
  });

  it("shows reveal screen when phase is reveal", () => {
    useGameStore.setState({
      connectionStatus: "connected",
      sessionId: "test-session",
      gamePhase: "reveal",
      revealData: {
        fear_profile: {
          scores: [{ fear_type: "darkness", score: 0.9, confidence: 0.8 }],
          primary_fear: "darkness",
          secondary_fear: null,
          total_observations: 100,
        },
        behavior_profile: {
          compliance: 0.6,
          resistance: 0.4,
          curiosity: 0.7,
          avoidance: 0.2,
          self_editing: 0.3,
          need_for_certainty: 0.5,
          ritualized_control: 0.4,
          recovery_after_escalation: 0.6,
          tolerance_after_violation: 0.7,
        },
        session_summary: {
          duration_seconds: 1800,
          total_beats: 12,
          focus_interruptions: 1,
          camera_permission_granted: true,
          microphone_permission_granted: false,
          contradiction_count: 2,
          media_exposures: [],
          completion_reason: "completed",
        },
        key_moments: [],
        adaptation_log: [],
        ending_classification: "compliant_witness",
        analysis: {
          summary: "Darkness dominated this run.",
          key_patterns: ["You slowed down in darkness-heavy scenes."],
          adaptation_summary: "The system escalated darkness cues over time.",
          closing_message: "Darkness kept resurfacing.",
        },
      },
    });
    render(<App />);
    expect(
      screen.getByText(/you stayed long enough to be modeled/i),
    ).toBeInTheDocument();
    expect(screen.getAllByText(/darkness/i).length).toBeGreaterThan(0);
  });

  it("renders meta overlay when currentMeta is present", () => {
    useGameStore.setState({
      connectionStatus: "connected",
      sessionId: "test-session",
      currentMeta: {
        text: "You looked away exactly when it became specific.",
        target: "overlay",
        delay_ms: 500,
      },
    });
    render(<App />);
    expect(screen.getByTestId("meta-overlay")).toBeInTheDocument();
  });
});
