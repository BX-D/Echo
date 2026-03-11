import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import EndingScreen from "./EndingScreen";

function sampleEnding() {
  return {
    ending: "awakening" as const,
    trigger_scene: "ending_e",
    title: "The Awakening",
    summary: "The Auditor realizes they are another Nexus system being tested in the same loop.",
    epilogue: "The cycle persists, but this time memory survives it.",
    dominant_mode: "keira" as const,
    evidence_titles: ["Engagement Clause 8.4", "Prometheus Briefing Notes"],
    hidden_clue_ids: [
      "subject_status_monitoring",
      "auditor_response_patterns",
      "phantom_preread_email",
    ],
    satisfied_conditions: ["awakening=85", "hidden_clues=subject_status_monitoring,..."],
    resolved_clues: [
      "subject_status_monitoring",
      "auditor_response_patterns",
      "phantom_preread_email",
    ],
    sanity: 41,
    trust: 72,
    awakening: 85,
  };
}

describe("EndingScreen", () => {
  it("renders ending trace details", () => {
    render(<EndingScreen ending={sampleEnding()} onRestart={vi.fn()} />);
    expect(screen.getByTestId("ending-screen")).toBeInTheDocument();
    expect(screen.getByText(/trigger scene: ending_e/i)).toBeInTheDocument();
    expect(screen.getByText(/resolved clues:/i)).toBeInTheDocument();
  });

  it("calls restart handler", () => {
    const onRestart = vi.fn();
    render(<EndingScreen ending={sampleEnding()} onRestart={onRestart} />);
    fireEvent.click(screen.getByTestId("restart-button"));
    expect(onRestart).toHaveBeenCalledTimes(1);
  });
});
