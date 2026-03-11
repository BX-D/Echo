import type { ConnectionStatus } from "../types/game";
import type { ErrorPayload, MetaPayload, SessionSurfaceMessagePayload } from "../types/ws";

export interface SessionChromeProps {
  connectionStatus: ConnectionStatus;
  sessionId: string | null;
  currentSurface: SessionSurfaceMessagePayload | null;
  currentMeta?: MetaPayload | null;
  currentError?: ErrorPayload | null;
}

export default function SessionChrome({
  connectionStatus,
  sessionId,
  currentSurface,
  currentMeta = null,
  currentError = null,
}: SessionChromeProps) {
  const timeLabel = new Date().toLocaleTimeString([], {
    hour: "2-digit",
    minute: "2-digit",
  });
  const statusLabel = currentError
    ? "error"
    : currentMeta
      ? "interruption"
      : connectionStatus === "connecting" && sessionId
        ? currentSurface
          ? "reconnecting"
          : "resuming"
        : connectionStatus;
  const accentClass = currentError
    ? "border-blood/25 bg-[#120606]/88"
    : currentMeta
      ? "border-blood/35 bg-[#090305]/92 shadow-[0_0_60px_rgba(139,0,0,0.14)]"
      : statusLabel === "reconnecting" || statusLabel === "resuming"
        ? "border-[#4e7e88]/25 bg-[#061015]/88"
        : "border-bone/10 bg-[#050608]/82";
  const title = currentSurface?.scene_title ?? currentSurface?.beat.title ?? "audit intake";
  const subtitle = currentSurface
    ? `${currentSurface.chapter.replace(/_/g, " ")} / ${currentSurface.status_line}`
    : "awaiting session start";

  return (
    <div
      className="fixed top-3 left-1/2 -translate-x-1/2 z-[105] w-[min(92vw,1180px)] pointer-events-none"
      data-testid="session-chrome"
    >
      <div className={`rounded-2xl border ${accentClass} backdrop-blur-xl px-4 py-3 shadow-[0_18px_40px_rgba(0,0,0,0.35)] transition-colors duration-500`}>
        <div className="flex items-center justify-between gap-4 text-[10px] uppercase tracking-[0.32em] font-body text-smoke/60">
          <div className="flex items-center gap-3 min-w-0">
            <span
              className={`h-2 w-2 rounded-full ${
                statusLabel === "connected"
                  ? "bg-bone/80"
                  : statusLabel === "connecting" || statusLabel === "reconnecting" || statusLabel === "resuming"
                    ? "bg-rust animate-pulse"
                    : statusLabel === "interruption"
                      ? "bg-blood animate-pulse"
                    : "bg-blood/70"
              }`}
            />
            <span>Echo Protocol</span>
            <span className="text-bone/20">/</span>
            <span className="truncate max-w-[34ch]">{title}</span>
          </div>
          <div className="hidden md:flex items-center gap-4">
            <span>{statusLabel}</span>
            <span>{subtitle}</span>
            <span>{timeLabel}</span>
            <span>{sessionId ? sessionId.slice(0, 8) : "pending"}</span>
          </div>
        </div>
        {(currentMeta || currentError) && (
          <div className="mt-3 border-t border-bone/10 pt-3">
            <p className={`text-[10px] uppercase tracking-[0.32em] font-body ${
              currentError ? "text-blood/75" : "text-blood/72"
            }`}>
              {currentError ? currentError.code : "live interruption"}
            </p>
            <p className="mt-1 text-sm text-bone/70 font-body leading-relaxed">
              {currentError?.message ?? currentMeta?.text}
            </p>
          </div>
        )}
      </div>
    </div>
  );
}
