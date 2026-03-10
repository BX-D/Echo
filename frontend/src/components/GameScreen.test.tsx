import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, screen, fireEvent, act } from "@testing-library/react";
import GameScreen, { type GameScreenProps } from "./GameScreen";
import type { NarrativePayload } from "../types/ws";

beforeEach(() => {
  vi.useFakeTimers();
  vi.spyOn(Math, "random").mockReturnValue(0.5); // no glitch
});

afterEach(() => {
  vi.useRealTimers();
  vi.restoreAllMocks();
});

function scene(id: string, text: string, opts?: Partial<NarrativePayload>): NarrativePayload {
  return {
    scene_id: id,
    text,
    atmosphere: "dread",
    choices: [
      { id: "c1", text: "Go left", approach: "investigate", fear_vector: "darkness" },
      { id: "c2", text: "Go right", approach: "flee", fear_vector: "stalking" },
    ],
    sound_cue: null,
    intensity: 0.5,
    effects: [],
    title: null,
    act: "calibration",
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
    ...opts,
  };
}

function renderScreen(overrides?: Partial<GameScreenProps>) {
  const defaults: GameScreenProps = {
    currentScene: scene("s1", "The corridor stretches before you."),
    sceneHistory: [scene("s1", "The corridor stretches before you.")],
    image: null,
    isLoading: false,
    send: vi.fn(),
  };
  return render(<GameScreen {...defaults} {...overrides} />);
}

describe("GameScreen", () => {
  it("displays narrative text", () => {
    renderScreen();
    // Typewriter is active — skip to show full text.
    fireEvent.keyDown(window, { key: "Space" });
    expect(screen.getByTestId("current-scene").textContent).toContain(
      "corridor stretches",
    );
  });

  it("shows choices after text completes", () => {
    renderScreen();
    // Before typing completes, choices should not be visible.
    expect(screen.queryByTestId("choices-panel")).toBeNull();

    // Skip typewriter.
    fireEvent.keyDown(window, { key: "Space" });
    act(() => vi.advanceTimersByTime(600));

    expect(screen.getByTestId("choices-panel")).toBeInTheDocument();
    expect(screen.getByText("Go left")).toBeInTheDocument();
    expect(screen.getByText("Go right")).toBeInTheDocument();
  });

  it("transitions between scenes with fade", () => {
    const { rerender } = render(
      <GameScreen
        currentScene={scene("s1", "Scene one.")}
        sceneHistory={[scene("s1", "Scene one.")]}
        image={null}
        isLoading={false}
        send={vi.fn()}
      />,
    );

    const firstScene = screen.getByTestId("current-scene");
    expect(firstScene.className).toContain("animate-fade-in");

    // Re-render with a new scene.
    rerender(
      <GameScreen
        currentScene={scene("s2", "Scene two.")}
        sceneHistory={[scene("s1", "Scene one."), scene("s2", "Scene two.")]}
        image={null}
        isLoading={false}
        send={vi.fn()}
      />,
    );

    // The new scene also has the fade-in class (keyed by scene_id).
    const newScene = screen.getByTestId("current-scene");
    expect(newScene.className).toContain("animate-fade-in");
  });

  it("applies atmosphere-based visual effects", () => {
    renderScreen({
      currentScene: scene("s1", "Panic.", { atmosphere: "panic" }),
    });
    const container = screen.getByTestId("game-screen");
    expect(container.dataset.atmosphere).toBe("panic");
  });

  it("shows loading indicator during AI generation", () => {
    renderScreen({ isLoading: true });
    expect(screen.getByTestId("scene-loading")).toBeInTheDocument();
    expect(screen.getByText(/darkness shifts/i)).toBeInTheDocument();
  });

  it("preserves previous scenes in scroll history", () => {
    renderScreen({
      currentScene: scene("s2", "Scene two."),
      sceneHistory: [scene("s1", "Scene one."), scene("s2", "Scene two.")],
    });
    const historyEntries = screen.getAllByTestId("scene-history-entry");
    expect(historyEntries.length).toBe(1);
    expect(historyEntries[0]!.textContent).toContain("Scene one.");
  });

  it("image displays when provided", () => {
    renderScreen({
      image: {
        scene_id: "s1",
        image_url: "data:image/png;base64,abc",
        display_mode: "fade_in",
      },
    });
    const img = screen.getByTestId("scene-image");
    expect(img).toBeInTheDocument();
    const imgEl = img.querySelector("img");
    expect(imgEl?.src).toContain("data:image/png;base64,abc");
  });

  it("handles missing image gracefully", () => {
    renderScreen({ image: null });
    expect(screen.queryByTestId("scene-image")).toBeNull();
  });

  it("choice click calls send with correct payload", () => {
    const send = vi.fn();
    renderScreen({ send });

    // Skip typewriter to show choices.
    fireEvent.keyDown(window, { key: "Space" });
    act(() => vi.advanceTimersByTime(600));

    fireEvent.click(screen.getByText("Go left"));
    expect(send).toHaveBeenCalledWith({
      type: "choice",
      payload: {
        scene_id: "s1",
        choice_id: "c1",
        time_to_decide_ms: expect.any(Number),
        approach: "investigate",
      },
    });
  });

  it("no choices shown when scene has none", () => {
    renderScreen({
      currentScene: scene("s1", "Dead end.", { choices: [] }),
    });
    fireEvent.keyDown(window, { key: "Space" });
    expect(screen.queryByTestId("choices-panel")).toBeNull();
  });

  it("does not duplicate transcript content across overlay and feed", () => {
    renderScreen({
      currentScene: scene("s1", "A fragment repeats. Then a second line lands after it.", {
        medium: "transcript",
        transcript_lines: [
          "[00:12] A fragment repeats.",
        ],
      }),
    });

    fireEvent.keyDown(window, { key: "Space" });
    act(() => vi.advanceTimersByTime(600));

    expect(screen.queryByText("Transcript Overlay")).toBeNull();
    expect(screen.getByText("Transcript Feed")).toBeInTheDocument();
    expect(screen.getByText("Session Note")).toBeInTheDocument();
  });
});
