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
      currentSurface: {
        session_id: "session-1",
        case_title: "Nexus AI Labs / Echo Audit",
        scene_id: "scene_3_3",
        chapter: "ghost",
        scene_title: "Mirrored Conversation",
        scene_mode: "document",
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
          id: "s5",
          chapter: "ghost",
          title: "Mirrored Conversation",
          input_mode: "choice_only",
          freeform_topics: [],
          forced_clue_queue: [],
          reconverge_beat_id: null,
          fallback_reply: "fallback",
        },
        status_line: "Archive Comparison",
        input_enabled: false,
        input_placeholder: "",
        transcript: [{ id: "a", sequence: 1, role: "system", speaker: "Audit Shell", text: "x", glitch: false }],
        inline_choices: [],
        investigation_items: [],
        system_alerts: [],
        sanity: 64,
        trust: 48,
        awakening: 22,
        echo_mode: "anomalous",
        available_panels: ["logs"],
        active_panel: "logs",
        shutdown_countdown: null,
        glitch_level: 0.65,
        suggested_glitches: [],
        sound_cue: null,
        image_prompt: null,
        provisional: false,
      },
    });
    renderOverlay();
    expect(screen.getByTestId("debug-overlay")).toBeInTheDocument();
    expect(screen.getByText("ghost")).toBeInTheDocument();
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
