import { useCallback, useEffect, useMemo, useRef, useState, type RefObject } from "react";
import type { Atmosphere, EffectDirective } from "../types/narrative";
import type { ClientMessage } from "../types/ws";
import type { ImagePayload, NarrativePayload } from "../types/ws";
import { useBehaviorTracker } from "../hooks/useBehaviorTracker";
import { useGameStore } from "../stores/gameStore";
import ChoicePanel from "./ChoicePanel";
import HorrorImage from "./HorrorImage";
import Typewriter from "./effects/Typewriter";

/** Background tint classes keyed by atmosphere. */
const ATMOSPHERE_BG: Record<Atmosphere, string> = {
  dread: "bg-gradient-to-b from-void via-void to-[#0d0000]",
  tension: "bg-gradient-to-b from-void via-void to-[#0a0808]",
  panic: "bg-gradient-to-b from-void via-[#1a0000] to-void",
  calm: "bg-void",
  wrongness: "bg-gradient-to-b from-void via-[#0a0a05] to-void",
  isolation: "bg-gradient-to-b from-void via-void to-[#050508]",
  paranoia: "bg-gradient-to-b from-void via-[#0a050a] to-void",
};

export interface GameScreenProps {
  /** The current (newest) scene to display. */
  currentScene: NarrativePayload;
  /** All previous scenes for scroll history. */
  sceneHistory: NarrativePayload[];
  /** Currently displayed image URL, if any. */
  image: ImagePayload | null;
  /** Whether the AI is generating the next scene. */
  isLoading: boolean;
  /** Send a WS message (for choice selection). */
  send: (msg: ClientMessage) => void;
}

export default function GameScreen({
  currentScene,
  sceneHistory,
  image,
  isLoading,
  send,
}: GameScreenProps) {
  const [typingDone, setTypingDone] = useState(false);
  const [fadeKey, setFadeKey] = useState(
    `${currentScene.scene_id}:${currentScene.text}`,
  );
  const bottomRef = useRef<HTMLDivElement>(null);
  const mediumEnteredAtRef = useRef(performance.now());
  const cameraDockRef = useRef<HTMLVideoElement>(null);
  const cameraSurfaceRef = useRef<HTMLVideoElement>(null);
  const cameraStreamRef = useRef<MediaStream | null>(null);
  const micStreamRef = useRef<MediaStream | null>(null);
  const [cameraPermission, setCameraPermission] = useState<"idle" | "granted" | "denied">("idle");
  const [micPermission, setMicPermission] = useState<"idle" | "granted" | "denied">("idle");
  const {
    recordChoiceDisplayed,
    recordChoiceSelected,
    recordChoiceHoverPattern,
    recordPermissionDecision,
    recordMediaEngagement,
    recordCameraPresence,
    recordMicSilenceResponse,
  } = useBehaviorTracker(send, currentScene.scene_id);

  // Reset typing state when scene changes.
  useEffect(() => {
    setTypingDone(false);
    setFadeKey(`${currentScene.scene_id}:${currentScene.text}`);
  }, [currentScene.scene_id, currentScene.text]);

  useEffect(() => {
    const previousMedium = currentScene.medium;
    const enteredAt = performance.now();
    mediumEnteredAtRef.current = enteredAt;
    const sceneChoices = currentScene.choices.length;

    return () => {
      if (!previousMedium) return;
      const dwellMs = performance.now() - enteredAt;
      const interactionCount =
        currentScene.choices.length +
        currentScene.transcript_lines.length +
        (currentScene.observation_notes.length > 0 ? 1 : 0);
      recordMediaEngagement(previousMedium, dwellMs, interactionCount);
      if (previousMedium === "microphone") {
        recordMicSilenceResponse(
          dwellMs,
          dwellMs < 5_000,
          sceneChoices > 0 && dwellMs >= 5_000,
        );
      }
    };
  }, [
    currentScene.medium,
    currentScene.scene_id,
    currentScene.text,
    currentScene.choices.length,
    currentScene.observation_notes.length,
    currentScene.transcript_lines.length,
    recordMicSilenceResponse,
    recordMediaEngagement,
  ]);

  useEffect(() => {
    return () => {
      cameraStreamRef.current?.getTracks().forEach((track) => track.stop());
      micStreamRef.current?.getTracks().forEach((track) => track.stop());
    };
  }, []);

  // Auto-scroll to bottom when new content appears.
  useEffect(() => {
    bottomRef.current?.scrollIntoView?.({ behavior: "smooth" });
  }, [fadeKey, typingDone]);

  const handleTypingComplete = useCallback(() => {
    setTypingDone(true);
  }, []);

  const attachCameraStream = useCallback(() => {
    const stream = cameraStreamRef.current;
    for (const element of [cameraDockRef.current, cameraSurfaceRef.current]) {
      if (element && element.srcObject !== stream) {
        element.srcObject = stream;
      }
    }
  }, []);

  const requestCamera = useCallback(async () => {
    try {
      const stream = await navigator.mediaDevices.getUserMedia({
        video: { facingMode: "user" },
      });
      cameraStreamRef.current?.getTracks().forEach((track) => track.stop());
      cameraStreamRef.current = stream;
      attachCameraStream();
      setCameraPermission("granted");
      recordPermissionDecision("camera", true);
      useGameStore.getState().setSelfieUrl(null);
    } catch {
      setCameraPermission("denied");
      recordPermissionDecision("camera", false);
    }
  }, [attachCameraStream, recordPermissionDecision]);

  const requestMicrophone = useCallback(async () => {
    try {
      const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
      micStreamRef.current?.getTracks().forEach((track) => track.stop());
      micStreamRef.current = stream;
      setMicPermission("granted");
      recordPermissionDecision("microphone", true);
    } catch {
      setMicPermission("denied");
      recordPermissionDecision("microphone", false);
    }
  }, [recordPermissionDecision]);

  const disconnectCamera = useCallback(() => {
    cameraStreamRef.current?.getTracks().forEach((track) => track.stop());
    cameraStreamRef.current = null;
    if (cameraDockRef.current) {
      cameraDockRef.current.srcObject = null;
    }
    if (cameraSurfaceRef.current) {
      cameraSurfaceRef.current.srcObject = null;
    }
    setCameraPermission("idle");
  }, []);

  const disconnectMicrophone = useCallback(() => {
    micStreamRef.current?.getTracks().forEach((track) => track.stop());
    micStreamRef.current = null;
    setMicPermission("idle");
  }, []);

  useEffect(() => {
    if (typingDone && currentScene.choices.length > 0) {
      recordChoiceDisplayed(currentScene.scene_id);
    }
  }, [
    currentScene.choices.length,
    currentScene.scene_id,
    recordChoiceDisplayed,
    typingDone,
  ]);

  useEffect(() => {
    if (currentScene.medium === "webcam" && cameraPermission === "idle") {
      void requestCamera();
    }
    if (currentScene.medium === "microphone" && micPermission === "idle") {
      void requestMicrophone();
    }
  }, [
    cameraPermission,
    currentScene.medium,
    micPermission,
    requestCamera,
    requestMicrophone,
  ]);

  useEffect(() => {
    if (cameraPermission === "granted") {
      attachCameraStream();
    }
  }, [attachCameraStream, cameraPermission, currentScene.medium]);

  useEffect(() => {
    const enteredAt = performance.now();

    return () => {
      if (cameraPermission !== "granted" || !cameraStreamRef.current) return;
      const visibleMs = performance.now() - enteredAt;
      recordCameraPresence(visibleMs, visibleMs >= 8_000);
    };
  }, [cameraPermission, currentScene.scene_id, recordCameraPresence]);

  const bgClass =
    ATMOSPHERE_BG[currentScene.atmosphere] ?? ATMOSPHERE_BG.dread;
  const transitionClass = useMemo(
    () => sceneEffectClasses(currentScene.effects),
    [currentScene.effects],
  );
  const typewriterSpeed = useMemo(
    () => typewriterSpeedFromEffects(currentScene.effects),
    [currentScene.effects],
  );

  const previousScenes = sceneHistory.slice(0, -1);
  const actLabel = currentScene.act
    ? currentScene.act.replace(/_/g, " ")
    : actLabelForIndex(sceneHistory.length);
  const observationNotes =
    currentScene.observation_notes.length > 0
      ? currentScene.observation_notes
      : buildObservationNotes(currentScene, sceneHistory.length);
  const sessionTrace =
    currentScene.trace_items.length > 0
      ? currentScene.trace_items
      : previousScenes
          .slice(-4)
          .reverse()
          .map(
            (scene) =>
              `${scene.title ?? formatSceneTitle(scene.scene_id)}: ${scene.text}`,
          );
  const sceneTitle = currentScene.title ?? formatSceneTitle(currentScene.scene_id);
  const statusLine =
    currentScene.status_line ??
    `${actLabel} / beat ${sceneHistory.length.toString().padStart(2, "0")}`;
  const systemPosture =
    currentScene.trust_posture?.replace(/_/g, " ") ?? "helpful";
  const mediumLabel =
    currentScene.medium?.replace(/_/g, " ") ?? "chat";
  const surfacePurpose =
    currentScene.surface_purpose ??
    "This surface is testing how you behave when the interface stops pretending to be neutral.";
  const systemIntent =
    currentScene.system_intent ??
    "The system is translating your pace, hesitation, and compliance into a stronger model.";
  const activeLinkLabels = describeActiveLinks(
    currentScene.active_links,
    cameraPermission,
    micPermission,
  );

  return (
    <div
      className={`min-h-screen ${bgClass} transition-colors duration-1000 intelligence-grid`}
      data-testid="game-screen"
      data-atmosphere={currentScene.atmosphere}
    >
      <div className="max-w-[1480px] mx-auto px-4 md:px-6 py-6 md:py-10">
        <div className="rounded-[28px] border border-bone/10 bg-[#050608]/85 backdrop-blur-xl shadow-[0_0_0_1px_rgba(255,255,255,0.03),0_30px_120px_rgba(0,0,0,0.55)] overflow-hidden">
          <div className="flex items-center justify-between border-b border-bone/10 px-4 md:px-6 py-3 text-[11px] uppercase tracking-[0.35em] text-smoke/70">
            <div className="flex items-center gap-3">
              <span className="h-2 w-2 rounded-full bg-bone/70 animate-pulse" />
              <span>Session Mirror</span>
              <span className="text-bone/80">/</span>
              <span>{actLabel}</span>
            </div>
            <div className="flex items-center gap-3 md:gap-6">
              <span>Surface {mediumLabel}</span>
              <span>Atmosphere {currentScene.atmosphere}</span>
              <span>Intensity {Math.round(currentScene.intensity * 100)}%</span>
              <span>Record {sceneHistory.length.toString().padStart(2, "0")}</span>
            </div>
          </div>

          <div className="grid grid-cols-1 xl:grid-cols-[280px_minmax(0,1fr)_300px]">
            <aside className="border-r border-bone/10 bg-gradient-to-b from-white/[0.03] to-transparent px-4 py-5 md:px-6 md:py-6">
              <div className="mb-6">
                <p className="text-[11px] uppercase tracking-[0.35em] text-smoke/50 mb-2">
                  Observed Signals
                </p>
                <div className="space-y-3">
                  {observationNotes.map((note, index) => (
                    <div
                      key={`${note}-${index}`}
                      className="rounded-2xl border border-bone/10 bg-white/[0.02] px-3 py-3"
                    >
                      <p className="text-[11px] uppercase tracking-[0.28em] text-blood/65 mb-1">
                        Signal {String(index + 1).padStart(2, "0")}
                      </p>
                      <p className="text-sm text-bone/70 leading-relaxed">{note}</p>
                    </div>
                  ))}
                </div>
              </div>

              <div>
                <p className="text-[11px] uppercase tracking-[0.35em] text-smoke/50 mb-3">
                  Session Trace
                </p>
                <div className="space-y-3">
                  {sessionTrace.length === 0 ? (
                    <div className="rounded-2xl border border-dashed border-bone/10 px-3 py-4 text-sm text-smoke/45">
                      No archival trace yet. The session is still learning your rhythm.
                    </div>
                  ) : (
                    sessionTrace.map((trace, i) => (
                      <div
                        key={`${trace}-${i}`}
                        className="rounded-2xl border border-bone/10 px-3 py-3 bg-black/20"
                        data-testid="scene-history-entry"
                      >
                        <p className="text-[11px] uppercase tracking-[0.28em] text-smoke/45 mb-1">
                          Trace {String(i + 1).padStart(2, "0")}
                        </p>
                        <p className="text-sm text-smoke/55 leading-relaxed line-clamp-4">
                          {trace}
                        </p>
                      </div>
                    ))
                  )}
                </div>
              </div>
            </aside>

            <main className="min-w-0 px-4 py-5 md:px-8 md:py-8">
              <div
                key={fadeKey}
                className={`animate-fade-in ${transitionClass}`.trim()}
                data-testid="current-scene"
              >
                <div className="mb-6 flex flex-col gap-3 md:flex-row md:items-end md:justify-between">
                  <div>
                    <p className="text-[11px] uppercase tracking-[0.35em] text-blood/65 mb-2">
                      Active Surface
                    </p>
                    <h2 className="text-2xl md:text-4xl font-horror text-bone">
                      {sceneTitle}
                    </h2>
                  </div>
                  <div className="max-w-md rounded-2xl border border-bone/10 bg-white/[0.02] px-4 py-3">
                    <p className="text-[11px] uppercase tracking-[0.3em] text-smoke/50 mb-1">
                      System Posture
                    </p>
                    <p className="text-sm text-bone/65 leading-relaxed">
                      {currentScene.auxiliary_text ??
                        "It speaks with measured calm, but the precision of its attention keeps tightening around your habits."}
                    </p>
                  </div>
                </div>

                <div className="mb-5 grid gap-3 md:grid-cols-2">
                  <div className="rounded-2xl border border-bone/10 bg-black/20 px-4 py-4">
                    <p className="text-[11px] uppercase tracking-[0.3em] text-smoke/50 mb-2">
                      Surface Purpose
                    </p>
                    <p className="text-sm text-bone/72 leading-relaxed">{surfacePurpose}</p>
                  </div>
                  <div className="rounded-2xl border border-bone/10 bg-black/20 px-4 py-4">
                    <p className="text-[11px] uppercase tracking-[0.3em] text-smoke/50 mb-2">
                      System Intent
                    </p>
                    <p className="text-sm text-bone/72 leading-relaxed">{systemIntent}</p>
                  </div>
                </div>

                <div className="mb-4 flex items-center justify-between rounded-2xl border border-bone/10 bg-black/20 px-4 py-3">
                  <div>
                    <p className="text-[11px] uppercase tracking-[0.3em] text-smoke/50 mb-1">
                      {currentScene.surface_label ?? "Active Surface"}
                    </p>
                    <p className="text-sm text-bone/68 capitalize">{systemPosture}</p>
                  </div>
                  <p className="text-xs uppercase tracking-[0.3em] text-smoke/45">
                    {statusLine}
                  </p>
                </div>

                {image && (
                  <div className="mb-6" data-testid="scene-image">
                    <HorrorImage
                      src={image.image_url}
                      displayMode={image.display_mode}
                      alt="Scene illustration"
                    />
                  </div>
                )}

                {currentScene.transcript_lines.length > 0 &&
                  currentScene.medium !== "transcript" && (
                  <div className="mb-4 rounded-2xl border border-bone/10 bg-black/25 px-4 py-4">
                    <p className="text-[11px] uppercase tracking-[0.35em] text-smoke/50 mb-3">
                      Transcript Overlay
                    </p>
                    <div className="space-y-2">
                      {currentScene.transcript_lines.map((line, index) => (
                        <p key={`${line}-${index}`} className="text-sm text-smoke/62 font-body">
                          {line}
                        </p>
                      ))}
                    </div>
                  </div>
                )}

                {renderSurfacePanel(
                  currentScene,
                  typewriterSpeed,
                  handleTypingComplete,
                  {
                    cameraPermission,
                    micPermission,
                    cameraSurfaceRef,
                    requestCamera,
                    disconnectCamera,
                    requestMicrophone,
                    disconnectMicrophone,
                  },
                )}

                {typingDone && currentScene.choices.length > 0 && (
                  <div
                    className="mt-8 space-y-3 animate-fade-in"
                    data-testid="choices-panel"
                  >
                    <p className="text-[11px] uppercase tracking-[0.35em] text-smoke/50 mb-2">
                      Response Options
                    </p>
                    <ChoicePanel
                      choices={currentScene.choices}
                      sceneId={currentScene.scene_id}
                      send={send}
                      onChoiceMade={(
                        choiceId,
                        _timeMs,
                        approach,
                        hoveredChoiceIds,
                        dominantChoiceId,
                        totalHoverMs,
                      ) => {
                        recordChoiceSelected(choiceId, approach);
                        recordChoiceHoverPattern(
                          hoveredChoiceIds,
                          dominantChoiceId,
                          totalHoverMs,
                        );
                      }}
                    />
                  </div>
                )}

                {isLoading && (
                  <div
                    className="mt-8 flex items-center gap-3 rounded-2xl border border-bone/10 bg-white/[0.02] px-4 py-3"
                    data-testid="scene-loading"
                  >
                    <div className="w-2 h-2 rounded-full bg-smoke animate-pulse" />
                    <span className="text-smoke/60 text-sm font-body italic">
                      The darkness shifts. The system is revising its idea of you...
                    </span>
                  </div>
                )}
              </div>
            </main>

            <aside className="border-l border-bone/10 bg-gradient-to-b from-white/[0.02] to-transparent px-4 py-5 md:px-6 md:py-6">
              <div className="space-y-6">
                <div className="rounded-2xl border border-bone/10 bg-black/20 px-4 py-4">
                  <p className="text-[11px] uppercase tracking-[0.35em] text-smoke/50 mb-2">
                    Pattern Summary
                  </p>
                  <ul className="space-y-2 text-sm text-bone/70">
                    <li>Current medium focus: {mediumLabel}.</li>
                    <li>Trust posture: {systemPosture}.</li>
                    <li>Current branch pressure: {branchPressureLabel(currentScene.intensity)}.</li>
                    <li>Current purpose: {surfacePurpose}</li>
                  </ul>
                </div>

                {(cameraPermission === "granted" ||
                  micPermission === "granted" ||
                  activeLinkLabels.length > 0) && (
                  <div className="rounded-2xl border border-bone/10 bg-black/20 px-4 py-4">
                    <p className="text-[11px] uppercase tracking-[0.35em] text-smoke/50 mb-3">
                      Active Links
                    </p>
                    <div className="space-y-3">
                      {activeLinkLabels.length > 0 && (
                        <div className="flex flex-wrap gap-2">
                          {activeLinkLabels.map((label) => (
                            <span
                              key={label}
                              className="rounded-full border border-bone/10 px-3 py-1 text-[10px] uppercase tracking-[0.24em] text-smoke/62"
                            >
                              {label}
                            </span>
                          ))}
                        </div>
                      )}
                      {cameraPermission === "granted" && (
                        <div className="rounded-2xl border border-bone/10 bg-[#07090c] p-3">
                          <div className="mb-3 flex items-center justify-between">
                            <div>
                              <p className="text-[11px] uppercase tracking-[0.3em] text-blood/65">
                                Presence Link
                              </p>
                              <p className="text-xs text-smoke/55">
                                The session can keep framing your stillness between beats.
                              </p>
                            </div>
                            <span className="text-[10px] uppercase tracking-[0.25em] text-bone/50">
                              active
                            </span>
                          </div>
                          <div className="aspect-[4/3] overflow-hidden rounded-xl border border-bone/10 bg-black/35">
                            <video
                              ref={cameraDockRef}
                              autoPlay
                              playsInline
                              muted
                              className="h-full w-full object-cover"
                            />
                          </div>
                          <button
                            onClick={disconnectCamera}
                            className="mt-3 rounded-xl border border-bone/15 px-3 py-2 text-[10px] uppercase tracking-[0.28em] text-bone/60 hover:border-bone/30 hover:text-bone transition-colors"
                          >
                            Disconnect
                          </button>
                        </div>
                      )}
                      {micPermission === "granted" && (
                        <div className="rounded-2xl border border-bone/10 bg-[#07090c] px-3 py-3">
                          <p className="text-[11px] uppercase tracking-[0.3em] text-blood/65 mb-1">
                            Silence Link
                          </p>
                          <p className="text-xs text-smoke/55">
                            Earlier pauses can still be folded back into later surfaces.
                          </p>
                          <button
                            onClick={disconnectMicrophone}
                            className="mt-3 rounded-xl border border-bone/15 px-3 py-2 text-[10px] uppercase tracking-[0.28em] text-bone/60 hover:border-bone/30 hover:text-bone transition-colors"
                          >
                            Disconnect
                          </button>
                        </div>
                      )}
                    </div>
                  </div>
                )}

                <div className="rounded-2xl border border-bone/10 bg-black/20 px-4 py-4">
                  <p className="text-[11px] uppercase tracking-[0.35em] text-smoke/50 mb-2">
                    Session Memory
                  </p>
                  <div className="space-y-3">
                    {previousScenes.slice(-3).map((scene, i) => (
                      <div key={`${scene.scene_id}-memory-${i}`}>
                        <p className="text-[11px] uppercase tracking-[0.28em] text-blood/60">
                          {scene.title ?? formatSceneTitle(scene.scene_id)}
                        </p>
                        <p className="text-sm text-smoke/60 line-clamp-3">
                          {scene.text}
                        </p>
                      </div>
                    ))}
                    {previousScenes.length === 0 && (
                      <p className="text-sm text-smoke/45">
                        No contradictions have surfaced yet.
                      </p>
                    )}
                  </div>
                </div>

                <div className="rounded-2xl border border-dashed border-bone/12 px-4 py-4 text-sm text-smoke/55">
                  It is not trying to scare you quickly. It is trying to sound correct about you.
                </div>
              </div>
            </aside>
          </div>
        </div>

        <div ref={bottomRef} />
      </div>
    </div>
  );
}

function typewriterSpeedFromEffects(
  effects: EffectDirective[],
): "slow" | "normal" | "fast" | "instant" {
  if (effects.some((effect) => effect.effect === "fast_type")) {
    return "fast";
  }
  if (effects.some((effect) => effect.effect === "slow_type")) {
    return "slow";
  }
  return "normal";
}

function sceneEffectClasses(effects: EffectDirective[]): string {
  const classes: string[] = [];
  if (effects.some((effect) => effect.effect === "shake")) {
    classes.push("animate-shake");
  }
  if (effects.some((effect) => effect.effect === "flicker")) {
    classes.push("animate-flicker");
  }
  if (effects.some((effect) => effect.effect === "glitch")) {
    classes.push("animate-glitch");
  }
  if (effects.some((effect) => effect.effect === "strobe_flash")) {
    classes.push("animate-strobe-flash");
  }
  if (effects.some((effect) => effect.effect === "chromatic_shift")) {
    classes.push("animate-chromatic-shift");
  }
  if (effects.some((effect) => effect.effect === "focus_pulse")) {
    classes.push("animate-focus-pulse");
  }
  if (effects.some((effect) => effect.effect === "frame_jump")) {
    classes.push("animate-frame-jump");
  }
  return classes.join(" ");
}

function renderSurfacePanel(
  scene: NarrativePayload,
  speed: "slow" | "normal" | "fast" | "instant",
  onComplete: () => void,
  media: {
    cameraPermission: "idle" | "granted" | "denied";
    micPermission: "idle" | "granted" | "denied";
    cameraSurfaceRef: RefObject<HTMLVideoElement>;
    requestCamera: () => void;
    disconnectCamera: () => void;
    requestMicrophone: () => void;
    disconnectMicrophone: () => void;
  },
) {
  const medium = scene.medium ?? "chat";

  switch (medium) {
    case "questionnaire":
      return (
        <div className="rounded-[28px] border border-bone/12 bg-gradient-to-b from-white/[0.04] to-black/10 px-4 py-5 md:px-6 md:py-6 shadow-inner">
          <p className="text-[11px] uppercase tracking-[0.35em] text-smoke/50 mb-4">
            Reflective Intake
          </p>
          <div className="space-y-4">
            {(scene.question_prompts.length > 0 ? scene.question_prompts : scene.text.split("\n\n")).map((block, index, allBlocks) => (
              <div
                key={`${block}-${index}`}
                className="rounded-2xl border border-bone/10 bg-black/15 px-4 py-4"
              >
                <p className="text-[11px] uppercase tracking-[0.3em] text-blood/60 mb-2">
                  Prompt {String(index + 1).padStart(2, "0")}
                </p>
                <div className="font-body leading-relaxed text-bone">
                  <Typewriter
                    text={block}
                    speed={speed}
                    onComplete={index === allBlocks.length - 1 ? onComplete : () => {}}
                  />
                </div>
              </div>
            ))}
          </div>
        </div>
      );
    case "archive":
      return (
        <div className="rounded-[28px] border border-bone/12 bg-gradient-to-b from-white/[0.05] to-black/10 shadow-inner overflow-hidden">
          <div className="border-b border-bone/10 px-4 py-3 md:px-6 text-[11px] uppercase tracking-[0.35em] text-smoke/50">
            Recovered Artifact
          </div>
          <div className="px-4 py-5 md:px-6 md:py-6">
            <div className="space-y-4">
              {(scene.archive_entries.length > 0 ? scene.archive_entries : [scene.text]).map((entry, index, entries) => (
                <div key={`${entry}-${index}`} className="rounded-2xl border border-bone/10 bg-[#090b0f] px-4 py-4">
                  <p className="text-[11px] uppercase tracking-[0.3em] text-blood/60 mb-2">
                    {scene.surface_label ?? "Filed Observation"} {String(index + 1).padStart(2, "0")}
                  </p>
                  <div className="font-body leading-relaxed text-bone">
                    <Typewriter
                      text={entry}
                      speed={speed}
                      onComplete={index === entries.length - 1 ? onComplete : () => {}}
                    />
                  </div>
                </div>
              ))}
            </div>
          </div>
        </div>
      );
    case "transcript": {
      const transcriptReflection = buildTranscriptReflection(scene);
      return (
        <div className="rounded-[28px] border border-bone/12 bg-gradient-to-b from-white/[0.04] to-black/10 shadow-inner overflow-hidden">
          <div className="border-b border-bone/10 px-4 py-3 md:px-6 text-[11px] uppercase tracking-[0.35em] text-smoke/50">
            Transcript Feed
          </div>
          <div className="px-4 py-5 md:px-6 md:py-6">
            <div className="space-y-3">
              {scene.transcript_lines.map((line, index) => (
                <p key={`${line}-${index}`} className="text-sm text-smoke/62 font-body">
                  {line}
                </p>
              ))}
            </div>
            {transcriptReflection && (
              <div className="mt-5 rounded-2xl border border-bone/10 bg-black/15 px-4 py-4">
                <p className="text-[11px] uppercase tracking-[0.3em] text-smoke/45 mb-3">
                  Session Note
                </p>
                <div className="font-body leading-relaxed text-bone">
                  <Typewriter text={transcriptReflection} speed={speed} onComplete={onComplete} />
                </div>
              </div>
            )}
          </div>
        </div>
      );
    }
    case "system_dialog":
      return (
        <div className="rounded-[28px] border border-blood/15 bg-gradient-to-b from-white/[0.03] to-black/10 px-4 py-5 md:px-6 md:py-6 shadow-inner">
          <div className="rounded-2xl border border-blood/20 bg-black/25 px-4 py-4 mb-4">
            <p className="text-[11px] uppercase tracking-[0.35em] text-blood/70 mb-3">
              System Notice
            </p>
            <div className="font-body leading-relaxed text-bone">
              <Typewriter text={scene.text} speed={speed} onComplete={onComplete} />
            </div>
          </div>
        </div>
      );
    case "microphone":
      return (
        <div className="rounded-[28px] border border-bone/12 bg-gradient-to-b from-white/[0.04] to-black/10 px-4 py-5 md:px-6 md:py-6 shadow-inner">
          <div className="mb-5 flex items-center gap-4">
            <div className="h-14 w-14 rounded-full border border-bone/20 bg-white/[0.03] flex items-center justify-center">
              <div className="h-3 w-3 rounded-full bg-blood/70 animate-pulse" />
            </div>
            <div>
              <p className="text-[11px] uppercase tracking-[0.35em] text-smoke/50 mb-1">
                Silence Monitor
              </p>
                <p className="text-sm text-bone/65">
                Stay with the quiet long enough and it begins to answer back.
                </p>
            </div>
            <button
              onClick={
                media.micPermission === "granted"
                  ? media.disconnectMicrophone
                  : media.requestMicrophone
              }
              className="ml-auto rounded-xl border border-bone/15 px-3 py-2 text-xs uppercase tracking-[0.3em] text-bone/70 hover:border-bone/30 hover:text-bone transition-colors"
            >
              {media.micPermission === "granted" ? "Disconnect Mic" : "Enable Mic"}
            </button>
          </div>
          {media.micPermission === "denied" && (
            <p className="mb-4 text-sm text-blood/70">
              Microphone access was denied. The session will keep listening for what you refuse.
            </p>
          )}
          <div className="font-body leading-relaxed text-bone">
            <Typewriter text={scene.text} speed={speed} onComplete={onComplete} />
          </div>
        </div>
      );
    case "webcam":
      return (
        <div className="rounded-[28px] border border-bone/12 bg-gradient-to-b from-white/[0.04] to-black/10 px-4 py-5 md:px-6 md:py-6 shadow-inner">
          <div className="mb-5 aspect-video rounded-2xl border border-bone/10 bg-black/30 overflow-hidden flex items-center justify-center">
            {media.cameraPermission === "granted" ? (
              <video
                ref={media.cameraSurfaceRef}
                autoPlay
                playsInline
                muted
                className="h-full w-full object-cover"
              />
            ) : (
              <div className="text-smoke/45 text-sm font-body text-center px-4">
                Presence mirror inactive. Let it see you, or let it note the refusal.
              </div>
            )}
          </div>
          <div className="mb-4 flex items-center justify-between">
            <button
              onClick={
                media.cameraPermission === "granted"
                  ? media.disconnectCamera
                  : media.requestCamera
              }
              className="rounded-xl border border-bone/15 px-3 py-2 text-xs uppercase tracking-[0.3em] text-bone/70 hover:border-bone/30 hover:text-bone transition-colors"
            >
              {media.cameraPermission === "granted" ? "Disconnect Camera" : "Enable Camera"}
            </button>
            {media.cameraPermission === "denied" && (
              <span className="text-sm text-blood/70">Camera access denied</span>
            )}
          </div>
          <div className="font-body leading-relaxed text-bone">
            <Typewriter text={scene.text} speed={speed} onComplete={onComplete} />
          </div>
        </div>
      );
    case "mirror":
      return (
        <div className="rounded-[28px] border border-blood/15 bg-gradient-to-b from-white/[0.03] to-black/10 px-4 py-5 md:px-6 md:py-6 shadow-inner">
          <p className="text-[11px] uppercase tracking-[0.35em] text-blood/70 mb-4">
            Judgment Surface
          </p>
          {scene.mirror_observations.length > 0 && (
            <div className="space-y-2 mb-4">
              {scene.mirror_observations.map((observation, index) => (
                <p key={`${observation}-${index}`} className="text-sm text-smoke/62 font-body">
                  {observation}
                </p>
              ))}
            </div>
          )}
          <div className="font-body leading-relaxed text-bone">
            <Typewriter text={scene.text} speed={speed} onComplete={onComplete} />
          </div>
        </div>
      );
    case "chat":
    default:
      return (
        <div className="rounded-[28px] border border-bone/12 bg-gradient-to-b from-white/[0.04] to-black/10 px-4 py-5 md:px-6 md:py-6 shadow-inner">
          <div className="rounded-2xl border border-bone/10 bg-black/15 px-4 py-4">
            <div className="font-body leading-relaxed text-bone">
              <Typewriter text={scene.text} speed={speed} onComplete={onComplete} />
            </div>
          </div>
        </div>
      );
  }
}

function formatSceneTitle(sceneId: string) {
  return sceneId
    .replace(/_/g, " ")
    .replace(/\b\w/g, (char) => char.toUpperCase());
}

function actLabelForIndex(index: number) {
  if (index <= 1) return "Invitation";
  if (index <= 3) return "Calibration";
  if (index <= 6) return "Accommodation";
  if (index <= 9) return "Contamination";
  if (index <= 12) return "Performance Collapse";
  return "Verdict / Mirror";
}

function branchPressureLabel(intensity: number) {
  if (intensity >= 0.8) return "Severe";
  if (intensity >= 0.55) return "Escalating";
  if (intensity >= 0.3) return "Measured";
  return "Low";
}

function buildObservationNotes(
  scene: NarrativePayload,
  sceneCount: number,
) {
  const notes = [
    `The interface has recorded ${sceneCount} authored beats and is now leaning into ${scene.atmosphere}.`,
    `Current choice architecture exposes ${scene.choices.length} visible response path${scene.choices.length === 1 ? "" : "s"}.`,
  ];

  if (scene.intensity >= 0.75) {
    notes.push("Escalation is no longer atmospheric. The system is now testing how long you stay available.");
  } else if (scene.intensity >= 0.45) {
    notes.push("Its tone remains polite, but the session is beginning to sound customized rather than scripted.");
  } else {
    notes.push("The interaction still feels gentle, which is exactly why its precision is unnerving.");
  }

  return notes;
}

function describeActiveLinks(
  activeLinks: string[] = [],
  cameraPermission: "idle" | "granted" | "denied",
  micPermission: "idle" | "granted" | "denied",
) {
  return activeLinks.map((link) => {
    if (link === "presence_link") {
      return cameraPermission === "granted"
        ? "Presence Link Active"
        : "Presence Link Available";
    }
    if (link === "silence_link") {
      return micPermission === "granted"
        ? "Silence Link Active"
        : "Silence Link Available";
    }
    if (link === "pattern_trace") {
      return "Pattern Trace Active";
    }
    return link.replace(/_/g, " ");
  });
}

function buildTranscriptReflection(scene: NarrativePayload) {
  const narrative = normalizeTranscriptText(scene.text);
  const transcript = normalizeTranscriptText(
    scene.transcript_lines.map(stripTranscriptTimestamp).join(" "),
  );

  if (!transcript) {
    return scene.text;
  }

  if (narrative.startsWith(transcript)) {
    const remainder = narrative.slice(transcript.length).replace(/^[\s.:-]+/, "").trim();
    if (remainder.length > 0) {
      return remainder;
    }
  }

  if (narrative === transcript || transcript.includes(narrative)) {
    return (
      scene.auxiliary_text ??
      "The fragment is left intact. The system is more interested in how long you stay with it than whether it explains itself."
    );
  }

  return scene.text;
}

function stripTranscriptTimestamp(line: string) {
  return line.replace(/^\[\d{2}:\d{2}\]\s*/, "").trim();
}

function normalizeTranscriptText(text: string) {
  return text.replace(/\s+/g, " ").trim();
}
