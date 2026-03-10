import { describe, it, vi, beforeEach, afterEach } from "vitest";
import { renderHook, act } from "@testing-library/react";
import { useAudio } from "./useAudio";

// Minimal AudioContext mock.
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
}
class MockStereoPannerNode {
  pan = { value: 0 };
  connect = vi.fn().mockReturnThis();
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
  vi.stubGlobal("AudioContext", MockAudioContext);
});

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("useAudio", () => {
  it("connects to game state via initAudio", async () => {
    const { result } = renderHook(() => useAudio());
    await act(async () => {
      await result.current.initAudio();
    });
    // Should not throw — engine is initialised.
  });

  it("adjusts audio based on intensity", async () => {
    const { result } = renderHook(() => useAudio());
    await act(async () => {
      await result.current.initAudio();
    });
    act(() => {
      result.current.setIntensity(0.8);
    });
    // Should not throw.
  });

  it("plays sound cues from narrative messages", async () => {
    const { result } = renderHook(() => useAudio());
    await act(async () => {
      await result.current.initAudio();
    });
    act(() => {
      result.current.playCue("door_creak");
      result.current.playCue("whisper");
    });
    // Should not throw.
  });

  it("toggle mute does not crash", async () => {
    const { result } = renderHook(() => useAudio());
    await act(async () => {
      await result.current.initAudio();
    });
    act(() => {
      result.current.toggleMute();
      result.current.toggleMute();
    });
  });

  it("disposes on unmount", async () => {
    const { result, unmount } = renderHook(() => useAudio());
    await act(async () => {
      await result.current.initAudio();
    });
    unmount();
    // Should clean up without errors.
  });
});
