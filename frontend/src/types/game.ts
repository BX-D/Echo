/** Legacy phase model retained for compatibility with the original storage schema. */
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
