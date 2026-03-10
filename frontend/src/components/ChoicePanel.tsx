import { useCallback, useEffect, useRef, useState } from "react";
import type { Choice } from "../types/narrative";
import type { ClientMessage } from "../types/ws";

const STAGGER_DELAY_MS = 200;
const FEEDBACK_DURATION_MS = 400;

export interface ChoicePanelProps {
  choices: Choice[];
  sceneId: string;
  send: (msg: ClientMessage) => void;
  /** Called with the chosen metadata and decision time. */
  onChoiceMade?: (
    choiceId: string,
    timeMs: number,
    approach: Choice["approach"],
    hoveredChoiceIds: string[],
    dominantChoiceId: string | null,
    totalHoverMs: number,
  ) => void;
}

export default function ChoicePanel({
  choices,
  sceneId,
  send,
  onChoiceMade,
}: ChoicePanelProps) {
  const [visibleCount, setVisibleCount] = useState(0);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [disabled, setDisabled] = useState(false);

  // Timing
  const displayTimeRef = useRef(performance.now());
  const hoverStartRef = useRef<Record<string, number>>({});
  const hoverDurations = useRef<Record<string, number>>({});

  // Staggered reveal
  useEffect(() => {
    setVisibleCount(0);
    setSelectedId(null);
    setDisabled(false);
    displayTimeRef.current = performance.now();
    hoverStartRef.current = {};
    hoverDurations.current = {};

    let count = 0;
    const timer = setInterval(() => {
      count += 1;
      setVisibleCount(count);
      if (count >= choices.length) clearInterval(timer);
    }, STAGGER_DELAY_MS);

    return () => clearInterval(timer);
  }, [choices]);

  // Keyboard shortcuts (1–4)
  useEffect(() => {
    const handleKey = (e: KeyboardEvent) => {
      if (disabled) return;
      const idx = parseInt(e.key, 10) - 1;
      if (idx >= 0 && idx < choices.length && idx < visibleCount) {
        selectChoice(choices[idx]!.id);
      }
    };
    window.addEventListener("keydown", handleKey);
    return () => window.removeEventListener("keydown", handleKey);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [choices, disabled, visibleCount]);

  const selectChoice = useCallback(
    (choiceId: string) => {
      if (disabled) return;
      setSelectedId(choiceId);
      setDisabled(true);

      const elapsed = Math.round(performance.now() - displayTimeRef.current);
      const selectedChoice =
        choices.find((choice) => choice.id === choiceId) ?? null;
      const hoverEntries = Object.entries(hoverDurations.current);
      const hoveredChoiceIds = hoverEntries.map(([id]) => id);
      const totalHoverMs = hoverEntries.reduce((total, [, duration]) => total + duration, 0);
      const dominantChoiceId =
        hoverEntries.sort((a, b) => b[1] - a[1])[0]?.[0] ?? null;

      // Send the choice message.
      send({
        type: "choice",
        payload: {
          scene_id: sceneId,
          choice_id: choiceId,
          time_to_decide_ms: elapsed,
          approach: selectedChoice?.approach ?? "investigate",
        },
      });

      onChoiceMade?.(
        choiceId,
        elapsed,
        selectedChoice?.approach ?? "investigate",
        hoveredChoiceIds,
        dominantChoiceId,
        totalHoverMs,
      );

      // Brief visual feedback before the scene transitions.
      setTimeout(() => {
        setSelectedId(null);
      }, FEEDBACK_DURATION_MS);
    },
    [disabled, sceneId, send, onChoiceMade],
  );

  const handleHoverStart = useCallback((id: string) => {
    hoverStartRef.current[id] = performance.now();
  }, []);

  const handleHoverEnd = useCallback((id: string) => {
    const start = hoverStartRef.current[id];
    if (start) {
      const duration = performance.now() - start;
      hoverDurations.current[id] =
        (hoverDurations.current[id] ?? 0) + duration;
      delete hoverStartRef.current[id];
    }
  }, []);

  return (
    <div className="space-y-3" data-testid="choice-panel" role="group" aria-label="Choices">
      {choices.map((choice, i) => {
        const isVisible = i < visibleCount;
        const isSelected = selectedId === choice.id;

        return (
          <button
            key={choice.id}
            onClick={() => selectChoice(choice.id)}
            onMouseEnter={() => handleHoverStart(choice.id)}
            onMouseLeave={() => handleHoverEnd(choice.id)}
            disabled={disabled}
            data-testid={`choice-${choice.id}`}
            data-approach={choice.approach}
            className={`
              block w-full text-left px-5 py-3 border font-body
              transition-all duration-300
              ${isVisible ? "opacity-100 translate-y-0" : "opacity-0 translate-y-2 pointer-events-none"}
              ${isSelected
                ? "border-bone bg-bone/10 text-parchment scale-[1.01]"
                : "border-smoke/20 text-smoke hover:text-bone hover:border-bone/40 hover:bg-shadow/50 hover:translate-x-1"
              }
              ${disabled && !isSelected ? "opacity-40 cursor-not-allowed" : "cursor-pointer"}
            `}
            style={{
              transitionDelay: isVisible ? "0ms" : `${i * STAGGER_DELAY_MS}ms`,
            }}
          >
            <span className="text-smoke/40 mr-3 text-sm">{i + 1}.</span>
            {choice.text}
          </button>
        );
      })}
    </div>
  );
}
