/** Scene emotional tone, drives audio and visual effects. */
export type Atmosphere =
  | "dread"
  | "tension"
  | "panic"
  | "calm"
  | "wrongness"
  | "isolation"
  | "paranoia";

/** The 10 psychological fear axes. */
export type FearType =
  | "claustrophobia"
  | "isolation"
  | "body_horror"
  | "stalking"
  | "loss_of_control"
  | "uncanny_valley"
  | "darkness"
  | "sound_based"
  | "doppelganger"
  | "abandonment";

/** How the player approaches a decision. */
export type ChoiceApproach =
  | "investigate"
  | "avoid"
  | "confront"
  | "flee"
  | "interact"
  | "wait";

/** A single player-facing choice within a scene. */
export interface Choice {
  id: string;
  text: string;
  approach: ChoiceApproach;
  fear_vector: FearType;
}

/** Frontend effect type. */
export type EffectType =
  | "shake"
  | "flicker"
  | "glitch"
  | "darkness"
  | "flashlight"
  | "crt"
  | "slow_type"
  | "fast_type"
  | "strobe_flash"
  | "chromatic_shift"
  | "focus_pulse"
  | "frame_jump";

/** An instruction for the frontend to play a visual/audio effect. */
export interface EffectDirective {
  effect: EffectType;
  intensity: number;
  duration_ms: number;
  delay_ms: number;
}

/** Where a meta-horror break is rendered in the UI. */
export type MetaTarget = "title" | "overlay" | "whisper" | "glitch_text";

/** A fourth-wall-breaking moment. */
export interface MetaBreak {
  text: string;
  target: MetaTarget;
}

/** How an AI-generated image should be displayed. */
export type DisplayMode = "fade_in" | "glitch" | "flash";

export type SessionAct =
  | "invitation"
  | "calibration"
  | "accommodation"
  | "contamination"
  | "performance_collapse"
  | "verdict";

export type SurfaceMedium =
  | "chat"
  | "questionnaire"
  | "archive"
  | "transcript"
  | "webcam"
  | "microphone"
  | "system_dialog"
  | "mirror";

export type TrustPosture =
  | "helpful"
  | "curious"
  | "clinical"
  | "manipulative"
  | "confessional"
  | "hostile";

/** Single fear axis score with confidence. */
export interface FearScore {
  fear_type: FearType;
  score: number;
  confidence: number;
}

/** End-of-game fear profile summary. */
export interface FearProfileSummary {
  scores: FearScore[];
  primary_fear: FearType;
  secondary_fear: FearType | null;
  total_observations: number;
}

/** A notable moment during gameplay. */
export interface KeyMoment {
  scene_id: string;
  description: string;
  fear_revealed: FearType;
  behavior_trigger: string;
}

/** Record of one adaptation the engine made. */
export interface AdaptationRecord {
  scene_id: string;
  strategy: string;
  fear_targeted: FearType;
  intensity: number;
}

/** AI-generated analysis of the player's real session data. */
export interface RevealAnalysis {
  summary: string;
  key_patterns: string[];
  adaptation_summary: string;
  closing_message: string;
}

export interface MediumExposure {
  medium: SurfaceMedium;
  count: number;
}

export interface SessionSummary {
  duration_seconds: number;
  total_beats: number;
  focus_interruptions: number;
  camera_permission_granted: boolean | null;
  microphone_permission_granted: boolean | null;
  contradiction_count: number;
  media_exposures: MediumExposure[];
  completion_reason: string;
}

export interface BehaviorProfileSummary {
  compliance: number;
  resistance: number;
  curiosity: number;
  avoidance: number;
  self_editing: number;
  need_for_certainty: number;
  ritualized_control: number;
  recovery_after_escalation: number;
  tolerance_after_violation: number;
}

export type EndingClassification =
  | "compliant_witness"
  | "resistant_subject"
  | "curious_accomplice"
  | "fractured_mirror"
  | "quiet_exit";
