/**
 * Subtle session pressure meter.
 *
 * The old "fear bar" semantics are preserved for gameplay, but the presentation
 * is reframed as how readable / modelled the player has become to the system.
 */

export interface FearMeterProps {
  /** Current fear value, 0–100. */
  value: number;
  /** Maximum before bad ending triggers. */
  max?: number;
}

export default function FearMeter({ value, max = 100 }: FearMeterProps) {
  const pct = Math.min((value / max) * 100, 100);
  const isCritical = pct >= 80;
  const isDanger = pct >= 50;

  return (
    <div
      className="fixed left-5 bottom-6 z-40 w-[176px] sm:w-[192px]"
      data-testid="fear-meter"
    >
      <div className="rounded-[22px] border border-bone/10 bg-[#06080c]/90 backdrop-blur-xl px-4 py-4 shadow-[0_18px_40px_rgba(0,0,0,0.35)]">
        <div className="flex items-end justify-between gap-3">
          <div>
            <p className="text-[10px] font-body text-smoke/55 tracking-[0.28em] uppercase">
              Readability
            </p>
            <p className="mt-1 text-[11px] text-smoke/42 font-body uppercase tracking-[0.22em]">
              Session Pressure
            </p>
          </div>
          <p
            className={`text-sm font-body ${
              isCritical ? "text-blood animate-pulse" : "text-bone/72"
            }`}
          >
            {Math.round(pct)}%
          </p>
        </div>

        <div className="mt-4 h-2.5 rounded-full border border-ash/25 bg-shadow/85 overflow-hidden">
          <div
            className={`h-full rounded-full transition-all duration-700 ${
              isCritical
                ? "bg-gradient-to-r from-blood via-red-700 to-parchment/70 animate-pulse"
                : isDanger
                  ? "bg-gradient-to-r from-blood via-rust to-bone/40"
                  : "bg-gradient-to-r from-[#311012] to-[#6a191d]"
            }`}
            style={{ width: `${pct}%` }}
            data-testid="fear-meter-fill"
          />
        </div>

        <div className="mt-3 flex items-start justify-between gap-3 text-[11px] font-body leading-relaxed">
          <p className="text-smoke/50">
            {isCritical
              ? "The session has a stable model of you."
              : isDanger
                ? "Its confidence is increasing."
                : "Your pattern is still incomplete."}
          </p>
          <span className="shrink-0 text-smoke/38 uppercase tracking-[0.22em]">
            {pct >= 67 ? "high" : pct >= 34 ? "rising" : "low"}
          </span>
        </div>
      </div>
    </div>
  );
}
