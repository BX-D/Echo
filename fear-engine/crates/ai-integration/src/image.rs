//! Image generation client — builds horror-styled prompts, calls an external
//! image API, caches results, and degrades gracefully on failure.

use std::collections::HashMap;
use std::sync::Mutex;

use fear_engine_common::types::FearType;
use fear_engine_common::{FearEngineError, Result};
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Prompt building
// ---------------------------------------------------------------------------

const STYLE_PREFIX: &str = "Dark atmospheric horror, high contrast, desaturated, \
    cinematic lighting, photorealistic, 35mm film grain, ";

const NEGATIVE_PROMPT: &str = "cartoon, anime, bright colors, happy, cheerful, \
    gore, extreme violence, text, watermark, low quality";

/// Builds a complete image generation prompt from a scene description
/// and the player's dominant fear type.
///
/// # Example
///
/// ```
/// use fear_engine_ai_integration::image::build_image_prompt;
/// use fear_engine_common::types::FearType;
///
/// let prompt = build_image_prompt("abandoned hospital corridor", Some(&FearType::Darkness));
/// assert!(prompt.contains("Dark atmospheric horror"));
/// assert!(prompt.contains("hospital corridor"));
/// ```
pub fn build_image_prompt(scene_description: &str, fear: Option<&FearType>) -> String {
    let fear_modifier = fear.map(fear_style_modifier).unwrap_or("");
    format!("{STYLE_PREFIX}{fear_modifier}{scene_description}")
}

/// Returns the negative prompt used for all generations.
///
/// # Example
///
/// ```
/// use fear_engine_ai_integration::image::negative_prompt;
/// assert!(negative_prompt().contains("cartoon"));
/// ```
pub fn negative_prompt() -> &'static str {
    NEGATIVE_PROMPT
}

fn fear_style_modifier(fear: &FearType) -> &'static str {
    match fear {
        FearType::Claustrophobia => "tight enclosed space, walls closing in, ",
        FearType::Isolation => "vast empty space, lone figure, desolate, ",
        FearType::BodyHorror => "organic distortion, uncanny anatomy, ",
        FearType::Stalking => "shadowy figure in background, watched, ",
        FearType::LossOfControl => "disorienting perspective, tilted angles, ",
        FearType::UncannyValley => "almost human, subtly wrong, mannequin-like, ",
        FearType::Darkness => "deep shadows, barely visible, darkness encroaching, ",
        FearType::SoundBased => "visual representation of sound waves, vibration, ",
        FearType::Doppelganger => "mirror reflection, double, duplicate figure, ",
        FearType::Abandonment => "abandoned place, remnants of people, empty, ",
    }
}

// ---------------------------------------------------------------------------
// Image client
// ---------------------------------------------------------------------------

/// Async client for image generation with in-memory caching and graceful
/// degradation.
///
/// # Example
///
/// ```
/// use fear_engine_ai_integration::image::ImageClient;
///
/// let client = ImageClient::new("test-key".into());
/// ```
pub struct ImageClient {
    api_key: String,
    base_url: String,
    cache: Mutex<HashMap<String, ImageResult>>,
}

/// The result of an image generation request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageResult {
    /// Base64-encoded data URL (`data:image/png;base64,...`).
    pub data_url: String,
    /// The prompt that was used.
    pub prompt: String,
}

impl ImageClient {
    /// Creates a new image client targeting the OpenAI DALL-E API.
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            base_url: "https://api.openai.com".into(),
            cache: Mutex::new(HashMap::new()),
        }
    }

    /// Creates a client with a custom base URL (for testing).
    pub fn with_base_url(api_key: String, base_url: String) -> Self {
        Self {
            api_key,
            base_url,
            cache: Mutex::new(HashMap::new()),
        }
    }

    /// Generates an image (or returns a cached result) for the given scene
    /// description and dominant fear.
    ///
    /// Returns `Ok(None)` if the API fails — the game continues without images.
    pub async fn generate(
        &self,
        scene_description: &str,
        fear: Option<&FearType>,
    ) -> Result<Option<ImageResult>> {
        let prompt = build_image_prompt(scene_description, fear);
        let cache_key = cache_key_for(&prompt);

        // Check cache.
        {
            let cache = self.cache.lock().expect("image cache lock");
            if let Some(cached) = cache.get(&cache_key) {
                return Ok(Some(cached.clone()));
            }
        }

        // Call API (graceful degradation on error).
        match self.call_api(&prompt).await {
            Ok(result) => {
                let mut cache = self.cache.lock().expect("image cache lock");
                cache.insert(cache_key, result.clone());
                Ok(Some(result))
            }
            Err(_) => Ok(None),
        }
    }

    /// Returns `true` if the prompt is already cached.
    pub fn is_cached(&self, scene_description: &str, fear: Option<&FearType>) -> bool {
        let prompt = build_image_prompt(scene_description, fear);
        let key = cache_key_for(&prompt);
        let cache = self.cache.lock().expect("image cache lock");
        cache.contains_key(&key)
    }

    async fn call_api(&self, prompt: &str) -> Result<ImageResult> {
        let url = format!("{}/v1/images/generations", self.base_url);

        let body = serde_json::json!({
            "model": "dall-e-3",
            "prompt": format!("{prompt}\n\nNegative: {NEGATIVE_PROMPT}"),
            "n": 1,
            "size": "1792x1024",
            "quality": "standard",
            "response_format": "b64_json"
        });

        let http = reqwest::Client::new();
        let response = http
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| FearEngineError::ImageGeneration(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let text = response.text().await.unwrap_or_default();
            return Err(FearEngineError::ImageGeneration(format!(
                "image API error {status}: {text}"
            )));
        }

        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| FearEngineError::Serialization(e.to_string()))?;

        let base64 = json["data"][0]["b64_json"]
            .as_str()
            .ok_or_else(|| {
                FearEngineError::ImageGeneration("missing b64_json in response".into())
            })?;

        Ok(ImageResult {
            data_url: format!("data:image/png;base64,{base64}"),
            prompt: prompt.to_string(),
        })
    }
}

fn cache_key_for(prompt: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    prompt.hash(&mut hasher);
    format!("img_{:x}", hasher.finish())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    fn test_image_prompt_includes_style_prefix() {
        let p = build_image_prompt("dark room", None);
        assert!(p.starts_with("Dark atmospheric horror"));
    }

    #[test]
    fn test_image_prompt_includes_fear_modifiers() {
        let p = build_image_prompt("corridor", Some(&FearType::Darkness));
        assert!(p.contains("deep shadows"));
        assert!(p.contains("corridor"));
    }

    #[test]
    fn test_image_prompt_includes_scene_description() {
        let p = build_image_prompt("abandoned ward", Some(&FearType::Isolation));
        assert!(p.contains("abandoned ward"));
    }

    #[test]
    fn test_negative_prompt_content() {
        let np = negative_prompt();
        assert!(np.contains("cartoon"));
        assert!(np.contains("gore"));
    }

    #[test]
    fn test_all_fears_have_modifiers() {
        for fear in FearType::all() {
            let modifier = fear_style_modifier(&fear);
            assert!(!modifier.is_empty(), "no modifier for {fear}");
        }
    }

    #[tokio::test]
    async fn test_image_generation_success() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/images/generations"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "created": 1234567890,
                "data": [{"b64_json": "dGVzdA==", "revised_prompt": "test"}]
            })))
            .mount(&server)
            .await;

        let client = ImageClient::with_base_url("key".into(), server.uri());
        let result = client.generate("dark room", Some(&FearType::Darkness)).await.unwrap();
        assert!(result.is_some());
        let img = result.unwrap();
        assert!(img.data_url.starts_with("data:image/png;base64,"));
    }

    #[tokio::test]
    async fn test_image_generation_caching() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "created": 1234567890,
                "data": [{"b64_json": "Y2FjaGVk", "revised_prompt": "cached"}]
            })))
            .expect(1) // Only one API call expected.
            .mount(&server)
            .await;

        let client = ImageClient::with_base_url("key".into(), server.uri());
        let r1 = client.generate("room", None).await.unwrap();
        let r2 = client.generate("room", None).await.unwrap();
        assert!(r1.is_some());
        assert!(r2.is_some());
        assert_eq!(r1.unwrap().data_url, r2.unwrap().data_url);
    }

    #[tokio::test]
    async fn test_image_generation_api_error_graceful() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(500).set_body_string("error"))
            .mount(&server)
            .await;

        let client = ImageClient::with_base_url("key".into(), server.uri());
        let result = client.generate("scene", None).await.unwrap();
        assert!(result.is_none()); // Graceful degradation.
    }

    #[tokio::test]
    async fn test_image_generation_timeout_graceful() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(200).set_delay(std::time::Duration::from_secs(60)),
            )
            .mount(&server)
            .await;

        let client = ImageClient::with_base_url("key".into(), server.uri());
        // Will timeout and return None (graceful).
        let result = client.generate("scene", None).await.unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_cache_key_deterministic() {
        let k1 = cache_key_for("prompt A");
        let k2 = cache_key_for("prompt A");
        assert_eq!(k1, k2);
    }

    #[test]
    fn test_cache_key_different_for_different_prompts() {
        let k1 = cache_key_for("prompt A");
        let k2 = cache_key_for("prompt B");
        assert_ne!(k1, k2);
    }
}
