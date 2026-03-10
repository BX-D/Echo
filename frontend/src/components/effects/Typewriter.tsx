import { useCallback, useEffect, useRef, useState } from "react";

/** Typing speed presets in milliseconds per character. */
export type TypeSpeed = "slow" | "normal" | "fast" | "instant";

const SPEED_MS: Record<TypeSpeed, number> = {
  slow: 50,
  normal: 30,
  fast: 15,
  instant: 0,
};

/** Pause multipliers for dramatic punctuation. */
const PAUSE_CHARS: Record<string, number> = {
  ".": 6,
  "!": 5,
  "?": 5,
  "\u2014": 4, // em-dash
  "\n": 3,
};

const GLITCH_CHARS = "!@#$%^&*<>{}[]|/\\~`";
const GLITCH_PROBABILITY = 0.05; // 1 in 20
const GLITCH_DURATION_MS = 60;

export interface TypewriterProps {
  /** The full text to reveal. */
  text: string;
  /** Typing speed preset. */
  speed?: TypeSpeed;
  /** Called when all characters have been revealed. */
  onComplete?: () => void;
  /** Additional CSS class for the container. */
  className?: string;
}

/**
 * Character-by-character text reveal with glitch insertions and dramatic
 * pauses on punctuation.
 *
 * Press any key to skip the animation and show the full text.
 */
export default function Typewriter({
  text,
  speed = "normal",
  onComplete,
  className = "",
}: TypewriterProps) {
  const [displayed, setDisplayed] = useState("");
  const [glitchChar, setGlitchChar] = useState<string | null>(null);
  const [done, setDone] = useState(false);

  const indexRef = useRef(0);
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const completedRef = useRef(false);
  const onCompleteRef = useRef(onComplete);
  onCompleteRef.current = onComplete;

  // ── Core tick ──────────────────────────────────────────────────────────
  const tick = useCallback(() => {
    if (indexRef.current >= text.length) {
      setDone(true);
      if (!completedRef.current) {
        completedRef.current = true;
        onCompleteRef.current?.();
      }
      return;
    }

    const char = text[indexRef.current]!;
    const baseDelay = SPEED_MS[speed];

    // Glitch: briefly show a wrong character before the real one.
    if (baseDelay > 0 && Math.random() < GLITCH_PROBABILITY) {
      const wrong = GLITCH_CHARS[Math.floor(Math.random() * GLITCH_CHARS.length)]!;
      setGlitchChar(wrong);
      timerRef.current = setTimeout(() => {
        setGlitchChar(null);
        indexRef.current += 1;
        setDisplayed(text.slice(0, indexRef.current));
        timerRef.current = setTimeout(tick, baseDelay);
      }, GLITCH_DURATION_MS);
      return;
    }

    indexRef.current += 1;
    setDisplayed(text.slice(0, indexRef.current));

    // Dramatic pause on certain punctuation.
    const pauseMultiplier = PAUSE_CHARS[char] ?? 1;
    // Check for ellipsis: three dots in a row.
    if (
      char === "." &&
      indexRef.current >= 3 &&
      text.slice(indexRef.current - 3, indexRef.current) === "..."
    ) {
      timerRef.current = setTimeout(tick, baseDelay * 8);
      return;
    }

    timerRef.current = setTimeout(tick, baseDelay * pauseMultiplier);
  }, [text, speed]);

  // ── Start / restart on text change ─────────────────────────────────────
  useEffect(() => {
    indexRef.current = 0;
    completedRef.current = false;
    setDisplayed("");
    setDone(false);
    setGlitchChar(null);

    if (SPEED_MS[speed] === 0) {
      // Instant mode: show everything at once.
      setDisplayed(text);
      setDone(true);
      completedRef.current = true;
      onCompleteRef.current?.();
      return;
    }

    timerRef.current = setTimeout(tick, SPEED_MS[speed]);

    return () => {
      if (timerRef.current !== null) clearTimeout(timerRef.current);
    };
  }, [text, speed, tick]);

  // ── Skip on keypress ───────────────────────────────────────────────────
  useEffect(() => {
    const handleKey = () => {
      if (completedRef.current) return;
      if (timerRef.current !== null) clearTimeout(timerRef.current);
      setGlitchChar(null);
      indexRef.current = text.length;
      setDisplayed(text);
      setDone(true);
      if (!completedRef.current) {
        completedRef.current = true;
        onCompleteRef.current?.();
      }
    };
    window.addEventListener("keydown", handleKey);
    return () => window.removeEventListener("keydown", handleKey);
  }, [text]);

  // ── Render ─────────────────────────────────────────────────────────────
  const visibleText = glitchChar !== null ? displayed + glitchChar : displayed;

  return (
    <span className={className} data-testid="typewriter" aria-live="polite">
      <span className="whitespace-pre-line">{visibleText}</span>
      {!done && (
        <span
          className="inline-block w-[2px] h-[1.1em] bg-bone ml-[1px] align-text-bottom animate-pulse"
          data-testid="typewriter-cursor"
        />
      )}
      {done && (
        <span
          className="inline-block w-[2px] h-[1.1em] bg-bone/60 ml-[1px] align-text-bottom animate-pulse"
          data-testid="typewriter-cursor-done"
        />
      )}
    </span>
  );
}
