/**
 * Procedural horror audio engine built on the Web Audio API.
 *
 * Generates ambient drones, heartbeat rhythms, and triggered sound cues
 * entirely in code — no audio file dependencies.
 */

export interface AudioEngineOptions {
  /** Master volume (0–1). */
  volume?: number;
  /** Start muted. */
  muted?: boolean;
}

export class AudioEngine {
  private ctx: AudioContext | null = null;
  private masterGain: GainNode | null = null;
  private droneOsc: OscillatorNode | null = null;
  private droneGain: GainNode | null = null;
  private heartbeatInterval: ReturnType<typeof setInterval> | null = null;
  private _muted: boolean;
  private _volume: number;
  private _heartbeatBpm = 60;
  private _disposed = false;

  constructor(opts: AudioEngineOptions = {}) {
    this._volume = opts.volume ?? 0.3;
    this._muted = opts.muted ?? false;
  }

  /** Whether the engine has been initialised (context created). */
  get initialised(): boolean {
    return this.ctx !== null && !this._disposed;
  }

  get muted(): boolean {
    return this._muted;
  }

  get volume(): number {
    return this._volume;
  }

  get heartbeatBpm(): number {
    return this._heartbeatBpm;
  }

  /**
   * Initialises the AudioContext. Must be called after a user gesture
   * (click/keypress) to satisfy browser autoplay policies.
   */
  init(): void {
    if (this.ctx || this._disposed) return;
    this.ctx = new AudioContext();
    this.masterGain = this.ctx.createGain();
    this.masterGain.gain.value = this._muted ? 0 : this._volume;
    this.masterGain.connect(this.ctx.destination);
  }

  /**
   * Resumes the AudioContext if it was suspended by the browser.
   */
  async resume(): Promise<void> {
    if (this.ctx?.state === "suspended") {
      await this.ctx.resume();
    }
  }

  // ── Ambient drone ──────────────────────────────────────────────────────

  /**
   * Starts a low-frequency ambient drone.
   * @param frequency Base frequency in Hz (default 55 — low A).
   */
  startDrone(frequency = 55): void {
    if (!this.ctx || !this.masterGain) return;
    this.stopDrone();

    this.droneGain = this.ctx.createGain();
    this.droneGain.gain.value = 0.15;
    this.droneGain.connect(this.masterGain);

    this.droneOsc = this.ctx.createOscillator();
    this.droneOsc.type = "sawtooth";
    this.droneOsc.frequency.value = frequency;

    // Low-pass filter for a muffled, ominous tone.
    const filter = this.ctx.createBiquadFilter();
    filter.type = "lowpass";
    filter.frequency.value = 200;
    filter.Q.value = 1;

    this.droneOsc.connect(filter);
    filter.connect(this.droneGain);
    this.droneOsc.start();
  }

  /** Adjusts the ambient profile as the session becomes more aggressive. */
  setThreatLevel(intensity: number): void {
    if (!this.ctx || !this.masterGain) return;

    const normalized = Math.max(0, Math.min(1, intensity));
    if (this.droneGain) {
      this.droneGain.gain.value = 0.12 + normalized * 0.18;
    }
    if (this.droneOsc) {
      this.droneOsc.frequency.value = 45 + normalized * 28;
    }
    this.masterGain.gain.value = this._muted ? 0 : Math.min(0.8, this._volume + normalized * 0.08);
  }

  /** Stops the ambient drone. */
  stopDrone(): void {
    this.droneOsc?.stop();
    this.droneOsc?.disconnect();
    this.droneGain?.disconnect();
    this.droneOsc = null;
    this.droneGain = null;
  }

  // ── Heartbeat ──────────────────────────────────────────────────────────

  /**
   * Starts a procedural heartbeat at the given BPM.
   * @param bpm Beats per minute (60 = resting, 120 = anxious, 160 = panic).
   */
  startHeartbeat(bpm = 60): void {
    if (!this.ctx || !this.masterGain) return;
    this.stopHeartbeat();
    this._heartbeatBpm = bpm;

    const intervalMs = (60 / bpm) * 1000;
    this.heartbeatInterval = setInterval(() => {
      this.playBeat();
    }, intervalMs);
    // Play first beat immediately.
    this.playBeat();
  }

  /** Adjusts heartbeat BPM without restarting. */
  setHeartbeatBpm(bpm: number): void {
    if (this.heartbeatInterval !== null) {
      this.stopHeartbeat();
      this.startHeartbeat(bpm);
    } else {
      this._heartbeatBpm = bpm;
    }
  }

  /** Stops the heartbeat. */
  stopHeartbeat(): void {
    if (this.heartbeatInterval !== null) {
      clearInterval(this.heartbeatInterval);
      this.heartbeatInterval = null;
    }
  }

  private playBeat(): void {
    if (!this.ctx || !this.masterGain) return;

    // Double-thump: two quick tones.
    const now = this.ctx.currentTime;
    for (const offset of [0, 0.08]) {
      const osc = this.ctx.createOscillator();
      const gain = this.ctx.createGain();
      osc.type = "sine";
      osc.frequency.value = offset === 0 ? 50 : 40;
      gain.gain.setValueAtTime(0.3, now + offset);
      gain.gain.exponentialRampToValueAtTime(0.001, now + offset + 0.15);
      osc.connect(gain);
      gain.connect(this.masterGain);
      osc.start(now + offset);
      osc.stop(now + offset + 0.2);
    }
  }

  // ── Sound cues ─────────────────────────────────────────────────────────

  /**
   * Triggers a one-shot sound cue.
   *
   * Cues are generated procedurally based on the `cueName`. Unknown
   * cues play a generic low rumble.
   */
  playCue(cueName: string): void {
    if (!this.ctx || !this.masterGain) return;

    const params = CUE_PARAMS[cueName] ?? CUE_PARAMS["default"]!;
    const now = this.ctx.currentTime;

    const osc = this.ctx.createOscillator();
    const gain = this.ctx.createGain();

    osc.type = params.type;
    osc.frequency.value = params.freq;
    gain.gain.setValueAtTime(params.volume, now);
    gain.gain.exponentialRampToValueAtTime(0.001, now + params.duration);

    if (params.pan !== 0) {
      const panner = this.ctx.createStereoPanner();
      panner.pan.value = params.pan;
      osc.connect(gain);
      gain.connect(panner);
      panner.connect(this.masterGain);
    } else {
      osc.connect(gain);
      gain.connect(this.masterGain);
    }

    osc.start(now);
    osc.stop(now + params.duration + 0.05);
  }

  // ── Volume / mute ─────────────────────────────────────────────────────

  /** Sets the master volume (0–1). */
  setVolume(v: number): void {
    this._volume = Math.max(0, Math.min(1, v));
    if (this.masterGain && !this._muted) {
      this.masterGain.gain.value = this._volume;
    }
  }

  /** Toggles mute on/off. */
  toggleMute(): void {
    this._muted = !this._muted;
    if (this.masterGain) {
      this.masterGain.gain.value = this._muted ? 0 : this._volume;
    }
  }

  /** Sets muted state directly. */
  setMuted(m: boolean): void {
    this._muted = m;
    if (this.masterGain) {
      this.masterGain.gain.value = m ? 0 : this._volume;
    }
  }

  // ── Cleanup ────────────────────────────────────────────────────────────

  /** Stops all audio and releases resources. */
  dispose(): void {
    this._disposed = true;
    this.stopDrone();
    this.stopHeartbeat();
    this.masterGain?.disconnect();
    this.ctx?.close().catch(() => {});
    this.ctx = null;
    this.masterGain = null;
  }
}

// ── Cue parameter table ──────────────────────────────────────────────────

interface CueParams {
  type: OscillatorType;
  freq: number;
  duration: number;
  volume: number;
  pan: number; // -1 = left, 0 = centre, 1 = right
}

const CUE_PARAMS: Record<string, CueParams> = {
  default: { type: "sine", freq: 40, duration: 0.5, volume: 0.2, pan: 0 },
  dripping_water: { type: "sine", freq: 800, duration: 0.1, volume: 0.15, pan: 0.3 },
  door_creak: { type: "sawtooth", freq: 120, duration: 0.8, volume: 0.25, pan: -0.4 },
  footsteps: { type: "triangle", freq: 100, duration: 0.15, volume: 0.2, pan: 0.5 },
  whisper: { type: "sawtooth", freq: 300, duration: 1.2, volume: 0.08, pan: -0.8 },
  static_hum: { type: "sawtooth", freq: 60, duration: 2.0, volume: 0.1, pan: 0 },
  lock_click: { type: "square", freq: 2000, duration: 0.05, volume: 0.3, pan: 0 },
  scraping_floor: { type: "sawtooth", freq: 80, duration: 1.5, volume: 0.15, pan: 0.6 },
  distorted_lullaby: { type: "triangle", freq: 440, duration: 2.0, volume: 0.1, pan: -0.3 },
  muffled_phone_ring: { type: "square", freq: 500, duration: 0.3, volume: 0.2, pan: 0.2 },
  car_engine_distant: { type: "sawtooth", freq: 45, duration: 3.0, volume: 0.08, pan: 0.7 },
  sub_boom: { type: "sine", freq: 28, duration: 1.4, volume: 0.32, pan: 0 },
  feedback_burst: { type: "square", freq: 1700, duration: 0.22, volume: 0.24, pan: -0.2 },
  metal_scrape: { type: "sawtooth", freq: 95, duration: 1.8, volume: 0.2, pan: 0.45 },
  breath_near: { type: "triangle", freq: 180, duration: 1.4, volume: 0.1, pan: -0.75 },
  dropout_hum: { type: "sine", freq: 42, duration: 2.4, volume: 0.18, pan: 0 },
  false_notification_click: { type: "square", freq: 1200, duration: 0.07, volume: 0.22, pan: 0.55 },
};
