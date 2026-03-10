import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, screen, fireEvent, act } from "@testing-library/react";
import ChoicePanel, { type ChoicePanelProps } from "./ChoicePanel";
import type { Choice } from "../types/narrative";

const STAGGER = 200;

beforeEach(() => {
  vi.useFakeTimers();
  vi.spyOn(performance, "now").mockReturnValue(1000);
});

afterEach(() => {
  vi.useRealTimers();
  vi.restoreAllMocks();
});

const choices: Choice[] = [
  { id: "c1", text: "Go left", approach: "investigate", fear_vector: "darkness" },
  { id: "c2", text: "Go right", approach: "flee", fear_vector: "stalking" },
  { id: "c3", text: "Stay still", approach: "wait", fear_vector: "isolation" },
];

function renderPanel(overrides?: Partial<ChoicePanelProps>) {
  const defaults: ChoicePanelProps = {
    choices,
    sceneId: "scene_1",
    send: vi.fn(),
    onChoiceMade: vi.fn(),
  };
  return render(<ChoicePanel {...defaults} {...overrides} />);
}

describe("ChoicePanel", () => {
  it("renders all choices", () => {
    renderPanel();
    // All choice elements exist in the DOM (some may be invisible due to stagger).
    expect(screen.getByTestId("choice-c1")).toBeInTheDocument();
    expect(screen.getByTestId("choice-c2")).toBeInTheDocument();
    expect(screen.getByTestId("choice-c3")).toBeInTheDocument();
  });

  it("staggers choice appearance", () => {
    renderPanel();
    // Initially, visibleCount is 0 — all have opacity-0.
    const c1 = screen.getByTestId("choice-c1");
    expect(c1.className).toContain("opacity-0");

    // After one stagger interval, first choice visible.
    act(() => vi.advanceTimersByTime(STAGGER));
    expect(screen.getByTestId("choice-c1").className).toContain("opacity-100");
    expect(screen.getByTestId("choice-c2").className).toContain("opacity-0");

    // After two intervals, second visible.
    act(() => vi.advanceTimersByTime(STAGGER));
    expect(screen.getByTestId("choice-c2").className).toContain("opacity-100");
  });

  it("hover effect applies via CSS class", () => {
    renderPanel();
    // Make choices visible.
    act(() => vi.advanceTimersByTime(STAGGER * 3));

    const btn = screen.getByTestId("choice-c1");
    // Hover classes are in the className (hover:translate-x-1 etc).
    expect(btn.className).toContain("hover:translate-x-1");
  });

  it("click sends choice message", () => {
    const send = vi.fn();
    renderPanel({ send });
    act(() => vi.advanceTimersByTime(STAGGER * 3));

    fireEvent.click(screen.getByTestId("choice-c1"));

    expect(send).toHaveBeenCalledWith({
      type: "choice",
      payload: {
        scene_id: "scene_1",
        choice_id: "c1",
        time_to_decide_ms: expect.any(Number),
        approach: "investigate",
      },
    });
  });

  it("choice timing is measured correctly", () => {
    const onChoiceMade = vi.fn();
    let now = 1000;
    vi.spyOn(performance, "now").mockImplementation(() => now);

    renderPanel({ onChoiceMade });
    act(() => vi.advanceTimersByTime(STAGGER * 3));

    // Simulate 500ms of deliberation.
    now = 1500;
    fireEvent.click(screen.getByTestId("choice-c2"));

    expect(onChoiceMade).toHaveBeenCalledWith("c2", 500, "flee", [], null, 0);
  });

  it("keyboard shortcuts work (1, 2, 3)", () => {
    const send = vi.fn();
    renderPanel({ send });
    act(() => vi.advanceTimersByTime(STAGGER * 3));

    fireEvent.keyDown(window, { key: "2" });

    expect(send).toHaveBeenCalledWith(
      expect.objectContaining({
        type: "choice",
        payload: expect.objectContaining({ choice_id: "c2" }),
      }),
    );
  });

  it("selected choice shows feedback", () => {
    renderPanel();
    act(() => vi.advanceTimersByTime(STAGGER * 3));

    fireEvent.click(screen.getByTestId("choice-c1"));

    const btn = screen.getByTestId("choice-c1");
    expect(btn.className).toContain("border-bone");
    expect(btn.className).toContain("scale-");
  });

  it("choices disabled after selection", () => {
    renderPanel();
    act(() => vi.advanceTimersByTime(STAGGER * 3));

    fireEvent.click(screen.getByTestId("choice-c1"));

    // All buttons should be disabled.
    expect(screen.getByTestId("choice-c1")).toBeDisabled();
    expect(screen.getByTestId("choice-c2")).toBeDisabled();
    expect(screen.getByTestId("choice-c3")).toBeDisabled();
  });

  it("behavior data sent with choice", () => {
    const send = vi.fn();
    renderPanel({ send });
    act(() => vi.advanceTimersByTime(STAGGER * 3));

    fireEvent.click(screen.getByTestId("choice-c1"));

    const call = send.mock.calls[0]![0] as {
      type: string;
      payload: { time_to_decide_ms: number; approach: string };
    };
    expect(call.type).toBe("choice");
    expect(typeof call.payload.time_to_decide_ms).toBe("number");
    expect(call.payload.approach).toBe("investigate");
  });

  it("keyboard shortcut ignored when choices not yet visible", () => {
    const send = vi.fn();
    renderPanel({ send });
    // Don't advance timers — no choices visible yet.
    fireEvent.keyDown(window, { key: "1" });
    expect(send).not.toHaveBeenCalled();
  });

  it("keyboard shortcut out of range is ignored", () => {
    const send = vi.fn();
    renderPanel({ send });
    act(() => vi.advanceTimersByTime(STAGGER * 3));
    fireEvent.keyDown(window, { key: "9" });
    expect(send).not.toHaveBeenCalled();
  });
});
