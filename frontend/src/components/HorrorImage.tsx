import { useCallback, useEffect, useRef, useState } from "react";
import type { DisplayMode } from "../types/narrative";

export interface HorrorImageProps {
  /** Image source (data URL or http URL). `null` while loading. */
  src: string | null;
  /** How the image should be presented. */
  displayMode: DisplayMode;
  /** Alt text for accessibility. */
  alt?: string;
}

type Phase = "loading" | "revealing" | "visible" | "flash" | "error";

const FADE_DURATION_MS = 2000;
const REVEAL_TICK_MS = 20;
const FLASH_VISIBLE_MS = 300;
const FLASH_TOTAL_MS = 1500;

/**
 * AI-generated image display with horror presentation effects.
 *
 * - `fade_in`: slowly emerges from darkness.
 * - `glitch`: RGB channel-split + scanline overlay.
 * - `flash`: briefly visible (subliminal), then fades to dark.
 */
export default function HorrorImage({
  src,
  displayMode,
  alt = "Scene illustration",
}: HorrorImageProps) {
  const [phase, setPhase] = useState<Phase>("loading");
  const [hasError, setHasError] = useState(false);
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const flashDoneRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // Reset when src changes.
  useEffect(() => {
    setHasError(false);
    if (timerRef.current) {
      clearTimeout(timerRef.current);
      timerRef.current = null;
    }
    if (flashDoneRef.current) {
      clearTimeout(flashDoneRef.current);
      flashDoneRef.current = null;
    }

    if (!src) {
      setPhase("loading");
      return;
    }

    if (displayMode === "flash") {
      setPhase("flash");
      timerRef.current = setTimeout(() => {
        setPhase("loading"); // back to dark
      }, FLASH_VISIBLE_MS);
      // After the full flash cycle, show normally.
      flashDoneRef.current = setTimeout(() => setPhase("visible"), FLASH_TOTAL_MS);
      return () => {
        if (timerRef.current) clearTimeout(timerRef.current);
        if (flashDoneRef.current) clearTimeout(flashDoneRef.current);
      };
    }

    setPhase("revealing");
    // Move to `visible` on the next tick so opacity transition runs once.
    timerRef.current = setTimeout(() => setPhase("visible"), REVEAL_TICK_MS);
    return () => {
      if (timerRef.current) clearTimeout(timerRef.current);
      if (flashDoneRef.current) clearTimeout(flashDoneRef.current);
    };
  }, [src, displayMode]);

  const handleError = useCallback(() => {
    setHasError(true);
    setPhase("error");
  }, []);

  // --- Render ---

  if (phase === "loading" && !src) {
    return (
      <div
        className="w-full aspect-video bg-shadow rounded overflow-hidden relative"
        data-testid="horror-image-loading"
      >
        <div className="absolute inset-0 bg-gradient-to-b from-transparent via-ash/20 to-transparent animate-slow-pulse" />
      </div>
    );
  }

  if (phase === "error" || hasError) {
    return (
      <div
        className="w-full aspect-video bg-shadow rounded overflow-hidden relative"
        data-testid="horror-image-error"
      >
        <div className="absolute inset-0 flex items-center justify-center">
          <span className="text-smoke/30 font-body text-sm animate-glitch">
            [signal lost]
          </span>
        </div>
        <div className="absolute inset-0 bg-gradient-to-b from-blood/5 to-transparent" />
      </div>
    );
  }

  const isGlitch = displayMode === "glitch";
  const isFlashHidden = phase === "loading"; // after flash fades

  const opacityClass =
    phase === "revealing"
      ? "opacity-0"
      : phase === "flash"
        ? "opacity-90"
        : isFlashHidden
          ? "opacity-0"
          : "opacity-80";

  return (
    <div
      className="w-full aspect-video rounded overflow-hidden relative bg-shadow"
      data-testid="horror-image"
      data-display-mode={displayMode}
      data-phase={phase}
    >
      {src && (
        <img
          src={src}
          alt={alt}
          onError={handleError}
          loading="lazy"
          className={`w-full h-full object-cover transition-opacity ${opacityClass}`}
          style={{
            transitionDuration: `${FADE_DURATION_MS}ms`,
            filter: isGlitch ? "saturate(1.5) contrast(1.2)" : undefined,
          }}
          data-testid="horror-image-img"
        />
      )}

      {/* Glitch: RGB channel split overlay */}
      {isGlitch && phase === "visible" && (
        <div
          className="absolute inset-0 pointer-events-none mix-blend-screen"
          data-testid="horror-image-glitch"
        >
          <div
            className="absolute inset-0 opacity-30"
            style={{
              backgroundImage: src ? `url(${src})` : undefined,
              backgroundSize: "cover",
              transform: "translateX(3px)",
              filter: "hue-rotate(120deg) saturate(3)",
            }}
          />
        </div>
      )}

      {/* Scanlines for glitch mode */}
      {isGlitch && (
        <div className="absolute inset-0 pointer-events-none crt-overlay opacity-30" />
      )}

      {/* Flash: bright overlay that fades */}
      {phase === "flash" && (
        <div
          className="absolute inset-0 bg-clinical/20 pointer-events-none"
          data-testid="horror-image-flash"
        />
      )}
    </div>
  );
}
