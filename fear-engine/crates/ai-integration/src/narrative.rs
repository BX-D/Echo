//! Narrative generation pipeline — prompt assembly → API call → response
//! parsing → validation → scene creation, with fallback to template scenes.

use fear_engine_common::types::{
    Atmosphere, Choice, ChoiceApproach, FearType, MetaBreak, MetaTarget, NarrativeResponse,
};
use fear_engine_common::{FearEngineError, Result};
use serde_json::Value;

use crate::claude_client::{ClaudeClient, GenerateRequest, Message, Role};
use crate::prompt::context::PromptContext;
use crate::prompt::PromptBuilder;

// ---------------------------------------------------------------------------
// Pipeline
// ---------------------------------------------------------------------------

/// Orchestrates the full narrative generation flow.
///
/// # Example
///
/// ```no_run
/// use fear_engine_ai_integration::narrative::NarrativePipeline;
/// use fear_engine_ai_integration::claude_client::{ClaudeClient, ClientConfig};
///
/// let client = ClaudeClient::new("key".into(), ClientConfig::default());
/// let pipeline = NarrativePipeline::new(client);
/// ```
pub struct NarrativePipeline {
    client: ClaudeClient,
}

impl NarrativePipeline {
    /// Creates a new pipeline with the given Claude client.
    pub fn new(client: ClaudeClient) -> Self {
        Self { client }
    }

    /// Generates a narrative scene from the current game context.
    ///
    /// 1. Builds the prompt from [`PromptContext`].
    /// 2. Calls the Claude API.
    /// 3. Parses and validates the JSON response.
    /// 4. Falls back to a template scene on any error.
    pub async fn generate(&self, context: &PromptContext) -> NarrativeResponse {
        match self.try_generate(context).await {
            Ok(response) => response,
            Err(_) => Self::fallback_response(),
        }
    }

    async fn try_generate(&self, context: &PromptContext) -> Result<NarrativeResponse> {
        let system = PromptBuilder::build_system_prompt();
        let user_msg = PromptBuilder::build_user_message(context);

        let request = GenerateRequest {
            system_prompt: system,
            messages: vec![Message {
                role: Role::User,
                content: user_msg,
            }],
            temperature: 0.8,
        };

        let response = self.client.generate(&request).await?;
        let parsed = parse_narrative_json(&response.content)?;
        validate_narrative(&parsed)?;
        Ok(parsed)
    }

    /// A safe fallback when AI generation fails.
    fn fallback_response() -> NarrativeResponse {
        NarrativeResponse {
            narrative: "The corridor stretches before you, silent except for the hum of \
                        failing lights. Something has changed in the air — a weight, a \
                        presence — but you cannot place it."
                .into(),
            atmosphere: Atmosphere::Dread,
            sound_cue: Some("ambient_hum".into()),
            image_prompt: None,
            choices: vec![
                Choice {
                    id: "proceed".into(),
                    text: "Continue forward".into(),
                    approach: ChoiceApproach::Investigate,
                    fear_vector: FearType::Darkness,
                },
                Choice {
                    id: "wait".into(),
                    text: "Stay still and listen".into(),
                    approach: ChoiceApproach::Wait,
                    fear_vector: FearType::Stalking,
                },
            ],
            hidden_elements: vec![],
            intensity: 0.4,
            meta_break: None,
            transcript_lines: vec![],
            question_prompts: vec![],
            archive_entries: vec![],
            mirror_observations: vec![],
        }
    }
}

// ---------------------------------------------------------------------------
// JSON parsing
// ---------------------------------------------------------------------------

/// Parses the LLM's raw JSON string into a [`NarrativeResponse`].
///
/// # Example
///
/// ```
/// use fear_engine_ai_integration::narrative::parse_narrative_json;
///
/// let json = r#"{
///   "narrative": "A dark room.",
///   "atmosphere": "dread",
///   "sound_cue": null,
///   "image_prompt": null,
///   "choices": [
///     {"id": "go", "text": "Go", "approach": "investigate", "fear_vector": "darkness"}
///   ],
///   "hidden_elements": [],
///   "intensity": 0.5,
///   "meta_break": null
/// }"#;
/// let resp = parse_narrative_json(json).unwrap();
/// assert_eq!(resp.narrative, "A dark room.");
/// ```
pub fn parse_narrative_json(raw: &str) -> Result<NarrativeResponse> {
    // Strip markdown fences if the LLM wraps output.
    let trimmed = raw.trim();
    let json_str = if trimmed.starts_with("```") {
        trimmed
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim()
    } else {
        trimmed
    };

    let value: Value = serde_json::from_str(json_str)
        .map_err(|e| FearEngineError::Serialization(e.to_string()))?;

    let narrative = value["narrative"]
        .as_str()
        .ok_or_else(|| FearEngineError::AiGeneration("missing 'narrative' field".into()))?
        .to_string();

    let atmosphere = parse_atmosphere(value["atmosphere"].as_str().unwrap_or("dread"));

    let sound_cue = value["sound_cue"].as_str().map(String::from);
    let image_prompt = value["image_prompt"].as_str().map(String::from);

    let choices = parse_choices(&value["choices"])?;

    let hidden_elements = value["hidden_elements"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    let intensity = value["intensity"].as_f64().unwrap_or(0.5);

    let meta_break = parse_meta_break(&value["meta_break"]);
    let transcript_lines = parse_string_array(&value["transcript_lines"]);
    let question_prompts = parse_string_array(&value["question_prompts"]);
    let archive_entries = parse_string_array(&value["archive_entries"]);
    let mirror_observations = parse_string_array(&value["mirror_observations"]);

    Ok(NarrativeResponse {
        narrative,
        atmosphere,
        sound_cue,
        image_prompt,
        choices,
        hidden_elements,
        intensity,
        meta_break,
        transcript_lines,
        question_prompts,
        archive_entries,
        mirror_observations,
    })
}

fn parse_atmosphere(s: &str) -> Atmosphere {
    match s {
        "dread" => Atmosphere::Dread,
        "tension" => Atmosphere::Tension,
        "panic" => Atmosphere::Panic,
        "calm" => Atmosphere::Calm,
        "wrongness" => Atmosphere::Wrongness,
        "isolation" => Atmosphere::Isolation,
        "paranoia" => Atmosphere::Paranoia,
        _ => Atmosphere::Dread,
    }
}

fn parse_choices(value: &Value) -> Result<Vec<Choice>> {
    let arr = value.as_array().ok_or_else(|| {
        FearEngineError::AiGeneration("missing or invalid 'choices' array".into())
    })?;
    let mut choices = Vec::new();
    for item in arr {
        let id = item["id"].as_str().unwrap_or("unknown").to_string();
        let text = item["text"].as_str().unwrap_or("Continue").to_string();
        let approach = match item["approach"].as_str().unwrap_or("investigate") {
            "investigate" => ChoiceApproach::Investigate,
            "avoid" => ChoiceApproach::Avoid,
            "confront" => ChoiceApproach::Confront,
            "flee" => ChoiceApproach::Flee,
            "interact" => ChoiceApproach::Interact,
            "wait" => ChoiceApproach::Wait,
            _ => ChoiceApproach::Investigate,
        };
        let fear_vector = item["fear_vector"]
            .as_str()
            .unwrap_or("darkness")
            .parse::<FearType>()
            .unwrap_or(FearType::Darkness);
        choices.push(Choice {
            id,
            text,
            approach,
            fear_vector,
        });
    }
    Ok(choices)
}

fn parse_meta_break(value: &Value) -> Option<MetaBreak> {
    if value.is_null() {
        return None;
    }
    let text = value["text"].as_str()?.to_string();
    let target = match value["target"].as_str().unwrap_or("overlay") {
        "title" => MetaTarget::Title,
        "overlay" => MetaTarget::Overlay,
        "whisper" => MetaTarget::Whisper,
        "glitch_text" => MetaTarget::GlitchText,
        _ => MetaTarget::Overlay,
    };
    Some(MetaBreak { text, target })
}

fn parse_string_array(value: &Value) -> Vec<String> {
    value
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|item| item.as_str().map(ToOwned::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

/// Validates that a parsed response meets quality requirements.
///
/// # Example
///
/// ```
/// use fear_engine_ai_integration::narrative::{parse_narrative_json, validate_narrative};
///
/// let json = r#"{"narrative":"x","atmosphere":"dread","sound_cue":null,"image_prompt":null,"choices":[{"id":"a","text":"b","approach":"wait","fear_vector":"darkness"}],"hidden_elements":[],"intensity":0.5,"meta_break":null}"#;
/// let resp = parse_narrative_json(json).unwrap();
/// // Narrative too short, but validation is lenient on length for AI output.
/// assert!(validate_narrative(&resp).is_ok());
/// ```
pub fn validate_narrative(response: &NarrativeResponse) -> Result<()> {
    if response.narrative.is_empty() {
        return Err(FearEngineError::AiGeneration(
            "narrative text is empty".into(),
        ));
    }
    if response.intensity < 0.0 || response.intensity > 1.0 {
        return Err(FearEngineError::AiGeneration(format!(
            "intensity {} out of [0, 1] range",
            response.intensity
        )));
    }
    if response.choices.is_empty() {
        return Err(FearEngineError::AiGeneration("no choices provided".into()));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::claude_client::ClientConfig;
    use std::time::Duration;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn valid_ai_response() -> String {
        serde_json::json!({
            "narrative": "The hallway stretches before you, narrow and dimly lit. Water stains pattern the ceiling like a map of somewhere you don't want to go.",
            "atmosphere": "dread",
            "sound_cue": "dripping_water",
            "image_prompt": "dark hospital hallway, water stains, flickering lights",
            "choices": [
                {"id": "proceed", "text": "Walk forward", "approach": "investigate", "fear_vector": "darkness"},
                {"id": "turn_back", "text": "Turn around", "approach": "flee", "fear_vector": "claustrophobia"}
            ],
            "hidden_elements": ["water stain shaped like a face"],
            "intensity": 0.6,
            "meta_break": null
        }).to_string()
    }

    fn claude_response_body(content: &str) -> serde_json::Value {
        serde_json::json!({
            "id": "msg_test",
            "type": "message",
            "role": "assistant",
            "model": "claude-sonnet-4-20250514",
            "content": [{"type": "text", "text": content}],
            "stop_reason": "end_turn",
            "usage": {"input_tokens": 500, "output_tokens": 200}
        })
    }

    fn test_config() -> ClientConfig {
        ClientConfig {
            timeout: Duration::from_secs(5),
            max_retries: 0,
            base_retry_delay: Duration::from_millis(10),
            max_retry_delay: Duration::from_millis(100),
        }
    }

    fn sample_context() -> PromptContext {
        use crate::prompt::context::*;
        use fear_engine_common::types::GamePhase;
        use fear_engine_fear_profile::adaptation::AdaptationEngine;
        use fear_engine_fear_profile::profile::FearProfile;

        let mut engine = AdaptationEngine::new();
        let profile = FearProfile::new();
        let adaptation = engine.compute_directive(GamePhase::Exploring, &profile, 5);

        PromptContext {
            fear_profile: FearProfileContext {
                top_fears: vec![(FearType::Darkness, 0.7, 0.5)],
                anxiety_threshold: 0.6,
                behavioral_pattern: "cautious".into(),
                estimated_emotional_state: "tense".into(),
            },
            game_state: GameStateContext {
                location: "Ward B".into(),
                phase: GamePhase::Exploring,
                medium: Some(fear_engine_common::types::SurfaceMedium::Archive),
                trust_posture: Some(fear_engine_common::types::TrustPosture::Clinical),
                scene_number: 5,
                last_scene_summary: "Found locked door".into(),
                last_choice: "Tried handle".into(),
                active_threads: vec![],
                inventory: vec![],
                established_details: vec![],
            },
            adaptation,
        }
    }

    #[tokio::test]
    async fn test_pipeline_generates_valid_scene() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(claude_response_body(&valid_ai_response())),
            )
            .mount(&server)
            .await;

        let client = ClaudeClient::with_base_url("k".into(), server.uri(), test_config());
        let pipeline = NarrativePipeline::new(client);
        let resp = pipeline.generate(&sample_context()).await;
        assert!(resp.narrative.contains("hallway"));
        assert!(!resp.choices.is_empty());
    }

    #[tokio::test]
    async fn test_pipeline_parses_choices_correctly() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(claude_response_body(&valid_ai_response())),
            )
            .mount(&server)
            .await;

        let client = ClaudeClient::with_base_url("k".into(), server.uri(), test_config());
        let pipeline = NarrativePipeline::new(client);
        let resp = pipeline.generate(&sample_context()).await;
        assert_eq!(resp.choices.len(), 2);
        assert_eq!(resp.choices[0].id, "proceed");
        assert_eq!(resp.choices[0].approach, ChoiceApproach::Investigate);
    }

    #[tokio::test]
    async fn test_pipeline_extracts_image_prompt() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(claude_response_body(&valid_ai_response())),
            )
            .mount(&server)
            .await;

        let client = ClaudeClient::with_base_url("k".into(), server.uri(), test_config());
        let pipeline = NarrativePipeline::new(client);
        let resp = pipeline.generate(&sample_context()).await;
        assert!(resp.image_prompt.is_some());
        assert!(resp.image_prompt.unwrap().contains("hospital"));
    }

    #[tokio::test]
    async fn test_pipeline_falls_back_on_api_error() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&server)
            .await;

        let client = ClaudeClient::with_base_url("k".into(), server.uri(), test_config());
        let pipeline = NarrativePipeline::new(client);
        let resp = pipeline.generate(&sample_context()).await;
        // Should get fallback, not panic.
        assert!(resp.narrative.contains("corridor"));
        assert!(!resp.choices.is_empty());
    }

    #[tokio::test]
    async fn test_pipeline_falls_back_on_invalid_json() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(claude_response_body("This is not JSON at all.")),
            )
            .mount(&server)
            .await;

        let client = ClaudeClient::with_base_url("k".into(), server.uri(), test_config());
        let pipeline = NarrativePipeline::new(client);
        let resp = pipeline.generate(&sample_context()).await;
        assert!(resp.narrative.contains("corridor"));
    }

    #[tokio::test]
    async fn test_pipeline_validates_intensity_range() {
        let bad = serde_json::json!({
            "narrative": "ok",
            "atmosphere": "dread",
            "sound_cue": null,
            "image_prompt": null,
            "choices": [{"id":"a","text":"b","approach":"wait","fear_vector":"darkness"}],
            "hidden_elements": [],
            "intensity": 5.0,
            "meta_break": null
        })
        .to_string();

        let result = parse_narrative_json(&bad).and_then(|r| {
            validate_narrative(&r)?;
            Ok(r)
        });
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_pipeline_meta_break_extraction() {
        let with_meta = serde_json::json!({
            "narrative": "Something watches.",
            "atmosphere": "paranoia",
            "sound_cue": null,
            "image_prompt": null,
            "choices": [{"id":"a","text":"b","approach":"wait","fear_vector":"stalking"}],
            "hidden_elements": [],
            "intensity": 0.8,
            "meta_break": {"text": "I see you.", "target": "whisper"}
        })
        .to_string();

        let resp = parse_narrative_json(&with_meta).unwrap();
        assert!(resp.meta_break.is_some());
        let mb = resp.meta_break.unwrap();
        assert_eq!(mb.text, "I see you.");
        assert!(matches!(mb.target, MetaTarget::Whisper));
    }

    #[test]
    fn test_parse_strips_markdown_fences() {
        let fenced = "```json\n{\"narrative\":\"hi\",\"atmosphere\":\"calm\",\"sound_cue\":null,\"image_prompt\":null,\"choices\":[{\"id\":\"a\",\"text\":\"b\",\"approach\":\"wait\",\"fear_vector\":\"darkness\"}],\"hidden_elements\":[],\"intensity\":0.3,\"meta_break\":null}\n```";
        let resp = parse_narrative_json(fenced).unwrap();
        assert_eq!(resp.narrative, "hi");
    }

    #[test]
    fn test_validate_empty_narrative_fails() {
        let resp = NarrativeResponse {
            narrative: "".into(),
            atmosphere: Atmosphere::Calm,
            sound_cue: None,
            image_prompt: None,
            choices: vec![Choice {
                id: "a".into(),
                text: "b".into(),
                approach: ChoiceApproach::Wait,
                fear_vector: FearType::Darkness,
            }],
            hidden_elements: vec![],
            intensity: 0.5,
            meta_break: None,
            transcript_lines: vec![],
            question_prompts: vec![],
            archive_entries: vec![],
            mirror_observations: vec![],
        };
        assert!(validate_narrative(&resp).is_err());
    }

    #[test]
    fn test_validate_no_choices_fails() {
        let resp = NarrativeResponse {
            narrative: "Something.".into(),
            atmosphere: Atmosphere::Dread,
            sound_cue: None,
            image_prompt: None,
            choices: vec![],
            hidden_elements: vec![],
            intensity: 0.5,
            meta_break: None,
            transcript_lines: vec![],
            question_prompts: vec![],
            archive_entries: vec![],
            mirror_observations: vec![],
        };
        assert!(validate_narrative(&resp).is_err());
    }
}
