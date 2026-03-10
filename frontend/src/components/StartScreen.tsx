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
                Session Mirror / Intake
              </p>
              <h1 className="text-5xl md:text-7xl font-horror text-bone mb-6 leading-[0.95]">
                It Learns Your Fear
              </h1>
              <p className="text-lg text-bone/72 font-body leading-relaxed max-w-2xl">
                You are entering a session, not a story. The system will watch how you hesitate,
                what you refuse, when you keep going, and how long you remain available once it
                becomes too specific to trust.
              </p>
              <p className="mt-3 text-sm text-smoke/55 font-body italic">
                It is not trying to jump-scare you. It is trying to decide what kind of person you are under observation.
              </p>
              <div className="mt-10 grid gap-3 md:max-w-xl">
                <div className="rounded-2xl border border-bone/10 bg-white/[0.03] px-4 py-3 text-sm text-smoke/70">
                  The session measures hesitation, pacing, attention, and whether you keep choosing once you know you are being read.
                </div>
                <div className="rounded-2xl border border-bone/10 bg-white/[0.03] px-4 py-3 text-sm text-smoke/70">
                  Camera and microphone are optional. Granting them changes the session. Refusing them also changes the session.
                </div>
                <div className="rounded-2xl border border-bone/10 bg-white/[0.03] px-4 py-3 text-sm text-smoke/70">
                  At the end, the system does not tell you whether you won. It tells you what it thinks your behavior meant.
                </div>
              </div>
            </div>

            <div className="px-8 py-10 md:px-10 md:py-14 flex flex-col justify-between bg-gradient-to-b from-white/[0.03] to-transparent">
              <div>
                <p className="text-[11px] uppercase tracking-[0.35em] text-smoke/55 mb-4">
                  Consent Conditions
                </p>
                <ul className="space-y-3 text-sm text-bone/68 font-body">
                  <li>Observation may feel reassuring before it feels invasive.</li>
                  <li>The system is interested in consent, withdrawal, silence, and continued attention.</li>
                  <li>You may stop at any time, but departure is still a behavior pattern.</li>
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
                  This is a psychological session about how you behave once the system stops sounding neutral.
                </p>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
