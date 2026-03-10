import { useCallback, useEffect, useMemo } from "react";
import AmbientDarkness from "./components/effects/AmbientDarkness";
import CRTOverlay from "./components/effects/CRTOverlay";
import Vignette from "./components/effects/Vignette";
import BadEnding from "./components/BadEnding";
import FearMeter from "./components/FearMeter";
import FearReveal from "./components/FearReveal";
import GameScreen from "./components/GameScreen";
import LoadingScreen from "./components/LoadingScreen";
import SessionChrome from "./components/SessionChrome";
import StartScreen from "./components/StartScreen";
import { useAudio } from "./hooks/useAudio";
import { useWebSocket } from "./hooks/useWebSocket";
import { useGameStore } from "./stores/gameStore";

const WS_URL =
  import.meta.env.VITE_WS_URL ?? "ws://localhost:3001/ws";
const API_URL = apiUrlFromWs(WS_URL);

export default function App() {
  const sessionId = useGameStore((s) => s.sessionId);
  const { send } = useWebSocket(WS_URL, sessionId);
  const { initAudio, playCue, setIntensity } = useAudio();
  const connectionStatus = useGameStore((s) => s.connectionStatus);
  const gamePhase = useGameStore((s) => s.gamePhase);
  const currentScene = useGameStore((s) => s.currentScene);
  const revealData = useGameStore((s) => s.revealData);
  const sceneHistory = useGameStore((s) => s.sceneHistory);
  const currentImage = useGameStore((s) => s.currentImage);
  const currentError = useGameStore((s) => s.currentError);
  const currentMeta = useGameStore((s) => s.currentMeta);
  const fearLevel = useGameStore((s) => s.fearLevel);
  const maxFear = useGameStore((s) => s.maxFear);
  const selfieUrl = useGameStore((s) => s.selfieUrl);
  const clearError = useGameStore((s) => s.clearError);
  const clearMeta = useGameStore((s) => s.clearMeta);
  const reset = useGameStore((s) => s.reset);
  const setConnectionStatus = useGameStore((s) => s.setConnectionStatus);
  const setSessionId = useGameStore((s) => s.setSessionId);
  const processError = useGameStore((s) => s.processError);

  const ensureSession = useCallback(async () => {
    if (useGameStore.getState().sessionId) return;

    setConnectionStatus("connecting");
    clearError();

    try {
      const response = await fetch(API_URL, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
        },
        body: JSON.stringify({ player_name: null }),
      });
      if (!response.ok) {
        throw new Error(`Session bootstrap failed with ${response.status}`);
      }
      const data = (await response.json()) as { session_id?: string };
      if (!data.session_id) {
        throw new Error("Missing session_id in create_game response");
      }
      setSessionId(data.session_id);
    } catch (error) {
      processError({
        code: "SESSION_BOOTSTRAP_FAILED",
        message:
          error instanceof Error
            ? error.message
            : "Unable to create a game session.",
        recoverable: false,
      });
      setConnectionStatus("error");
    }
  }, [clearError, processError, setConnectionStatus, setSessionId]);

  useEffect(() => {
    if (!sessionId) {
      void ensureSession();
    }
  }, [ensureSession, sessionId]);

  useEffect(() => {
    if (!currentScene) return;
    setIntensity(currentScene.intensity);
    if (currentScene.sound_cue) {
      playCue(currentScene.sound_cue);
    }
  }, [currentScene, playCue, setIntensity]);

  useEffect(() => {
    if (!currentMeta) return;

    const originalTitle = document.title;
    if (currentMeta.target === "title") {
      document.title = currentMeta.text;
    }
    const cue = metaCue(currentMeta.target);
    if (cue) {
      playCue(cue);
    }

    const timer = window.setTimeout(() => {
      clearMeta();
      document.title = originalTitle;
    }, Math.max(currentMeta.delay_ms, 2200));

    return () => {
      clearTimeout(timer);
      document.title = originalTitle;
    };
  }, [clearMeta, currentMeta, playCue]);

  useEffect(() => {
    if (currentMeta?.target === "title") return;
    document.title = currentScene?.title
      ? `${currentScene.title} | Session Mirror`
      : "It Learns Your Fear";
  }, [currentMeta?.target, currentScene?.title]);

  const handleStart = useCallback(() => {
    void initAudio();
    send({ type: "start_game", payload: { player_name: null } });
  }, [initAudio, send]);

  const handleRestart = useCallback(() => {
    reset();
  }, [reset]);

  const isBadEnding = fearLevel >= maxFear;
  const isInGame = currentScene && currentScene.scene_id !== "welcome";
  const vignetteIntensity = useMemo(() => {
    const base = currentScene ? 0.42 + currentScene.intensity * 0.45 : 0.5;
    return currentMeta ? Math.min(base + 0.12, 1) : base;
  }, [currentMeta, currentScene]);
  const ambientIntensity = useMemo(() => {
    const base = currentScene ? 0.28 + currentScene.intensity * 0.4 : 0.35;
    return currentMeta?.target === "whisper" ? Math.min(base + 0.1, 1) : base;
  }, [currentMeta, currentScene]);
  const crtEnabled = Boolean(
    currentMeta?.target === "glitch_text" ||
      currentMeta?.target === "title" ||
      currentScene?.effects.some((effect) => effect.effect === "crt"),
  );

  let content;

  if (connectionStatus !== "connected" || !sessionId) {
    content = <LoadingScreen />;
  } else if (isBadEnding && isInGame) {
    // Bad ending — fear meter filled up!
    content = <BadEnding selfieUrl={selfieUrl} onRestart={handleRestart} />;
  } else if (gamePhase === "reveal" && revealData) {
    content = (
      <div>
        <FearReveal data={revealData} />
        <div className="flex justify-center pb-12">
          <button
            onClick={handleRestart}
            className="px-8 py-3 border border-bone/30 text-bone hover:text-parchment
                       hover:border-bone/60 transition-colors duration-300 font-body text-lg
                       cursor-pointer"
            data-testid="restart-button"
          >
            Play Again
          </button>
        </div>
      </div>
    );
  } else if (isInGame) {
    content = (
      <>
        <FearMeter value={fearLevel} max={maxFear} />
        <GameScreen
          currentScene={currentScene}
          sceneHistory={sceneHistory}
          image={currentImage}
          isLoading={currentScene.choices.length === 0}
          send={send}
        />
      </>
    );
  } else {
    content = <StartScreen onStart={handleStart} />;
  }

  return (
    <>
      <Vignette intensity={vignetteIntensity} />
      <CRTOverlay enabled={crtEnabled} />
      <AmbientDarkness intensity={ambientIntensity} />
      <SessionChrome
        connectionStatus={connectionStatus}
        sessionId={sessionId}
        currentScene={currentScene}
        currentMeta={currentMeta}
        currentError={currentError}
      />
      {currentError && (
        <div
          className="fixed top-4 left-1/2 -translate-x-1/2 z-[120] max-w-xl
                     border border-blood/40 bg-shadow/95 px-4 py-3 text-sm
                     font-body text-bone"
          data-testid="error-banner"
        >
          <span className="text-blood mr-2">[error]</span>
          {currentError.message}
        </div>
      )}
      {currentMeta && (
        <div
          className={`fixed z-[115] pointer-events-none flex justify-center px-6 ${
            currentMeta.target === "whisper"
              ? "inset-x-0 bottom-12"
              : currentMeta.target === "title"
                ? "inset-x-0 top-4"
                : currentMeta.target === "overlay"
                  ? "inset-0 items-center"
                  : "inset-x-0 top-20"
          }`}
          data-testid="meta-overlay"
        >
          <div
            className={`font-body text-sm ${
              currentMeta.target === "glitch_text"
                ? "max-w-2xl rounded-2xl border border-blood/35 bg-[#120606]/94 px-5 py-4 text-blood animate-glitch"
                : currentMeta.target === "whisper"
                  ? "max-w-xl rounded-2xl border border-bone/10 bg-black/78 px-5 py-4 text-smoke/75 italic shadow-[0_0_50px_rgba(0,0,0,0.45)]"
                  : currentMeta.target === "title"
                    ? "w-full max-w-4xl rounded-2xl border border-blood/25 bg-[#050608]/97 px-5 py-4 text-parchment uppercase tracking-[0.35em] shadow-[0_0_80px_rgba(139,0,0,0.18)]"
                    : currentMeta.target === "overlay"
                      ? "w-full max-w-3xl rounded-[28px] border border-blood/24 bg-[radial-gradient(circle_at_top,rgba(139,0,0,0.14),rgba(5,6,8,0.96))] px-6 py-6 text-bone/86 shadow-[0_0_120px_rgba(139,0,0,0.16)] backdrop-blur-xl"
                      : "max-w-2xl rounded-2xl border border-bone/15 bg-[#050608]/90 px-5 py-4 text-bone/80"
            }`}
          >
            {currentMeta.text}
          </div>
        </div>
      )}
      {content}
    </>
  );
}

function apiUrlFromWs(wsUrl: string) {
  const url = new URL(wsUrl);
  url.protocol = url.protocol === "wss:" ? "https:" : "http:";
  url.pathname = "/api/game";
  url.search = "";
  return url.toString();
}

function metaCue(target: "title" | "overlay" | "whisper" | "glitch_text") {
  switch (target) {
    case "title":
      return "dropout_hum";
    case "overlay":
      return "sub_boom";
    case "whisper":
      return "breath_near";
    case "glitch_text":
      return "feedback_burst";
    default:
      return null;
  }
}
