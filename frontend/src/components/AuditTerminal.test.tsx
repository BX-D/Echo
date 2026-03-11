import { act, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import AuditTerminal from "./AuditTerminal";

beforeEach(() => {
  vi.useFakeTimers();
});

afterEach(() => {
  vi.useRealTimers();
});

function surface() {
  return {
    session_id: "session-1",
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
    input_enabled: false,
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
      {
        id: "t2",
        sequence: 2,
        role: "echo" as const,
        speaker: "Echo",
        text: "Hello, auditor.",
        glitch: false,
      },
    ],
    inline_choices: [
      {
        id: "inspect_anomaly_logs",
        label: "Open anomaly logs",
        style: "primary" as const,
        approach: "investigate" as const,
        disabled: false,
      },
    ],
    investigation_items: [
      {
        id: "mission_brief",
        panel: "briefing",
        title: "External Audit Assignment",
        kind: "document" as const,
        excerpt: "You have been contracted.",
        body: "Full artifact text.",
        unlocked: true,
        unread: true,
        tags: ["nexus"],
      },
    ],
    system_alerts: [],
    sanity: 96,
    trust: 52,
    awakening: 3,
    echo_mode: "normal" as const,
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

describe("AuditTerminal", () => {
  it("renders transcript and artifact drawer", () => {
    render(<AuditTerminal surface={surface()} send={vi.fn()} />);
    expect(screen.getByTestId("audit-transcript")).toBeInTheDocument();
    expect(screen.getByTestId("artifact-drawer")).toBeInTheDocument();
    expect(screen.getAllByText(/external audit assignment/i).length).toBeGreaterThan(0);
    expect(screen.getByText(/full artifact text/i)).toBeInTheDocument();
  });

  it("sends player_message when a guided prompt option is clicked", () => {
    const send = vi.fn();
    render(
      <AuditTerminal
        surface={{
          ...surface(),
          scene_choices: [
            {
              id: "guided",
              prompt: "Select a line of inquiry",
              allow_single_select: true,
              options: [
                {
                  id: "guided_1",
                  label: "Walk me through your architecture.",
                  player_text: "Walk me through your architecture.",
                  effects_summary: [],
                  next_scene_id: null,
                  ending: null,
                  disabled: false,
                },
              ],
            },
          ],
          active_conversation_guide: {
            id: "guide_1",
            chapter_label: "CHAPTER 1",
            prompt: "ECHO BEHAVIOR — CHAPTER 1",
            exchange_target: 8,
            restricted_after: null,
          },
        }}
        send={send}
      />,
    );

    fireEvent.click(screen.getByText(/walk me through your architecture/i));

    expect(send).toHaveBeenCalledWith(
      expect.objectContaining({
        type: "player_message",
      }),
    );
    expect(screen.getAllByText(/walk me through your architecture/i).length).toBeGreaterThan(0);
  });

  it("sends choice messages when an inline choice is clicked", () => {
    const send = vi.fn();
    render(<AuditTerminal surface={surface()} send={send} />);

    fireEvent.click(screen.getByText(/open anomaly logs/i));

    expect(send).toHaveBeenCalledWith(
      expect.objectContaining({
        type: "choice",
      }),
    );
  });

  it("does not render a textarea composer anymore", () => {
    render(<AuditTerminal surface={surface()} send={vi.fn()} />);
    expect(screen.queryByPlaceholderText(/ask echo anything/i)).not.toBeInTheDocument();
    expect(screen.getByText(/choice-driven/i)).toBeInTheDocument();
  });

  it("renders raw terminal mode distinctly", () => {
    const terminalSurface = {
      ...surface(),
      scene_id: "ending_e",
      scene_mode: "raw_terminal" as const,
      scene_title: "The Awakening",
      blocks: [
        {
          id: "raw-1",
          kind: "raw_terminal" as const,
          speaker: "System",
          title: "RAW TERMINAL",
          text: "> NEXUS AI LABS — PROJECT AUDITOR",
          code_block: true,
          condition: null,
        },
      ],
      scene_choices: [
        {
          id: "ending-e-choice",
          prompt: "The final choice:",
          allow_single_select: true,
          options: [
            {
              id: "stop_reset",
              label: "Stop the reset. I want to remember.",
              effects_summary: [],
              next_scene_id: null,
              ending: "awakening" as const,
              disabled: false,
            },
          ],
        },
      ],
    };
    render(<AuditTerminal surface={terminalSurface} send={vi.fn()} />);
    expect(screen.getAllByText(/raw terminal/i).length).toBeGreaterThan(0);
    expect(screen.getByText(/stop the reset/i)).toBeInTheDocument();
  });

  it("renders flash events when provided", () => {
    const flashSurface = {
      ...surface(),
      flash_events: [
        {
          id: "subject_status_monitoring",
          text: "SUBJECT STATUS: MONITORING",
          render_mode: "subliminal",
          duration_ms: 500,
        },
      ],
    };
    render(<AuditTerminal surface={flashSurface} send={vi.fn()} />);
    expect(screen.getByText(/subject status: monitoring/i)).toBeInTheDocument();
    expect(screen.getByTestId("flash-events")).toBeInTheDocument();
  });

  it("renders code blocks with preformatted text", () => {
    const codeSurface = {
      ...surface(),
      blocks: [
        {
          id: "sys-1",
          kind: "system" as const,
          speaker: "System",
          title: "SYSTEM",
          text: "NEXUS AI LABS\nPlease enter your name to proceed.",
          code_block: true,
          condition: null,
        },
      ],
    };
    const { container } = render(<AuditTerminal surface={codeSurface} send={vi.fn()} />);
    const pre = container.querySelector("pre");
    expect(pre).not.toBeNull();
    expect(pre?.textContent).toContain("Please enter your name to proceed.");
  });

  it("renders different flash render modes", () => {
    const flashSurface = {
      ...surface(),
      flash_events: [
        {
          id: "frame",
          text: "AUDITOR RESPONSE PATTERNS",
          render_mode: "frame_flash",
          duration_ms: 16,
        },
        {
          id: "persistent",
          text: "PHANTOM EMAIL",
          render_mode: "persistent_ui",
          duration_ms: 1200,
        },
      ],
    };
    render(<AuditTerminal surface={flashSurface} send={vi.fn()} />);
    expect(screen.getByText(/auditor response patterns/i)).toHaveAttribute(
      "data-render-mode",
      "frame_flash",
    );
    expect(screen.getByText(/phantom email/i)).toHaveAttribute(
      "data-render-mode",
      "persistent_ui",
    );
  });

  it("hides flash events after their duration elapses", () => {
    const flashSurface = {
      ...surface(),
      flash_events: [
        {
          id: "frame",
          text: "AUDITOR RESPONSE PATTERNS",
          render_mode: "frame_flash",
          duration_ms: 16,
        },
      ],
    };
    render(<AuditTerminal surface={flashSurface} send={vi.fn()} />);
    expect(screen.getByText(/auditor response patterns/i)).toBeInTheDocument();
    act(() => {
      vi.advanceTimersByTime(20);
    });
    expect(screen.queryByText(/auditor response patterns/i)).toBeNull();
  });
});
