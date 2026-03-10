/** The five sequential phases of a Fear Engine game session. */
export type GamePhase =
  | "calibrating"
  | "exploring"
  | "escalating"
  | "climax"
  | "reveal";

/** WebSocket connection lifecycle states. */
export type ConnectionStatus =
  | "connecting"
  | "connected"
  | "disconnected"
  | "error";
