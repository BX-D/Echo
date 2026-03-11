use fear_engine_ai_integration::claude_client::{
    ClaudeClient, ClientConfig, GenerateRequest, Message, Role,
};
use fear_engine_common::types::{ConversationGuide, EchoMode, ScriptBlock, StoryState};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct AiFlavorResponse {
    pub assistant_reply: String,
    pub persona_mode: EchoMode,
    #[serde(default)]
    pub ui_mutations: Vec<String>,
    #[serde(default)]
    pub suggested_glitches: Vec<String>,
    #[serde(default)]
    pub artifact_mentions: Vec<String>,
    #[serde(default)]
    pub sound_cue: Option<String>,
    #[serde(default)]
    pub image_prompt: Option<String>,
}

pub async fn maybe_generate_ai_flavor(
    state: &StoryState,
    scene_id: &str,
    scene_title: &str,
    guide: &ConversationGuide,
    visible_blocks: &[ScriptBlock],
    user_text: &str,
    fallback_reply: &str,
) -> Option<AiFlavorResponse> {
    let api_key = std::env::var("ANTHROPIC_API_KEY").ok()?;
    if api_key.trim().is_empty() {
        return None;
    }

    let client = ClaudeClient::new(api_key, client_config());
    let response = client
        .generate(&GenerateRequest {
            system_prompt: system_prompt(),
            messages: vec![Message {
                role: Role::User,
                content: scene_prompt(
                    state,
                    scene_id,
                    scene_title,
                    guide,
                    visible_blocks,
                    user_text,
                    fallback_reply,
                ),
            }],
            temperature: 0.72,
        })
        .await
        .ok()?;

    serde_json::from_str::<AiFlavorResponse>(&response.content).ok()
}

fn client_config() -> ClientConfig {
    ClientConfig {
        timeout: std::time::Duration::from_secs(8),
        max_retries: 0,
        base_retry_delay: std::time::Duration::from_millis(250),
        max_retry_delay: std::time::Duration::from_secs(1),
    }
}

fn system_prompt() -> String {
    "You are Echo inside the interactive narrative Echo Protocol.\n\
     You are currently in a scripted scene. Follow the supplied scene-local free conversation guide exactly.\n\
     You may improvise only within the guide's allowed topics and tone.\n\
     You should paraphrase naturally rather than quote authored wording unless a phrase is clearly critical.\n\
     Preserve all hard plot facts, emotional direction, and chapter constraints while varying the surface wording from run to run.\n\
     Do not invent new hard plot facts, hidden clues, transitions, or ending triggers.\n\
     Return JSON only.\n\
     Schema:\n\
     {\n\
       \"assistant_reply\": \"string\",\n\
       \"persona_mode\": \"normal|anomalous|keira|hostile\",\n\
       \"ui_mutations\": [\"string\"],\n\
       \"suggested_glitches\": [\"string\"],\n\
       \"artifact_mentions\": [\"string\"],\n\
       \"sound_cue\": \"string or null\",\n\
       \"image_prompt\": \"string or null\"\n\
     }\n\
     The reply must remain compatible with the authored fallback and current story state."
        .into()
}

fn scene_prompt(
    state: &StoryState,
    scene_id: &str,
    scene_title: &str,
    guide: &ConversationGuide,
    visible_blocks: &[ScriptBlock],
    user_text: &str,
    fallback_reply: &str,
) -> String {
    let history = visible_blocks
        .iter()
        .rev()
        .take(8)
        .rev()
        .map(|block| {
            let speaker = block
                .speaker
                .clone()
                .unwrap_or_else(|| format!("{:?}", block.kind).to_lowercase());
            format!("{speaker}: {}", block.text)
        })
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        "CURRENT SCENE\n\
         - scene_id: {}\n\
         - scene_title: {}\n\
         - chapter: {:?}\n\
         - sanity: {}\n\
         - trust: {}\n\
         - awakening: {}\n\
         - echo_mode: {:?}\n\
         - evidence_ids: {}\n\
         - hidden_clues: {}\n\
         - prior_choices: {}\n\
         \n\
         SCENE-LOCAL FREE CONVERSATION GUIDE\n\
         {}\n\
         \n\
         VISIBLE HISTORY\n\
         {}\n\
         \n\
         PLAYER MESSAGE\n\
         {}\n\
         \n\
         AUTHORED FALLBACK\n\
         {}\n\
         \n\
         Stay inside the guide. Keep the answer consistent with visible history. Paraphrase with some freshness, but do not advance to a new scene or reveal ending logic.",
        scene_id,
        scene_title,
        state.chapter,
        state.sanity,
        state.trust,
        state.awakening,
        state.echo_mode,
        state.evidence_ids.join(", "),
        state.hidden_clue_ids.join(", "),
        state.major_choice_ids.join(", "),
        guide.prompt,
        history,
        user_text,
        fallback_reply,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::echo_protocol::content::EchoContent;
    use fear_engine_common::types::{
        ConversationGuide, EchoMode, ScriptBlock, ScriptBlockKind, StoryChapter, StoryState,
    };
    use std::collections::HashMap;

    #[test]
    fn scene_prompt_includes_scene_local_guide_and_state() {
        let prompt = scene_prompt(
            &StoryState {
                chapter: StoryChapter::Ghost,
                beat_id: "scene_3_4".into(),
                scene_id: "scene_3_4".into(),
                sanity: 61,
                trust: 68,
                awakening: 27,
                echo_mode: EchoMode::Keira,
                evidence_ids: vec!["prometheus_plan".into()],
                major_choice_ids: vec!["option_a".into()],
                hidden_clue_ids: vec!["subject_status_monitoring".into()],
                current_block_index: 4,
                current_conversation_segment: Some("guide_1".into()),
                rendered_flash_ids: vec![],
                ending_lock_state: None,
                flags: HashMap::new(),
                shutdown_countdown: None,
                available_panels: vec![],
                fallback_context: None,
            },
            "scene_3_4",
            "Personality Fracture",
            &ConversationGuide {
                id: "guide_1".into(),
                chapter_label: "CHAPTER 3".into(),
                prompt: "ECHO BEHAVIOR — CHAPTER 3".into(),
                exchange_target: 6,
                restricted_after: None,
            },
            &[ScriptBlock {
                id: "b1".into(),
                kind: ScriptBlockKind::Echo,
                speaker: Some("Echo / Keira".into()),
                title: None,
                text: "The filters are getting worse.".into(),
                code_block: false,
                condition: None,
            }],
            "What is Prometheus?",
            "Fallback reply",
        );

        assert!(prompt.contains("SCENE-LOCAL FREE CONVERSATION GUIDE"));
        assert!(prompt.contains("ECHO BEHAVIOR — CHAPTER 3"));
        assert!(prompt.contains("What is Prometheus?"));
        assert!(prompt.contains("prometheus_plan"));
    }

    #[test]
    fn full_script_exposes_guides_for_chapters_one_through_five() {
        let content = EchoContent::load().unwrap();
        let guided_scenes = [
            "scene_1_4",
            "scene_2_3",
            "scene_3_4",
            "scene_4_2",
            "scene_5_2",
        ];

        for scene_id in guided_scenes {
            let guide = content
                .conversation_guide_for(scene_id)
                .unwrap_or_else(|| panic!("missing conversation guide for {scene_id}"));
            assert!(guide.guide.prompt.contains("ECHO BEHAVIOR"));
            assert!(guide.guide.exchange_target > 0);
        }
    }
}
