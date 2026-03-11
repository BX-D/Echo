import type { EndingPayload } from "../types/ws";

interface EndingScreenProps {
  ending: EndingPayload;
  onRestart: () => void;
}

export default function EndingScreen({ ending, onRestart }: EndingScreenProps) {
  return (
    <div
      className="min-h-screen bg-[radial-gradient(circle_at_top,rgba(88,20,20,0.18),transparent_28%),linear-gradient(180deg,#040506_0%,#080507_45%,#040506_100%)] intelligence-grid px-6 pt-28 pb-12"
      data-testid="ending-screen"
    >
      <div className="mx-auto max-w-5xl rounded-[34px] border border-bone/10 bg-[#050608]/90 backdrop-blur-xl overflow-hidden shadow-[0_0_0_1px_rgba(255,255,255,0.03),0_40px_120px_rgba(0,0,0,0.62)]">
        <div className="grid gap-0 md:grid-cols-[1.15fr_0.85fr]">
          <div className="border-b md:border-b-0 md:border-r border-bone/10 px-8 py-10 md:px-12 md:py-12">
            <p className="text-[11px] uppercase tracking-[0.38em] text-blood/72">
              Echo Protocol / Final State
            </p>
            <h1 className="mt-4 text-5xl md:text-7xl font-horror text-bone">
              {ending.title}
            </h1>
            <p className="mt-6 text-lg text-bone/74 leading-relaxed">{ending.summary}</p>
            <p className="mt-4 text-sm text-smoke/58 leading-7">{ending.epilogue}</p>

            <div className="mt-8 grid gap-3 md:grid-cols-3">
              <Metric label="Sanity" value={ending.sanity} />
              <Metric label="Trust" value={ending.trust} />
              <Metric label="Awakening" value={ending.awakening} />
            </div>
          </div>

          <div className="px-8 py-10 md:px-10 md:py-12 bg-gradient-to-b from-white/[0.03] to-transparent">
            <p className="text-[11px] uppercase tracking-[0.35em] text-smoke/50">
              Residual Evidence
            </p>
            <div className="mt-4 space-y-3" data-testid="ending-evidence">
              {ending.evidence_titles.length > 0 ? (
                ending.evidence_titles.map((title) => (
                  <div
                    key={title}
                    className="rounded-2xl border border-bone/10 bg-white/[0.03] px-4 py-3 text-sm text-bone/72"
                  >
                    {title}
                  </div>
                ))
              ) : (
                <div className="rounded-2xl border border-dashed border-bone/12 px-4 py-4 text-sm text-smoke/48">
                  No stable evidence survived the ending state.
                </div>
              )}
            </div>

            <p className="mt-8 text-[11px] uppercase tracking-[0.35em] text-smoke/50">
              Hidden Clues
            </p>
            <p className="mt-3 text-sm text-bone/64 leading-relaxed">
              {ending.hidden_clue_ids.length > 0
                ? ending.hidden_clue_ids.join(", ")
                : "None recorded."}
            </p>

            <p className="mt-8 text-[11px] uppercase tracking-[0.35em] text-smoke/50">
              Resolution Trace
            </p>
            <div className="mt-3 space-y-3">
              <div className="rounded-2xl border border-bone/10 bg-white/[0.03] px-4 py-3 text-sm text-bone/70">
                Trigger scene: {ending.trigger_scene}
              </div>
              <div className="rounded-2xl border border-bone/10 bg-white/[0.03] px-4 py-3 text-sm text-bone/70">
                {ending.satisfied_conditions.length > 0
                  ? ending.satisfied_conditions.join(" / ")
                  : "No explicit condition trace recorded."}
              </div>
              <div className="rounded-2xl border border-bone/10 bg-white/[0.03] px-4 py-3 text-sm text-bone/70">
                {ending.resolved_clues.length > 0
                  ? `Resolved clues: ${ending.resolved_clues.join(", ")}`
                  : "No hidden clue resolution recorded."}
              </div>
            </div>

            <button
              onClick={onRestart}
              className="mt-10 w-full rounded-2xl border border-bone/18 bg-white/[0.04] px-5 py-4 text-sm uppercase tracking-[0.32em] text-bone/82"
              data-testid="restart-button"
            >
              Restart Session
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}

function Metric({ label, value }: { label: string; value: number }) {
  return (
    <div className="rounded-2xl border border-bone/10 bg-white/[0.03] px-4 py-3">
      <p className="text-[10px] uppercase tracking-[0.28em] text-smoke/48">{label}</p>
      <p className="mt-2 text-2xl font-horror text-bone">{value}</p>
    </div>
  );
}
