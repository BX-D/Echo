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
  const gamePhase = useGameStore((s) => s.gamePhase);
  const currentScene = useGameStore((s) => s.currentScene);
  const sceneHistory = useGameStore((s) => s.sceneHistory);

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
        <div>Phase: <span className="text-bone">{gamePhase ?? "none"}</span></div>
        <div>Scene: <span className="text-bone">{currentScene?.scene_id ?? "—"}</span></div>
        <div>History: <span className="text-bone">{sceneHistory.length}</span></div>
        <div>Intensity: <span className="text-bone">{currentScene?.intensity?.toFixed(2) ?? "—"}</span></div>
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
