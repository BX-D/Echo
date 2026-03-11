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

export type StoryChapter =
  | "onboarding"
  | "cracks"
  | "ghost"
  | "hunt"
  | "protocol"
  | "ending";

export type EchoMode = "normal" | "anomalous" | "keira" | "hostile";

export type StoryEnding =
  | "shutdown"
  | "whistleblower"
  | "merge"
  | "collapse"
  | "awakening";

export type InputMode = "choice_only" | "freeform" | "hybrid";

export type AlertLevel = "info" | "warning" | "critical";

export type InvestigationKind =
  | "document"
  | "report"
  | "email"
  | "transcript"
  | "log"
  | "fragment";

export type TranscriptRole = "system" | "player" | "echo" | "company";

export type ChoiceStyle = "primary" | "secondary" | "danger";

export type SceneMode =
  | "prologue"
  | "login"
  | "workspace"
  | "document"
  | "chat"
  | "transition"
  | "countdown"
  | "raw_terminal"
  | "ending";

export type ScriptBlockKind =
  | "env"
  | "narration"
  | "player"
  | "echo"
  | "system"
  | "raw_terminal";

export interface ScriptCondition {
  id: string;
  raw: string;
  satisfied: boolean;
}

export interface ScriptBlock {
  id: string;
  kind: ScriptBlockKind;
  speaker: string | null;
  title: string | null;
  text: string;
  code_block: boolean;
  condition: ScriptCondition | null;
}

export interface ConversationGuide {
  id: string;
  chapter_label: string;
  prompt: string;
  exchange_target: number;
  restricted_after: number | null;
}

export interface BeatDefinition {
  id: string;
  chapter: StoryChapter;
  title: string;
  input_mode: InputMode;
  freeform_topics: string[];
  forced_clue_queue: string[];
  reconverge_beat_id: string | null;
  fallback_reply: string;
}

export interface TranscriptEntry {
  id: string;
  sequence: number;
  role: TranscriptRole;
  speaker: string;
  text: string;
  glitch: boolean;
}

export interface InvestigationItem {
  id: string;
  panel: string;
  title: string;
  kind: InvestigationKind;
  excerpt: string;
  body: string;
  unlocked: boolean;
  unread: boolean;
  tags: string[];
}

export interface SystemAlert {
  id: string;
  level: AlertLevel;
  text: string;
}

export interface InlineChoice {
  id: string;
  label: string;
  style: ChoiceStyle;
  approach: ChoiceApproach;
  disabled: boolean;
}

export interface ScriptChoiceOption {
  id: string;
  label: string;
  player_text?: string | null;
  effects_summary: string[];
  next_scene_id: string | null;
  ending: StoryEnding | null;
  disabled: boolean;
}

export interface ScriptChoicePrompt {
  id: string;
  prompt: string;
  options: ScriptChoiceOption[];
  allow_single_select: boolean;
}

export interface FlashEvent {
  id: string;
  text: string;
  render_mode: string;
  duration_ms: number;
}

export interface HiddenClueState {
  discovered_ids: string[];
  rendered_flash_ids: string[];
}

export interface TransitionState {
  label: string;
  auto_advance: boolean;
}

export interface SessionSurfacePayload {
  session_id: string;
  case_title: string;
  scene_id: string;
  chapter: StoryChapter;
  scene_title: string;
  scene_mode: SceneMode;
  blocks: ScriptBlock[];
  documents: InvestigationItem[];
  scene_choices: ScriptChoicePrompt[];
  active_conversation_guide: ConversationGuide | null;
  flash_events: FlashEvent[];
  transition_state: TransitionState | null;
  hidden_clue_state: HiddenClueState;
  ending_override: StoryEnding | null;
  beat: BeatDefinition;
  status_line: string;
  input_enabled: boolean;
  input_placeholder: string;
  transcript: TranscriptEntry[];
  inline_choices: InlineChoice[];
  investigation_items: InvestigationItem[];
  system_alerts: SystemAlert[];
  sanity: number;
  trust: number;
  awakening: number;
  echo_mode: EchoMode;
  available_panels: string[];
  active_panel: string | null;
  shutdown_countdown: number | null;
  glitch_level: number;
  suggested_glitches: string[];
  sound_cue: string | null;
  image_prompt: string | null;
  provisional: boolean;
}

export interface EndingPayload {
  ending: StoryEnding;
  trigger_scene: string;
  title: string;
  summary: string;
  epilogue: string;
  dominant_mode: EchoMode;
  evidence_titles: string[];
  hidden_clue_ids: string[];
  satisfied_conditions: string[];
  resolved_clues: string[];
  sanity: number;
  trust: number;
  awakening: number;
}

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
