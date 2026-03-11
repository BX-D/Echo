import { useGameStore } from "../stores/gameStore";



export interface DebugOverlayProps {
  visible: boolean;
  speedMultiplier: number;
  onSpeedChange: (speed: number) => void;
  onReset: () => void;
}

/**
 * Toggleable debug overlay showing real-time fear profile data,
 * speed controls, and a reset button. For presenters only.
 */
export default function DebugOverlay({
  visible,
  speedMultiplier,
  onSpeedChange,
  onReset,
}: DebugOverlayProps) {
  const currentSurface = useGameStore((s) => s.currentSurface);
  const currentEnding = useGameStore((s) => s.currentEnding);

  if (!visible) return null;

  return (
    <div
      className="fixed top-2 right-2 z-[100] bg-shadow/90 border border-ash/50
                 rounded p-3 text-xs font-body text-smoke max-w-xs"
      data-testid="debug-overlay"
    >
      <div className="flex items-center justify-between mb-2">
        <span className="text-bone font-bold">DEBUG</span>
        <button
          onClick={onReset}
          className="text-blood hover:text-parchment text-xs px-2 py-0.5 border border-blood/30 rounded"
          data-testid="debug-reset"
        >
          Reset
        </button>
      </div>

      <div className="space-y-1 mb-2">
        <div>Chapter: <span className="text-bone">{currentSurface?.beat.chapter ?? "none"}</span></div>
        <div>Beat: <span className="text-bone">{currentSurface?.beat.id ?? "—"}</span></div>
        <div>Transcript: <span className="text-bone">{currentSurface?.transcript.length ?? 0}</span></div>
        <div>Glitch: <span className="text-bone">{currentSurface?.glitch_level?.toFixed(2) ?? "—"}</span></div>
        <div>Ending: <span className="text-bone">{currentEnding?.ending ?? "—"}</span></div>
      </div>

      <div className="flex gap-1 mt-2" data-testid="speed-controls">
        {[1, 2, 4].map((s) => (
          <button
            key={s}
            onClick={() => onSpeedChange(s)}
            className={`px-2 py-0.5 rounded text-xs border ${
              speedMultiplier === s
                ? "border-bone text-bone"
                : "border-ash text-smoke hover:text-bone"
            }`}
            data-testid={`speed-${s}x`}
          >
            {s}x
          </button>
        ))}
      </div>
    </div>
  );
}
