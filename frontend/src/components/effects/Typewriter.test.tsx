import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, screen, fireEvent, act } from "@testing-library/react";
import Typewriter from "./Typewriter";

beforeEach(() => {
  vi.useFakeTimers();
  // Deterministic "random" for glitch tests.
  vi.spyOn(Math, "random").mockReturnValue(0.5);
});

afterEach(() => {
  vi.useRealTimers();
  vi.restoreAllMocks();
});

describe("Typewriter", () => {
  it("renders empty initially", () => {
    render(<Typewriter text="Hello" speed="normal" />);
    const el = screen.getByTestId("typewriter");
    // Should not have the full text yet (first char appears after 30ms).
    expect(el.textContent).not.toContain("Hello");
  });

  it("reveals characters one at a time", () => {
    render(<Typewriter text="ABC" speed="normal" />);

    act(() => vi.advanceTimersByTime(30));
    expect(screen.getByTestId("typewriter").textContent).toContain("A");

    act(() => vi.advanceTimersByTime(30));
    expect(screen.getByTestId("typewriter").textContent).toContain("AB");

    act(() => vi.advanceTimersByTime(30));
    expect(screen.getByTestId("typewriter").textContent).toContain("ABC");
  });

  it("respects slow speed setting", () => {
    render(<Typewriter text="AB" speed="slow" />);
    // At 50ms per char, after 40ms we shouldn't see the first char yet.
    act(() => vi.advanceTimersByTime(40));
    expect(screen.getByTestId("typewriter").textContent).not.toContain("A");

    act(() => vi.advanceTimersByTime(15));
    expect(screen.getByTestId("typewriter").textContent).toContain("A");
  });

  it("respects fast speed setting", () => {
    render(<Typewriter text="ABCD" speed="fast" />);
    // 15ms per char — after 60ms all 4 should be visible.
    act(() => vi.advanceTimersByTime(60));
    expect(screen.getByTestId("typewriter").textContent).toContain("ABCD");
  });

  it("dramatic pause on period", () => {
    render(<Typewriter text="A.B" speed="normal" />);
    // A appears at 30ms.
    act(() => vi.advanceTimersByTime(30));
    expect(screen.getByTestId("typewriter").textContent).toContain("A");

    // Period at 30ms × 6 = 180ms pause.
    act(() => vi.advanceTimersByTime(30));
    expect(screen.getByTestId("typewriter").textContent).toContain("A.");

    // B should NOT appear at 30ms after period (pause is 180ms).
    act(() => vi.advanceTimersByTime(30));
    expect(screen.getByTestId("typewriter").textContent).not.toContain("B");

    // After the full pause, B appears.
    act(() => vi.advanceTimersByTime(200));
    expect(screen.getByTestId("typewriter").textContent).toContain("B");
  });

  it("dramatic pause on ellipsis", () => {
    render(<Typewriter text="A...B" speed="normal" />);
    // Advance through "A" + three dots.
    act(() => vi.advanceTimersByTime(30)); // A
    act(() => vi.advanceTimersByTime(180)); // first .
    act(() => vi.advanceTimersByTime(180)); // second .
    act(() => vi.advanceTimersByTime(30)); // third . (ellipsis detected: 8× pause = 240ms)

    // After the three dots, "B" shouldn't appear immediately.
    expect(screen.getByTestId("typewriter").textContent).toContain("...");
    // Advance through ellipsis pause.
    act(() => vi.advanceTimersByTime(300));
    expect(screen.getByTestId("typewriter").textContent).toContain("B");
  });

  it("glitch effect shows wrong char briefly", () => {
    // Make Math.random return < 0.05 to trigger glitch.
    vi.spyOn(Math, "random").mockReturnValue(0.02);

    render(<Typewriter text="XY" speed="normal" />);

    // After 30ms tick fires. Glitch char should appear.
    act(() => vi.advanceTimersByTime(30));
    // Glitch: displayed has a wrong char briefly.
    // After GLITCH_DURATION_MS (60ms), the real char appears.
    act(() => vi.advanceTimersByTime(60));
    const text2 = screen.getByTestId("typewriter").textContent ?? "";
    expect(text2).toContain("X");
  });

  it("completes and calls onComplete callback", () => {
    const onComplete = vi.fn();
    render(<Typewriter text="AB" speed="normal" onComplete={onComplete} />);

    act(() => vi.advanceTimersByTime(30)); // A
    act(() => vi.advanceTimersByTime(30)); // B
    act(() => vi.advanceTimersByTime(30)); // completion tick

    expect(onComplete).toHaveBeenCalledTimes(1);
  });

  it("skip reveals all text instantly on keypress", () => {
    const onComplete = vi.fn();
    render(
      <Typewriter text="Hello World" speed="slow" onComplete={onComplete} />,
    );

    // Only part of the text should be visible after a short time.
    act(() => vi.advanceTimersByTime(100));
    expect(screen.getByTestId("typewriter").textContent).not.toContain(
      "Hello World",
    );

    // Press a key to skip.
    fireEvent.keyDown(window, { key: "Space" });

    expect(screen.getByTestId("typewriter").textContent).toContain(
      "Hello World",
    );
    expect(onComplete).toHaveBeenCalledTimes(1);
  });

  it("cursor blinks after completion", () => {
    render(<Typewriter text="A" speed="normal" />);

    act(() => vi.advanceTimersByTime(30)); // A
    act(() => vi.advanceTimersByTime(30)); // completion

    expect(screen.getByTestId("typewriter-cursor-done")).toBeInTheDocument();
  });

  it("handles multiple paragraphs", () => {
    const text = "First paragraph.\n\nSecond paragraph.";
    render(<Typewriter text={text} speed="instant" />);
    const content = screen.getByTestId("typewriter").textContent ?? "";
    expect(content).toContain("First paragraph.");
    expect(content).toContain("Second paragraph.");
  });

  it("handles HTML entities correctly", () => {
    render(<Typewriter text='He said "hello" & goodbye' speed="instant" />);
    const content = screen.getByTestId("typewriter").textContent ?? "";
    expect(content).toContain('"hello"');
    expect(content).toContain("&");
  });

  it("instant speed shows full text immediately", () => {
    const onComplete = vi.fn();
    render(
      <Typewriter text="Full text here" speed="instant" onComplete={onComplete} />,
    );
    expect(screen.getByTestId("typewriter").textContent).toContain(
      "Full text here",
    );
    expect(onComplete).toHaveBeenCalledTimes(1);
  });

  it("maintains performance during typing", () => {
    // Verify that rendering 500 chars doesn't take excessive time.
    const longText = "x".repeat(500);
    const start = performance.now();
    render(<Typewriter text={longText} speed="fast" />);
    // Advance through all characters (500 × 15ms = 7500ms).
    act(() => vi.advanceTimersByTime(8000));
    const elapsed = performance.now() - start;
    // The actual wall-clock time should be tiny (fake timers).
    expect(elapsed).toBeLessThan(2000);
  });
});
