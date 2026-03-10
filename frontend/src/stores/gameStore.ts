import { create } from "zustand";
import type { ConnectionStatus, GamePhase } from "../types/game";
import type {
  ImagePayload,
  ErrorPayload,
  MetaPayload,
  NarrativePayload,
  PhaseChangePayload,
  RevealPayload,
} from "../types/ws";

export interface GameState {
  // Connection
  connectionStatus: ConnectionStatus;

  // Game
  sessionId: string | null;
  gamePhase: GamePhase | null;
  currentScene: NarrativePayload | null;
  sceneHistory: NarrativePayload[];
  currentImage: ImagePayload | null;
  currentMeta: MetaPayload | null;
  revealData: RevealPayload | null;
  currentError: ErrorPayload | null;

  // Fear meter
  fearLevel: number; // 0–100
  maxFear: number;

  // Player selfie
  selfieUrl: string | null;

  // Camera step done
  cameraStepDone: boolean;

  // Actions
  setConnectionStatus: (status: ConnectionStatus) => void;
  setSessionId: (id: string) => void;
  processNarrative: (msg: NarrativePayload) => void;
  processPhaseChange: (msg: PhaseChangePayload) => void;
  processMeta: (msg: MetaPayload) => void;
  clearMeta: () => void;
  processImage: (msg: ImagePayload) => void;
  processReveal: (msg: RevealPayload) => void;
  processError: (msg: ErrorPayload) => void;
  clearError: () => void;
  setSelfieUrl: (url: string | null) => void;
  setCameraStepDone: () => void;
  reset: () => void;
}

const initialState = {
  connectionStatus: "disconnected" as ConnectionStatus,
  sessionId: null as string | null,
  gamePhase: null as GamePhase | null,
  currentScene: null as NarrativePayload | null,
  sceneHistory: [] as NarrativePayload[],
  currentImage: null as ImagePayload | null,
  currentMeta: null as MetaPayload | null,
  revealData: null as RevealPayload | null,
  currentError: null as ErrorPayload | null,
  fearLevel: 0,
  maxFear: 100,
  selfieUrl: null as string | null,
  cameraStepDone: false,
};

export const useGameStore = create<GameState>((set) => ({
  ...initialState,

  setConnectionStatus: (status) => set({ connectionStatus: status }),

  setSessionId: (id) => set({ sessionId: id }),

  processNarrative: (msg) =>
    set((state) => {
      if (state.currentScene?.scene_id === msg.scene_id) {
        const sceneHistory =
          state.sceneHistory.length > 0 &&
          state.sceneHistory[state.sceneHistory.length - 1]?.scene_id ===
            msg.scene_id
            ? [...state.sceneHistory.slice(0, -1), msg]
            : [...state.sceneHistory, msg];

        return {
          currentScene: msg,
          sceneHistory,
          currentError: null,
        };
      }

      // Increase fear based on scene intensity.
      const fearIncrease = msg.intensity * 12;
      return {
        currentScene: msg,
        sceneHistory: [...state.sceneHistory, msg],
        currentImage:
          state.currentImage?.scene_id === msg.scene_id ? state.currentImage : null,
        fearLevel: Math.min(state.fearLevel + fearIncrease, state.maxFear),
        currentError: null,
      };
    }),

  processPhaseChange: (msg) => set({ gamePhase: msg.to, currentError: null }),

  processMeta: (msg) => set({ currentMeta: msg, currentError: null }),

  clearMeta: () => set({ currentMeta: null }),

  processImage: (msg) => set({ currentImage: msg, currentError: null }),

  processReveal: (msg) =>
    set({ revealData: msg, gamePhase: "reveal", currentError: null }),

  processError: (msg) => set({ currentError: msg }),

  clearError: () => set({ currentError: null }),

  setSelfieUrl: (url) => set({ selfieUrl: url }),

  setCameraStepDone: () => set({ cameraStepDone: true }),

  reset: () => set(initialState),
}));
