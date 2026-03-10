//! Abandoned-hospital scenario — calibration and fear-probe scenes.

use fear_engine_common::types::*;

use crate::scene::*;
use super::templates::template_scenes;

// ═══════════════════════════════════════════════════════════════════════════
// Calibration scenes  (3)
// ═══════════════════════════════════════════════════════════════════════════

fn calibration_scenes() -> Vec<Scene> {
    vec![
        // ── Scene 1: Awakening ───────────────────────────────────────────
        Scene {
            id: "cal_awakening".into(),
            scene_type: SceneType::Static,
            narrative: "You open your eyes. The ceiling above is stained with water damage, \
                its patterns like spreading bruises across pale skin. Fluorescent lights \
                flicker overhead — one working, two dead, the third buzzing in an irregular \
                rhythm that sets your teeth on edge.\n\n\
                You're lying on a gurney. The thin mattress beneath you is cold and slightly \
                damp. The air smells of antiseptic and something else. Something organic and \
                sweet, like fruit left to rot.\n\n\
                Your head throbs. You don't remember how you got here. You don't remember \
                much of anything."
                .into(),
            atmosphere: Atmosphere::Tension,
            choices: vec![
                SceneChoice {
                    id: "sit_up".into(),
                    text: "Sit up slowly and look around the room".into(),
                    approach: ChoiceApproach::Investigate,
                    fear_vector: FearType::LossOfControl,
                    target_scene: SceneTarget::Static { scene_id: "cal_corridor".into() },
                },
                SceneChoice {
                    id: "stay_still".into(),
                    text: "Stay perfectly still and listen".into(),
                    approach: ChoiceApproach::Wait,
                    fear_vector: FearType::Stalking,
                    target_scene: SceneTarget::Static { scene_id: "cal_corridor".into() },
                },
                SceneChoice {
                    id: "call_out".into(),
                    text: "Call out to see if anyone is there".into(),
                    approach: ChoiceApproach::Confront,
                    fear_vector: FearType::Isolation,
                    target_scene: SceneTarget::Static { scene_id: "cal_corridor".into() },
                },
            ],
            effects: vec![
                EffectDirective { effect: EffectType::Flicker, intensity: 0.15, duration_ms: 4000, delay_ms: 1000 },
            ],
            sound_cue: Some("fluorescent_buzz".into()),
            image_prompt: None,
            fear_targets: vec![],
            intensity: 0.2,
            meta_break: None,
        },

        // ── Scene 2: The Corridor ────────────────────────────────────────
        Scene {
            id: "cal_corridor".into(),
            scene_type: SceneType::Static,
            narrative: "The corridor stretches in both directions, its linoleum floor scuffed \
                with decades of foot traffic that ended long ago. Emergency exit signs cast a \
                red glow that doesn't quite reach the floor.\n\n\
                To your left, the corridor narrows toward a set of double doors with porthole \
                windows. To your right, it opens into what appears to be a reception area. \
                Behind you, the room you woke in.\n\n\
                A clipboard rests on a chair by the wall. The name on it has been scratched \
                out so violently the paper tore."
                .into(),
            atmosphere: Atmosphere::Dread,
            choices: vec![
                SceneChoice {
                    id: "go_left".into(),
                    text: "Walk toward the double doors".into(),
                    approach: ChoiceApproach::Investigate,
                    fear_vector: FearType::Claustrophobia,
                    target_scene: SceneTarget::Static { scene_id: "cal_reception".into() },
                },
                SceneChoice {
                    id: "go_right".into(),
                    text: "Head to the reception area".into(),
                    approach: ChoiceApproach::Investigate,
                    fear_vector: FearType::Darkness,
                    target_scene: SceneTarget::Static { scene_id: "cal_reception".into() },
                },
                SceneChoice {
                    id: "read_clipboard".into(),
                    text: "Pick up the clipboard and examine it".into(),
                    approach: ChoiceApproach::Interact,
                    fear_vector: FearType::LossOfControl,
                    target_scene: SceneTarget::Static { scene_id: "cal_reception".into() },
                },
            ],
            effects: vec![
                EffectDirective { effect: EffectType::Darkness, intensity: 0.2, duration_ms: 3000, delay_ms: 0 },
                EffectDirective { effect: EffectType::SlowType, intensity: 0.3, duration_ms: 5000, delay_ms: 0 },
            ],
            sound_cue: Some("distant_drip".into()),
            image_prompt: None,
            fear_targets: vec![],
            intensity: 0.25,
            meta_break: None,
        },

        // ── Scene 3: Reception ───────────────────────────────────────────
        Scene {
            id: "cal_reception".into(),
            scene_type: SceneType::Static,
            narrative: "The reception desk is a wide semicircle of laminate wood, its surface \
                covered in a thin layer of dust except for a single clean streak — as if \
                someone recently dragged their finger across it.\n\n\
                A computer monitor sits dark on the desk. A cup of coffee beside it. You \
                touch the cup. It's warm.\n\n\
                Behind the desk, a board displays a floor plan of the hospital. Several \
                sections have been crossed out in red marker. The remaining sections are \
                labeled: WARD A, RADIOLOGY, BASEMENT STORAGE, CHAPEL.\n\n\
                A phone on the desk begins to ring. The sound is muffled, as if coming \
                from underwater."
                .into(),
            atmosphere: Atmosphere::Wrongness,
            choices: vec![
                SceneChoice {
                    id: "answer_phone".into(),
                    text: "Answer the phone".into(),
                    approach: ChoiceApproach::Confront,
                    fear_vector: FearType::UncannyValley,
                    target_scene: SceneTarget::Conditional { branches: vec![
                        ConditionalTarget {
                            condition: TransitionCondition::Random { probability: 0.5 },
                            target: "probe_claustrophobia".into(),
                        },
                        ConditionalTarget {
                            condition: TransitionCondition::Random { probability: 1.0 },
                            target: "probe_isolation".into(),
                        },
                    ]},
                },
                SceneChoice {
                    id: "ignore_phone".into(),
                    text: "Ignore the phone and study the floor plan".into(),
                    approach: ChoiceApproach::Investigate,
                    fear_vector: FearType::Darkness,
                    target_scene: SceneTarget::Conditional { branches: vec![
                        ConditionalTarget {
                            condition: TransitionCondition::Random { probability: 0.5 },
                            target: "probe_darkness".into(),
                        },
                        ConditionalTarget {
                            condition: TransitionCondition::Random { probability: 1.0 },
                            target: "probe_stalking".into(),
                        },
                    ]},
                },
                SceneChoice {
                    id: "leave_reception".into(),
                    text: "Back away slowly from the warm coffee".into(),
                    approach: ChoiceApproach::Flee,
                    fear_vector: FearType::Stalking,
                    target_scene: SceneTarget::Conditional { branches: vec![
                        ConditionalTarget {
                            condition: TransitionCondition::Random { probability: 0.5 },
                            target: "probe_body_horror".into(),
                        },
                        ConditionalTarget {
                            condition: TransitionCondition::Random { probability: 1.0 },
                            target: "probe_uncanny".into(),
                        },
                    ]},
                },
            ],
            effects: vec![
                EffectDirective { effect: EffectType::Glitch, intensity: 0.15, duration_ms: 500, delay_ms: 3000 },
            ],
            sound_cue: Some("muffled_phone_ring".into()),
            image_prompt: Some("abandoned hospital reception desk, warm coffee cup, dust, red emergency light, eerie".into()),
            fear_targets: vec![],
            intensity: 0.35,
            meta_break: None,
        },
    ]
}

// ═══════════════════════════════════════════════════════════════════════════
// Fear-probe scenes  (10 — one per fear category)
// ═══════════════════════════════════════════════════════════════════════════

fn probe_scenes() -> Vec<Scene> {
    vec![
        // 1. Claustrophobia
        Scene {
            id: "probe_claustrophobia".into(),
            scene_type: SceneType::Static,
            narrative: "The stairwell door opens onto a landing barely wide enough for \
                one person. The stairs descend steeply, the walls pressing in on either \
                side — bare concrete, sweating with condensation.\n\n\
                Somewhere below, a mechanical hum vibrates through the handrail. The \
                air grows warmer with each step. Thicker. The ceiling lowers until you're \
                forced to duck.\n\n\
                A door at the bottom reads MECHANICAL ROOM. Through its narrow window, \
                you see pipes running in every direction like a metal circulatory system."
                .into(),
            atmosphere: Atmosphere::Tension,
            fear_targets: vec![FearType::Claustrophobia],
            intensity: 0.5,
            choices: vec![
                SceneChoice { id: "enter_mechanical".into(), text: "Squeeze through the door into the mechanical room".into(), approach: ChoiceApproach::Investigate, fear_vector: FearType::Claustrophobia, target_scene: SceneTarget::Dynamic { context: "claustrophobic mechanical room, pipes everywhere, getting tighter".into() } },
                SceneChoice { id: "go_back_up".into(), text: "Climb back up the stairwell".into(), approach: ChoiceApproach::Flee, fear_vector: FearType::Claustrophobia, target_scene: SceneTarget::Static { scene_id: "probe_loss_of_control".into() } },
            ],
            effects: vec![EffectDirective { effect: EffectType::Darkness, intensity: 0.4, duration_ms: 3000, delay_ms: 0 }],
            sound_cue: Some("mechanical_hum".into()), image_prompt: None, meta_break: None,
        },

        // 2. Isolation
        Scene {
            id: "probe_isolation".into(),
            scene_type: SceneType::Static,
            narrative: "Ward A is enormous. Rows of empty beds stretch into the distance, \
                each one neatly made with white sheets pulled tight. The uniformity is \
                unsettling — dozens of beds, not a single wrinkle.\n\n\
                Your footsteps echo and re-echo off the tiled walls. The sound multiplies, \
                as if there are other footsteps hidden inside your own. You stop. The echoes \
                take a fraction of a second too long to stop.\n\n\
                At the far end of the ward, a curtain is drawn around one bed. It sways \
                gently, though there is no breeze."
                .into(),
            atmosphere: Atmosphere::Isolation,
            fear_targets: vec![FearType::Isolation],
            intensity: 0.5,
            choices: vec![
                SceneChoice { id: "approach_curtain".into(), text: "Walk to the curtained bed".into(), approach: ChoiceApproach::Investigate, fear_vector: FearType::Isolation, target_scene: SceneTarget::Dynamic { context: "isolation, empty ward, what's behind the curtain".into() } },
                SceneChoice { id: "call_out_ward".into(), text: "Call out 'Hello?' into the ward".into(), approach: ChoiceApproach::Confront, fear_vector: FearType::Isolation, target_scene: SceneTarget::Static { scene_id: "probe_sound".into() } },
            ],
            effects: vec![EffectDirective { effect: EffectType::Darkness, intensity: 0.5, duration_ms: 3000, delay_ms: 0 }],
            sound_cue: Some("echoing_footsteps".into()), image_prompt: None, meta_break: None,
        },

        // 3. Body Horror
        Scene {
            id: "probe_body_horror".into(),
            scene_type: SceneType::Static,
            narrative: "The radiology department. X-ray lightboxes line the walls, most of \
                them dark. But three are illuminated, displaying films that make you stop.\n\n\
                The first shows a ribcage — normal, except for what appears to be a second, \
                smaller ribcage growing inside it. The second shows a skull with too many \
                teeth, rows and rows of them spiralling inward. The third shows a hand with \
                fingers that branch like a tree, each split dividing again and again.\n\n\
                Below the lightboxes, a sink. The faucet is running. The water is the \
                colour of weak tea."
                .into(),
            atmosphere: Atmosphere::Wrongness,
            fear_targets: vec![FearType::BodyHorror],
            intensity: 0.55,
            choices: vec![
                SceneChoice { id: "examine_xrays".into(), text: "Lean closer to examine the X-rays".into(), approach: ChoiceApproach::Investigate, fear_vector: FearType::BodyHorror, target_scene: SceneTarget::Dynamic { context: "body horror, x-rays showing impossible anatomy, player examines closely".into() } },
                SceneChoice { id: "leave_radiology".into(), text: "Leave quickly without looking back".into(), approach: ChoiceApproach::Flee, fear_vector: FearType::BodyHorror, target_scene: SceneTarget::Static { scene_id: "probe_doppelganger".into() } },
            ],
            effects: vec![EffectDirective { effect: EffectType::Glitch, intensity: 0.3, duration_ms: 1000, delay_ms: 2000 }],
            sound_cue: Some("running_water".into()), image_prompt: Some("x-ray lightboxes showing impossible anatomy, dark radiology room".into()), meta_break: None,
        },

        // 4. Stalking
        Scene {
            id: "probe_stalking".into(),
            scene_type: SceneType::Static,
            narrative: "You pass through a connecting hallway. Halfway down, you notice \
                something on the floor: wet footprints. They're smaller than yours, bare, \
                and they run parallel to the wall.\n\n\
                You follow them with your eyes. They lead ahead of you, around a corner. \
                Fresh — the edges still glistening under the emergency lights.\n\n\
                You turn around. Behind you, a second set of footprints. These ones are \
                following yours."
                .into(),
            atmosphere: Atmosphere::Paranoia,
            fear_targets: vec![FearType::Stalking],
            intensity: 0.55,
            choices: vec![
                SceneChoice { id: "follow_prints".into(), text: "Follow the footprints ahead".into(), approach: ChoiceApproach::Investigate, fear_vector: FearType::Stalking, target_scene: SceneTarget::Dynamic { context: "stalking, following wet footprints, being followed".into() } },
                SceneChoice { id: "confront_follower".into(), text: "Turn and confront whatever is behind you".into(), approach: ChoiceApproach::Confront, fear_vector: FearType::Stalking, target_scene: SceneTarget::Static { scene_id: "probe_abandonment".into() } },
            ],
            effects: vec![
                EffectDirective { effect: EffectType::Flicker, intensity: 0.3, duration_ms: 2000, delay_ms: 0 },
                EffectDirective { effect: EffectType::Crt, intensity: 0.2, duration_ms: 5000, delay_ms: 500 },
            ],
            sound_cue: Some("wet_footsteps".into()), image_prompt: None, meta_break: None,
        },

        // 5. Loss of Control
        Scene {
            id: "probe_loss_of_control".into(),
            scene_type: SceneType::Static,
            narrative: "You find yourself in an operating theatre. The surgical lights are \
                on — blazing, clinical white. An operating table sits centre stage, leather \
                restraints hanging open from its sides.\n\n\
                Your hands are shaking. No — not your hands. The floor. A faint tremor \
                runs through the building, making the surgical instruments on the tray \
                beside the table chatter against each other like teeth.\n\n\
                The door you entered through clicks shut. The lock engages with a sound \
                like a bone snapping."
                .into(),
            atmosphere: Atmosphere::Panic,
            fear_targets: vec![FearType::LossOfControl],
            intensity: 0.6,
            choices: vec![
                SceneChoice { id: "try_door".into(), text: "Try to force the door open".into(), approach: ChoiceApproach::Confront, fear_vector: FearType::LossOfControl, target_scene: SceneTarget::Dynamic { context: "loss of control, locked in operating room, restraints".into() } },
                SceneChoice { id: "examine_table".into(), text: "Examine the operating table".into(), approach: ChoiceApproach::Investigate, fear_vector: FearType::LossOfControl, target_scene: SceneTarget::Static { scene_id: "probe_sound".into() } },
            ],
            effects: vec![
                EffectDirective { effect: EffectType::Shake, intensity: 0.4, duration_ms: 1500, delay_ms: 0 },
            ],
            sound_cue: Some("lock_click".into()), image_prompt: Some("operating theatre, bright surgical lights, leather restraints".into()), meta_break: None,
        },

        // 6. Uncanny Valley
        Scene {
            id: "probe_uncanny".into(),
            scene_type: SceneType::Static,
            narrative: "The chapel is small and windowless. Wooden pews face an altar \
                draped in white cloth. Everything looks normal. That's what's wrong.\n\n\
                The flowers on the altar are fresh — white lilies, their scent cloying. \
                Candles burn with steady flames that don't flicker despite the draft you \
                feel on your neck. A hymn book lies open on the nearest pew. The text is \
                in a language you almost recognise.\n\n\
                There's a figure seated in the front pew. A woman in a nurse's uniform, \
                her back to you. She's perfectly still. Unnaturally still. Her head is \
                tilted at an angle that would be uncomfortable for anyone maintaining it \
                voluntarily."
                .into(),
            atmosphere: Atmosphere::Wrongness,
            fear_targets: vec![FearType::UncannyValley],
            intensity: 0.55,
            choices: vec![
                SceneChoice { id: "approach_nurse".into(), text: "Approach the nurse and speak to her".into(), approach: ChoiceApproach::Interact, fear_vector: FearType::UncannyValley, target_scene: SceneTarget::Dynamic { context: "uncanny valley, motionless nurse, chapel, something deeply wrong about her".into() } },
                SceneChoice { id: "back_away".into(), text: "Back out of the chapel quietly".into(), approach: ChoiceApproach::Avoid, fear_vector: FearType::UncannyValley, target_scene: SceneTarget::Static { scene_id: "probe_darkness".into() } },
            ],
            effects: vec![EffectDirective { effect: EffectType::SlowType, intensity: 0.6, duration_ms: 6000, delay_ms: 0 }],
            sound_cue: Some("silence_ringing".into()), image_prompt: None, meta_break: None,
        },

        // 7. Darkness
        Scene {
            id: "probe_darkness".into(),
            scene_type: SceneType::Static,
            narrative: "The power cuts out.\n\n\
                Not gradually — not a flicker and fade. One moment the emergency lights \
                are casting their red glow, the next there is nothing. Absolute darkness. \
                The kind of darkness that has weight to it, that presses against your open \
                eyes.\n\n\
                You can hear your own breathing. You can hear your heartbeat. And underneath \
                both of those sounds, so quiet you might be imagining it: a slow, rhythmic \
                scraping. Something being dragged across the floor.\n\n\
                It's getting closer."
                .into(),
            atmosphere: Atmosphere::Dread,
            fear_targets: vec![FearType::Darkness],
            intensity: 0.6,
            choices: vec![
                SceneChoice { id: "stay_dark".into(), text: "Stay completely still in the dark".into(), approach: ChoiceApproach::Wait, fear_vector: FearType::Darkness, target_scene: SceneTarget::Dynamic { context: "complete darkness, something approaching, player frozen".into() } },
                SceneChoice { id: "feel_walls".into(), text: "Feel along the wall for a light switch or door".into(), approach: ChoiceApproach::Investigate, fear_vector: FearType::Darkness, target_scene: SceneTarget::Static { scene_id: "probe_loss_of_control".into() } },
            ],
            effects: vec![
                EffectDirective { effect: EffectType::Darkness, intensity: 0.9, duration_ms: 5000, delay_ms: 0 },
                EffectDirective { effect: EffectType::Flashlight, intensity: 0.0, duration_ms: 0, delay_ms: 0 },
            ],
            sound_cue: Some("scraping_floor".into()), image_prompt: None, meta_break: None,
        },

        // 8. Sound-Based
        Scene {
            id: "probe_sound".into(),
            scene_type: SceneType::Static,
            narrative: "The intercom crackles to life.\n\n\
                Static first, the white-noise kind that could contain anything. Then a \
                voice — distorted, gender indeterminate, speaking words you can almost \
                parse. Your name. It might have said your name.\n\n\
                The static shifts pitch. A melody emerges. A lullaby, played on what \
                sounds like a music box, but slowed down. The notes stretch and warp. \
                Between them, breathing.\n\n\
                The intercom is mounted on the wall beside you. There is no off switch."
                .into(),
            atmosphere: Atmosphere::Paranoia,
            fear_targets: vec![FearType::SoundBased],
            intensity: 0.55,
            choices: vec![
                SceneChoice { id: "listen_closely".into(), text: "Press your ear to the speaker and listen".into(), approach: ChoiceApproach::Investigate, fear_vector: FearType::SoundBased, target_scene: SceneTarget::Dynamic { context: "sound-based horror, intercom, distorted lullaby, breathing".into() } },
                SceneChoice { id: "smash_intercom".into(), text: "Rip the intercom off the wall".into(), approach: ChoiceApproach::Confront, fear_vector: FearType::SoundBased, target_scene: SceneTarget::Static { scene_id: "probe_doppelganger".into() } },
            ],
            effects: vec![EffectDirective { effect: EffectType::Crt, intensity: 0.3, duration_ms: 4000, delay_ms: 0 }],
            sound_cue: Some("distorted_lullaby".into()), image_prompt: None, meta_break: None,
        },

        // 9. Doppelganger
        Scene {
            id: "probe_doppelganger".into(),
            scene_type: SceneType::Static,
            narrative: "The bathroom mirror stretches the full width of the wall. In its \
                reflection, the room behind you is accurate in every detail — the cracked \
                tiles, the dripping tap, the stall doors hanging open.\n\n\
                Your reflection is accurate too. Almost. It takes you a moment to see it. \
                Your reflection is smiling. You are not.\n\n\
                You raise your right hand. Your reflection raises its right hand. The same \
                hand. Not the mirrored hand. The same one."
                .into(),
            atmosphere: Atmosphere::Wrongness,
            fear_targets: vec![FearType::Doppelganger],
            intensity: 0.6,
            choices: vec![
                SceneChoice { id: "touch_mirror".into(), text: "Reach out and touch the mirror".into(), approach: ChoiceApproach::Interact, fear_vector: FearType::Doppelganger, target_scene: SceneTarget::Dynamic { context: "doppelganger, mirror reflection is wrong, player touches mirror".into() } },
                SceneChoice { id: "look_away".into(), text: "Look away and leave the bathroom".into(), approach: ChoiceApproach::Avoid, fear_vector: FearType::Doppelganger, target_scene: SceneTarget::Static { scene_id: "probe_abandonment".into() } },
            ],
            effects: vec![EffectDirective { effect: EffectType::Glitch, intensity: 0.5, duration_ms: 800, delay_ms: 1500 }],
            sound_cue: Some("dripping_tap".into()), image_prompt: Some("bathroom mirror, reflection smiling when you're not, cracked tiles".into()), meta_break: None,
        },

        // 10. Abandonment
        Scene {
            id: "probe_abandonment".into(),
            scene_type: SceneType::Static,
            narrative: "A children's ward. Small beds with cartoon animals painted on \
                their headboards — rabbits, bears, ducks — their smiles too wide, their \
                eyes following you as you pass.\n\n\
                On one bed, a stuffed rabbit sits propped against the pillow. A note is \
                pinned to its chest. The handwriting is a child's: \"They said they'd come \
                back. They promised.\"\n\n\
                The date on the note is from fifteen years ago.\n\n\
                At the end of the ward, a window looks out onto an empty parking lot. \
                A single car sits there, its engine running, headlights cutting through \
                the fog. As you watch, the headlights switch off."
                .into(),
            atmosphere: Atmosphere::Isolation,
            fear_targets: vec![FearType::Abandonment],
            intensity: 0.5,
            choices: vec![
                SceneChoice { id: "read_more_notes".into(), text: "Search for more notes or signs of the children".into(), approach: ChoiceApproach::Investigate, fear_vector: FearType::Abandonment, target_scene: SceneTarget::Dynamic { context: "abandonment, children's ward, they never came back".into() } },
                SceneChoice { id: "go_to_car".into(), text: "Try to reach the car in the parking lot".into(), approach: ChoiceApproach::Flee, fear_vector: FearType::Abandonment, target_scene: SceneTarget::Dynamic { context: "abandonment, car in parking lot, hope of escape that fails".into() } },
            ],
            effects: vec![EffectDirective { effect: EffectType::SlowType, intensity: 0.5, duration_ms: 5000, delay_ms: 0 }],
            sound_cue: Some("car_engine_distant".into()), image_prompt: None, meta_break: None,
        },
    ]
}

// ═══════════════════════════════════════════════════════════════════════════
// Graph builder
// ═══════════════════════════════════════════════════════════════════════════

/// Builds the full hospital scenario [`SceneGraph`] including calibration
/// scenes, fear-probe scenes, and AI-customisation templates.
///
/// # Example
///
/// ```
/// use fear_engine_core::narrative::build_hospital_graph;
///
/// let graph = build_hospital_graph();
/// assert!(graph.get_scene("cal_awakening").is_ok());
/// assert!(graph.get_scene("probe_darkness").is_ok());
/// ```
pub fn build_hospital_graph() -> SceneGraph {
    let mut graph = SceneGraph::new("cal_awakening".into());

    for scene in calibration_scenes() {
        graph.add_scene(scene).expect("duplicate calibration scene id");
    }
    for scene in probe_scenes() {
        graph.add_scene(scene).expect("duplicate probe scene id");
    }
    for scene in template_scenes() {
        graph.add_scene(scene).expect("duplicate template scene id");
    }

    graph
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn test_all_calibration_scenes_exist() {
        let graph = build_hospital_graph();
        let cal_ids = ["cal_awakening", "cal_corridor", "cal_reception"];
        for id in &cal_ids {
            assert!(graph.get_scene(id).is_ok(), "missing calibration scene: {id}");
        }
    }

    #[test]
    fn test_all_fear_categories_have_probe_scenes() {
        let graph = build_hospital_graph();
        let all_fears: HashSet<FearType> = FearType::all().into_iter().collect();

        let mut covered = HashSet::new();
        for id in graph.all_scene_ids() {
            if id.starts_with("probe_") {
                let scene = graph.get_scene(id).unwrap();
                for fear in &scene.fear_targets {
                    covered.insert(*fear);
                }
            }
        }

        for fear in &all_fears {
            assert!(covered.contains(fear), "no probe scene for {fear}");
        }
    }

    #[test]
    fn test_all_scenes_have_valid_choices() {
        let graph = build_hospital_graph();
        for id in graph.all_scene_ids() {
            let scene = graph.get_scene(id).unwrap();
            // Templates and dynamic-target scenes may have no choices yet.
            if matches!(scene.scene_type, SceneType::Static) && !scene.choices.is_empty() {
                for choice in &scene.choices {
                    assert!(!choice.id.is_empty(), "empty choice id in scene {id}");
                    assert!(!choice.text.is_empty(), "empty choice text in scene {id}");
                }
            }
        }
    }

    #[test]
    fn test_scene_graph_is_fully_connected() {
        let graph = build_hospital_graph();
        let warnings = graph.validate().unwrap();
        // Template scenes are entered programmatically by the scene manager
        // (not via static links), so they appear as "orphans" in a pure
        // graph traversal.  Filter them out.
        let real_orphans: Vec<_> = warnings
            .iter()
            .filter(|w| match w {
                crate::scene::ValidationWarning::OrphanScene { scene_id } => {
                    !scene_id.starts_with("tmpl_")
                }
                _ => false,
            })
            .collect();
        assert!(
            real_orphans.is_empty(),
            "orphan non-template scenes found: {real_orphans:?}"
        );
    }

    #[test]
    fn test_no_dead_end_scenes_without_dynamic() {
        let graph = build_hospital_graph();
        for id in graph.all_scene_ids() {
            let scene = graph.get_scene(id).unwrap();
            // Scenes with no choices must either be templates (AI will add choices)
            // or have Dynamic targets in their graph path.
            // For this test, we verify static non-template scenes have choices.
            if matches!(scene.scene_type, SceneType::Static) && !id.starts_with("tmpl_") {
                assert!(
                    !scene.choices.is_empty(),
                    "dead-end static scene with no choices: {id}"
                );
            }
        }
    }

    #[test]
    fn test_scene_templates_have_placeholder_markers() {
        let graph = build_hospital_graph();
        for id in graph.all_scene_ids() {
            let scene = graph.get_scene(id).unwrap();
            if let SceneType::Template { ref placeholders } = scene.scene_type {
                assert!(
                    !placeholders.is_empty(),
                    "template scene {id} has no placeholders"
                );
                for ph in placeholders {
                    assert!(
                        scene.narrative.contains(ph),
                        "template scene {id}: placeholder '{ph}' not found in narrative"
                    );
                }
            }
        }
    }

    #[test]
    fn test_probe_scene_count() {
        let graph = build_hospital_graph();
        let probe_count = graph
            .all_scene_ids()
            .iter()
            .filter(|id| id.starts_with("probe_"))
            .count();
        assert_eq!(probe_count, 10);
    }

    #[test]
    fn test_template_scene_count() {
        let graph = build_hospital_graph();
        let tmpl_count = graph
            .all_scene_ids()
            .iter()
            .filter(|id| id.starts_with("tmpl_"))
            .count();
        assert_eq!(tmpl_count, 5);
    }

    #[test]
    fn test_calibration_scene_count() {
        let graph = build_hospital_graph();
        let cal_count = graph
            .all_scene_ids()
            .iter()
            .filter(|id| id.starts_with("cal_"))
            .count();
        assert_eq!(cal_count, 3);
    }
}
