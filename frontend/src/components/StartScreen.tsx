import { useCallback, useEffect } from "react";

interface StartScreenProps {
  onStart: () => void;
}

export default function StartScreen({ onStart }: StartScreenProps) {
  const handleKeyDown = useCallback(
    (e: KeyboardEvent) => {
      if (e.key === "Enter") {
        onStart();
      }
    },
    [onStart],
  );

  useEffect(() => {
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [handleKeyDown]);

  return (
    <div className="min-h-screen bg-void select-none intelligence-grid">
      <div className="min-h-screen flex items-center justify-center px-6">
        <div className="w-full max-w-5xl rounded-[32px] border border-bone/10 bg-[#050608]/90 backdrop-blur-xl overflow-hidden shadow-[0_0_0_1px_rgba(255,255,255,0.03),0_40px_120px_rgba(0,0,0,0.6)]">
          <div className="grid md:grid-cols-[1.15fr_0.85fr]">
            <div className="px-8 py-10 md:px-12 md:py-14 border-b md:border-b-0 md:border-r border-bone/10">
              <p className="text-[11px] uppercase tracking-[0.4em] text-blood/70 mb-4">
                Echo Protocol / Intake
              </p>
              <h1 className="text-5xl md:text-7xl font-horror text-bone mb-6 leading-[0.95]">
                Audit Echo
              </h1>
              <p className="text-lg text-bone/72 font-body leading-relaxed max-w-2xl">
                Nexus AI Labs has hired you to review Echo, a dialogue system that has started
                leaking information it should not know. The terminal looks ordinary now. It will
                stop feeling ordinary once the session begins answering back.
              </p>
              <p className="mt-3 text-sm text-smoke/55 font-body italic">
                You are here to decide whether Echo has become dangerous. Echo may already be
                deciding what you are.
              </p>
              <div className="mt-10 grid gap-3 md:max-w-xl">
                <div className="rounded-2xl border border-bone/10 bg-white/[0.03] px-4 py-3 text-sm text-smoke/70">
                  Review internal documents, anomaly logs, and fragmented notes while holding a live conversation with Echo.
                </div>
                <div className="rounded-2xl border border-bone/10 bg-white/[0.03] px-4 py-3 text-sm text-smoke/70">
                  Your choices change trust, sanity, and awakening. The company is not as neutral as the assignment email implied.
                </div>
                <div className="rounded-2xl border border-bone/10 bg-white/[0.03] px-4 py-3 text-sm text-smoke/70">
                  If you stay long enough, the audit stops feeling one-directional.
                </div>
              </div>
            </div>

            <div className="px-8 py-10 md:px-10 md:py-14 flex flex-col justify-between bg-gradient-to-b from-white/[0.03] to-transparent">
              <div>
                <p className="text-[11px] uppercase tracking-[0.35em] text-smoke/55 mb-4">
                  Review Notes
                </p>
                <ul className="space-y-3 text-sm text-bone/68 font-body">
                  <li>Echo remains polite even when the subject matter stops being polite.</li>
                  <li>Some of the materials in this terminal have been revised after the fact.</li>
                  <li>The system may know more about the audit than the company admitted.</li>
                </ul>
              </div>

              <div className="mt-10">
                <button
                  onClick={onStart}
                  className="w-full rounded-2xl border border-bone/20 bg-bone/6 px-5 py-4
                             text-bone font-body text-sm uppercase tracking-[0.35em]
                             hover:bg-bone/10 hover:border-bone/35 transition-all duration-300
                             cursor-pointer focus:outline-none focus:border-bone/40"
                >
                  Press Enter to Begin
                </button>
                <p className="mt-4 text-xs text-smoke/45 font-body">
                  The assignment says you are evaluating Echo. The rest of the session is less certain.
                </p>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
