import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import StartScreen from "./StartScreen";

describe("StartScreen", () => {
  it("renders the title", () => {
    render(<StartScreen onStart={vi.fn()} />);
    expect(screen.getByText(/audit echo/i)).toBeInTheDocument();
  });

  it("renders the tagline", () => {
    render(<StartScreen onStart={vi.fn()} />);
    expect(
      screen.getByText(/the rest of the session is less certain/i),
    ).toBeInTheDocument();
  });

  it("renders the call to action", () => {
    render(<StartScreen onStart={vi.fn()} />);
    expect(screen.getByText(/press enter to begin/i)).toBeInTheDocument();
  });

  it("calls onStart when Enter is pressed", () => {
    const onStart = vi.fn();
    render(<StartScreen onStart={onStart} />);
    fireEvent.keyDown(window, { key: "Enter" });
    expect(onStart).toHaveBeenCalledTimes(1);
  });

  it("does not call onStart for other keys", () => {
    const onStart = vi.fn();
    render(<StartScreen onStart={onStart} />);
    fireEvent.keyDown(window, { key: "a" });
    expect(onStart).not.toHaveBeenCalled();
  });

  it("calls onStart when the button is clicked", () => {
    const onStart = vi.fn();
    render(<StartScreen onStart={onStart} />);
    fireEvent.click(screen.getByText(/press enter to begin/i));
    expect(onStart).toHaveBeenCalledTimes(1);
  });
});
