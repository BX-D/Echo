import { describe, it, expect, beforeEach } from "vitest";
import { useGameStore } from "./gameStore";
import type { NarrativePayload, PhaseChangePayload, RevealPayload } from "../types/ws";

function store() {
  return useGameStore.getState();
}

beforeEach(() => {
  store().reset();
});

describe("gameStore", () => {
  it("starts with default state", () => {
    const s = store();
    expect(s.connectionStatus).toBe("disconnected");
    expect(s.sessionId).toBeNull();
    expect(s.gamePhase).toBeNull();
    expect(s.currentScene).toBeNull();
    expect(s.sceneHistory).toHaveLength(0);
  });

  it("setConnectionStatus updates connection status", () => {
    store().setConnectionStatus("connected");
    expect(store().connectionStatus).toBe("connected");
  });

  it("setSessionId stores the session id", () => {
    store().setSessionId("abc-123");
    expect(store().sessionId).toBe("abc-123");
  });

  it("processNarrative sets currentScene and appends to history", () => {
    const msg: NarrativePayload = {
      scene_id: "intro",
      text: "You awaken.",
      atmosphere: "dread",
      choices: [],
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
    };
    store().processNarrative(msg);
    expect(store().currentScene).toEqual(msg);
    expect(store().sceneHistory).toHaveLength(1);

    const msg2: NarrativePayload = { ...msg, scene_id: "hallway" };
    store().processNarrative(msg2);
    expect(store().currentScene?.scene_id).toBe("hallway");
    expect(store().sceneHistory).toHaveLength(2);
  });

  it("processPhaseChange updates gamePhase to target phase", () => {
    const msg: PhaseChangePayload = {
      from: "calibrating",
      to: "exploring",
    };
    store().processPhaseChange(msg);
    expect(store().gamePhase).toBe("exploring");
  });

  it("processMeta stores meta payload", () => {
    store().processMeta({
      text: "I see you.",
      target: "whisper",
      delay_ms: 500,
    });
    expect(store().currentMeta?.text).toBe("I see you.");
  });

  it("processImage stores image payload", () => {
    store().processImage({
      scene_id: "s1",
      image_url: "data:image/png;base64,abc",
      display_mode: "fade_in",
    });
    expect(store().currentImage?.image_url).toContain("abc");
  });

  it("processReveal stores reveal data and sets phase to reveal", () => {
    const reveal: RevealPayload = {
      fear_profile: {
        scores: [{ fear_type: "darkness", score: 0.9, confidence: 0.8 }],
        primary_fear: "darkness",
        secondary_fear: null,
        total_observations: 100,
      },
      behavior_profile: {
        compliance: 0.6,
        resistance: 0.3,
        curiosity: 0.7,
        avoidance: 0.2,
        self_editing: 0.4,
        need_for_certainty: 0.5,
        ritualized_control: 0.3,
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
        key_patterns: ["You froze when the lights went out."],
        adaptation_summary: "The system escalated darkness cues over time.",
        closing_message: "Darkness kept resurfacing.",
      },
    };
    store().processReveal(reveal);
    expect(store().revealData).toEqual(reveal);
    expect(store().gamePhase).toBe("reveal");
  });

  it("reset returns to initial state", () => {
    store().setConnectionStatus("connected");
    store().setSessionId("test");
    store().processNarrative({
      scene_id: "s",
      text: "t",
      atmosphere: "calm",
      choices: [],
      sound_cue: null,
      intensity: 0,
      effects: [],
      title: null,
      act: "invitation",
      medium: "system_dialog",
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
    });

    store().reset();
    expect(store().connectionStatus).toBe("disconnected");
    expect(store().sessionId).toBeNull();
    expect(store().currentScene).toBeNull();
    expect(store().sceneHistory).toHaveLength(0);
  });
});
