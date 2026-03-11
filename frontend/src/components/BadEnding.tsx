import { useEffect, useState } from "react";

export interface BadEndingProps {
  /** Player's selfie data URL (if taken). */
  selfieUrl: string | null;
  /** Called when player clicks restart. */
  onRestart: () => void;
}

/**
 * Session-over state when the system reaches overwhelming confidence.
 */
export default function BadEnding({ selfieUrl, onRestart }: BadEndingProps) {
  const [phase, setPhase] = useState<"flash" | "dark" | "message">("flash");

  useEffect(() => {
    // Flash phase: bright jumpscare for 500ms.
    const t1 = setTimeout(() => setPhase("dark"), 500);
    // Dark phase: blackout for 1s.
    const t2 = setTimeout(() => setPhase("message"), 1500);
    return () => {
      clearTimeout(t1);
      clearTimeout(t2);
    };
  }, []);

  if (phase === "flash") {
    return (
      <div
        className="fixed inset-0 z-[200] bg-clinical flex items-center justify-center animate-shake"
        data-testid="bad-ending-flash"
      >
        {selfieUrl ? (
          <img
            src={selfieUrl}
            alt=""
            className="w-full h-full object-cover"
            style={{ filter: "contrast(2) saturate(0.3) brightness(1.5) hue-rotate(340deg)" }}
          />
        ) : (
          <div className="text-9xl text-blood font-horror animate-glitch">DEAD</div>
        )}
      </div>
    );
  }

  if (phase === "dark") {
    return <div className="fixed inset-0 z-[200] bg-void" />;
  }

  return (
    <div
      className="fixed inset-0 z-[200] bg-void flex flex-col items-center justify-center animate-fade-in"
      data-testid="bad-ending"
    >
      <p className="text-[11px] uppercase tracking-[0.38em] text-smoke/55 mb-5 font-body">
        Echo Protocol / Terminal State
      </p>
      <h1 className="text-5xl font-horror text-blood mb-4 animate-flicker text-center">
        THE SESSION HAS
        <br />
        ENOUGH OF YOU
      </h1>
      <p className="text-smoke/70 font-body text-sm mb-3 max-w-xl text-center leading-relaxed">
        It did not need more fear. It needed enough consistency to decide who
        you become when the room starts sounding exact.
      </p>
      <p className="text-smoke/45 font-body text-xs mb-12 max-w-md text-center">
        What ended here was not the story. Only your ability to keep it from concluding.
      </p>

      <button
        onClick={onRestart}
        className="px-8 py-3 border border-blood/40 text-blood hover:text-parchment
                   hover:border-bone/40 transition-colors duration-300 font-body text-lg
                   cursor-pointer"
        data-testid="restart-button"
      >
        Restart Session
      </button>
    </div>
  );
}
