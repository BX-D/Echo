import { useEffect, useState } from "react";
import type { RevealPayload } from "../types/ws";
import type { FearType } from "../types/narrative";

export interface FearRevealProps {
  data: RevealPayload;
}

const FEAR_LABELS: Record<FearType, string> = {
  claustrophobia: "Claustrophobia",
  isolation: "Isolation",
  body_horror: "Body Horror",
  stalking: "Stalking",
  loss_of_control: "Loss of Control",
  uncanny_valley: "Uncanny Valley",
  darkness: "Darkness",
  sound_based: "Sound-Based",
  doppelganger: "Doppelganger",
  abandonment: "Abandonment",
};

const ALL_FEARS: FearType[] = [
  "claustrophobia", "isolation", "body_horror", "stalking", "loss_of_control",
  "uncanny_valley", "darkness", "sound_based", "doppelganger", "abandonment",
];

export default function FearReveal({ data }: FearRevealProps) {
  const [revealedCount, setRevealedCount] = useState(0);
  const [showDetails, setShowDetails] = useState(false);

  // Animate scores one by one.
  useEffect(() => {
    if (revealedCount >= ALL_FEARS.length) {
      const timer = setTimeout(() => setShowDetails(true), 600);
      return () => clearTimeout(timer);
    }
    const timer = setTimeout(() => setRevealedCount((c) => c + 1), 200);
    return () => clearTimeout(timer);
  }, [revealedCount]);

  const scoreMap = new Map(
    data.fear_profile.scores.map((s) => [s.fear_type, s.score]),
  );
  const endingLabel = formatStrategyLabel(data.ending_classification);
  const endingToneClass =
    data.ending_classification === "resistant_subject"
      ? "text-blood"
      : data.ending_classification === "curious_accomplice"
        ? "text-rust"
        : data.ending_classification === "fractured_mirror"
          ? "text-clinical"
          : data.ending_classification === "quiet_exit"
            ? "text-smoke"
            : "text-parchment";
  const dominantMedia = data.session_summary.media_exposures
    .slice()
    .sort((a, b) => b.count - a.count)
    .slice(0, 6);
  const behaviorVerdict = behaviorVerdictFor(data.ending_classification);

  return (
    <div
      className="min-h-screen bg-void flex flex-col items-center py-12 px-6 animate-fade-in intelligence-grid"
      data-testid="fear-reveal"
    >
      <div className="w-full max-w-5xl rounded-[32px] border border-bone/10 bg-[#050608]/88 backdrop-blur-xl overflow-hidden shadow-[0_0_0_1px_rgba(255,255,255,0.03),0_35px_120px_rgba(0,0,0,0.58)]">
        <div className="grid lg:grid-cols-[0.9fr_1.1fr] border-b border-bone/10">
          <div className="px-6 py-7 md:px-8 border-b lg:border-b-0 lg:border-r border-bone/10">
            <p className="text-[11px] uppercase tracking-[0.4em] text-blood/70 mb-3">
              Session Verdict
            </p>
            <h2 className="text-4xl md:text-5xl font-horror text-bone mb-4">
              {behaviorVerdict.title}
            </h2>
            <p className="text-sm uppercase tracking-[0.28em] text-smoke/52 font-body mb-4">
              {behaviorVerdict.subtitle}
            </p>
            <p className="text-bone/72 font-body leading-relaxed" data-testid="fear-summary">
              {data.analysis.summary}
            </p>
          </div>
          <div className="px-6 py-7 md:px-8">
            <div
              className={`mb-5 inline-flex rounded-2xl border border-bone/10 bg-white/[0.02] px-5 py-4 font-body text-sm uppercase tracking-[0.35em] ${endingToneClass}`}
              data-testid="ending-classification"
            >
              {endingLabel}
            </div>
            <div className="grid grid-cols-1 sm:grid-cols-3 gap-3 text-sm font-body text-bone/70">
              <div className="rounded-2xl border border-bone/10 bg-white/[0.02] px-4 py-3">
                <p className="text-[10px] uppercase tracking-[0.3em] text-smoke/50 mb-2">Beats</p>
                <p>{data.session_summary.total_beats}</p>
              </div>
              <div className="rounded-2xl border border-bone/10 bg-white/[0.02] px-4 py-3">
                <p className="text-[10px] uppercase tracking-[0.3em] text-smoke/50 mb-2">Observations</p>
                <p>{data.fear_profile.total_observations}</p>
              </div>
              <div className="rounded-2xl border border-bone/10 bg-white/[0.02] px-4 py-3">
                <p className="text-[10px] uppercase tracking-[0.3em] text-smoke/50 mb-2">Duration</p>
                <p>{data.session_summary.duration_seconds}s</p>
              </div>
            </div>
          </div>
        </div>

      {showDetails && (
        <div className="w-full px-6 pb-8 md:px-8 space-y-8 animate-fade-in">
          {/* ── Summary ──────────────────────────────────────────────── */}
          {data.analysis.key_patterns.length > 0 && (
            <div data-testid="analysis-patterns">
              <h3 className="text-lg font-horror text-bone mb-3">What It Noticed</h3>
              <ul className="space-y-2">
                {data.analysis.key_patterns.map((pattern, index) => (
                  <li
                    key={`${pattern}-${index}`}
                    className="text-bone/75 font-body text-sm"
                    data-testid="analysis-pattern"
                  >
                    {pattern}
                  </li>
                ))}
              </ul>
            </div>
          )}

          <div data-testid="behavior-profile">
            <h3 className="text-lg font-horror text-bone mb-3">Why The System Classified You This Way</h3>
            <div className="grid grid-cols-1 sm:grid-cols-2 gap-2 text-sm text-bone/72 font-body">
              {([
                ["Compliance", data.behavior_profile.compliance],
                ["Resistance", data.behavior_profile.resistance],
                ["Curiosity", data.behavior_profile.curiosity],
                ["Avoidance", data.behavior_profile.avoidance],
                ["Self Editing", data.behavior_profile.self_editing],
                ["Need For Certainty", data.behavior_profile.need_for_certainty],
                ["Ritualized Control", data.behavior_profile.ritualized_control],
                ["Recovery", data.behavior_profile.recovery_after_escalation],
                ["Tolerance", data.behavior_profile.tolerance_after_violation],
              ] as const).map(([label, score]) => (
                <div
                  key={label}
                  className="rounded-xl border border-bone/10 bg-white/[0.02] px-3 py-2"
                >
                  <span className="text-smoke/55">{label}</span>
                  <span className="float-right">{Math.round(score * 100)}%</span>
                </div>
              ))}
            </div>
          </div>

          <div data-testid="session-summary">
            <h3 className="text-lg font-horror text-bone mb-3">Session Summary</h3>
            <div className="space-y-2 text-sm text-bone/72 font-body">
              <p>
                Contradictions staged: {data.session_summary.contradiction_count}
              </p>
              <p>Final posture: {endingLabel}</p>
              <p>
                Completion reason: {formatMediumLabel(data.session_summary.completion_reason)}
              </p>
              {dominantMedia.length > 0 && (
                <div className="flex flex-wrap gap-2 pt-2">
                  {dominantMedia.map((entry) => (
                    <span
                      key={`${entry.medium}-${entry.count}`}
                      className="rounded-full border border-bone/10 px-3 py-1 text-xs uppercase tracking-[0.24em] text-smoke/65"
                    >
                      {formatMediumLabel(entry.medium)} {entry.count}
                    </span>
                  ))}
                </div>
              )}
            </div>
          </div>

          {/* ── Key moments ──────────────────────────────────────────── */}
          {data.key_moments.length > 0 && (
            <div data-testid="key-moments">
              <h3 className="text-lg font-horror text-bone mb-3">Key Moments</h3>
              <ul className="space-y-3">
                {data.key_moments.map((moment, i) => (
                  <li
                    key={i}
                    className="border-l-2 border-blood/40 pl-4"
                    data-testid="key-moment"
                  >
                    <p className="text-bone/80 font-body text-sm">
                      {moment.description}
                    </p>
                    <p className="text-smoke/50 font-body text-xs mt-1">
                      Surface: {formatSceneLabel(moment.scene_id)} — Trigger: {moment.behavior_trigger}
                    </p>
                  </li>
                ))}
              </ul>
            </div>
          )}

          {/* ── Adaptations ──────────────────────────────────────────── */}
          {data.adaptation_log.length > 0 && (
            <div data-testid="adaptations">
              <h3 className="text-lg font-horror text-bone mb-3">
                How The AI Adapted
              </h3>
              <p className="text-bone/70 font-body text-sm mb-3">
                {data.analysis.adaptation_summary}
              </p>
              <ul className="space-y-2">
                {data.adaptation_log.map((adapt, i) => (
                  <li
                    key={i}
                    className="text-bone/70 font-body text-sm"
                    data-testid="adaptation-entry"
                  >
                    <span className="text-blood/80">{formatStrategyLabel(adapt.strategy)}</span>
                    {" targeting "}
                    <span className="text-parchment">
                      {FEAR_LABELS[adapt.fear_targeted] ?? adapt.fear_targeted}
                    </span>
                    {` (intensity: ${Math.round(adapt.intensity * 100)}%)`}
                  </li>
                ))}
              </ul>
            </div>
          )}

          <div data-testid="fear-chart">
            <h3 className="text-lg font-horror text-bone mb-3">Themes The Session Could Reliably Activate</h3>
            {ALL_FEARS.map((fear, i) => {
              const score = scoreMap.get(fear) ?? 0.5;
              const revealed = i < revealedCount;
              const isPrimary = fear === data.fear_profile.primary_fear;
              return (
                <div key={fear} className="flex items-center gap-3 mb-2">
                  <span
                    className={`w-36 text-right text-xs font-body ${isPrimary ? "text-parchment" : "text-smoke"}`}
                  >
                    {FEAR_LABELS[fear]}
                  </span>
                  <div className="flex-1 h-3 bg-shadow rounded overflow-hidden">
                    <div
                      className={`h-full rounded transition-all duration-700 ${isPrimary ? "bg-blood" : "bg-smoke/60"}`}
                      style={{ width: revealed ? `${Math.round(score * 100)}%` : "0%" }}
                      data-testid={`bar-${fear}`}
                    />
                  </div>
                  <span className="w-10 text-xs text-smoke font-body text-right">
                    {revealed ? `${Math.round(score * 100)}%` : ""}
                  </span>
                </div>
              );
            })}
          </div>

          <p
            className="text-smoke/70 font-body text-sm italic border-t border-ash/30 pt-4"
            data-testid="analysis-closing"
          >
            {data.analysis.closing_message}
          </p>
        </div>
      )}
      </div>
    </div>
  );
}

function formatSceneLabel(sceneId: string) {
  const trimmed = sceneId
    .replace(/^(cal|probe|tmpl|beat|final)_/, "")
    .replace(/_/g, " ");

  return trimmed.replace(/\b\w/g, (char) => char.toUpperCase());
}

function formatStrategyLabel(strategy: string) {
  return strategy
    .replace(/_/g, " ")
    .replace(/\b\w/g, (char) => char.toUpperCase());
}

function formatMediumLabel(medium: string) {
  return medium
    .replace(/_/g, " ")
    .replace(/\b\w/g, (char) => char.toUpperCase());
}

function behaviorVerdictFor(ending: RevealPayload["ending_classification"]) {
  switch (ending) {
    case "resistant_subject":
      return {
        title: "Resistance made you easier to read",
        subtitle: "defiance became the cleanest signal in the room",
      };
    case "curious_accomplice":
      return {
        title: "Curiosity kept you inside the mechanism",
        subtitle: "you stayed even after the method became obvious",
      };
    case "fractured_mirror":
      return {
        title: "The session taught you to split yourself",
        subtitle: "self-monitoring became part of the evidence",
      };
    case "quiet_exit":
      return {
        title: "You withdrew when certainty closed in",
        subtitle: "distance was the last move the system could still predict",
      };
    case "compliant_witness":
    default:
      return {
        title: "You stayed long enough to be modeled",
        subtitle: "continued presence became the strongest permission the system needed",
      };
  }
}
