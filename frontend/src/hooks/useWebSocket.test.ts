import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { renderHook, act } from "@testing-library/react";
import { useWebSocket } from "./useWebSocket";
import { useGameStore } from "../stores/gameStore";

// ---------------------------------------------------------------------------
// Mock WebSocket
// ---------------------------------------------------------------------------

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

  // Test helpers
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

// ---------------------------------------------------------------------------

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

  it("sets connected status on open", () => {
    renderHook(() => useWebSocket("ws://test/ws", "session-1"));
    act(() => latestWs().simulateOpen());
    expect(useGameStore.getState().connectionStatus).toBe("connected");
  });

  it("routes narrative messages to the store", () => {
    renderHook(() => useWebSocket("ws://test/ws", "session-1"));
    act(() => latestWs().simulateOpen());

    const narrative = JSON.stringify({
      type: "narrative",
      payload: {
        scene_id: "intro",
        text: "Hello",
        atmosphere: "dread",
        choices: [],
        sound_cue: null,
        intensity: 0.3,
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
    });
    act(() => latestWs().simulateMessage(narrative));
    expect(useGameStore.getState().currentScene?.scene_id).toBe("intro");
  });

  it("routes phase_change messages to the store", () => {
    renderHook(() => useWebSocket("ws://test/ws", "session-1"));
    act(() => latestWs().simulateOpen());

    const msg = JSON.stringify({
      type: "phase_change",
      payload: { from: "calibrating", to: "exploring" },
    });
    act(() => latestWs().simulateMessage(msg));
    expect(useGameStore.getState().gamePhase).toBe("exploring");
  });

  it("reconnects with exponential backoff on close", () => {
    renderHook(() => useWebSocket("ws://test/ws", "session-1"));
    act(() => latestWs().simulateOpen());

    // Close the connection
    act(() => latestWs().simulateClose());
    expect(useGameStore.getState().connectionStatus).toBe("disconnected");
    expect(MockWebSocket.instances).toHaveLength(1);

    // After 1s, should reconnect
    act(() => {
      vi.advanceTimersByTime(1000);
    });
    expect(MockWebSocket.instances).toHaveLength(2);

    // Close again, next delay = 2s
    act(() => latestWs().simulateClose());
    act(() => {
      vi.advanceTimersByTime(1500);
    });
    expect(MockWebSocket.instances).toHaveLength(2); // not yet
    act(() => {
      vi.advanceTimersByTime(600);
    });
    expect(MockWebSocket.instances).toHaveLength(3); // now
  });

  it("resets reconnect delay on successful connect", () => {
    renderHook(() => useWebSocket("ws://test/ws", "session-1"));
    act(() => latestWs().simulateOpen());
    act(() => latestWs().simulateClose());

    // Wait for first reconnect (1s)
    act(() => vi.advanceTimersByTime(1000));
    expect(MockWebSocket.instances).toHaveLength(2);

    // Successfully connect
    act(() => latestWs().simulateOpen());

    // Close again — delay should be back to 1s (not 2s)
    act(() => latestWs().simulateClose());
    act(() => vi.advanceTimersByTime(1000));
    expect(MockWebSocket.instances).toHaveLength(3);
  });

  it("send() serializes ClientMessage as JSON", () => {
    const { result } = renderHook(() => useWebSocket("ws://test/ws", "session-1"));
    act(() => latestWs().simulateOpen());

    act(() => {
      result.current.send({
        type: "start_game",
        payload: { player_name: "Alice" },
      });
    });

    expect(latestWs().sent).toHaveLength(1);
    const parsed = JSON.parse(latestWs().sent[0]!);
    expect(parsed.type).toBe("start_game");
    expect(parsed.payload.player_name).toBe("Alice");
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
    // Should not throw — store unchanged
    expect(useGameStore.getState().currentScene).toBeNull();
  });
});
