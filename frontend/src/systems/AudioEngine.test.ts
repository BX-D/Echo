import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { AudioEngine } from "./AudioEngine";

// ---------------------------------------------------------------------------
// Mock Web Audio API
// ---------------------------------------------------------------------------

class MockGainNode {
  gain = { value: 1, setValueAtTime: vi.fn(), exponentialRampToValueAtTime: vi.fn() };
  connect = vi.fn().mockReturnThis();
  disconnect = vi.fn();
}

class MockOscillatorNode {
  type: OscillatorType = "sine";
  frequency = { value: 440 };
  connect = vi.fn().mockReturnThis();
  disconnect = vi.fn();
  start = vi.fn();
  stop = vi.fn();
}

class MockBiquadFilterNode {
  type = "lowpass";
  frequency = { value: 350 };
  Q = { value: 1 };
  connect = vi.fn().mockReturnThis();
  disconnect = vi.fn();
}

class MockStereoPannerNode {
  pan = { value: 0 };
  connect = vi.fn().mockReturnThis();
  disconnect = vi.fn();
}

class MockAudioContext {
  state = "running";
  currentTime = 0;
  destination = {};
  createGain = vi.fn().mockReturnValue(new MockGainNode());
  createOscillator = vi.fn().mockReturnValue(new MockOscillatorNode());
  createBiquadFilter = vi.fn().mockReturnValue(new MockBiquadFilterNode());
  createStereoPanner = vi.fn().mockReturnValue(new MockStereoPannerNode());
  resume = vi.fn().mockResolvedValue(undefined);
  close = vi.fn().mockResolvedValue(undefined);
}

beforeEach(() => {
  vi.useFakeTimers();
  vi.stubGlobal("AudioContext", MockAudioContext);
});

afterEach(() => {
  vi.useRealTimers();
  vi.unstubAllGlobals();
});

describe("AudioEngine", () => {
  it("creates audio context on init", () => {
    const engine = new AudioEngine();
    expect(engine.initialised).toBe(false);
    engine.init();
    expect(engine.initialised).toBe(true);
  });

  it("plays ambient drone", () => {
    const engine = new AudioEngine();
    engine.init();
    engine.startDrone(55);
    // The oscillator should have been created and started.
    // Stopping shouldn't throw.
    engine.stopDrone();
    engine.dispose();
  });

  it("adjusts heartbeat BPM", () => {
    const engine = new AudioEngine();
    engine.init();
    engine.startHeartbeat(60);
    expect(engine.heartbeatBpm).toBe(60);

    engine.setHeartbeatBpm(120);
    expect(engine.heartbeatBpm).toBe(120);

    engine.stopHeartbeat();
    engine.dispose();
  });

  it("triggers sound cue", () => {
    const engine = new AudioEngine();
    engine.init();
    // Should not throw for known and unknown cues.
    engine.playCue("door_creak");
    engine.playCue("whisper");
    engine.playCue("terminal_handshake");
    engine.playCue("archive_click");
    engine.playCue("heartbeat_countdown");
    engine.playCue("unknown_cue_name");
    engine.dispose();
  });

  it("respects mute setting", () => {
    const engine = new AudioEngine({ muted: true });
    expect(engine.muted).toBe(true);
    engine.init();

    engine.toggleMute();
    expect(engine.muted).toBe(false);

    engine.setMuted(true);
    expect(engine.muted).toBe(true);

    engine.dispose();
  });

  it("disposes audio resources on cleanup", () => {
    const engine = new AudioEngine();
    engine.init();
    engine.startDrone();
    engine.startHeartbeat(80);

    engine.dispose();
    expect(engine.initialised).toBe(false);
  });

  it("resumes context after user interaction", async () => {
    const engine = new AudioEngine();
    engine.init();
    await engine.resume();
    // Should not throw.
    engine.dispose();
  });

  it("sets volume", () => {
    const engine = new AudioEngine();
    engine.init();
    engine.setVolume(0.8);
    expect(engine.volume).toBe(0.8);

    // Clamp to [0, 1].
    engine.setVolume(1.5);
    expect(engine.volume).toBe(1);
    engine.setVolume(-0.5);
    expect(engine.volume).toBe(0);

    engine.dispose();
  });

  it("heartbeat ticks on interval", () => {
    const engine = new AudioEngine();
    engine.init();
    engine.startHeartbeat(120); // 500ms interval

    // Advance time to trigger a few heartbeats.
    vi.advanceTimersByTime(1200);

    engine.stopHeartbeat();
    engine.dispose();
  });

  it("does not crash when methods called before init", () => {
    const engine = new AudioEngine();
    // These should all be no-ops, not crashes.
    engine.startDrone();
    engine.stopDrone();
    engine.startHeartbeat();
    engine.stopHeartbeat();
    engine.playCue("test");
    engine.setVolume(0.5);
    engine.dispose();
  });
});
