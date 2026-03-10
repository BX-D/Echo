import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import DebugOverlay from "./DebugOverlay";
import { useGameStore } from "../stores/gameStore";

beforeEach(() => {
  useGameStore.getState().reset();
});

function renderOverlay(overrides?: Partial<Parameters<typeof DebugOverlay>[0]>) {
  const defaults = {
    visible: true,
    speedMultiplier: 1,
    onSpeedChange: vi.fn(),
    onReset: vi.fn(),
  };
  return render(<DebugOverlay {...defaults} {...overrides} />);
}

describe("DebugOverlay", () => {
  it("shows debug data when visible", () => {
    useGameStore.setState({
      gamePhase: "exploring",
      currentScene: {
        scene_id: "s5",
        text: "t",
        atmosphere: "dread",
        choices: [],
        sound_cue: null,
        intensity: 0.65,
        effects: [],
        title: null,
        act: "contamination",
        medium: "system_dialog",
        trust_posture: "manipulative",
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
      sceneHistory: [
        {
          scene_id: "s1",
          text: "a",
          atmosphere: "calm",
          choices: [],
          sound_cue: null,
          intensity: 0.1,
          effects: [],
          title: null,
          act: "invitation",
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
      ],
    });
    renderOverlay();
    expect(screen.getByTestId("debug-overlay")).toBeInTheDocument();
    expect(screen.getByText("exploring")).toBeInTheDocument();
    expect(screen.getByText("s5")).toBeInTheDocument();
  });

  it("hidden when visible=false", () => {
    renderOverlay({ visible: false });
    expect(screen.queryByTestId("debug-overlay")).toBeNull();
  });

  it("speed controls change speed", () => {
    const onSpeedChange = vi.fn();
    renderOverlay({ onSpeedChange });
    fireEvent.click(screen.getByTestId("speed-2x"));
    expect(onSpeedChange).toHaveBeenCalledWith(2);
    fireEvent.click(screen.getByTestId("speed-4x"));
    expect(onSpeedChange).toHaveBeenCalledWith(4);
  });

  it("reset button calls onReset", () => {
    const onReset = vi.fn();
    renderOverlay({ onReset });
    fireEvent.click(screen.getByTestId("debug-reset"));
    expect(onReset).toHaveBeenCalledTimes(1);
  });

  it("highlights current speed multiplier", () => {
    renderOverlay({ speedMultiplier: 4 });
    const btn = screen.getByTestId("speed-4x");
    expect(btn.className).toContain("border-bone");
  });
});
