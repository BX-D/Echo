/**
 * Subtle gradient overlay from all four edges, adding ambient darkness.
 */
export interface AmbientDarknessProps {
  intensity?: number;
}

export default function AmbientDarkness({
  intensity = 0.45,
}: AmbientDarknessProps) {
  return (
    <div
      className="ambient-darkness"
      data-testid="ambient-darkness"
      style={{ opacity: intensity }}
    />
  );
}
