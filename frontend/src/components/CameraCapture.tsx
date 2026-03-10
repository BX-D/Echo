import { useCallback, useRef, useState } from "react";

export interface CameraCaptureProps {
  /** Called with the data URL of the captured photo, or null if skipped. */
  onCapture: (dataUrl: string | null) => void;
}

/**
 * Asks the player if they want to take a selfie for "personalised horror".
 * The photo is used to generate scarier images later.
 */
export default function CameraCapture({ onCapture }: CameraCaptureProps) {
  const videoRef = useRef<HTMLVideoElement>(null);
  const [streaming, setStreaming] = useState(false);
  const [captured, setCaptured] = useState<string | null>(null);

  const startCamera = useCallback(async () => {
    try {
      const stream = await navigator.mediaDevices.getUserMedia({
        video: { facingMode: "user", width: 640, height: 480 },
      });
      if (videoRef.current) {
        videoRef.current.srcObject = stream;
        setStreaming(true);
      }
    } catch {
      // Camera denied — skip silently.
      onCapture(null);
    }
  }, [onCapture]);

  const takePhoto = useCallback(() => {
    if (!videoRef.current) return;
    const canvas = document.createElement("canvas");
    canvas.width = 640;
    canvas.height = 480;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;
    ctx.drawImage(videoRef.current, 0, 0);
    const dataUrl = canvas.toDataURL("image/jpeg", 0.7);
    setCaptured(dataUrl);

    // Stop camera stream.
    const stream = videoRef.current.srcObject as MediaStream;
    stream?.getTracks().forEach((t) => t.stop());
    setStreaming(false);
  }, []);

  const confirm = useCallback(() => {
    onCapture(captured);
  }, [captured, onCapture]);

  const skip = useCallback(() => {
    if (videoRef.current) {
      const stream = videoRef.current.srcObject as MediaStream;
      stream?.getTracks().forEach((t) => t.stop());
    }
    onCapture(null);
  }, [onCapture]);

  return (
    <div className="flex flex-col items-center justify-center min-h-screen bg-void px-6">
      <h2 className="text-2xl font-horror text-bone mb-2">One More Thing...</h2>
      <p className="text-smoke/70 font-body text-sm mb-8 text-center max-w-md">
        For a more... <span className="text-blood italic">personalised</span> experience,
        the AI would like to see your face.
      </p>

      {!captured ? (
        <>
          <div className="w-64 h-48 bg-shadow rounded overflow-hidden mb-6 border border-ash/30">
            <video
              ref={videoRef}
              autoPlay
              playsInline
              muted
              className={`w-full h-full object-cover ${streaming ? "" : "hidden"}`}
            />
            {!streaming && (
              <div className="w-full h-full flex items-center justify-center">
                <span className="text-smoke/30 text-4xl">📷</span>
              </div>
            )}
          </div>

          <div className="flex gap-4">
            {!streaming ? (
              <button
                onClick={startCamera}
                className="px-6 py-2 border border-bone/30 text-bone hover:bg-shadow
                           transition-colors font-body cursor-pointer"
              >
                Open Camera
              </button>
            ) : (
              <button
                onClick={takePhoto}
                className="px-6 py-2 border border-blood/50 text-blood hover:text-parchment
                           transition-colors font-body cursor-pointer"
              >
                Take Photo
              </button>
            )}
            <button
              onClick={skip}
              className="px-6 py-2 text-smoke/40 hover:text-smoke transition-colors
                         font-body cursor-pointer"
            >
              Skip
            </button>
          </div>
        </>
      ) : (
        <>
          <div className="w-64 h-48 bg-shadow rounded overflow-hidden mb-6 border border-ash/30">
            <img src={captured} alt="Your photo" className="w-full h-full object-cover" />
          </div>
          <div className="flex gap-4">
            <button
              onClick={confirm}
              className="px-6 py-2 border border-blood/50 text-blood hover:text-parchment
                         transition-colors font-body cursor-pointer"
            >
              Use This Photo
            </button>
            <button
              onClick={skip}
              className="px-6 py-2 text-smoke/40 hover:text-smoke transition-colors
                         font-body cursor-pointer"
            >
              Skip
            </button>
          </div>
        </>
      )}

      <p className="text-smoke/20 font-body text-xs mt-8 text-center max-w-sm">
        Your photo stays on your device. It's only used to make the horror
        more personal. We never upload or store it.
      </p>
    </div>
  );
}
