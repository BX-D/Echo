import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, screen, act } from "@testing-library/react";
import FearReveal from "./FearReveal";
import type { RevealPayload } from "../types/ws";

beforeEach(() => vi.useFakeTimers());
afterEach(() => vi.useRealTimers());

function sampleReveal(): RevealPayload {
  return {
    fear_profile: {
      scores: [
        { fear_type: "darkness", score: 0.85, confidence: 0.7 },
        { fear_type: "isolation", score: 0.65, confidence: 0.5 },
        { fear_type: "claustrophobia", score: 0.4, confidence: 0.3 },
        { fear_type: "body_horror", score: 0.3, confidence: 0.2 },
        { fear_type: "stalking", score: 0.5, confidence: 0.4 },
        { fear_type: "loss_of_control", score: 0.45, confidence: 0.3 },
        { fear_type: "uncanny_valley", score: 0.55, confidence: 0.4 },
        { fear_type: "sound_based", score: 0.35, confidence: 0.2 },
        { fear_type: "doppelganger", score: 0.6, confidence: 0.5 },
        { fear_type: "abandonment", score: 0.25, confidence: 0.1 },
      ],
      primary_fear: "darkness",
      secondary_fear: "isolation",
      total_observations: 150,
    },
    behavior_profile: {
      compliance: 0.72,
      resistance: 0.28,
      curiosity: 0.8,
      avoidance: 0.22,
      self_editing: 0.41,
      need_for_certainty: 0.63,
      ritualized_control: 0.37,
      recovery_after_escalation: 0.58,
      tolerance_after_violation: 0.77,
    },
    session_summary: {
      duration_seconds: 2880,
      total_beats: 14,
      focus_interruptions: 1,
      camera_permission_granted: true,
      microphone_permission_granted: false,
      contradiction_count: 2,
      media_exposures: [
        { medium: "chat", count: 4 },
        { medium: "archive", count: 3 },
        { medium: "mirror", count: 2 },
      ],
      completion_reason: "completed",
    },
    key_moments: [
      {
        scene_id: "probe_darkness",
        description: "You froze when the lights went out",
        fear_revealed: "darkness",
        behavior_trigger: "3.2s pause",
      },
      {
        scene_id: "probe_isolation",
        description: "Your typing speed dropped in the empty ward",
        fear_revealed: "isolation",
        behavior_trigger: "50% typing slowdown",
      },
    ],
    adaptation_log: [
      {
        scene_id: "s8",
        strategy: "gradual_escalation",
        fear_targeted: "darkness",
        intensity: 0.7,
      },
    ],
    ending_classification: "curious_accomplice",
    analysis: {
      summary: "Your run centered most strongly on Darkness with Isolation close behind.",
      key_patterns: [
        "You froze when sensory certainty collapsed.",
        "Empty spaces slowed your input noticeably.",
        "The adaptation layer kept steering back toward darkness.",
      ],
      adaptation_summary: "The system escalated darkness cues because that axis stayed strongest.",
      closing_message: "By the end, the clearest pattern was how often darkness kept resurfacing.",
    },
  };
}

describe("FearReveal", () => {
  it("renders with correct scores after animation", () => {
    render(<FearReveal data={sampleReveal()} />);
    // Advance through all 10 bar reveals (200ms each), stepping through React re-renders.
    for (let t = 0; t < 15; t++) act(() => vi.advanceTimersByTime(300));
    const bar = screen.getByTestId("bar-darkness");
    expect(bar.style.width).toBe("85%");
  });

  it("displays key moments with descriptions", () => {
    render(<FearReveal data={sampleReveal()} />);
    for (let t = 0; t < 15; t++) act(() => vi.advanceTimersByTime(300));
    const moments = screen.getAllByTestId("key-moment");
    expect(moments.length).toBe(2);
    expect(moments[0]!.textContent).toContain("lights went out");
  });

  it("shows adaptation reveals", () => {
    render(<FearReveal data={sampleReveal()} />);
    for (let t = 0; t < 15; t++) act(() => vi.advanceTimersByTime(300));
    expect(screen.getByTestId("adaptations")).toBeInTheDocument();
    const entries = screen.getAllByTestId("adaptation-entry");
    expect(entries.length).toBe(1);
    expect(entries[0]!.textContent).toContain("Darkness");
  });

  it("generates natural language summary", () => {
    render(<FearReveal data={sampleReveal()} />);
    for (let t = 0; t < 15; t++) act(() => vi.advanceTimersByTime(300));
    const summary = screen.getByTestId("fear-summary");
    expect(summary.textContent).toContain("Darkness");
    expect(summary.textContent).toContain("Isolation");
  });

  it("shows the behavior-first verdict on mount", () => {
    render(<FearReveal data={sampleReveal()} />);
    expect(
      screen.getByText(/curiosity kept you inside the mechanism/i),
    ).toBeInTheDocument();
  });

  it("handles missing data gracefully", () => {
    const minimal: RevealPayload = {
      fear_profile: {
        scores: [],
        primary_fear: "darkness",
        secondary_fear: null,
        total_observations: 0,
      },
      behavior_profile: {
        compliance: 0,
        resistance: 0,
        curiosity: 0,
        avoidance: 0,
        self_editing: 0,
        need_for_certainty: 0,
        ritualized_control: 0,
        recovery_after_escalation: 0,
        tolerance_after_violation: 0,
      },
      session_summary: {
        duration_seconds: 0,
        total_beats: 0,
        focus_interruptions: 0,
        camera_permission_granted: null,
        microphone_permission_granted: null,
        contradiction_count: 0,
        media_exposures: [],
        completion_reason: "completed",
      },
      key_moments: [],
      adaptation_log: [],
      ending_classification: "quiet_exit",
      analysis: {
        summary: "No strong pattern emerged.",
        key_patterns: [],
        adaptation_summary: "The system stayed conservative.",
        closing_message: "The data stayed sparse through the run.",
      },
    };
    render(<FearReveal data={minimal} />);
    for (let t = 0; t < 15; t++) act(() => vi.advanceTimersByTime(300));
    expect(screen.getByTestId("fear-reveal")).toBeInTheDocument();
    expect(screen.queryByTestId("key-moments")).toBeNull();
  });

  it("shows AI analysis text", () => {
    render(<FearReveal data={sampleReveal()} />);
    for (let t = 0; t < 15; t++) act(() => vi.advanceTimersByTime(300));
    expect(screen.getByTestId("analysis-closing").textContent).toContain(
      "darkness",
    );
  });

  it("shows ending classification badge", () => {
    render(<FearReveal data={sampleReveal()} />);
    expect(screen.getByTestId("ending-classification").textContent).toContain(
      "Curious Accomplice",
    );
  });

  it("formats reveal metadata for display", () => {
    render(<FearReveal data={sampleReveal()} />);
    for (let t = 0; t < 15; t++) act(() => vi.advanceTimersByTime(300));

    expect(screen.getByText(/Surface: Darkness/i)).toBeInTheDocument();
    expect(screen.getByText(/Chat 4/i)).toBeInTheDocument();
  });
});
