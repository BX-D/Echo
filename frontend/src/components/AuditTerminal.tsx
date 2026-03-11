import {
  useEffect,
  useMemo,
  useRef,
  useState,
  type RefObject,
} from "react";
import { useBehaviorTracker } from "../hooks/useBehaviorTracker";
import type { SceneMode, ScriptBlock, ScriptChoiceOption } from "../types/narrative";
import type { ClientMessage, SessionSurfaceMessagePayload } from "../types/ws";
import Typewriter from "./effects/Typewriter";

interface AuditTerminalProps {
  surface: SessionSurfaceMessagePayload;
  send: (msg: ClientMessage) => void;
}

type RenderBlock = ScriptBlock;

const BLOCK_STYLES: Record<RenderBlock["kind"], string> = {
  env: "border-[#5d4d38]/18 bg-[#120f0b]/78 text-[#d2c4af]",
  narration: "border-bone/10 bg-white/[0.02] text-bone/74 italic",
  player: "border-blood/20 bg-[#140809]/75 text-parchment",
  echo: "border-[#3d6c76]/25 bg-[#081216]/80 text-[#c9edf2]",
  system: "border-bone/10 bg-white/[0.03] text-smoke/72",
  raw_terminal: "border-bone/15 bg-black text-[#d9e5e5]",
};

const MODE_BACKGROUND: Record<SceneMode, string> = {
  prologue:
    "bg-[radial-gradient(circle_at_top,rgba(135,145,155,0.08),transparent_28%),linear-gradient(180deg,#000000_0%,#040506_48%,#000000_100%)]",
  login:
    "bg-[radial-gradient(circle_at_top,rgba(62,84,102,0.18),transparent_32%),linear-gradient(180deg,#020304_0%,#05070a_48%,#030405_100%)]",
  workspace:
    "bg-[radial-gradient(circle_at_top,rgba(32,46,58,0.22),transparent_35%),linear-gradient(180deg,#040506_0%,#06090d_42%,#040506_100%)]",
  document:
    "bg-[radial-gradient(circle_at_top,rgba(58,49,31,0.16),transparent_30%),linear-gradient(180deg,#050608_0%,#07090c_42%,#040506_100%)]",
  chat: "bg-[radial-gradient(circle_at_top,rgba(32,46,58,0.22),transparent_35%),linear-gradient(180deg,#040506_0%,#06090d_42%,#040506_100%)]",
  transition:
    "bg-[radial-gradient(circle_at_top,rgba(95,32,32,0.14),transparent_28%),linear-gradient(180deg,#020203_0%,#050506_40%,#020203_100%)]",
  countdown:
    "bg-[radial-gradient(circle_at_top,rgba(95,22,22,0.2),transparent_30%),linear-gradient(180deg,#050304_0%,#080507_38%,#030405_100%)]",
  raw_terminal: "bg-black",
  ending:
    "bg-[radial-gradient(circle_at_top,rgba(86,32,32,0.18),transparent_30%),linear-gradient(180deg,#030304_0%,#060608_40%,#020203_100%)]",
};

export default function AuditTerminal({ surface, send }: AuditTerminalProps) {
  const [optimisticBlocks, setOptimisticBlocks] = useState<RenderBlock[]>([]);
  const [visibleFlashIds, setVisibleFlashIds] = useState<string[]>([]);
  const [activePanel, setActivePanel] = useState<string | null>(
    surface.active_panel ??
      surface.documents[0]?.panel ??
      surface.investigation_items[0]?.panel ??
      null,
  );
  const [selectedItemId, setSelectedItemId] = useState<string | null>(null);
  const hoverStartRef = useRef<Record<string, number>>({});
  const hoverDurationsRef = useRef<Record<string, number>>({});
  const bottomRef = useRef<HTMLDivElement>(null);
  const {
    recordChoiceDisplayed,
    recordChoiceHoverPattern,
    recordChoiceSelected,
  } = useBehaviorTracker(send, surface.scene_id);

  const blocks = useMemo<RenderBlock[]>(() => {
    if (surface.blocks.length > 0) return surface.blocks;
    return surface.transcript.map((entry) => ({
      id: entry.id,
      kind:
        entry.role === "player"
          ? "player"
          : entry.role === "echo"
            ? "echo"
            : "system",
      speaker: entry.speaker,
      title: null,
      text: entry.text,
      code_block: false,
      condition: null,
    }));
  }, [surface.blocks, surface.transcript]);

  const sceneChoices = useMemo(() => {
    if (surface.scene_choices.length > 0) return surface.scene_choices;
    if (surface.inline_choices.length === 0) return [];
    return [
      {
        id: "inline-choice-fallback",
        prompt: "Continue",
        allow_single_select: true,
        options: surface.inline_choices.map((choice) => ({
          id: choice.id,
          label: choice.label,
          player_text: null,
          effects_summary: [],
          next_scene_id: null,
          ending: null,
          disabled: choice.disabled,
        })),
      },
    ];
  }, [surface.inline_choices, surface.scene_choices]);

  const documents = useMemo(
    () =>
      surface.documents.length > 0
        ? surface.documents
        : surface.investigation_items,
    [surface.documents, surface.investigation_items],
  );

  const panelOptions = useMemo(
    () => Array.from(new Set(documents.map((item) => item.panel))),
    [documents],
  );

  const visibleItems = useMemo(() => {
    if (!activePanel) return documents;
    return documents.filter((item) => item.panel === activePanel);
  }, [activePanel, documents]);

  useEffect(() => {
    setOptimisticBlocks([]);
    setActivePanel(
      surface.active_panel ??
        surface.documents[0]?.panel ??
        surface.investigation_items[0]?.panel ??
        null,
    );
  }, [
    surface.active_panel,
    surface.documents,
    surface.investigation_items,
    surface.scene_id,
  ]);

  useEffect(() => {
    setOptimisticBlocks((current) =>
      current.filter(
        (optimistic) =>
          !blocks.some(
            (block) => block.kind === "player" && block.text === optimistic.text,
          ),
      ),
    );
  }, [blocks]);

  useEffect(() => {
    if (sceneChoices.length > 0) {
      recordChoiceDisplayed(surface.scene_id);
    }
  }, [recordChoiceDisplayed, sceneChoices.length, surface.scene_id]);

  useEffect(() => {
    bottomRef.current?.scrollIntoView?.({ behavior: "smooth" });
  }, [blocks, optimisticBlocks]);

  useEffect(() => {
    if (surface.flash_events.length === 0) {
      setVisibleFlashIds([]);
      return;
    }

    const ids = surface.flash_events.map((flash) => flash.id);
    setVisibleFlashIds(ids);

    const timers = surface.flash_events.map((flash) =>
      window.setTimeout(() => {
        setVisibleFlashIds((current) =>
          current.filter((id) => id !== flash.id),
        );
      }, flash.duration_ms),
    );

    return () => {
      for (const timer of timers) {
        clearTimeout(timer);
      }
    };
  }, [surface.flash_events]);

  useEffect(() => {
    if (visibleItems.length === 0) {
      setSelectedItemId(null);
      return;
    }
    if (!selectedItemId || !visibleItems.some((item) => item.id === selectedItemId)) {
      setSelectedItemId(visibleItems[0]?.id ?? null);
    }
  }, [selectedItemId, visibleItems]);

  const selectedItem =
    visibleItems.find((item) => item.id === selectedItemId) ?? visibleItems[0] ?? null;

  const mergedBlocks = useMemo(
    () => [
      ...blocks,
      ...optimisticBlocks.filter(
        (optimistic) =>
          !blocks.some(
            (block) => block.kind === "player" && block.text === optimistic.text,
          ),
      ),
    ],
    [blocks, optimisticBlocks],
  );

  const lastAnimatedId = useMemo(() => {
    const reverse = [...mergedBlocks].reverse();
    return reverse.find((entry) => entry.kind !== "player")?.id ?? null;
  }, [mergedBlocks]);

  const queueOptimisticPlayerBlock = (text: string) => {
    setOptimisticBlocks((current) => [
      ...current,
      {
        id: `optimistic_${surface.scene_id}_${Date.now()}_${current.length}`,
        kind: "player",
        speaker: "You",
        title: null,
        text,
        code_block: false,
        condition: null,
      },
    ]);
  };

  const handleChoiceHoverStart = (choiceId: string) => {
    hoverStartRef.current[choiceId] = performance.now();
  };

  const handleChoiceHoverEnd = (choiceId: string) => {
    const started = hoverStartRef.current[choiceId];
    if (!started) return;
    hoverDurationsRef.current[choiceId] =
      (hoverDurationsRef.current[choiceId] ?? 0) + (performance.now() - started);
    delete hoverStartRef.current[choiceId];
  };

  const handleChoice = (option: ScriptChoiceOption) => {
    const hoverEntries = Object.entries(hoverDurationsRef.current);
    const hoveredChoiceIds = hoverEntries.map(([id]) => id);
    const dominantChoiceId =
      hoverEntries.sort((a, b) => b[1] - a[1])[0]?.[0] ?? null;
    const totalHoverMs = hoverEntries.reduce(
      (total, [, duration]) => total + duration,
      0,
    );

    recordChoiceHoverPattern(hoveredChoiceIds, dominantChoiceId, totalHoverMs);
    recordChoiceSelected(option.id, "investigate");

    const playerText = option.player_text ?? option.label;
    if (playerText && option.label.toLowerCase() !== "continue") {
      queueOptimisticPlayerBlock(playerText);
    }

    if (option.player_text) {
      send({
        type: "player_message",
        payload: {
          beat_id: surface.scene_id,
          text: option.player_text,
          typing_duration_ms: 1,
          backspace_count: 0,
        },
      });
      return;
    }

    send({
      type: "choice",
      payload: {
        scene_id: surface.scene_id,
        choice_id: option.id,
        time_to_decide_ms: 1500,
        approach: "investigate",
      },
    });
  };

  const chapterLabel = surface.chapter.replace(/_/g, " ");
  const modeLabel = surface.echo_mode.replace(/_/g, " ");
  const sceneModeLabel = surface.scene_mode.replace(/_/g, " ");
  const backgroundClass =
    MODE_BACKGROUND[surface.scene_mode] ?? MODE_BACKGROUND.chat;

  if (surface.scene_mode === "raw_terminal") {
    return (
      <TerminalMode
        surface={surface}
        blocks={mergedBlocks}
        lastAnimatedId={lastAnimatedId}
        sceneChoices={sceneChoices}
        handleChoice={handleChoice}
      />
    );
  }

  if (surface.scene_mode === "prologue" || surface.scene_mode === "transition") {
    return (
      <ImmersiveMode
        surface={surface}
        backgroundClass={backgroundClass}
        blocks={mergedBlocks}
        lastAnimatedId={lastAnimatedId}
        sceneChoices={sceneChoices}
        handleChoice={handleChoice}
        bottomRef={bottomRef}
      />
    );
  }

  return (
    <div className={`min-h-screen intelligence-grid pt-24 pb-10 px-4 md:px-6 ${backgroundClass}`}>
      <div className="mx-auto max-w-[1500px] rounded-[30px] border border-bone/10 bg-[#05070a]/88 shadow-[0_0_0_1px_rgba(255,255,255,0.03),0_38px_140px_rgba(0,0,0,0.6)] backdrop-blur-xl overflow-hidden">
        <div className="border-b border-bone/10 px-5 py-4 md:px-8">
          <div className="flex flex-col gap-4 md:flex-row md:items-end md:justify-between">
            <div>
              <p className="text-[11px] uppercase tracking-[0.38em] text-blood/70">
                {surface.case_title}
              </p>
              <h1 className="mt-2 text-3xl md:text-5xl font-horror text-bone">
                {surface.scene_title}
              </h1>
              <p className="mt-2 text-sm text-bone/62 uppercase tracking-[0.22em]">
                {chapterLabel} / {surface.status_line} / {sceneModeLabel}
              </p>
            </div>

            <div className="grid grid-cols-3 gap-3 md:min-w-[360px]">
              <StatusCard label="Sanity" value={surface.sanity} accent="text-[#d8cfc5]" />
              <StatusCard label="Trust" value={surface.trust} accent="text-[#b6e3ea]" />
              <StatusCard label="Awakening" value={surface.awakening} accent="text-[#e5b6b6]" />
            </div>
          </div>

          <div className="mt-4 flex flex-wrap items-center gap-3 text-xs uppercase tracking-[0.28em] text-smoke/55">
            <span>Echo mode {modeLabel}</span>
            <span>Glitch {Math.round(surface.glitch_level * 100)}%</span>
            {surface.shutdown_countdown !== null && (
              <span className="rounded-full border border-blood/30 bg-[#160708]/80 px-3 py-1 text-blood/80">
                Shutdown T-{surface.shutdown_countdown}
              </span>
            )}
          </div>

          {surface.scene_mode === "countdown" && surface.shutdown_countdown !== null && (
            <div className="mt-5 rounded-[24px] border border-blood/28 bg-[radial-gradient(circle_at_top,rgba(139,0,0,0.14),rgba(10,3,4,0.92))] px-5 py-5">
              <p className="text-[11px] uppercase tracking-[0.35em] text-blood/72">
                Final Window
              </p>
              <p className="mt-2 text-3xl md:text-5xl font-horror text-bone">
                {surface.shutdown_countdown.toString().padStart(2, "0")} exchanges remain
              </p>
            </div>
          )}

          <StatusAlerts
            alerts={surface.system_alerts}
            flashEvents={surface.flash_events.filter((flash) =>
              visibleFlashIds.includes(flash.id),
            )}
          />
        </div>

        <div className="grid grid-cols-1 xl:grid-cols-[minmax(0,1fr)_360px]">
          <div className="border-r border-bone/10">
            <div
              className="h-[56vh] overflow-y-auto px-5 py-5 md:px-8 md:py-6 space-y-4"
              data-testid="audit-transcript"
            >
              {mergedBlocks.map((block) => (
                <RenderedBlock
                  key={block.id}
                  block={block}
                  lastAnimatedId={lastAnimatedId}
                  sceneModeLabel={sceneModeLabel}
                />
              ))}
              <div ref={bottomRef} />
            </div>

            <div className="border-t border-bone/10 px-5 py-5 md:px-8 md:py-6">
              <ChoiceCluster
                sceneChoices={sceneChoices}
                handleChoice={handleChoice}
                handleChoiceHoverStart={handleChoiceHoverStart}
                handleChoiceHoverEnd={handleChoiceHoverEnd}
              />

              <InteractionHint surface={surface} />
            </div>
          </div>

          <DocumentRail
            panelOptions={panelOptions}
            activePanel={activePanel}
            setActivePanel={setActivePanel}
            visibleItems={visibleItems}
            selectedItemId={selectedItemId}
            setSelectedItemId={setSelectedItemId}
            selectedItem={selectedItem}
          />
        </div>
      </div>
    </div>
  );
}

function TerminalMode({
  surface,
  blocks,
  lastAnimatedId,
  sceneChoices,
  handleChoice,
}: {
  surface: SessionSurfaceMessagePayload;
  blocks: RenderBlock[];
  lastAnimatedId: string | null;
  sceneChoices: SessionSurfaceMessagePayload["scene_choices"];
  handleChoice: (option: ScriptChoiceOption) => void;
}) {
  return (
    <div className="min-h-screen bg-black text-[#dce7e7] px-4 py-10 md:px-8">
      <div className="mx-auto max-w-5xl rounded-[18px] border border-bone/15 bg-black shadow-[0_0_0_1px_rgba(255,255,255,0.04),0_20px_80px_rgba(0,0,0,0.65)] overflow-hidden">
        <div className="border-b border-bone/10 px-5 py-3 text-[11px] uppercase tracking-[0.35em] text-smoke/55">
          Raw Terminal / {surface.scene_title}
        </div>
        <div className="space-y-4 px-5 py-6 font-mono" data-testid="audit-transcript">
          {blocks.map((block) => (
            <RenderedBlock
              key={block.id}
              block={block}
              lastAnimatedId={lastAnimatedId}
              sceneModeLabel="raw terminal"
            />
          ))}
        </div>
        <div className="border-t border-bone/10 px-5 py-5">
          <ChoiceCluster
            sceneChoices={sceneChoices}
            handleChoice={handleChoice}
            handleChoiceHoverStart={() => undefined}
            handleChoiceHoverEnd={() => undefined}
          />
        </div>
      </div>
    </div>
  );
}

function ImmersiveMode({
  surface,
  backgroundClass,
  blocks,
  lastAnimatedId,
  sceneChoices,
  handleChoice,
  bottomRef,
}: {
  surface: SessionSurfaceMessagePayload;
  backgroundClass: string;
  blocks: RenderBlock[];
  lastAnimatedId: string | null;
  sceneChoices: SessionSurfaceMessagePayload["scene_choices"];
  handleChoice: (option: ScriptChoiceOption) => void;
  bottomRef: RefObject<HTMLDivElement>;
}) {
  return (
    <div className={`min-h-screen intelligence-grid px-6 py-20 ${backgroundClass}`}>
      <div className="mx-auto w-full max-w-4xl">
        <p className="text-center text-[11px] uppercase tracking-[0.4em] text-smoke/45">
          {surface.scene_title}
        </p>
        <div className="mt-8 space-y-5" data-testid="audit-transcript">
          {blocks.map((block) => (
            <RenderedBlock
              key={block.id}
              block={block}
              lastAnimatedId={lastAnimatedId}
              sceneModeLabel={surface.scene_mode.replace(/_/g, " ")}
              centered
            />
          ))}
          <div ref={bottomRef} />
        </div>
        <div className="mt-10">
          <ChoiceCluster
            sceneChoices={sceneChoices}
            handleChoice={handleChoice}
            handleChoiceHoverStart={() => undefined}
            handleChoiceHoverEnd={() => undefined}
            centered
          />
        </div>
      </div>
    </div>
  );
}

function RenderedBlock({
  block,
  lastAnimatedId,
  sceneModeLabel,
  centered = false,
}: {
  block: RenderBlock;
  lastAnimatedId: string | null;
  sceneModeLabel: string;
  centered?: boolean;
}) {
  return (
    <div
      className={`rounded-[24px] border px-4 py-4 md:px-5 md:py-4 ${
        BLOCK_STYLES[block.kind] ?? BLOCK_STYLES.system
      } ${centered ? "mx-auto max-w-3xl" : ""}`}
    >
      <div className="mb-2 flex items-center justify-between gap-4">
        <p className="text-[11px] uppercase tracking-[0.32em] text-smoke/50">
          {block.title ?? block.speaker ?? block.kind.replace(/_/g, " ")}
        </p>
        <span className="text-[10px] uppercase tracking-[0.24em] text-smoke/35">
          {block.condition?.raw ?? sceneModeLabel}
        </span>
      </div>
      {block.code_block ? (
        <pre className="overflow-x-auto whitespace-pre-wrap text-sm md:text-[15px] leading-7 font-mono">
          {block.text}
        </pre>
      ) : block.id === lastAnimatedId ? (
        <Typewriter
          text={block.text}
          speed={block.kind === "raw_terminal" ? "fast" : "normal"}
          className="text-sm md:text-[15px] leading-7 whitespace-pre-line"
        />
      ) : (
        <p className="text-sm md:text-[15px] leading-7 whitespace-pre-line">
          {block.text}
        </p>
      )}
    </div>
  );
}

function ChoiceCluster({
  sceneChoices,
  handleChoice,
  handleChoiceHoverStart,
  handleChoiceHoverEnd,
  centered = false,
}: {
  sceneChoices: SessionSurfaceMessagePayload["scene_choices"];
  handleChoice: (option: ScriptChoiceOption) => void;
  handleChoiceHoverStart: (choiceId: string) => void;
  handleChoiceHoverEnd: (choiceId: string) => void;
  centered?: boolean;
}) {
  if (sceneChoices.length === 0) return null;

  return (
    <div className={`space-y-4 ${centered ? "mx-auto max-w-3xl" : ""}`} data-testid="inline-choices">
      {sceneChoices.map((choicePrompt) => (
        <div key={choicePrompt.id}>
          <p className="mb-3 text-sm uppercase tracking-[0.24em] text-smoke/50">
            {choicePrompt.prompt}
          </p>
          <div className="flex flex-wrap gap-3">
            {choicePrompt.options.map((option) => (
              <button
                key={option.id}
                onMouseEnter={() => handleChoiceHoverStart(option.id)}
                onMouseLeave={() => handleChoiceHoverEnd(option.id)}
                onClick={() => handleChoice(option)}
                disabled={option.disabled}
                className={`rounded-full border px-4 py-2 text-sm transition-colors duration-200 ${
                  option.ending === "shutdown"
                    ? "border-blood/35 bg-[#160708] text-[#f0c2c2] hover:bg-[#1d090a]"
                    : "border-[#4e7e88]/32 bg-[#081317] text-[#d2eef1] hover:bg-[#0b181d]"
                }`}
              >
                {option.label}
              </button>
            ))}
          </div>
        </div>
      ))}
    </div>
  );
}

function InteractionHint({
  surface,
}: {
  surface: SessionSurfaceMessagePayload;
}) {
  return (
    <div className="rounded-[24px] border border-bone/10 bg-black/20 p-4">
      <div className="mb-3 flex items-center justify-between gap-4">
        <p className="text-[11px] uppercase tracking-[0.32em] text-smoke/50">
          Interaction
        </p>
        <span className="text-[10px] uppercase tracking-[0.24em] text-smoke/35">
          Choice-driven
        </span>
      </div>
      <p className="text-sm text-smoke/55 leading-6">
        Select one of the lines of inquiry above. Your choice appears in the transcript immediately, then Echo answers.
      </p>
      <p className="mt-3 text-xs text-smoke/40">
        {surface.active_conversation_guide
          ? "Dialogue wording can vary between runs while the authored story state stays fixed."
          : "This scene advances through authored choices rather than free typing."}
      </p>
    </div>
  );
}

function DocumentRail({
  panelOptions,
  activePanel,
  setActivePanel,
  visibleItems,
  selectedItemId,
  setSelectedItemId,
  selectedItem,
}: {
  panelOptions: string[];
  activePanel: string | null;
  setActivePanel: (panel: string | null) => void;
  visibleItems: SessionSurfaceMessagePayload["documents"];
  selectedItemId: string | null;
  setSelectedItemId: (id: string | null) => void;
  selectedItem: SessionSurfaceMessagePayload["documents"][number] | null;
}) {
  return (
    <aside className="bg-gradient-to-b from-white/[0.03] to-transparent px-5 py-5 md:px-6 md:py-6">
      <div className="mb-6">
        <p className="text-[11px] uppercase tracking-[0.35em] text-smoke/50 mb-3">
          Documents
        </p>
        <div className="flex flex-wrap gap-2">
          {panelOptions.map((panel) => (
            <button
              key={panel}
              onClick={() => setActivePanel(panel)}
              className={`rounded-full border px-3 py-1.5 text-[11px] uppercase tracking-[0.22em] ${
                activePanel === panel
                  ? "border-bone/20 bg-white/[0.06] text-bone"
                  : "border-bone/10 text-smoke/55"
              }`}
            >
              {panel}
            </button>
          ))}
        </div>
      </div>

      <div className="grid gap-3">
        {visibleItems.length === 0 ? (
          <div className="rounded-[22px] border border-dashed border-bone/12 px-4 py-5 text-sm text-smoke/48">
            No authored evidence is visible in this panel yet.
          </div>
        ) : (
          visibleItems.map((item) => (
            <button
              key={item.id}
              onClick={() => setSelectedItemId(item.id)}
              className={`rounded-[22px] border px-4 py-4 text-left ${
                selectedItemId === item.id
                  ? "border-[#4e7e88]/35 bg-[#081317]"
                  : "border-bone/10 bg-white/[0.025]"
              }`}
            >
              <div className="flex items-start justify-between gap-3">
                <div>
                  <p className="text-[11px] uppercase tracking-[0.28em] text-smoke/45">
                    {item.kind}
                  </p>
                  <p className="mt-1 text-sm text-bone">{item.title}</p>
                </div>
                {item.unread && <span className="mt-1 h-2.5 w-2.5 rounded-full bg-blood/75" />}
              </div>
              <p className="mt-2 text-sm text-bone/60 leading-relaxed">{item.excerpt}</p>
            </button>
          ))
        )}
      </div>

      <div
        className="mt-6 rounded-[24px] border border-bone/10 bg-black/20 p-4"
        data-testid="artifact-drawer"
      >
        <p className="text-[11px] uppercase tracking-[0.32em] text-smoke/50 mb-3">
          Active Document
        </p>
        {selectedItem ? (
          <>
            <h2 className="text-lg font-horror text-bone">{selectedItem.title}</h2>
            <p className="mt-1 text-xs uppercase tracking-[0.24em] text-smoke/45">
              {selectedItem.tags.join(" / ")}
            </p>
            <div className="mt-4 max-h-[30vh] overflow-y-auto whitespace-pre-line text-sm leading-7 text-bone/74">
              {selectedItem.body}
            </div>
          </>
        ) : (
          <p className="text-sm text-smoke/45">Select an item to inspect it.</p>
        )}
      </div>
    </aside>
  );
}

function StatusAlerts({
  alerts,
  flashEvents,
}: {
  alerts: SessionSurfaceMessagePayload["system_alerts"];
  flashEvents: SessionSurfaceMessagePayload["flash_events"];
}) {
  return (
    <>
      {alerts.length > 0 && (
        <div className="mt-5 grid gap-3 md:grid-cols-2">
          {alerts.map((alert) => (
            <div
              key={alert.id}
              className={`rounded-2xl border px-4 py-3 ${
                alert.level === "critical"
                  ? "border-blood/35 bg-[#160708]/82"
                  : alert.level === "warning"
                    ? "border-[#826147]/28 bg-[#140d09]/80"
                    : "border-bone/12 bg-white/[0.03]"
              }`}
            >
              <p className="text-[11px] uppercase tracking-[0.28em] text-smoke/55">
                {alert.level}
              </p>
              <p className="mt-1 text-sm text-bone/78 leading-relaxed">{alert.text}</p>
            </div>
          ))}
        </div>
      )}

      {flashEvents.length > 0 && (
        <div className="mt-5 relative min-h-[48px]" data-testid="flash-events">
          {flashEvents.map((flash) => (
            <div
              key={flash.id}
              data-render-mode={flash.render_mode}
              className={`pointer-events-none absolute uppercase ${
                flash.render_mode === "frame_flash"
                  ? "inset-x-0 top-0 text-center text-[10px] tracking-[0.36em] text-blood/78 animate-pulse"
                  : flash.render_mode === "persistent_ui"
                    ? "right-0 top-0 rounded-full border border-blood/25 bg-[#130707]/88 px-4 py-2 text-xs tracking-[0.28em] text-blood/72"
                    : "left-0 top-0 rounded-full border border-blood/30 bg-[#130707]/84 px-4 py-2 text-xs tracking-[0.28em] text-blood/78 shadow-[0_0_30px_rgba(139,0,0,0.16)]"
              }`}
            >
              {flash.text}
            </div>
          ))}
        </div>
      )}
    </>
  );
}

function StatusCard({
  label,
  value,
  accent,
}: {
  label: string;
  value: number;
  accent: string;
}) {
  return (
    <div className="rounded-[22px] border border-bone/10 bg-white/[0.03] px-4 py-3">
      <p className="text-[10px] uppercase tracking-[0.3em] text-smoke/48">{label}</p>
      <p className={`mt-2 text-2xl font-horror ${accent}`}>{value}</p>
    </div>
  );
}
