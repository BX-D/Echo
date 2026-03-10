/**
 * Full-screen vignette overlay — darkens screen edges.
 * Uses `pointer-events: none` so it never blocks interaction.
 */
export interface VignetteProps {
  intensity?: number;
}

export default function Vignette({ intensity = 0.6 }: VignetteProps) {
  return (
    <div
      className="vignette-overlay"
      data-testid="vignette"
      style={{ opacity: intensity }}
    />
  );
}
