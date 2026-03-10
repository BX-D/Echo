import type { BehaviorEvent } from "./behavior";
import type { GamePhase } from "./game";
import type {
  AdaptationRecord,
  Atmosphere,
  BehaviorProfileSummary,
  Choice,
  DisplayMode,
  EndingClassification,
  EffectDirective,
  FearProfileSummary,
  KeyMoment,
  MetaTarget,
  RevealAnalysis,
  SessionAct,
  SessionSummary,
  SurfaceMedium,
  TrustPosture,
} from "./narrative";

// ---------------------------------------------------------------------------
// Server → Client
// ---------------------------------------------------------------------------

export interface NarrativePayload {
  scene_id: string;
  text: string;
  atmosphere: Atmosphere;
  choices: Choice[];
  sound_cue: string | null;
  intensity: number;
  effects: EffectDirective[];
  title: string | null;
  act: SessionAct | null;
  medium: SurfaceMedium | null;
  trust_posture: TrustPosture | null;
  status_line: string | null;
  observation_notes: string[];
  trace_items: string[];
  transcript_lines: string[];
  question_prompts: string[];
  archive_entries: string[];
  mirror_observations: string[];
  surface_label: string | null;
  auxiliary_text: string | null;
  surface_purpose?: string | null;
  system_intent?: string | null;
  active_links?: string[];
  provisional: boolean;
}

export interface ImagePayload {
  scene_id: string;
  image_url: string;
  display_mode: DisplayMode;
}

export interface PhaseChangePayload {
  from: GamePhase;
  to: GamePhase;
}

export interface MetaPayload {
  text: string;
  target: MetaTarget;
  delay_ms: number;
}

export interface RevealPayload {
  fear_profile: FearProfileSummary;
  behavior_profile: BehaviorProfileSummary;
  session_summary: SessionSummary;
  key_moments: KeyMoment[];
  adaptation_log: AdaptationRecord[];
  ending_classification: EndingClassification;
  analysis: RevealAnalysis;
}

export interface ErrorPayload {
  code: string;
  message: string;
  recoverable: boolean;
}

export type ServerMessage =
  | { type: "narrative"; payload: NarrativePayload }
  | { type: "image"; payload: ImagePayload }
  | { type: "phase_change"; payload: PhaseChangePayload }
  | { type: "meta"; payload: MetaPayload }
  | { type: "reveal"; payload: RevealPayload }
  | { type: "error"; payload: ErrorPayload };

// ---------------------------------------------------------------------------
// Client → Server
// ---------------------------------------------------------------------------

export type ClientMessage =
  | { type: "start_game"; payload: { player_name?: string | null } }
  | {
      type: "choice";
      payload: {
        scene_id: string;
        choice_id: string;
        time_to_decide_ms: number;
        approach: Choice["approach"];
      };
    }
  | {
      type: "behavior_batch";
      payload: { events: BehaviorEvent[]; timestamp: string };
    }
  | {
      type: "text_input";
      payload: {
        scene_id: string;
        text: string;
        typing_duration_ms: number;
        backspace_count: number;
      };
    };
