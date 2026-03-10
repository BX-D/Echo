import { useCallback, useEffect, useRef } from "react";
import { AudioEngine } from "../systems/AudioEngine";

/**
 * React hook that owns an {@link AudioEngine} instance, connecting it
 * to the game's intensity level and narrative sound cues.
 */
export function useAudio() {
  const engineRef = useRef<AudioEngine | null>(null);

  // Create engine once (lazy — context is created on first user gesture).
  if (!engineRef.current) {
    engineRef.current = new AudioEngine({ volume: 0.3 });
  }

  // Cleanup on unmount.
  useEffect(() => {
    return () => {
      engineRef.current?.dispose();
      engineRef.current = null;
    };
  }, []);

  /** Initialise + resume the audio context (call on a user gesture). */
  const initAudio = useCallback(async () => {
    const engine = engineRef.current;
    if (!engine) return;
    engine.init();
    await engine.resume();
    engine.startDrone();
    engine.startHeartbeat(60);
  }, []);

  /** Adjust audio parameters based on horror intensity (0–1). */
  const setIntensity = useCallback((intensity: number) => {
    const engine = engineRef.current;
    if (!engine || !engine.initialised) return;

    // Map intensity to heartbeat BPM: 60 (calm) → 160 (panic).
    const bpm = Math.round(60 + intensity * 100);
    engine.setHeartbeatBpm(bpm);
    engine.setThreatLevel(intensity);
  }, []);

  /** Play a narrative sound cue by name. */
  const playCue = useCallback((cueName: string) => {
    const engine = engineRef.current;
    if (!engine || !engine.initialised) return;
    engine.playCue(cueName);
  }, []);

  /** Toggle mute. */
  const toggleMute = useCallback(() => {
    engineRef.current?.toggleMute();
  }, []);

  return { initAudio, setIntensity, playCue, toggleMute };
}
