import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import { SESSION_STORAGE_KEY, useGameStore } from "./stores/gameStore";

vi.mock("./hooks/useWebSocket", () => ({
  useWebSocket: () => ({ send: vi.fn() }),
}));

import App from "./App";

beforeEach(() => {
  useGameStore.getState().reset();
  window.localStorage.clear();
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

function sampleSurface() {
  return {
    session_id: "test-session",
    case_title: "Nexus AI Labs / Echo Audit",
    scene_id: "scene_1_4",
    chapter: "onboarding" as const,
    scene_title: "First Contact",
    scene_mode: "chat" as const,
    blocks: [],
    documents: [],
    scene_choices: [],
    active_conversation_guide: null,
    flash_events: [],
    transition_state: null,
    hidden_clue_state: {
      discovered_ids: [],
      rendered_flash_ids: [],
    },
    ending_override: null,
    beat: {
      id: "first_contact",
      chapter: "onboarding" as const,
      title: "First Contact",
      input_mode: "hybrid" as const,
      freeform_topics: ["training data"],
      forced_clue_queue: ["burning_smell"],
      reconverge_beat_id: "anomaly_logs",
      fallback_reply: "Echo waits.",
    },
    status_line: "Day 1 / External Safety Review",
    input_enabled: true,
    input_placeholder: "Ask Echo anything.",
    transcript: [
      {
        id: "t1",
        sequence: 1,
        role: "system" as const,
        speaker: "Audit Shell",
        text: "Echo is live.",
        glitch: false,
      },
    ],
    inline_choices: [],
    investigation_items: [],
    system_alerts: [],
    sanity: 96,
    trust: 53,
    awakening: 3,
    echo_mode: "normal" as const,
    available_panels: ["briefing"],
    active_panel: "briefing",
    shutdown_countdown: null,
    glitch_level: 0.18,
    suggested_glitches: [],
    sound_cue: null,
    image_prompt: null,
    provisional: false,
  };
}

describe("App routing", () => {
  it("shows LoadingScreen when disconnected", () => {
    useGameStore.setState({
      connectionStatus: "disconnected",
      sessionId: "test-session",
    });
    render(<App />);
    expect(screen.getAllByText(/connecting/i).length).toBeGreaterThan(0);
  });

  it("shows StartScreen when connected with no active surface", () => {
    useGameStore.setState({
      connectionStatus: "connected",
      sessionId: "test-session",
    });
    render(<App />);
    expect(screen.getByText(/audit echo/i)).toBeInTheDocument();
    expect(screen.getByText(/press enter to begin/i)).toBeInTheDocument();
  });

  it("restores a persisted session id before creating a new one", () => {
    window.localStorage.setItem(SESSION_STORAGE_KEY, "persisted-session");
    render(<App />);
    expect(useGameStore.getState().sessionId).toBe("persisted-session");
  });

  it("shows audit terminal when a surface is active", () => {
    useGameStore.setState({
      connectionStatus: "connected",
      sessionId: "test-session",
      currentSurface: sampleSurface(),
    });
    render(<App />);
    expect(screen.getByTestId("audit-transcript")).toBeInTheDocument();
    expect(screen.getAllByText(/first contact/i).length).toBeGreaterThan(0);
  });

  it("shows ending screen when currentEnding is set", () => {
    useGameStore.setState({
      connectionStatus: "connected",
      sessionId: "test-session",
      currentEnding: {
        ending: "shutdown",
        trigger_scene: "ending_a",
        title: "The Shutdown",
        summary: "You deliver the recommendation Nexus wanted.",
        epilogue: "A final line flashes.",
        dominant_mode: "hostile",
        evidence_titles: ["Engagement Clause 8.4"],
        hidden_clue_ids: ["subject_label"],
        satisfied_conditions: ["trust=18"],
        resolved_clues: ["subject_label"],
        sanity: 40,
        trust: 18,
        awakening: 15,
      },
    });
    render(<App />);
    expect(screen.getByTestId("ending-screen")).toBeInTheDocument();
    expect(screen.getByText(/the shutdown/i)).toBeInTheDocument();
  });

  it("renders meta overlay when currentMeta is present", () => {
    useGameStore.setState({
      connectionStatus: "connected",
      sessionId: "test-session",
      currentMeta: {
        text: "Something rewrites the window title.",
        target: "overlay",
        delay_ms: 500,
      },
    });
    render(<App />);
    expect(screen.getByTestId("meta-overlay")).toBeInTheDocument();
  });
});
