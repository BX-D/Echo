import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import SessionChrome from "./SessionChrome";

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
      freeform_topics: [],
      forced_clue_queue: [],
      reconverge_beat_id: null,
      fallback_reply: "",
    },
    status_line: "Day 1 / External Safety Review",
    input_enabled: true,
    input_placeholder: "",
    transcript: [],
    inline_choices: [],
    investigation_items: [],
    system_alerts: [],
    sanity: 100,
    trust: 50,
    awakening: 0,
    echo_mode: "normal" as const,
    available_panels: [],
    active_panel: null,
    shutdown_countdown: null,
    glitch_level: 0,
    suggested_glitches: [],
    sound_cue: null,
    image_prompt: null,
    provisional: false,
  };
}

describe("SessionChrome", () => {
  it("shows resuming state when a stored session is reconnecting without a surface", () => {
    render(
      <SessionChrome
        connectionStatus="connecting"
        sessionId="persisted-session"
        currentSurface={null}
      />,
    );
    expect(screen.getByText(/resuming/i)).toBeInTheDocument();
  });

  it("shows reconnecting state when a live surface is present during reconnect", () => {
    render(
      <SessionChrome
        connectionStatus="connecting"
        sessionId="persisted-session"
        currentSurface={surface()}
      />,
    );
    expect(screen.getByText(/reconnecting/i)).toBeInTheDocument();
  });
});
