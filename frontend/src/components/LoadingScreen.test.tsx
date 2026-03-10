import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import LoadingScreen from "./LoadingScreen";

describe("LoadingScreen", () => {
  it("renders the connecting text", () => {
    render(<LoadingScreen />);
    expect(screen.getByText(/connecting/i)).toBeInTheDocument();
  });

  it("has the dark background class", () => {
    const { container } = render(<LoadingScreen />);
    const outer = container.firstChild as HTMLElement;
    expect(outer.className).toContain("bg-void");
  });
});
