/**
 * Invisible behavior collector that captures player input patterns
 * and batches them for transmission to the server.
 *
 * All event listeners are passive and the processing cost per event
 * is kept well under 1 ms to avoid any perceptible impact.
 */

import type { BehaviorEvent, BehaviorEventType } from "../types/behavior";
import type { ChoiceApproach, SurfaceMedium } from "../types/narrative";

/** Callback the collector calls every flush interval. */
export type SendBatchFn = (events: BehaviorEvent[], sceneId: string) => void;

const FLUSH_INTERVAL_MS = 2_000;
const PAUSE_THRESHOLD_MS = 3_000;
const MOUSE_WINDOW_MS = 500;
const REREADING_THRESHOLD = 0.7;

export class BehaviorCollector {
  private events: BehaviorEvent[] = [];

  // Keystroke tracking
  private keystrokeCount = 0;
  private backspaceCount = 0;
  private keystrokeWindowStart = 0;

  // Scroll / rereading tracking
  private maxScrollPosition = 0;
  private currentScrollPosition = 0;

  // Mouse tremor tracking
  private mousePositions: Array<{ x: number; y: number; t: number }> = [];

  // Choice timing
  private choiceDisplayTime = 0;

  // Scene context
  private currentSceneId = "";

  // Pause detection
  private lastInputTime = 0;
  private pauseCheckTimer: ReturnType<typeof setInterval> | null = null;
  private lastPauseRecordedAt = 0;
  private blurStartedAt = 0;

  // Flush timer
  private flushTimer: ReturnType<typeof setInterval> | null = null;

  // Bound handlers (for removal)
  private boundKeydown: (e: KeyboardEvent) => void;
  private boundMouseMove: (e: MouseEvent) => void;
  private boundScroll: () => void;
  private boundBlur: () => void;
  private boundFocus: () => void;

  constructor(private sendBatch: SendBatchFn) {
    this.boundKeydown = this.handleKeydown.bind(this);
    this.boundMouseMove = this.handleMouseMove.bind(this);
    this.boundScroll = this.handleScroll.bind(this);
    this.boundBlur = this.handleBlur.bind(this);
    this.boundFocus = this.handleFocus.bind(this);
  }

  /** Begin capturing DOM events. */
  attach(): void {
    window.addEventListener("keydown", this.boundKeydown, { passive: true });
    window.addEventListener("mousemove", this.boundMouseMove, {
      passive: true,
    });
    window.addEventListener("scroll", this.boundScroll, { passive: true });
    window.addEventListener("blur", this.boundBlur, { passive: true });
    window.addEventListener("focus", this.boundFocus, { passive: true });

    this.flushTimer = setInterval(() => this.flush(), FLUSH_INTERVAL_MS);
    this.pauseCheckTimer = setInterval(
      () => this.checkForPause(),
      PAUSE_THRESHOLD_MS,
    );
  }

  /** Stop capturing and flush remaining events. */
  detach(): void {
    window.removeEventListener("keydown", this.boundKeydown);
    window.removeEventListener("mousemove", this.boundMouseMove);
    window.removeEventListener("scroll", this.boundScroll);
    window.removeEventListener("blur", this.boundBlur);
    window.removeEventListener("focus", this.boundFocus);

    if (this.flushTimer !== null) {
      clearInterval(this.flushTimer);
      this.flushTimer = null;
    }
    if (this.pauseCheckTimer !== null) {
      clearInterval(this.pauseCheckTimer);
      this.pauseCheckTimer = null;
    }

    this.flush();
  }

  /** Called when the game displays choices for the current scene. */
  recordChoiceDisplayed(sceneId: string): void {
    this.choiceDisplayTime = performance.now();
    this.currentSceneId = sceneId;
  }

  /** Called when the player clicks a choice. */
  recordChoiceSelected(
    choiceId: string,
    approach: ChoiceApproach,
  ): void {
    const decideMs =
      this.choiceDisplayTime > 0
        ? Math.round(performance.now() - this.choiceDisplayTime)
        : 0;

    this.pushEvent({
      type: "choice",
      choice_id: choiceId,
      time_to_decide_ms: decideMs,
      approach,
    });
    this.choiceDisplayTime = 0;
  }

  recordChoiceHoverPattern(
    hoveredChoiceIds: string[],
    dominantChoiceId: string | null,
    totalHoverMs: number,
  ): void {
    if (hoveredChoiceIds.length === 0) return;
    this.pushEvent({
      type: "choice_hover_pattern",
      hovered_choice_ids: hoveredChoiceIds,
      dominant_choice_id: dominantChoiceId,
      total_hover_ms: Math.round(totalHoverMs),
    });
  }

  /** Update the scene context so events are tagged correctly. */
  setCurrentScene(sceneId: string): void {
    // Flush old scene's events before switching.
    if (this.currentSceneId && this.currentSceneId !== sceneId) {
      this.flush();
    }
    this.currentSceneId = sceneId;
    this.maxScrollPosition = 0;
    this.currentScrollPosition = 0;
    this.keystrokeCount = 0;
    this.backspaceCount = 0;
    this.keystrokeWindowStart = performance.now();
    this.lastInputTime = performance.now();
    this.lastPauseRecordedAt = 0;
    this.blurStartedAt = 0;
  }

  /** Returns the number of pending (unflushed) events. */
  pendingCount(): number {
    return this.events.length;
  }

  /** Records whether a device permission was granted. */
  recordPermissionDecision(device: string, granted: boolean): void {
    this.pushEvent({
      type: "device_permission",
      device,
      granted,
    });
  }

  recordMediaEngagement(
    medium: SurfaceMedium,
    dwellMs: number,
    interactionCount: number,
  ): void {
    this.pushEvent({
      type: "media_engagement",
      medium,
      dwell_ms: Math.max(0, Math.round(dwellMs)),
      interaction_count: Math.max(0, interactionCount),
    });
  }

  recordCameraPresence(visibleMs: number, sustainedPresence: boolean): void {
    this.pushEvent({
      type: "camera_presence",
      visible_ms: Math.max(0, Math.round(visibleMs)),
      sustained_presence: sustainedPresence,
    });
  }

  recordMicSilenceResponse(
    dwellMs: number,
    exitedEarly: boolean,
    returnedAfterPrompt: boolean,
  ): void {
    this.pushEvent({
      type: "mic_silence_response",
      dwell_ms: Math.max(0, Math.round(dwellMs)),
      exited_early: exitedEarly,
      returned_after_prompt: returnedAfterPrompt,
    });
  }

  // -- Private event handlers --------------------------------------------

  private handleKeydown(e: KeyboardEvent): void {
    const now = performance.now();
    this.lastInputTime = now;

    if (e.key === "Backspace") {
      this.backspaceCount++;
      return;
    }

    // Only count printable single characters.
    if (e.key.length !== 1) return;

    this.keystrokeCount++;

    if (this.keystrokeWindowStart === 0) {
      this.keystrokeWindowStart = now;
    }

    const elapsed = now - this.keystrokeWindowStart;
    // Every ~20 keystrokes, emit a keystroke event.
    if (this.keystrokeCount >= 20 || elapsed > 5_000) {
      const seconds = elapsed / 1000;
      const cps = seconds > 0 ? this.keystrokeCount / seconds : 0;

      this.pushEvent({
        type: "keystroke",
        chars_per_second: Math.round(cps * 100) / 100,
        backspace_count: this.backspaceCount,
        total_chars: this.keystrokeCount,
      });

      this.keystrokeCount = 0;
      this.backspaceCount = 0;
      this.keystrokeWindowStart = now;
    }

  }

  private handleMouseMove(e: MouseEvent): void {
    const now = performance.now();
    this.lastInputTime = now;

    this.mousePositions.push({ x: e.clientX, y: e.clientY, t: now });

    // Keep only the last MOUSE_WINDOW_MS of positions.
    const cutoff = now - MOUSE_WINDOW_MS;
    while (
      this.mousePositions.length > 0 &&
      this.mousePositions[0]!.t < cutoff
    ) {
      this.mousePositions.shift();
    }

    // Compute tremor every ~30 samples (avoid per-pixel cost).
    if (this.mousePositions.length >= 30) {
      const { velocity, tremor } = this.computeMouseMetrics();
      this.pushEvent({
        type: "mouse_movement",
        velocity: Math.round(velocity * 10) / 10,
        tremor_score: Math.round(tremor * 1000) / 1000,
      });
      this.mousePositions = [];
    }
  }

  private handleScroll(): void {
    const now = performance.now();
    this.lastInputTime = now;

    const docHeight = Math.max(
      document.documentElement.scrollHeight - window.innerHeight,
      1,
    );
    const pos = window.scrollY / docHeight;
    this.currentScrollPosition = Math.min(pos, 1);

    const wasRereading =
      this.maxScrollPosition > 0 &&
      this.currentScrollPosition <
        this.maxScrollPosition * REREADING_THRESHOLD;

    if (this.currentScrollPosition > this.maxScrollPosition) {
      this.maxScrollPosition = this.currentScrollPosition;
    }

    this.pushEvent({
      type: "scroll",
      direction: this.currentScrollPosition >= this.maxScrollPosition ? "down" : "up",
      to_position: Math.round(this.currentScrollPosition * 1000) / 1000,
      rereading: wasRereading,
    });
  }

  private handleBlur(): void {
    this.blurStartedAt = performance.now();
    this.pushEvent({
      type: "focus_change",
      focused: false,
      return_latency_ms: null,
    });
  }

  private handleFocus(): void {
    const now = performance.now();
    const latency =
      this.blurStartedAt > 0 ? Math.round(now - this.blurStartedAt) : null;
    this.blurStartedAt = 0;
    this.pushEvent({
      type: "focus_change",
      focused: true,
      return_latency_ms: latency,
    });
  }

  // -- Pause detection ---------------------------------------------------

  private checkForPause(): void {
    if (this.lastInputTime === 0 || !this.currentSceneId) return;

    const now = performance.now();
    const silent = now - this.lastInputTime;

    if (
      silent >= PAUSE_THRESHOLD_MS &&
      this.lastPauseRecordedAt < this.lastInputTime
    ) {
      this.pushEvent({
        type: "pause",
        duration_ms: Math.round(silent),
        scene_content_hash: this.currentSceneId,
      });
      this.lastPauseRecordedAt = now;
    }
  }

  // -- Mouse metrics -----------------------------------------------------

  private computeMouseMetrics(): { velocity: number; tremor: number } {
    if (this.mousePositions.length < 2) {
      return { velocity: 0, tremor: 0 };
    }

    let totalDist = 0;
    const angles: number[] = [];

    for (let i = 1; i < this.mousePositions.length; i++) {
      const prev = this.mousePositions[i - 1]!;
      const curr = this.mousePositions[i]!;
      const dx = curr.x - prev.x;
      const dy = curr.y - prev.y;
      totalDist += Math.sqrt(dx * dx + dy * dy);
      angles.push(Math.atan2(dy, dx));
    }

    const first = this.mousePositions[0]!;
    const last = this.mousePositions[this.mousePositions.length - 1]!;
    const dtSec = Math.max((last.t - first.t) / 1000, 0.001);
    const velocity = totalDist / dtSec;

    // Tremor = direction variance (high = shaky mouse).
    let tremor = 0;
    if (angles.length >= 2) {
      const mean = angles.reduce((a, b) => a + b, 0) / angles.length;
      const variance =
        angles.reduce((sum, a) => sum + (a - mean) ** 2, 0) / angles.length;
      // Normalise to [0, 1] — variance of angles ranges roughly 0 to π².
      tremor = Math.min(variance / (Math.PI * Math.PI), 1);
    }

    return { velocity, tremor };
  }

  // -- Flush / push ------------------------------------------------------

  private pushEvent(eventType: BehaviorEventType): void {
    this.events.push({
      event_type: eventType,
      timestamp: new Date().toISOString(),
      scene_id: this.currentSceneId,
    });
  }

  private flush(): void {
    if (this.events.length === 0) return;
    const batch = this.events.splice(0);
    this.sendBatch(batch, this.currentSceneId);
  }
}
