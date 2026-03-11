export default function LoadingScreen() {
  return (
    <div className="min-h-screen bg-void intelligence-grid flex items-center justify-center px-6">
      <div className="w-full max-w-xl rounded-[28px] border border-bone/10 bg-[#050608]/88 backdrop-blur-xl px-8 py-10 text-center shadow-[0_0_0_1px_rgba(255,255,255,0.03),0_30px_100px_rgba(0,0,0,0.55)]">
        <p className="text-[11px] uppercase tracking-[0.4em] text-blood/70 mb-6">
          Echo Protocol / Handshake
        </p>
        <div className="relative mb-8 flex justify-center">
          <div className="w-3 h-3 rounded-full bg-bone animate-pulse" />
          <div className="absolute inset-0 mx-auto w-3 h-3 rounded-full bg-bone/30 animate-ping" />
        </div>
        <p className="text-smoke font-body text-sm tracking-[0.35em] uppercase animate-pulse">
          Connecting...
        </p>
        <p className="mt-4 text-sm text-bone/55 font-body leading-relaxed">
          Establishing a quiet channel. The system is preparing an environment calibrated to your pace.
        </p>
      </div>
    </div>
  );
}
