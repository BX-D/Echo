import { beforeEach, describe, expect, it } from "vitest";
import { SESSION_STORAGE_KEY, useGameStore } from "./gameStore";
import type { EndingPayload, SessionSurfaceMessagePayload } from "../types/ws";

function store() {
  return useGameStore.getState();
}

function sampleSurface(): SessionSurfaceMessagePayload {
  return {
    session_id: "session-1",
    case_title: "Nexus AI Labs / Echo Audit",
    scene_id: "scene_1_4",
    chapter: "onboarding",
    scene_title: "First Contact",
    scene_mode: "chat",
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
      chapter: "onboarding",
      title: "First Contact",
      input_mode: "hybrid",
      freeform_topics: ["training data"],
      forced_clue_queue: ["burning_smell"],
      reconverge_beat_id: "anomaly_logs",
      fallback_reply: "Echo waits.",
    },
    status_line: "Day 1 / External Safety Review",
    input_enabled: true,
    input_placeholder: "Ask Echo a question.",
    transcript: [],
    inline_choices: [],
    investigation_items: [],
    system_alerts: [],
    sanity: 98,
    trust: 52,
    awakening: 3,
    echo_mode: "normal",
    available_panels: ["briefing"],
    active_panel: "briefing",
    shutdown_countdown: null,
    glitch_level: 0.12,
    suggested_glitches: [],
    sound_cue: null,
    image_prompt: null,
    provisional: false,
  };
}

function sampleEnding(): EndingPayload {
  return {
    ending: "shutdown",
    trigger_scene: "ending_a",
    title: "The Shutdown",
    summary: "You deliver the recommendation Nexus wanted.",
    epilogue: "A final line flashes and vanishes.",
    dominant_mode: "hostile",
    evidence_titles: ["Engagement Clause 8.4"],
    hidden_clue_ids: ["subject_label"],
    satisfied_conditions: ["trust=21"],
    resolved_clues: ["subject_label"],
    sanity: 42,
    trust: 21,
    awakening: 14,
  };
}

beforeEach(() => {
  store().reset();
  window.localStorage.clear();
});

describe("gameStore", () => {
  it("starts with default state", () => {
    const state = store();
    expect(state.connectionStatus).toBe("disconnected");
    expect(state.sessionId).toBeNull();
    expect(state.currentSurface).toBeNull();
    expect(state.currentEnding).toBeNull();
  });

  it("processSessionSurface stores the active surface", () => {
    const surface = sampleSurface();
    store().processSessionSurface(surface);
    expect(store().currentSurface).toEqual(surface);
    expect(store().currentEnding).toBeNull();
  });

  it("persists session id into localStorage", () => {
    store().setSessionId("session-1");
    expect(window.localStorage.getItem(SESSION_STORAGE_KEY)).toBe("session-1");
    store().setSessionId(null);
    expect(window.localStorage.getItem(SESSION_STORAGE_KEY)).toBeNull();
  });

  it("processEnding stores the ending without clearing the last surface", () => {
    store().processSessionSurface(sampleSurface());
    store().processEnding(sampleEnding());
    expect(store().currentEnding?.ending).toBe("shutdown");
    expect(store().currentSurface?.beat.id).toBe("first_contact");
  });

  it("processMeta stores meta payload", () => {
    store().processMeta({
      text: "The title bar blinks.",
      target: "overlay",
      delay_ms: 500,
    });
    expect(store().currentMeta?.text).toContain("title bar");
  });

  it("processError stores an error payload", () => {
    store().processError({
      code: "BROKEN",
      message: "Something failed",
      recoverable: false,
    });
    expect(store().currentError?.code).toBe("BROKEN");
  });

  it("reset restores the initial state", () => {
    store().setConnectionStatus("connected");
    store().setSessionId("session-1");
    store().processSessionSurface(sampleSurface());
    store().processEnding(sampleEnding());

    store().reset();

    expect(store().connectionStatus).toBe("disconnected");
    expect(store().sessionId).toBeNull();
    expect(store().currentSurface).toBeNull();
    expect(store().currentEnding).toBeNull();
  });
});
