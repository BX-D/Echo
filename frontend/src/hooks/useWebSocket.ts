import { useCallback, useEffect, useRef } from "react";
import { useGameStore } from "../stores/gameStore";
import type { ClientMessage, ServerMessage } from "../types/ws";

const MAX_RECONNECT_DELAY_MS = 30_000;
const HEARTBEAT_INTERVAL_MS = 30_000;

/**
 * Manages a WebSocket connection with auto-reconnect and message routing.
 *
 * Connects on mount, reconnects with exponential backoff on close, and
 * dispatches incoming {@link ServerMessage}s to the Zustand game store.
 */
export function useWebSocket(url: string, sessionId: string | null) {
  const wsRef = useRef<WebSocket | null>(null);
  const reconnectTimeout = useRef<ReturnType<typeof setTimeout> | null>(null);
  const heartbeatInterval = useRef<ReturnType<typeof setInterval> | null>(null);
  const attemptRef = useRef(0);
  const mountedRef = useRef(true);

  const send = useCallback((msg: ClientMessage) => {
    if (wsRef.current?.readyState === WebSocket.OPEN) {
      wsRef.current.send(JSON.stringify(msg));
    }
  }, []);

  useEffect(() => {
    mountedRef.current = true;
    const activeSessionId = sessionId;
    if (!activeSessionId) {
      return () => {
        mountedRef.current = false;
        if (reconnectTimeout.current !== null) {
          clearTimeout(reconnectTimeout.current);
        }
        if (heartbeatInterval.current !== null) {
          clearInterval(heartbeatInterval.current);
          heartbeatInterval.current = null;
        }
        wsRef.current?.close();
      };
    }
    const sessionIdForEffect: string = activeSessionId;

    function connect() {
      if (!mountedRef.current) return;

      useGameStore.getState().setConnectionStatus("connecting");

      const wsUrl = new URL(url);
      wsUrl.searchParams.set("session_id", sessionIdForEffect);
      const ws = new WebSocket(wsUrl.toString());
      wsRef.current = ws;

      ws.onopen = () => {
        if (!mountedRef.current) return;
        useGameStore.getState().clearError();
        useGameStore.getState().setConnectionStatus("connected");
        attemptRef.current = 0;
        startHeartbeat();
      };

      ws.onmessage = (event: MessageEvent) => {
        try {
          const msg = JSON.parse(event.data as string) as ServerMessage;
          routeMessage(msg);
        } catch {
          // ignore unparseable frames
        }
      };

      ws.onclose = () => {
        if (!mountedRef.current) return;
        useGameStore.getState().setConnectionStatus("disconnected");
        stopHeartbeat();
        scheduleReconnect();
      };

      ws.onerror = () => {
        if (!mountedRef.current) return;
        useGameStore.getState().setConnectionStatus("error");
      };
    }

    function scheduleReconnect() {
      if (!mountedRef.current) return;
      const delay = Math.min(
        1000 * Math.pow(2, attemptRef.current),
        MAX_RECONNECT_DELAY_MS,
      );
      attemptRef.current += 1;
      reconnectTimeout.current = setTimeout(connect, delay);
    }

    function startHeartbeat() {
      stopHeartbeat();
      heartbeatInterval.current = setInterval(() => {
        if (
          !wsRef.current ||
          wsRef.current.readyState !== WebSocket.OPEN
        ) {
          stopHeartbeat();
          wsRef.current?.close();
        }
      }, HEARTBEAT_INTERVAL_MS);
    }

    function stopHeartbeat() {
      if (heartbeatInterval.current !== null) {
        clearInterval(heartbeatInterval.current);
        heartbeatInterval.current = null;
      }
    }

    connect();

    return () => {
      mountedRef.current = false;
      if (reconnectTimeout.current !== null) {
        clearTimeout(reconnectTimeout.current);
      }
      stopHeartbeat();
      wsRef.current?.close();
    };
  }, [sessionId, url]);

  return { send };
}

/** Dispatches a parsed server message to the Zustand store. */
function routeMessage(msg: ServerMessage) {
  const store = useGameStore.getState();
  switch (msg.type) {
    case "session_surface":
      store.processSessionSurface(msg.payload.surface);
      break;
    case "ending":
      store.processEnding(msg.payload.ending);
      break;
    case "narrative":
      break;
    case "phase_change":
      break;
    case "meta":
      store.processMeta(msg.payload);
      break;
    case "image":
      store.processImage(msg.payload);
      break;
    case "reveal":
      break;
    case "error":
      if (
        msg.payload.code === "SESSION_RESUME_FAILED" ||
        msg.payload.code === "PROFILE_RESUME_FAILED"
      ) {
        store.setSessionId(null);
      }
      store.processError(msg.payload);
      break;
  }
}
