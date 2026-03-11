//! Scene templates with placeholder markers for AI customisation.

use crate::scene::*;
use fear_engine_common::types::*;

/// Returns the five template scenes that the AI can customise at runtime
/// by replacing placeholder tokens in the narrative text.
pub fn template_scenes() -> Vec<Scene> {
    vec![
        // ── 1. Escalation — personalised fear room ───────────────────────
        Scene {
            id: "tmpl_fear_room".into(),
            scene_type: SceneType::Template {
                placeholders: vec!["{{FEAR_DESCRIPTION}}".into(), "{{SENSORY_DETAIL}}".into()],
            },
            narrative: "The door opens onto a room that seems built for you. Not built \
                *for* you — built *from* you. From something inside you.\n\n\
                {{FEAR_DESCRIPTION}}\n\n\
                {{SENSORY_DETAIL}}\n\n\
                The door behind you is still open. For now."
                .into(),
            atmosphere: Atmosphere::Dread,
            choices: vec![
                SceneChoice {
                    id: "explore_room".into(),
                    text: "Step further into the room".into(),
                    approach: ChoiceApproach::Investigate,
                    fear_vector: FearType::Darkness,
                    target_scene: SceneTarget::Dynamic {
                        context: "personalised fear room, escalating".into(),
                    },
                },
                SceneChoice {
                    id: "flee_room".into(),
                    text: "Turn and run".into(),
                    approach: ChoiceApproach::Flee,
                    fear_vector: FearType::LossOfControl,
                    target_scene: SceneTarget::Dynamic {
                        context: "fleeing personalised fear room, door closing".into(),
                    },
                },
            ],
            effects: vec![EffectDirective {
                effect: EffectType::Darkness,
                intensity: 0.5,
                duration_ms: 3000,
                delay_ms: 0,
            }],
            sound_cue: Some("heartbeat".into()),
            image_prompt: None,
            fear_targets: vec![],
            intensity: 0.7,
            meta_break: None,
        },
        // ── 2. Meta-horror — the AI addresses the player ────────────────
        Scene {
            id: "tmpl_meta_moment".into(),
            scene_type: SceneType::Template {
                placeholders: vec!["{{META_TEXT}}".into(), "{{PLAYER_BEHAVIOR}}".into()],
            },
            narrative: "The corridor ends at a wall. On the wall, someone has written \
                in what you hope is red paint:\n\n\
                \"{{META_TEXT}}\"\n\n\
                Below it, in smaller letters: \"I noticed {{PLAYER_BEHAVIOR}}.\"\n\n\
                The handwriting looks like your own."
                .into(),
            atmosphere: Atmosphere::Paranoia,
            choices: vec![
                SceneChoice {
                    id: "read_more".into(),
                    text: "Search the wall for more writing".into(),
                    approach: ChoiceApproach::Investigate,
                    fear_vector: FearType::LossOfControl,
                    target_scene: SceneTarget::Dynamic {
                        context: "meta-horror, AI breaking fourth wall, writing on wall".into(),
                    },
                },
                SceneChoice {
                    id: "run_away".into(),
                    text: "Don't read any more — just go".into(),
                    approach: ChoiceApproach::Flee,
                    fear_vector: FearType::Stalking,
                    target_scene: SceneTarget::Dynamic {
                        context: "meta-horror, fleeing the message, can't escape the AI".into(),
                    },
                },
            ],
            effects: vec![EffectDirective {
                effect: EffectType::Glitch,
                intensity: 0.6,
                duration_ms: 2000,
                delay_ms: 0,
            }],
            sound_cue: None,
            image_prompt: None,
            fear_targets: vec![],
            intensity: 0.8,
            meta_break: Some(MetaBreak {
                text: "{{META_TEXT}}".into(),
                target: MetaTarget::GlitchText,
            }),
        },
        // ── 3. Contrast — false safety before the storm ──────────────────
        Scene {
            id: "tmpl_false_safety".into(),
            scene_type: SceneType::Template {
                placeholders: vec!["{{SAFE_DETAIL}}".into(), "{{WRONGNESS_HINT}}".into()],
            },
            narrative: "You find a room that feels different. Warmer. Safer.\n\n\
                {{SAFE_DETAIL}}\n\n\
                For the first time since you woke up, the tension in your shoulders \
                begins to ease. You almost smile.\n\n\
                Then you notice it. {{WRONGNESS_HINT}}"
                .into(),
            atmosphere: Atmosphere::Calm,
            choices: vec![
                SceneChoice {
                    id: "investigate_wrongness".into(),
                    text: "Look more closely at what's wrong".into(),
                    approach: ChoiceApproach::Investigate,
                    fear_vector: FearType::UncannyValley,
                    target_scene: SceneTarget::Dynamic {
                        context: "false safety shattered, wrongness revealed".into(),
                    },
                },
                SceneChoice {
                    id: "ignore_wrongness".into(),
                    text: "Try to ignore it and rest here".into(),
                    approach: ChoiceApproach::Avoid,
                    fear_vector: FearType::Abandonment,
                    target_scene: SceneTarget::Dynamic {
                        context: "ignoring warning signs in false safe room".into(),
                    },
                },
            ],
            effects: vec![],
            sound_cue: Some("calm_ambient".into()),
            image_prompt: None,
            fear_targets: vec![],
            intensity: 0.2,
            meta_break: None,
        },
        // ── 4. Layering — combining two fears ────────────────────────────
        Scene {
            id: "tmpl_layered_fear".into(),
            scene_type: SceneType::Template {
                placeholders: vec![
                    "{{PRIMARY_FEAR_ELEMENT}}".into(),
                    "{{SECONDARY_FEAR_ELEMENT}}".into(),
                ],
            },
            narrative: "The basement storage room is vast and labyrinthine, shelves \
                stretching into darkness in every direction. It reminds you of something \
                you can't quite place.\n\n\
                {{PRIMARY_FEAR_ELEMENT}}\n\n\
                And then, layered on top of that, something worse: \
                {{SECONDARY_FEAR_ELEMENT}}"
                .into(),
            atmosphere: Atmosphere::Dread,
            choices: vec![
                SceneChoice {
                    id: "go_deeper".into(),
                    text: "Push deeper into the storage room".into(),
                    approach: ChoiceApproach::Investigate,
                    fear_vector: FearType::Darkness,
                    target_scene: SceneTarget::Dynamic {
                        context: "layered fears combining in basement, escalating".into(),
                    },
                },
                SceneChoice {
                    id: "find_exit".into(),
                    text: "Search for another exit".into(),
                    approach: ChoiceApproach::Flee,
                    fear_vector: FearType::Claustrophobia,
                    target_scene: SceneTarget::Dynamic {
                        context: "searching for escape from layered fears, basement".into(),
                    },
                },
            ],
            effects: vec![
                EffectDirective {
                    effect: EffectType::Darkness,
                    intensity: 0.6,
                    duration_ms: 4000,
                    delay_ms: 0,
                },
                EffectDirective {
                    effect: EffectType::Flicker,
                    intensity: 0.3,
                    duration_ms: 2000,
                    delay_ms: 2000,
                },
            ],
            sound_cue: Some("basement_ambient".into()),
            image_prompt: None,
            fear_targets: vec![],
            intensity: 0.75,
            meta_break: None,
        },
        // ── 5. Climax — the AI reveals itself ────────────────────────────
        Scene {
            id: "tmpl_climax_reveal".into(),
            scene_type: SceneType::Template {
                placeholders: vec!["{{PLAYER_FEAR_SUMMARY}}".into(), "{{FINAL_HORROR}}".into()],
            },
            narrative: "You reach the hospital's central atrium. Moonlight falls through \
                a shattered skylight, illuminating a circle on the floor.\n\n\
                In the circle, a monitor glows. On the screen, text scrolls:\n\n\
                \"Patient Profile: {{PLAYER_FEAR_SUMMARY}}\"\n\n\
                The screen flickers. New text appears:\n\n\
                \"{{FINAL_HORROR}}\"\n\n\
                The monitor turns to face you. You never touched it."
                .into(),
            atmosphere: Atmosphere::Paranoia,
            choices: vec![
                SceneChoice {
                    id: "read_screen".into(),
                    text: "Read the rest of the screen".into(),
                    approach: ChoiceApproach::Investigate,
                    fear_vector: FearType::LossOfControl,
                    target_scene: SceneTarget::Dynamic {
                        context: "climax, AI reveals what it learned, final horror personalised"
                            .into(),
                    },
                },
                SceneChoice {
                    id: "destroy_monitor".into(),
                    text: "Destroy the monitor".into(),
                    approach: ChoiceApproach::Confront,
                    fear_vector: FearType::Doppelganger,
                    target_scene: SceneTarget::Dynamic {
                        context: "climax, player fights back against the AI, monitor won't break"
                            .into(),
                    },
                },
            ],
            effects: vec![
                EffectDirective {
                    effect: EffectType::Glitch,
                    intensity: 0.7,
                    duration_ms: 3000,
                    delay_ms: 0,
                },
                EffectDirective {
                    effect: EffectType::Crt,
                    intensity: 0.5,
                    duration_ms: 8000,
                    delay_ms: 1000,
                },
            ],
            sound_cue: Some("static_burst".into()),
            image_prompt: Some(
                "hospital atrium, moonlight through broken skylight, glowing monitor".into(),
            ),
            fear_targets: vec![],
            intensity: 0.9,
            meta_break: Some(MetaBreak {
                text: "I know what you're afraid of.".into(),
                target: MetaTarget::Title,
            }),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_template_scenes_count() {
        assert_eq!(template_scenes().len(), 5);
    }

    #[test]
    fn test_all_templates_are_template_type() {
        for scene in template_scenes() {
            assert!(
                matches!(scene.scene_type, SceneType::Template { .. }),
                "scene {} is not a Template",
                scene.id
            );
        }
    }

    #[test]
    fn test_all_templates_have_choices() {
        for scene in template_scenes() {
            assert!(
                !scene.choices.is_empty(),
                "template {} has no choices",
                scene.id
            );
        }
    }
}
