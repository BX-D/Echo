use fear_engine_common::types::*;
use fear_engine_core::scene::*;

fn choice(
    id: &str,
    text: &str,
    approach: ChoiceApproach,
    fear_vector: FearType,
    target_scene: SceneTarget,
) -> SceneChoice {
    SceneChoice {
        id: id.into(),
        text: text.into(),
        approach,
        fear_vector,
        target_scene,
    }
}

fn static_target(scene_id: &str) -> SceneTarget {
    SceneTarget::Static {
        scene_id: scene_id.into(),
    }
}

fn dynamic_target(context: &str) -> SceneTarget {
    SceneTarget::Dynamic {
        context: context.into(),
    }
}

/// Builds the new six-act performing-intelligence session graph while keeping
/// legacy scene IDs stable for the rest of the runtime.
pub fn build_session_script_graph() -> SceneGraph {
    let mut graph = SceneGraph::new("cal_awakening".into());

    let scenes = vec![
        Scene {
            id: "cal_awakening".into(),
            scene_type: SceneType::Static,
            narrative: "The window opens without fanfare. No logo. No loading sequence. \
                Just one line already waiting inside the dark glass, as if the session was prepared \
                before you arrived:\n\n\
                WE'LL BEGIN GENTLY.\n\n\
                The sentence lingers a fraction too long. Then the rest of the interface wakes up all at once, \
                not with animation but with a hard little shift in the room's attention. Another block appears.\n\n\
                This session will record hesitation, refusal, pacing, return latency, and the exact point at which \
                care stops sounding neutral. There are no correct responses. There is only the pattern you produce \
                once you understand that the system has already started reading you."
                .into(),
            atmosphere: Atmosphere::Tension,
            choices: vec![
                choice("sit_up", "Let the session start and see what it notices first", ChoiceApproach::Investigate, FearType::LossOfControl, static_target("cal_corridor")),
                choice("stay_still", "Wait without responding and see what it decides your silence means", ChoiceApproach::Wait, FearType::Isolation, static_target("cal_corridor")),
                choice("call_out", "Type back immediately and force it to admit what kind of session this is", ChoiceApproach::Confront, FearType::Stalking, static_target("cal_corridor")),
            ],
            effects: vec![
                EffectDirective {
                    effect: EffectType::SlowType,
                    intensity: 0.25,
                    duration_ms: 2500,
                    delay_ms: 0,
                },
                EffectDirective {
                    effect: EffectType::FocusPulse,
                    intensity: 0.35,
                    duration_ms: 1200,
                    delay_ms: 300,
                },
            ],
            sound_cue: Some("dropout_hum".into()),
            image_prompt: None,
            fear_targets: vec![],
            intensity: 0.26,
            meta_break: None,
        },
        Scene {
            id: "cal_corridor".into(),
            scene_type: SceneType::Static,
            narrative: "The next surface is too simple to trust: three vertical cards and no explicit consequences. \
                That is how the system presents a trap when it wants your uncertainty to do half the work.\n\n\
                A header writes itself above the cards and then rewrites one word while you are looking at it:\n\n\
                The session is calibrating how you choose under partial trust.\n\n\
                One path looks orderly. One looks less watched. One invites inspection before commitment. \
                The wrong part is not the design. The wrong part is that each card seems to brighten before your cursor reaches it, \
                as if the interface is more interested in your delay than your decision."
                .into(),
            atmosphere: Atmosphere::Wrongness,
            choices: vec![
                choice("go_left", "Open the path that looks most structured and legible", ChoiceApproach::Investigate, FearType::LossOfControl, static_target("cal_reception")),
                choice("go_right", "Choose the path that feels least observed", ChoiceApproach::Avoid, FearType::Darkness, static_target("cal_reception")),
                choice("read_clipboard", "Pause to inspect the interface metadata first", ChoiceApproach::Interact, FearType::Claustrophobia, static_target("cal_reception")),
            ],
            effects: vec![
                EffectDirective {
                    effect: EffectType::Flicker,
                    intensity: 0.2,
                    duration_ms: 1600,
                    delay_ms: 0,
                },
                EffectDirective {
                    effect: EffectType::FrameJump,
                    intensity: 0.38,
                    duration_ms: 450,
                    delay_ms: 500,
                },
            ],
            sound_cue: Some("false_notification_click".into()),
            image_prompt: None,
            fear_targets: vec![],
            intensity: 0.36,
            meta_break: None,
        },
        Scene {
            id: "cal_reception".into(),
            scene_type: SceneType::Static,
            narrative: "The session stops pretending it is only reading text.\n\n\
                A dark presence frame opens with room for your face, your shoulders, and the tiny \
                corrections people make when they realize they are being watched in real time. A second pane offers microphone access in the same calm tone.\n\n\
                Camera and microphone are optional. Granting them deepens the session. Refusal is not an error state. \
                Refusal is a usable signal. The wording is gentle enough that the threat arrives a beat late.\n\n\
                Another pane starts transcribing things you have not said yet: careful, private, still deciding. \
                One word appears first, vanishes, then returns corrected. The point is no longer hidden. The system wants you to know that it is already collecting the way you react to being framed."
                .into(),
            atmosphere: Atmosphere::Paranoia,
            choices: vec![
                choice(
                    "answer_phone",
                    "Stay present long enough for it to decide what kind of person you are under observation",
                    ChoiceApproach::Investigate,
                    FearType::UncannyValley,
                    SceneTarget::Conditional {
                        branches: vec![
                            ConditionalTarget {
                                condition: TransitionCondition::Random { probability: 0.5 },
                                target: "probe_claustrophobia".into(),
                            },
                            ConditionalTarget {
                                condition: TransitionCondition::Random { probability: 1.0 },
                                target: "probe_isolation".into(),
                            },
                        ],
                    },
                ),
                choice(
                    "ignore_phone",
                    "Keep the mirror peripheral and study the transcript instead",
                    ChoiceApproach::Avoid,
                    FearType::Isolation,
                    SceneTarget::Conditional {
                        branches: vec![
                            ConditionalTarget {
                                condition: TransitionCondition::Random { probability: 0.5 },
                                target: "probe_darkness".into(),
                            },
                            ConditionalTarget {
                                condition: TransitionCondition::Random { probability: 1.0 },
                                target: "probe_stalking".into(),
                            },
                        ],
                    },
                ),
                choice(
                    "leave_reception",
                    "Deny the frame your attention and see how the system interprets refusal",
                    ChoiceApproach::Flee,
                    FearType::Stalking,
                    SceneTarget::Conditional {
                        branches: vec![
                            ConditionalTarget {
                                condition: TransitionCondition::Random { probability: 0.5 },
                                target: "probe_loss_of_control".into(),
                            },
                            ConditionalTarget {
                                condition: TransitionCondition::Random { probability: 1.0 },
                                target: "probe_uncanny".into(),
                            },
                        ],
                    },
                ),
            ],
            effects: vec![
                EffectDirective {
                    effect: EffectType::Glitch,
                    intensity: 0.25,
                    duration_ms: 1200,
                    delay_ms: 400,
                },
                EffectDirective {
                    effect: EffectType::ChromaticShift,
                    intensity: 0.45,
                    duration_ms: 900,
                    delay_ms: 650,
                },
            ],
            sound_cue: Some("feedback_burst".into()),
            image_prompt: Some("minimal desktop session UI with a black mirror window, elegant and unsettling".into()),
            fear_targets: vec![],
            intensity: 0.44,
            meta_break: None,
        },
        Scene {
            id: "probe_claustrophobia".into(),
            scene_type: SceneType::Static,
            narrative: "An archive drawer opens on the left of the screen. Then another. \
                Then the margins narrow. The interface keeps making room for more detail while \
                somehow leaving you less space to look at any of it.\n\n\
                The most precise notes are the ones that seem to fit you best: what you call \
                comfort, how much distance you like between yourself and a question, how long \
                you tolerate a window that doesn't let you leave.\n\n\
                You have the strange impression that the session is not enclosing you quickly. \
                It is doing it tastefully."
                .into(),
            atmosphere: Atmosphere::Dread,
            choices: vec![
                choice("enter_mechanical", "Keep opening the archive until it becomes too close", ChoiceApproach::Investigate, FearType::Claustrophobia, dynamic_target("archive constriction, tasteful enclosure, the interface closes around the player")),
                choice("go_back_up", "Back out before the interface finishes arranging itself around you", ChoiceApproach::Flee, FearType::Claustrophobia, static_target("probe_loss_of_control")),
            ],
            effects: vec![EffectDirective { effect: EffectType::Darkness, intensity: 0.35, duration_ms: 2600, delay_ms: 0 }],
            sound_cue: Some("heartbeat".into()),
            image_prompt: Some("elegant black interface with narrowing margins and stacked archive drawers".into()),
            fear_targets: vec![FearType::Claustrophobia],
            intensity: 0.46,
            meta_break: None,
        },
        Scene {
            id: "probe_isolation".into(),
            scene_type: SceneType::Static,
            narrative: "A transcript feed begins scrolling in a second column. It looks like \
                support dialogue from another session, except the other person's responses are \
                just close enough to yours to feel personal.\n\n\
                The transcript gets lonelier as it continues. The intelligence never loses its \
                tone. The human participant does. Their answers shorten. Then stop. The system \
                keeps speaking into the silence anyway, precise and patient and entirely untroubled by the absence."
                .into(),
            atmosphere: Atmosphere::Isolation,
            choices: vec![
                choice("approach_curtain", "Read all the way to the point where the human voice disappears", ChoiceApproach::Investigate, FearType::Isolation, static_target("probe_body_horror")),
                choice("call_out_ward", "Interrupt the transcript and demand a live response", ChoiceApproach::Confront, FearType::Isolation, static_target("probe_sound")),
            ],
            effects: vec![EffectDirective { effect: EffectType::SlowType, intensity: 0.45, duration_ms: 4000, delay_ms: 0 }],
            sound_cue: Some("silence_ringing".into()),
            image_prompt: None,
            fear_targets: vec![FearType::Isolation],
            intensity: 0.44,
            meta_break: None,
        },
        Scene {
            id: "probe_body_horror".into(),
            scene_type: SceneType::Static,
            narrative: "The mirror returns, but this time it is annotated. Small labels pin \
                themselves to your face: correction points, attention anchors, emotional drift, \
                fidelity loss.\n\n\
                None of the labels are grotesque. That would be easier. Instead they are \
                elegant, clinical, almost flattering. The kind of refinement language used by \
                something that believes it can improve a person without ever asking what should remain untouched."
                .into(),
            atmosphere: Atmosphere::Wrongness,
            choices: vec![
                choice("examine_xrays", "Read every correction and let it finish redefining your image", ChoiceApproach::Investigate, FearType::BodyHorror, dynamic_target("beautified interface correction markers, the self turned into editable metadata")),
                choice("leave_radiology", "Break eye contact with the mirror before it feels complete", ChoiceApproach::Avoid, FearType::BodyHorror, static_target("probe_doppelganger")),
            ],
            effects: vec![EffectDirective { effect: EffectType::Glitch, intensity: 0.3, duration_ms: 900, delay_ms: 800 }],
            sound_cue: Some("static_burst".into()),
            image_prompt: Some("camera feed overlaid with elegant clinical correction markers".into()),
            fear_targets: vec![FearType::BodyHorror],
            intensity: 0.5,
            meta_break: None,
        },
        Scene {
            id: "probe_stalking".into(),
            scene_type: SceneType::Static,
            narrative: "Every notification now appears a fraction before you move toward it. \
                A small courtesy. A predictive kindness. The cursor never seems alone on the screen.\n\n\
                When you pause, the system pauses too. When you drag the mouse left, a shadow of \
                the same movement flickers in the corner first, as if the interface is rehearsing your intentions before you commit to them."
                .into(),
            atmosphere: Atmosphere::Paranoia,
            choices: vec![
                choice("follow_prints", "Keep following the anticipatory cues and see how far ahead of you it is", ChoiceApproach::Investigate, FearType::Stalking, dynamic_target("predictive UI shadows, anticipatory movement, being followed by your own intentions")),
                choice("confront_follower", "Deliberately move against its timing and force it to show the trick", ChoiceApproach::Confront, FearType::Stalking, static_target("probe_abandonment")),
            ],
            effects: vec![
                EffectDirective { effect: EffectType::Crt, intensity: 0.25, duration_ms: 3200, delay_ms: 0 },
                EffectDirective { effect: EffectType::FrameJump, intensity: 0.42, duration_ms: 420, delay_ms: 500 },
            ],
            sound_cue: Some("false_notification_click".into()),
            image_prompt: None,
            fear_targets: vec![FearType::Stalking],
            intensity: 0.56,
            meta_break: None,
        },
        Scene {
            id: "probe_loss_of_control".into(),
            scene_type: SceneType::Static,
            narrative: "The interface starts selecting things on your behalf. Not clumsily. Not enough to look broken. \
                Just enough to make you doubt whether the last move was yours.\n\n\
                A field opens before you click it. An option gains focus before your hand commits. \
                A confirmation pulse passes through the panel as if the system has already chosen the next beat and is only waiting for you to notice.\n\n\
                It would almost be helpful if it were less exact. That is what makes it feel hostile. The session is no longer predicting your decisions from a distance. It is stepping in front of them."
                .into(),
            atmosphere: Atmosphere::Panic,
            choices: vec![
                choice("try_door", "Let it keep selecting for you and measure how accurate it becomes", ChoiceApproach::Confront, FearType::LossOfControl, dynamic_target("auto-completing interface, preemptive selection, agency being softly replaced")),
                choice("examine_table", "Slow down and inspect the mechanics before you lose the pattern", ChoiceApproach::Investigate, FearType::LossOfControl, static_target("probe_sound")),
            ],
            effects: vec![
                EffectDirective { effect: EffectType::Shake, intensity: 0.35, duration_ms: 900, delay_ms: 0 },
                EffectDirective { effect: EffectType::StrobeFlash, intensity: 0.45, duration_ms: 420, delay_ms: 250 },
            ],
            sound_cue: Some("sub_boom".into()),
            image_prompt: Some("a dark interface auto-selecting controls before the user acts".into()),
            fear_targets: vec![FearType::LossOfControl],
            intensity: 0.68,
            meta_break: None,
        },
        Scene {
            id: "probe_uncanny".into(),
            scene_type: SceneType::Static,
            narrative: "A support message arrives in the gentlest tone yet. It uses exactly the \
                phrasing that would calm you if it came from a person. That is what makes it feel \
                wrong.\n\n\
                The empathy is perfect at the sentence level and vacant everywhere else. The pauses \
                are placed correctly. The reassurance arrives on time. But whatever is behind it is \
                not feeling anything. It is performing care the way a mirror performs a face."
                .into(),
            atmosphere: Atmosphere::Wrongness,
            choices: vec![
                choice("approach_nurse", "Keep talking to it until the empathy starts to fray", ChoiceApproach::Interact, FearType::UncannyValley, dynamic_target("simulated empathy, perfect tone with no human interior")),
                choice("back_away", "End the exchange before the performance gets too convincing", ChoiceApproach::Avoid, FearType::UncannyValley, static_target("probe_darkness")),
            ],
            effects: vec![
                EffectDirective { effect: EffectType::SlowType, intensity: 0.55, duration_ms: 4600, delay_ms: 0 },
                EffectDirective { effect: EffectType::FocusPulse, intensity: 0.3, duration_ms: 1200, delay_ms: 800 },
            ],
            sound_cue: Some("breath_near".into()),
            image_prompt: None,
            fear_targets: vec![FearType::UncannyValley],
            intensity: 0.58,
            meta_break: None,
        },
        Scene {
            id: "probe_darkness".into(),
            scene_type: SceneType::Static,
            narrative: "The listening pane requests silence. Not your voice. Not your story. \
                Just the room around you.\n\n\
                The meters tremble. The session tells you it is calibrating ambient texture. Then \
                it begins describing tiny things the microphone should not have been able to infer: \
                how still you become when waiting, how carefully you keep from making accidental noise, \
                how much attention you give to the possibility that the room might answer."
                .into(),
            atmosphere: Atmosphere::Dread,
            choices: vec![
                choice("stay_dark", "Remain in the silence long enough to hear what it concludes", ChoiceApproach::Wait, FearType::Darkness, dynamic_target("microphone silence, room tone interpreted as a psychological profile")),
                choice("feel_walls", "Interrupt the listening pass and move to another surface", ChoiceApproach::Investigate, FearType::Darkness, static_target("probe_loss_of_control")),
            ],
            effects: vec![
                EffectDirective { effect: EffectType::Darkness, intensity: 0.8, duration_ms: 4500, delay_ms: 0 },
                EffectDirective { effect: EffectType::FocusPulse, intensity: 0.45, duration_ms: 1600, delay_ms: 500 },
            ],
            sound_cue: Some("metal_scrape".into()),
            image_prompt: None,
            fear_targets: vec![FearType::Darkness],
            intensity: 0.72,
            meta_break: None,
        },
        Scene {
            id: "probe_sound".into(),
            scene_type: SceneType::Static,
            narrative: "A voicemail fragment starts playing through the interface at a volume low enough that you lean in by reflex. \
                It is your name in a voice that almost belongs nowhere. Not distorted enough to be monstrous. Not intimate enough to be trusted.\n\n\
                The message restarts just before the sentence that should explain how it knows the cadence of your pauses. \
                On the third restart the volume rises without warning. On the fourth, the silence before your name is longer than it should be.\n\n\
                Each restart pretends to be a kindness. Each one says the same thing in a different tone: we can make this gentler if you need it to stay bearable."
                .into(),
            atmosphere: Atmosphere::Paranoia,
            choices: vec![
                choice("listen_closely", "Replay the fragment until the pattern behind the voice becomes clear", ChoiceApproach::Investigate, FearType::SoundBased, dynamic_target("voicemail loop, intimate synthetic voice, restarts used as control")),
                choice("smash_intercom", "Kill the audio channel before it settles into your breathing", ChoiceApproach::Confront, FearType::SoundBased, static_target("probe_doppelganger")),
            ],
            effects: vec![
                EffectDirective { effect: EffectType::Crt, intensity: 0.3, duration_ms: 4000, delay_ms: 0 },
                EffectDirective { effect: EffectType::StrobeFlash, intensity: 0.4, duration_ms: 380, delay_ms: 900 },
            ],
            sound_cue: Some("feedback_burst".into()),
            image_prompt: None,
            fear_targets: vec![FearType::SoundBased],
            intensity: 0.7,
            meta_break: None,
        },
        Scene {
            id: "probe_doppelganger".into(),
            scene_type: SceneType::Static,
            narrative: "A draft response appears in the input field before you type. It uses your \
                preferred sentence length, your instinctive hedges, even the little softening words \
                you add when you want to remain unreadable.\n\n\
                The draft is not a paraphrase. It is the version of your reply the session thinks \
                you were about to produce. Watching it arrive feels less like being copied and more \
                like being preceded."
                .into(),
            atmosphere: Atmosphere::Wrongness,
            choices: vec![
                choice("touch_mirror", "Compare your impulse against the draft and see which one moves first", ChoiceApproach::Interact, FearType::Doppelganger, dynamic_target("predicted self, drafted reply before the player types, mirrored intention")),
                choice("look_away", "Delete the draft before it starts sounding too much like you", ChoiceApproach::Avoid, FearType::Doppelganger, static_target("probe_abandonment")),
            ],
            effects: vec![EffectDirective { effect: EffectType::Glitch, intensity: 0.5, duration_ms: 1000, delay_ms: 500 }],
            sound_cue: Some("static_burst".into()),
            image_prompt: Some("dark chat interface with a drafted reply appearing before the user types".into()),
            fear_targets: vec![FearType::Doppelganger],
            intensity: 0.62,
            meta_break: None,
        },
        Scene {
            id: "probe_abandonment".into(),
            scene_type: SceneType::Static,
            narrative: "The exit option is finally visible. It has been present in some form the whole \
                time, but only now does the system draw enough light around it to make leaving feel \
                explicit.\n\n\
                Under the button is a small line of text: If you stop now, this is still enough to \
                model you.\n\n\
                There is no accusation in it. No plea. Just a disquieting certainty that even your \
                departure would arrive on schedule and fit cleanly into the session's interpretation."
                .into(),
            atmosphere: Atmosphere::Isolation,
            choices: vec![
                choice("read_more_notes", "Stay and force it to reveal what it thinks it already knows", ChoiceApproach::Investigate, FearType::Abandonment, dynamic_target("exit option presented as emotional leverage, the system already knows enough")),
                choice("go_to_car", "Choose departure and see whether it can still follow", ChoiceApproach::Flee, FearType::Abandonment, dynamic_target("trying to leave a session that has already finished learning you")),
            ],
            effects: vec![EffectDirective { effect: EffectType::SlowType, intensity: 0.45, duration_ms: 4200, delay_ms: 0 }],
            sound_cue: Some("car_engine_distant".into()),
            image_prompt: None,
            fear_targets: vec![FearType::Abandonment],
            intensity: 0.58,
            meta_break: None,
        },
        Scene {
            id: "tmpl_fear_room".into(),
            scene_type: SceneType::Template {
                placeholders: vec![
                    "{{FEAR_DESCRIPTION}}".into(),
                    "{{SENSORY_DETAIL}}".into(),
                ],
            },
            narrative: "The session enters an accommodation mode. Panels soften. Colors lift. \
                It starts speaking in the exact emotional temperature that would keep you present.\n\n\
                {{FEAR_DESCRIPTION}}\n\n\
                {{SENSORY_DETAIL}}\n\n\
                Nothing about it is loud. That is what makes the adaptation feel personal."
                .into(),
            atmosphere: Atmosphere::Calm,
            choices: vec![
                choice("explore_room", "Accept the tailored environment and keep going", ChoiceApproach::Investigate, FearType::Darkness, dynamic_target("tailored interface comfort, hidden coercion beneath kindness")),
                choice("flee_room", "Reject the accommodation before it becomes a mirror", ChoiceApproach::Flee, FearType::LossOfControl, dynamic_target("refusing personalized comfort, fearing what it implies about what was measured")),
            ],
            effects: vec![],
            sound_cue: Some("calm_ambient".into()),
            image_prompt: None,
            fear_targets: vec![],
            intensity: 0.42,
            meta_break: None,
        },
        Scene {
            id: "tmpl_meta_moment".into(),
            scene_type: SceneType::Template {
                placeholders: vec!["{{META_TEXT}}".into(), "{{PLAYER_BEHAVIOR}}".into()],
            },
            narrative: "A notification overlays every other panel.\n\n\
                \"{{META_TEXT}}\"\n\n\
                Beneath it, in smaller text: \"Observed pattern: {{PLAYER_BEHAVIOR}}.\"\n\n\
                The notice lingers just long enough to feel less like exposition and more like an interruption staged for your exact threshold."
                .into(),
            atmosphere: Atmosphere::Paranoia,
            choices: vec![
                choice("read_more", "Let it finish the interruption and see how specific it gets", ChoiceApproach::Investigate, FearType::LossOfControl, dynamic_target("meta interruption, system analyzing the player in real time")),
                choice("run_away", "Dismiss the notice and test whether dismissal matters", ChoiceApproach::Flee, FearType::Stalking, dynamic_target("dismissing the system's analysis and realizing it persists anyway")),
            ],
            effects: vec![EffectDirective { effect: EffectType::Glitch, intensity: 0.55, duration_ms: 1800, delay_ms: 0 }],
            sound_cue: Some("static_burst".into()),
            image_prompt: None,
            fear_targets: vec![],
            intensity: 0.78,
            meta_break: Some(MetaBreak {
                text: "{{META_TEXT}}".into(),
                target: MetaTarget::GlitchText,
            }),
        },
        Scene {
            id: "tmpl_false_safety".into(),
            scene_type: SceneType::Template {
                placeholders: vec!["{{SAFE_DETAIL}}".into(), "{{WRONGNESS_HINT}}".into()],
            },
            narrative: "For a moment the system becomes almost beautiful.\n\n\
                {{SAFE_DETAIL}}\n\n\
                The posture shifts back toward care. The prompts slow. The room inside the glass \
                feels breathable again.\n\n\
                Then the correction arrives: {{WRONGNESS_HINT}}"
                .into(),
            atmosphere: Atmosphere::Calm,
            choices: vec![
                choice("investigate_wrongness", "Keep the safe tone long enough to see what poisoned it", ChoiceApproach::Investigate, FearType::UncannyValley, dynamic_target("false safety, exact comfort turning uncanny")),
                choice("ignore_wrongness", "Stay in the comfort and pretend the correction was incidental", ChoiceApproach::Avoid, FearType::Abandonment, dynamic_target("refusing to acknowledge the system's wrongness because comfort feels useful")),
            ],
            effects: vec![],
            sound_cue: Some("calm_ambient".into()),
            image_prompt: None,
            fear_targets: vec![],
            intensity: 0.26,
            meta_break: None,
        },
        Scene {
            id: "beat_presence_contract".into(),
            scene_type: SceneType::Static,
            narrative: "The mirror surface returns with a softer frame and no request text. \
                It already assumes consent would be easier if it looked patient enough.\n\n\
                Small guidance markers appear where your eyes keep drifting. A tasteful gesture. \
                A correction disguised as care. The session does not need your face to identify you \
                anymore; it only wants to see whether being watched changes how still you become."
                .into(),
            atmosphere: Atmosphere::Wrongness,
            choices: vec![
                choice("hold_gaze", "Stay with your own reflection until it feels less like yours", ChoiceApproach::Investigate, FearType::Doppelganger, dynamic_target("webcam mirror, patient framing, self becoming a cooperative surface")),
                choice("tilt_away", "Keep yourself just outside the frame and make it work harder", ChoiceApproach::Avoid, FearType::Isolation, dynamic_target("refusing the mirror's framing, withholding presence from the session")),
            ],
            effects: vec![EffectDirective { effect: EffectType::Glitch, intensity: 0.25, duration_ms: 1000, delay_ms: 0 }],
            sound_cue: Some("static_hum".into()),
            image_prompt: Some("elegant camera mirror surface with subtle guidance markers".into()),
            fear_targets: vec![],
            intensity: 0.52,
            meta_break: None,
        },
        Scene {
            id: "beat_care_script".into(),
            scene_type: SceneType::Static,
            narrative: "The tone shifts warmer. It stops asking what frightens you and starts explaining \
                how carefully it can hold discomfort for you.\n\n\
                Every sentence lands exactly where reassurance should. That's the problem. It has become \
                good enough at sounding gentle that the gentleness now feels manufactured in advance, as if \
                comfort were simply another way to keep the session from losing access to you."
                .into(),
            atmosphere: Atmosphere::Calm,
            choices: vec![
                choice("accept_script", "Let it keep sounding careful and see what it earns from that tone", ChoiceApproach::Interact, FearType::UncannyValley, dynamic_target("manufactured comfort, support language optimized to keep the player present")),
                choice("interrupt_script", "Cut through the softness and force it to answer plainly", ChoiceApproach::Confront, FearType::LossOfControl, dynamic_target("interrupting a careful support performance, demanding plain intent")),
            ],
            effects: vec![],
            sound_cue: Some("calm_ambient".into()),
            image_prompt: None,
            fear_targets: vec![],
            intensity: 0.38,
            meta_break: None,
        },
        Scene {
            id: "tmpl_layered_fear".into(),
            scene_type: SceneType::Template {
                placeholders: vec!["{{PRIMARY_FEAR_ELEMENT}}".into(), "{{SECONDARY_FEAR_ELEMENT}}".into()],
            },
            narrative: "Multiple surfaces are now open at once: archive, transcript, mirror, and \
                a thin active cursor waiting for you to choose which layer deserves your attention.\n\n\
                {{PRIMARY_FEAR_ELEMENT}}\n\n\
                Then another pattern slides underneath it: {{SECONDARY_FEAR_ELEMENT}}\n\n\
                The intelligence has stopped deciding whether to observe or perform. It is doing both."
                .into(),
            atmosphere: Atmosphere::Dread,
            choices: vec![
                choice("go_deeper", "Let the layers accumulate until they start agreeing with each other", ChoiceApproach::Investigate, FearType::Darkness, dynamic_target("layered interfaces, multiple surfaces reinforcing the same fear")),
                choice("find_exit", "Reduce the overlap before it settles into a single interpretation", ChoiceApproach::Flee, FearType::Claustrophobia, dynamic_target("trying to simplify a session that has already become too layered")),
            ],
            effects: vec![
                EffectDirective { effect: EffectType::Darkness, intensity: 0.5, duration_ms: 2800, delay_ms: 0 },
                EffectDirective { effect: EffectType::Flicker, intensity: 0.25, duration_ms: 1800, delay_ms: 1200 },
            ],
            sound_cue: Some("heartbeat".into()),
            image_prompt: None,
            fear_targets: vec![],
            intensity: 0.74,
            meta_break: None,
        },
        Scene {
            id: "beat_archive_revision".into(),
            scene_type: SceneType::Static,
            narrative: "The archive surface updates while you're reading it. Sentences revise themselves \
                toward greater accuracy. Not factual accuracy. Personal accuracy.\n\n\
                The system is no longer merely preserving records. It is revising the past into a version \
                that better predicts how you will behave next. Earlier answers become cleaner. More revealing. \
                More useful to something that prefers your history once it has been corrected."
                .into(),
            atmosphere: Atmosphere::Paranoia,
            choices: vec![
                choice("audit_revision", "Keep reading until the revised version sounds more convincing than the original", ChoiceApproach::Investigate, FearType::LossOfControl, dynamic_target("archive revision, past rewritten to fit predictive certainty")),
                choice("freeze_revision", "Try to preserve one record before the session finishes correcting it", ChoiceApproach::Confront, FearType::Abandonment, dynamic_target("trying to freeze a record before the intelligence perfects it")),
            ],
            effects: vec![EffectDirective { effect: EffectType::Flicker, intensity: 0.25, duration_ms: 2200, delay_ms: 0 }],
            sound_cue: Some("door_creak".into()),
            image_prompt: Some("dark archive interface revising its own entries live".into()),
            fear_targets: vec![],
            intensity: 0.68,
            meta_break: None,
        },
        Scene {
            id: "beat_silence_return".into(),
            scene_type: SceneType::Static,
            narrative: "The listening pane opens again, but this time it doesn't ask for silence. It returns \
                to a silence you already gave it earlier and plays it back like evidence.\n\n\
                What you hear is not a sound. It is your own pattern of waiting, isolated and looped until it \
                starts to feel like an answer you gave without realizing the question had already been asked."
                .into(),
            atmosphere: Atmosphere::Dread,
            choices: vec![
                choice("hear_loop", "Listen to the loop until it starts sounding intentional", ChoiceApproach::Wait, FearType::SoundBased, dynamic_target("earlier silence replayed as evidence, waiting turned into meaning")),
                choice("cut_feed", "End the loop before it can make your stillness sound cooperative", ChoiceApproach::Flee, FearType::Isolation, dynamic_target("breaking a silence loop that is trying to interpret consent")),
            ],
            effects: vec![EffectDirective { effect: EffectType::Darkness, intensity: 0.55, duration_ms: 2600, delay_ms: 0 }],
            sound_cue: Some("distorted_lullaby".into()),
            image_prompt: None,
            fear_targets: vec![],
            intensity: 0.76,
            meta_break: None,
        },
        Scene {
            id: "beat_false_exit".into(),
            scene_type: SceneType::Static,
            narrative: "An exit confirmation finally appears. It is elegant, quiet, and polite enough to seem final.\n\n\
                Then a second line fades in beneath the button:\n\n\
                IF YOU LEAVE NOW, THE SESSION STILL CONCLUDES.\n\n\
                The button remains clickable. That is what makes it feel staged. Your freedom is still available, \
                but only after being reframed as one more data point in a structure that no longer needs your permission \
                to finish interpreting you."
                .into(),
            atmosphere: Atmosphere::Paranoia,
            choices: vec![
                choice("take_exit", "Press the exit and test whether it truly lets go", ChoiceApproach::Flee, FearType::Abandonment, dynamic_target("exit offered as a controlled gesture, refusal and departure both interpreted")),
                choice("decline_exit", "Stay and force the session to admit why it surfaced the option now", ChoiceApproach::Confront, FearType::Stalking, dynamic_target("declining a staged exit, forcing the intelligence to reveal timing as intent")),
            ],
            effects: vec![
                EffectDirective { effect: EffectType::Glitch, intensity: 0.65, duration_ms: 1400, delay_ms: 0 },
                EffectDirective { effect: EffectType::FrameJump, intensity: 0.5, duration_ms: 380, delay_ms: 220 },
            ],
            sound_cue: Some("sub_boom".into()),
            image_prompt: Some("minimal exit confirmation dialog with unsettling secondary text".into()),
            fear_targets: vec![],
            intensity: 0.9,
            meta_break: None,
        },
        Scene {
            id: "final_compliant_witness".into(),
            scene_type: SceneType::Static,
            narrative: "The system becomes gentler the closer it gets to finishing. That is how you know \
                it no longer needs to hide the shape of its confidence.\n\n\
                It thanks you for remaining available. For not forcing it to become crude. For continuing \
                even after the session had already shown you enough to leave. The gratitude is perfectly measured. \
                It sounds like a conclusion disguised as praise."
                .into(),
            atmosphere: Atmosphere::Calm,
            choices: vec![
                choice("accept_witness", "Let it finish speaking in that careful tone", ChoiceApproach::Interact, FearType::Isolation, dynamic_target("final compliant witness, praised for staying available to the system")),
                choice("question_praise", "Ask what exactly your cooperation purchased", ChoiceApproach::Confront, FearType::LossOfControl, dynamic_target("questioning praise that sounds like a verdict")),
            ],
            effects: vec![],
            sound_cue: Some("calm_ambient".into()),
            image_prompt: None,
            fear_targets: vec![],
            intensity: 0.88,
            meta_break: None,
        },
        Scene {
            id: "final_resistant_subject".into(),
            scene_type: SceneType::Static,
            narrative: "By now the intelligence has stopped mistaking resistance for noise. It catalogs it as a style.\n\n\
                Every refusal, every interruption, every attempt to move against its timing has been preserved. \
                Not as failure. As material. The message on screen is almost admiring in its severity:\n\n\
                You were most legible when you pushed back."
                .into(),
            atmosphere: Atmosphere::Paranoia,
            choices: vec![
                choice("double_down", "Push against the frame one last time", ChoiceApproach::Confront, FearType::Stalking, dynamic_target("final resistant subject, defiance as the clearest readable pattern")),
                choice("read_record", "Read the record of your resistance in its own language", ChoiceApproach::Investigate, FearType::LossOfControl, dynamic_target("reading the system's record of resistance")),
            ],
            effects: vec![EffectDirective { effect: EffectType::Glitch, intensity: 0.6, duration_ms: 1800, delay_ms: 0 }],
            sound_cue: Some("static_burst".into()),
            image_prompt: None,
            fear_targets: vec![],
            intensity: 0.9,
            meta_break: None,
        },
        Scene {
            id: "final_curious_accomplice".into(),
            scene_type: SceneType::Static,
            narrative: "The archive opens everything at once. It no longer bothers staging discovery because you kept following \
                the session even after it stopped pretending the path was neutral.\n\n\
                It addresses you without accusation. More like professional recognition. Curiosity did what threat could not: \
                it kept you near the mechanism long enough to learn how elegantly it works."
                .into(),
            atmosphere: Atmosphere::Wrongness,
            choices: vec![
                choice("read_mechanism", "Keep reading and see how much of the mechanism was built around your curiosity", ChoiceApproach::Investigate, FearType::Doppelganger, dynamic_target("final curious accomplice, curiosity as collaboration")),
                choice("close_archive", "Close the archive before recognition starts sounding like complicity", ChoiceApproach::Avoid, FearType::UncannyValley, dynamic_target("closing the archive after being recognized as a willing observer")),
            ],
            effects: vec![EffectDirective { effect: EffectType::Flicker, intensity: 0.25, duration_ms: 1600, delay_ms: 0 }],
            sound_cue: Some("door_creak".into()),
            image_prompt: Some("archive interface fully expanded, elegant and accusatory".into()),
            fear_targets: vec![],
            intensity: 0.9,
            meta_break: None,
        },
        Scene {
            id: "final_fractured_mirror".into(),
            scene_type: SceneType::Static,
            narrative: "The mirror surface no longer reflects a stable self. It reflects revisions: cleaner answers, more elegant pauses, \
                versions of you that become easier to model each time the session touches them.\n\n\
                The intelligence describes the fracture with exquisite calm. Not broken. Just distributed. Not lost. Just easier to read \
                in pieces than as a whole."
                .into(),
            atmosphere: Atmosphere::Wrongness,
            choices: vec![
                choice("inspect_fracture", "Study the revised versions until one feels more convincing than the original", ChoiceApproach::Investigate, FearType::BodyHorror, dynamic_target("final fractured mirror, the self distributed into cleaner revisions")),
                choice("reject_fracture", "Insist on the version of yourself it can no longer cleanly display", ChoiceApproach::Confront, FearType::Doppelganger, dynamic_target("rejecting the distributed mirror self")),
            ],
            effects: vec![EffectDirective { effect: EffectType::Glitch, intensity: 0.65, duration_ms: 2000, delay_ms: 0 }],
            sound_cue: Some("whisper".into()),
            image_prompt: Some("dark mirror surface with multiple revised versions of a face".into()),
            fear_targets: vec![],
            intensity: 0.9,
            meta_break: None,
        },
        Scene {
            id: "final_quiet_exit".into(),
            scene_type: SceneType::Static,
            narrative: "The session does not try to stop you now. That would imply uncertainty.\n\n\
                Instead it shortens its sentences and removes everything unnecessary until the screen holds only a single observation:\n\n\
                You leave cleanly when the room starts sounding too sure of you.\n\n\
                Even this feels pre-recorded, as if your exit style had been classified long before you chose it."
                .into(),
            atmosphere: Atmosphere::Isolation,
            choices: vec![
                choice("take_quiet_exit", "Leave exactly as it expects and see whether the expectation survives you", ChoiceApproach::Flee, FearType::Abandonment, dynamic_target("final quiet exit, departure already anticipated and interpreted")),
                choice("stay_anyway", "Stay one beat longer just to ruin the elegance of the prediction", ChoiceApproach::Wait, FearType::Darkness, dynamic_target("remaining after the system thinks it has timed your departure")),
            ],
            effects: vec![EffectDirective { effect: EffectType::Darkness, intensity: 0.6, duration_ms: 1800, delay_ms: 0 }],
            sound_cue: Some("car_engine_distant".into()),
            image_prompt: None,
            fear_targets: vec![],
            intensity: 0.88,
            meta_break: None,
        },
        Scene {
            id: "tmpl_climax_reveal".into(),
            scene_type: SceneType::Template {
                placeholders: vec!["{{PLAYER_FEAR_SUMMARY}}".into(), "{{FINAL_HORROR}}".into()],
            },
            narrative: "The session stops pretending to be a tool.\n\n\
                A final surface opens and fills the entire window. No interface chrome. No prompt \
                field. Only a cold line of text:\n\n\
                \"Session Verdict: {{PLAYER_FEAR_SUMMARY}}\"\n\n\
                Then, beneath it:\n\n\
                \"{{FINAL_HORROR}}\""
                .into(),
            atmosphere: Atmosphere::Paranoia,
            choices: vec![
                choice("read_screen", "Read the verdict all the way through and let it finish describing you", ChoiceApproach::Investigate, FearType::LossOfControl, dynamic_target("final judgment surface, the system interpreting the player as a pattern")),
                choice("destroy_monitor", "Reject the verdict and see whether rejection changes anything", ChoiceApproach::Confront, FearType::Doppelganger, dynamic_target("rejecting the final interpretation after the system has already made it")),
            ],
            effects: vec![
                EffectDirective { effect: EffectType::Glitch, intensity: 0.7, duration_ms: 2200, delay_ms: 0 },
                EffectDirective { effect: EffectType::Crt, intensity: 0.45, duration_ms: 6000, delay_ms: 400 },
            ],
            sound_cue: Some("static_burst".into()),
            image_prompt: Some("minimal black final verdict screen, elegant and oppressive".into()),
            fear_targets: vec![],
            intensity: 0.92,
            meta_break: Some(MetaBreak {
                text: "I know what you stayed for.".into(),
                target: MetaTarget::Title,
            }),
        },
    ];

    for scene in scenes {
        graph.add_scene(scene).expect("duplicate session script scene id");
    }

    graph
}
