import { useCallback, useEffect, useRef } from "react";
import type { ChoiceApproach } from "../types/narrative";
import { BehaviorCollector } from "../systems/BehaviorCollector";
import type { BehaviorEvent } from "../types/behavior";
import type { ClientMessage } from "../types/ws";

/**
 * React hook that wraps {@link BehaviorCollector}, attaching on mount and
 * detaching on unmount.  Batches are sent over the WebSocket as
 * `behavior_batch` messages.
 *
 * The tracker is completely invisible to the player — no UI, no side-effects.
 */
export function useBehaviorTracker(
  send: (msg: ClientMessage) => void,
  sceneId: string,
) {
  const collectorRef = useRef<BehaviorCollector | null>(null);

  // Stable send-batch callback.
  const sendBatch = useCallback(
    (events: BehaviorEvent[], _sceneId: string) => {
      send({
        type: "behavior_batch",
        payload: {
          events,
          timestamp: new Date().toISOString(),
        },
      });
    },
    [send],
  );

  // Create and attach once.
  useEffect(() => {
    const collector = new BehaviorCollector(sendBatch);
    collectorRef.current = collector;
    collector.attach();

    return () => {
      collector.detach();
      collectorRef.current = null;
    };
  }, [sendBatch]);

  // Update scene context when sceneId changes.
  useEffect(() => {
    collectorRef.current?.setCurrentScene(sceneId);
  }, [sceneId]);

  const recordChoiceDisplayed = useCallback((sid: string) => {
    collectorRef.current?.recordChoiceDisplayed(sid);
  }, []);

  const recordChoiceSelected = useCallback(
    (choiceId: string, approach: ChoiceApproach) => {
      collectorRef.current?.recordChoiceSelected(choiceId, approach);
    },
    [],
  );

  const recordChoiceHoverPattern = useCallback(
    (
      hoveredChoiceIds: string[],
      dominantChoiceId: string | null,
      totalHoverMs: number,
    ) => {
      collectorRef.current?.recordChoiceHoverPattern(
        hoveredChoiceIds,
        dominantChoiceId,
        totalHoverMs,
      );
    },
    [],
  );

  const recordPermissionDecision = useCallback(
    (device: string, granted: boolean) => {
      collectorRef.current?.recordPermissionDecision(device, granted);
    },
    [],
  );

  const recordMediaEngagement = useCallback(
    (medium: import("../types/narrative").SurfaceMedium, dwellMs: number, interactionCount: number) => {
      collectorRef.current?.recordMediaEngagement(medium, dwellMs, interactionCount);
    },
    [],
  );

  const recordCameraPresence = useCallback(
    (visibleMs: number, sustainedPresence: boolean) => {
      collectorRef.current?.recordCameraPresence(visibleMs, sustainedPresence);
    },
    [],
  );

  const recordMicSilenceResponse = useCallback(
    (
      dwellMs: number,
      exitedEarly: boolean,
      returnedAfterPrompt: boolean,
    ) => {
      collectorRef.current?.recordMicSilenceResponse(
        dwellMs,
        exitedEarly,
        returnedAfterPrompt,
      );
    },
    [],
  );

  return {
    recordChoiceDisplayed,
    recordChoiceSelected,
    recordChoiceHoverPattern,
    recordPermissionDecision,
    recordMediaEngagement,
    recordCameraPresence,
    recordMicSilenceResponse,
  };
}
