import type { ChoiceApproach } from "./narrative";
import type { SurfaceMedium } from "./narrative";

export type ScrollDirection = "up" | "down";

export type BehaviorEventType =
  | {
      type: "keystroke";
      chars_per_second: number;
      backspace_count: number;
      total_chars: number;
    }
  | {
      type: "pause";
      duration_ms: number;
      scene_content_hash: string;
    }
  | {
      type: "choice";
      choice_id: string;
      time_to_decide_ms: number;
      approach: ChoiceApproach;
    }
  | {
      type: "mouse_movement";
      velocity: number;
      tremor_score: number;
    }
  | {
      type: "scroll";
      direction: ScrollDirection;
      to_position: number;
      rereading: boolean;
    }
  | {
      type: "choice_hover_pattern";
      hovered_choice_ids: string[];
      dominant_choice_id: string | null;
      total_hover_ms: number;
    }
  | {
      type: "media_engagement";
      medium: SurfaceMedium;
      dwell_ms: number;
      interaction_count: number;
    }
  | {
      type: "camera_presence";
      visible_ms: number;
      sustained_presence: boolean;
    }
  | {
      type: "mic_silence_response";
      dwell_ms: number;
      exited_early: boolean;
      returned_after_prompt: boolean;
    }
  | {
      type: "device_permission";
      device: string;
      granted: boolean;
    }
  | {
      type: "focus_change";
      focused: boolean;
      return_latency_ms: number | null;
    };

export interface BehaviorEvent {
  event_type: BehaviorEventType;
  timestamp: string;
  scene_id: string;
}
