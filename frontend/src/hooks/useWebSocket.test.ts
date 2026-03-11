import { act, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { useGameStore } from "../stores/gameStore";
import { useWebSocket } from "./useWebSocket";

class MockWebSocket {
  static instances: MockWebSocket[] = [];
  static CONNECTING = 0;
  static OPEN = 1;
  static CLOSING = 2;
  static CLOSED = 3;

  url: string;
  readyState = MockWebSocket.CONNECTING;
  onopen: ((ev: Event) => void) | null = null;
  onclose: ((ev: CloseEvent) => void) | null = null;
  onmessage: ((ev: MessageEvent) => void) | null = null;
  onerror: ((ev: Event) => void) | null = null;
  sent: string[] = [];

  constructor(url: string) {
    this.url = url;
    MockWebSocket.instances.push(this);
  }

  send(data: string) {
    this.sent.push(data);
  }

  close() {
    this.readyState = MockWebSocket.CLOSED;
    this.onclose?.({} as CloseEvent);
  }

  simulateOpen() {
    this.readyState = MockWebSocket.OPEN;
    this.onopen?.({} as Event);
  }

  simulateMessage(data: string) {
    this.onmessage?.({ data } as MessageEvent);
  }

  simulateClose() {
    this.readyState = MockWebSocket.CLOSED;
    this.onclose?.({} as CloseEvent);
  }

  simulateError() {
    this.onerror?.({} as Event);
  }
}

beforeEach(() => {
  MockWebSocket.instances = [];
  vi.stubGlobal("WebSocket", MockWebSocket);
  useGameStore.getState().reset();
  vi.useFakeTimers();
});

afterEach(() => {
  vi.useRealTimers();
  vi.unstubAllGlobals();
});

function latestWs(): MockWebSocket {
  return MockWebSocket.instances[MockWebSocket.instances.length - 1]!;
}

describe("useWebSocket", () => {
  it("does not connect until a session id is available", () => {
    renderHook(() => useWebSocket("ws://test/ws", null));
    expect(MockWebSocket.instances).toHaveLength(0);
  });

  it("connects to the given URL on mount", () => {
    renderHook(() => useWebSocket("ws://test/ws", "session-1"));
    expect(MockWebSocket.instances).toHaveLength(1);
    expect(latestWs().url).toBe("ws://test/ws?session_id=session-1");
    expect(useGameStore.getState().connectionStatus).toBe("connecting");
  });

  it("routes session_surface messages to the store", () => {
    renderHook(() => useWebSocket("ws://test/ws", "session-1"));
    act(() => latestWs().simulateOpen());

    act(() =>
      latestWs().simulateMessage(
        JSON.stringify({
          type: "session_surface",
          payload: {
            surface: {
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
              input_placeholder: "Ask Echo anything.",
              transcript: [],
              inline_choices: [],
              investigation_items: [],
              system_alerts: [],
              sanity: 96,
              trust: 50,
              awakening: 4,
              echo_mode: "normal",
              available_panels: ["briefing"],
              active_panel: "briefing",
              shutdown_countdown: null,
              glitch_level: 0.15,
              suggested_glitches: [],
              sound_cue: null,
              image_prompt: null,
              provisional: false,
            },
          },
        }),
      ),
    );

    expect(useGameStore.getState().currentSurface?.beat.id).toBe("first_contact");
  });

  it("routes ending messages to the store", () => {
    renderHook(() => useWebSocket("ws://test/ws", "session-1"));
    act(() => latestWs().simulateOpen());

    act(() =>
      latestWs().simulateMessage(
        JSON.stringify({
          type: "ending",
          payload: {
            ending: {
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
          },
        }),
      ),
    );

    expect(useGameStore.getState().currentEnding?.ending).toBe("shutdown");
  });

  it("reconnects with exponential backoff on close", () => {
    renderHook(() => useWebSocket("ws://test/ws", "session-1"));
    act(() => latestWs().simulateOpen());
    act(() => latestWs().simulateClose());
    expect(useGameStore.getState().connectionStatus).toBe("disconnected");

    act(() => {
      vi.advanceTimersByTime(1000);
    });
    expect(MockWebSocket.instances).toHaveLength(2);
  });

  it("send() serializes ClientMessage as JSON", () => {
    const { result } = renderHook(() => useWebSocket("ws://test/ws", "session-1"));
    act(() => latestWs().simulateOpen());

    act(() => {
      result.current.send({
        type: "player_message",
        payload: {
          beat_id: "first_contact",
          text: "Tell me about Keira.",
          typing_duration_ms: 1200,
          backspace_count: 2,
        },
      });
    });

    const parsed = JSON.parse(latestWs().sent[0]!);
    expect(parsed.type).toBe("player_message");
    expect(parsed.payload.beat_id).toBe("first_contact");
  });

  it("sets error status on WebSocket error", () => {
    renderHook(() => useWebSocket("ws://test/ws", "session-1"));
    act(() => latestWs().simulateError());
    expect(useGameStore.getState().connectionStatus).toBe("error");
  });

  it("ignores invalid JSON messages without crashing", () => {
    renderHook(() => useWebSocket("ws://test/ws", "session-1"));
    act(() => latestWs().simulateOpen());
    act(() => latestWs().simulateMessage("not json {{{"));
    expect(useGameStore.getState().currentSurface).toBeNull();
  });

  it("clears persisted session on resume failure", () => {
    useGameStore.getState().setSessionId("session-1");
    renderHook(() => useWebSocket("ws://test/ws", "session-1"));
    act(() => latestWs().simulateOpen());

    act(() =>
      latestWs().simulateMessage(
        JSON.stringify({
          type: "error",
          payload: {
            code: "SESSION_RESUME_FAILED",
            message: "resume failed",
            recoverable: false,
          },
        }),
      ),
    );

    expect(useGameStore.getState().sessionId).toBeNull();
  });
});
