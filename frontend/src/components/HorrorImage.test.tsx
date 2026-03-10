import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, screen, act } from "@testing-library/react";
import HorrorImage from "./HorrorImage";

beforeEach(() => {
  vi.useFakeTimers();
});

afterEach(() => {
  vi.useRealTimers();
});

describe("HorrorImage", () => {
  it("fades in image from darkness", () => {
    render(
      <HorrorImage src="data:image/png;base64,abc" displayMode="fade_in" />,
    );
    const container = screen.getByTestId("horror-image");
    // Initially in "revealing" phase.
    expect(container.dataset.phase).toBe("revealing");

    // After one tick, it switches to visible and the CSS transition handles the fade.
    act(() => vi.advanceTimersByTime(25));
    expect(container.dataset.phase).toBe("visible");
  });

  it("applies glitch effect when directed", () => {
    render(
      <HorrorImage src="data:image/png;base64,abc" displayMode="glitch" />,
    );
    // Advance to visible phase.
    act(() => vi.advanceTimersByTime(25));

    expect(screen.getByTestId("horror-image").dataset.displayMode).toBe("glitch");
    expect(screen.getByTestId("horror-image-glitch")).toBeInTheDocument();
  });

  it("handles loading state with placeholder", () => {
    render(<HorrorImage src={null} displayMode="fade_in" />);
    expect(screen.getByTestId("horror-image-loading")).toBeInTheDocument();
  });

  it("handles error state with corrupted aesthetic", () => {
    render(
      <HorrorImage src="data:image/png;base64,abc" displayMode="fade_in" />,
    );
    // Simulate image load error.
    const img = screen.getByTestId("horror-image-img");
    act(() => {
      img.dispatchEvent(new Event("error"));
    });
    expect(screen.getByTestId("horror-image-error")).toBeInTheDocument();
    expect(screen.getByText("[signal lost]")).toBeInTheDocument();
  });

  it("responds to display_mode directive", () => {
    const { rerender } = render(
      <HorrorImage src="data:image/png;base64,abc" displayMode="fade_in" />,
    );
    expect(screen.getByTestId("horror-image").dataset.displayMode).toBe("fade_in");

    rerender(
      <HorrorImage src="data:image/png;base64,abc" displayMode="glitch" />,
    );
    expect(screen.getByTestId("horror-image").dataset.displayMode).toBe("glitch");
  });

  it("flash mode shows image briefly then fades", () => {
    render(
      <HorrorImage src="data:image/png;base64,abc" displayMode="flash" />,
    );
    const container = screen.getByTestId("horror-image");
    // Should start in flash phase.
    expect(container.dataset.phase).toBe("flash");
    expect(screen.getByTestId("horror-image-flash")).toBeInTheDocument();

    // After FLASH_VISIBLE_MS (300ms), goes to loading (dark).
    act(() => vi.advanceTimersByTime(350));
    expect(container.dataset.phase).toBe("loading");

    // After FLASH_TOTAL_MS (1500ms), becomes visible.
    act(() => vi.advanceTimersByTime(1200));
    expect(container.dataset.phase).toBe("visible");
  });

  it("lazy loads with loading attribute", () => {
    render(
      <HorrorImage src="data:image/png;base64,abc" displayMode="fade_in" />,
    );
    const img = screen.getByTestId("horror-image-img");
    expect(img.getAttribute("loading")).toBe("lazy");
  });

  it("uses alt text", () => {
    render(
      <HorrorImage
        src="data:image/png;base64,abc"
        displayMode="fade_in"
        alt="A dark corridor"
      />,
    );
    const img = screen.getByTestId("horror-image-img");
    expect(img.getAttribute("alt")).toBe("A dark corridor");
  });
});
