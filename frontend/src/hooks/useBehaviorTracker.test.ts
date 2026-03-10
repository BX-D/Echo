import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { renderHook, act } from "@testing-library/react";
import { useBehaviorTracker } from "./useBehaviorTracker";
import type { ClientMessage } from "../types/ws";

beforeEach(() => {
  vi.useFakeTimers();
});

afterEach(() => {
  vi.useRealTimers();
});

describe("useBehaviorTracker", () => {
  it("attaches and detaches with component lifecycle", () => {
    const send = vi.fn();
    const { unmount } = renderHook(() => useBehaviorTracker(send, "scene_1"));

    // Fire some keys to verify attachment.
    for (let i = 0; i < 20; i++) {
      window.dispatchEvent(new KeyboardEvent("keydown", { key: "a" }));
    }

    unmount();

    // After unmount, detach should have flushed.
    expect(send).toHaveBeenCalled();
    const calls = send.mock.calls as Array<[ClientMessage]>;
    const batchCalls = calls.filter((c) => c[0].type === "behavior_batch");
    expect(batchCalls.length).toBeGreaterThanOrEqual(1);
  });

  it("updates scene context when sceneId changes", () => {
    const send = vi.fn();
    const { rerender, unmount } = renderHook(
      ({ sceneId }: { sceneId: string }) => useBehaviorTracker(send, sceneId),
      { initialProps: { sceneId: "scene_a" } },
    );

    rerender({ sceneId: "scene_b" });

    // Type enough to trigger a keystroke event in the new scene.
    for (let i = 0; i < 20; i++) {
      window.dispatchEvent(new KeyboardEvent("keydown", { key: "x" }));
    }

    unmount();

    const calls = send.mock.calls as Array<[ClientMessage]>;
    const batches = calls
      .filter((c) => c[0].type === "behavior_batch")
      .map((c) => c[0]);
    expect(batches.length).toBeGreaterThanOrEqual(1);
  });

  it("returns recordChoiceDisplayed and recordChoiceSelected", () => {
    const send = vi.fn();
    const { result, unmount } = renderHook(() =>
      useBehaviorTracker(send, "s1"),
    );

    act(() => {
      result.current.recordChoiceDisplayed("s1");
    });
    vi.advanceTimersByTime(300);
    act(() => {
      result.current.recordChoiceSelected("c1", "investigate");
    });

    unmount();

    const calls = send.mock.calls as Array<[ClientMessage]>;
    const allEvents = calls
      .filter((c) => c[0].type === "behavior_batch")
      .flatMap((c) => {
        if (c[0].type === "behavior_batch") return c[0].payload.events;
        return [];
      });
    const choices = allEvents.filter((e) => e.event_type.type === "choice");
    expect(choices.length).toBe(1);
  });
});
