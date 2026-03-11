import { create } from "zustand";
import type { ConnectionStatus } from "../types/game";
import type {
  EndingPayload,
  ErrorPayload,
  ImagePayload,
  MetaPayload,
  SessionSurfaceMessagePayload,
} from "../types/ws";

export const SESSION_STORAGE_KEY = "echo_protocol_session_id";

export interface GameState {
  connectionStatus: ConnectionStatus;
  sessionId: string | null;
  currentSurface: SessionSurfaceMessagePayload | null;
  currentEnding: EndingPayload | null;
  currentImage: ImagePayload | null;
  currentMeta: MetaPayload | null;
  currentError: ErrorPayload | null;
  selfieUrl: string | null;
  setConnectionStatus: (status: ConnectionStatus) => void;
  setSessionId: (id: string | null) => void;
  processSessionSurface: (surface: SessionSurfaceMessagePayload) => void;
  processEnding: (ending: EndingPayload) => void;
  processMeta: (msg: MetaPayload) => void;
  clearMeta: () => void;
  processImage: (msg: ImagePayload) => void;
  processError: (msg: ErrorPayload) => void;
  clearError: () => void;
  setSelfieUrl: (url: string | null) => void;
  reset: () => void;
}

const initialState = {
  connectionStatus: "disconnected" as ConnectionStatus,
  sessionId: null as string | null,
  currentSurface: null as SessionSurfaceMessagePayload | null,
  currentEnding: null as EndingPayload | null,
  currentImage: null as ImagePayload | null,
  currentMeta: null as MetaPayload | null,
  currentError: null as ErrorPayload | null,
  selfieUrl: null as string | null,
};

export const useGameStore = create<GameState>((set) => ({
  ...initialState,

  setConnectionStatus: (status) => set({ connectionStatus: status }),

  setSessionId: (id) => {
    persistSessionId(id);
    set({ sessionId: id });
  },

  processSessionSurface: (surface) =>
    set((state) => ({
      currentSurface: surface,
      currentEnding: null,
      currentImage: surface.image_prompt ? state.currentImage : null,
      currentError: null,
    })),

  processEnding: (ending) =>
    set({
      currentEnding: ending,
      currentError: null,
    }),

  processMeta: (msg) => set({ currentMeta: msg, currentError: null }),

  clearMeta: () => set({ currentMeta: null }),

  processImage: (msg) => set({ currentImage: msg, currentError: null }),

  processError: (msg) => set({ currentError: msg }),

  clearError: () => set({ currentError: null }),

  setSelfieUrl: (url) => set({ selfieUrl: url }),

  reset: () => {
    persistSessionId(null);
    set(initialState);
  },
}));

function persistSessionId(id: string | null) {
  if (typeof window === "undefined") return;
  if (id) {
    window.localStorage.setItem(SESSION_STORAGE_KEY, id);
  } else {
    window.localStorage.removeItem(SESSION_STORAGE_KEY);
  }
}
