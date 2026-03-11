//! Production-grade async HTTP client for the Anthropic Messages API.
//!
//! Features: exponential-backoff retries, token-bucket rate limiting,
//! configurable timeout, and proper error mapping.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use fear_engine_common::{FearEngineError, Result};
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Configuration for the HTTP client.
#[derive(Debug, Clone)]
pub struct ClientConfig {
    /// Per-request timeout.
    pub timeout: Duration,
    /// Maximum number of retries on transient errors.
    pub max_retries: u32,
    /// Base delay between retries (doubled each attempt).
    pub base_retry_delay: Duration,
    /// Hard cap on the retry delay.
    pub max_retry_delay: Duration,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(30),
            max_retries: 3,
            base_retry_delay: Duration::from_secs(1),
            max_retry_delay: Duration::from_secs(16),
        }
    }
}

/// Role of a message in the conversation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    User,
    Assistant,
}

/// A single message in the conversation history.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: String,
}

/// Parameters for a generation request.
#[derive(Debug, Clone)]
pub struct GenerateRequest {
    pub system_prompt: String,
    pub messages: Vec<Message>,
    pub temperature: f64,
}

/// Parsed response from the Anthropic API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateResponse {
    pub content: String,
    pub model: String,
    pub usage: TokenUsage,
    pub stop_reason: String,
}

/// Token usage statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
}

// ---------------------------------------------------------------------------
// Rate limiter
// ---------------------------------------------------------------------------

/// A simple token-bucket rate limiter.
///
/// # Example
///
/// ```
/// use fear_engine_ai_integration::claude_client::TokenBucketRateLimiter;
///
/// let rl = TokenBucketRateLimiter::new(10, 5);
/// assert!(rl.try_acquire());
/// ```
pub struct TokenBucketRateLimiter {
    tokens: AtomicU32,
    max_tokens: u32,
    refill_rate: u32,
    last_refill: Mutex<Instant>,
}

impl TokenBucketRateLimiter {
    /// Creates a new limiter with `max_tokens` capacity, refilling
    /// `refill_rate` tokens per second.
    pub fn new(max_tokens: u32, refill_rate: u32) -> Self {
        Self {
            tokens: AtomicU32::new(max_tokens),
            max_tokens,
            refill_rate,
            last_refill: Mutex::new(Instant::now()),
        }
    }

    /// Tries to consume one token. Returns `true` if successful.
    pub fn try_acquire(&self) -> bool {
        self.refill();
        let mut current = self.tokens.load(Ordering::Relaxed);
        loop {
            if current == 0 {
                return false;
            }
            match self.tokens.compare_exchange_weak(
                current,
                current - 1,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => return true,
                Err(actual) => current = actual,
            }
        }
    }

    /// Blocks until a token is available.
    pub async fn acquire(&self) {
        loop {
            if self.try_acquire() {
                return;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }

    fn refill(&self) {
        let mut last = self.last_refill.lock().expect("rate limiter lock");
        let elapsed = last.elapsed();
        let new_tokens = (elapsed.as_secs_f64() * self.refill_rate as f64) as u32;
        if new_tokens > 0 {
            *last = Instant::now();
            let current = self.tokens.load(Ordering::Relaxed);
            let target = (current + new_tokens).min(self.max_tokens);
            self.tokens.store(target, Ordering::Relaxed);
        }
    }
}

// ---------------------------------------------------------------------------
// Claude client
// ---------------------------------------------------------------------------

/// Async client for the Anthropic Messages API.
///
/// # Example
///
/// ```no_run
/// use fear_engine_ai_integration::claude_client::{ClaudeClient, ClientConfig};
///
/// let client = ClaudeClient::new(
///     "sk-ant-test".into(),
///     ClientConfig::default(),
/// );
/// ```
pub struct ClaudeClient {
    http: reqwest::Client,
    api_key: String,
    base_url: String,
    model: String,
    max_tokens: u32,
    rate_limiter: TokenBucketRateLimiter,
    config: ClientConfig,
}

impl ClaudeClient {
    /// Creates a new client targeting the production Anthropic API.
    pub fn new(api_key: String, config: ClientConfig) -> Self {
        let http = reqwest::Client::builder()
            .timeout(config.timeout)
            .build()
            .expect("failed to build HTTP client");

        Self {
            http,
            api_key,
            base_url: "https://api.anthropic.com".into(),
            model: "claude-sonnet-4-20250514".into(),
            max_tokens: 1024,
            rate_limiter: TokenBucketRateLimiter::new(10, 5),
            config,
        }
    }

    /// Creates a client pointing at a custom base URL (for testing with wiremock).
    pub fn with_base_url(api_key: String, base_url: String, config: ClientConfig) -> Self {
        let http = reqwest::Client::builder()
            .timeout(config.timeout)
            .build()
            .expect("failed to build HTTP client");

        Self {
            http,
            api_key,
            base_url,
            model: "claude-sonnet-4-20250514".into(),
            max_tokens: 1024,
            rate_limiter: TokenBucketRateLimiter::new(100, 50),
            config,
        }
    }

    /// Sends a generation request to the Anthropic Messages API.
    ///
    /// Retries on transient errors (5xx, 429) with exponential backoff.
    /// Does **not** retry on client errors (4xx except 429).
    pub async fn generate(&self, request: &GenerateRequest) -> Result<GenerateResponse> {
        self.rate_limiter.acquire().await;

        let url = format!("{}/v1/messages", self.base_url);
        let body = self.build_request_body(request);

        let mut last_err = FearEngineError::AiGeneration("no attempts made".into());

        for attempt in 0..=self.config.max_retries {
            if attempt > 0 {
                let delay = self.retry_delay(attempt);
                tokio::time::sleep(delay).await;
            }

            let result = self
                .http
                .post(&url)
                .header("x-api-key", &self.api_key)
                .header("anthropic-version", "2023-06-01")
                .header("content-type", "application/json")
                .json(&body)
                .send()
                .await;

            let response = match result {
                Ok(r) => r,
                Err(e) => {
                    if e.is_timeout() {
                        last_err = FearEngineError::Timeout {
                            operation: "claude_generate".into(),
                            duration_ms: self.config.timeout.as_millis() as u64,
                        };
                        continue;
                    }
                    return Err(FearEngineError::AiGeneration(e.to_string()));
                }
            };

            let status = response.status().as_u16();

            match status {
                200 => return self.parse_response(response).await,
                429 => {
                    last_err = FearEngineError::RateLimit {
                        retry_after_ms: self.retry_delay(attempt + 1).as_millis() as u64,
                    };
                    continue;
                }
                500..=599 => {
                    let body_text = response.text().await.unwrap_or_default();
                    last_err = FearEngineError::AiGeneration(format!(
                        "server error {status}: {body_text}"
                    ));
                    continue;
                }
                401 => {
                    return Err(FearEngineError::Configuration(
                        "Anthropic API: invalid API key (401)".into(),
                    ));
                }
                _ => {
                    let body_text = response.text().await.unwrap_or_default();
                    return Err(FearEngineError::AiGeneration(format!(
                        "client error {status}: {body_text}"
                    )));
                }
            }
        }

        Err(last_err)
    }

    // -- private ----------------------------------------------------------

    fn build_request_body(&self, req: &GenerateRequest) -> serde_json::Value {
        let messages: Vec<serde_json::Value> = req
            .messages
            .iter()
            .map(|m| {
                serde_json::json!({
                    "role": m.role,
                    "content": m.content,
                })
            })
            .collect();

        serde_json::json!({
            "model": self.model,
            "max_tokens": self.max_tokens,
            "system": req.system_prompt,
            "messages": messages,
            "temperature": req.temperature,
        })
    }

    async fn parse_response(&self, response: reqwest::Response) -> Result<GenerateResponse> {
        let body: serde_json::Value = response
            .json()
            .await
            .map_err(|e| FearEngineError::Serialization(e.to_string()))?;

        let content = body["content"]
            .as_array()
            .and_then(|arr| arr.first())
            .and_then(|c| c["text"].as_str())
            .ok_or_else(|| {
                FearEngineError::AiGeneration(format!(
                    "malformed response: missing content[0].text in {body}"
                ))
            })?
            .to_string();

        let model = body["model"].as_str().unwrap_or("unknown").to_string();

        let input_tokens = body["usage"]["input_tokens"].as_u64().unwrap_or(0) as u32;
        let output_tokens = body["usage"]["output_tokens"].as_u64().unwrap_or(0) as u32;
        let stop_reason = body["stop_reason"]
            .as_str()
            .unwrap_or("end_turn")
            .to_string();

        Ok(GenerateResponse {
            content,
            model,
            usage: TokenUsage {
                input_tokens,
                output_tokens,
            },
            stop_reason,
        })
    }

    fn retry_delay(&self, attempt: u32) -> Duration {
        let delay = self.config.base_retry_delay * 2u32.saturating_pow(attempt.saturating_sub(1));
        delay.min(self.config.max_retry_delay)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn test_config() -> ClientConfig {
        ClientConfig {
            timeout: Duration::from_secs(5),
            max_retries: 2,
            base_retry_delay: Duration::from_millis(50),
            max_retry_delay: Duration::from_millis(200),
        }
    }

    fn test_request() -> GenerateRequest {
        GenerateRequest {
            system_prompt: "You are a test assistant.".into(),
            messages: vec![Message {
                role: Role::User,
                content: "Hello".into(),
            }],
            temperature: 0.7,
        }
    }

    fn success_body() -> serde_json::Value {
        serde_json::json!({
            "id": "msg_123",
            "type": "message",
            "role": "assistant",
            "model": "claude-sonnet-4-20250514",
            "content": [{"type": "text", "text": "Hello back!"}],
            "stop_reason": "end_turn",
            "usage": {"input_tokens": 10, "output_tokens": 5}
        })
    }

    #[tokio::test]
    async fn test_successful_request_response() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .and(header("x-api-key", "test-key"))
            .and(header("anthropic-version", "2023-06-01"))
            .respond_with(ResponseTemplate::new(200).set_body_json(success_body()))
            .expect(1)
            .mount(&server)
            .await;

        let client = ClaudeClient::with_base_url("test-key".into(), server.uri(), test_config());
        let resp = client.generate(&test_request()).await.unwrap();
        assert_eq!(resp.content, "Hello back!");
        assert_eq!(resp.usage.input_tokens, 10);
        assert_eq!(resp.usage.output_tokens, 5);
        assert_eq!(resp.stop_reason, "end_turn");
    }

    #[tokio::test]
    async fn test_retry_on_500() {
        let server = MockServer::start().await;
        // First request: 500, second request: 200.
        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .respond_with(ResponseTemplate::new(500).set_body_string("internal error"))
            .expect(1)
            .up_to_n_times(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .respond_with(ResponseTemplate::new(200).set_body_json(success_body()))
            .expect(1)
            .mount(&server)
            .await;

        let client = ClaudeClient::with_base_url("test-key".into(), server.uri(), test_config());
        let resp = client.generate(&test_request()).await.unwrap();
        assert_eq!(resp.content, "Hello back!");
    }

    #[tokio::test]
    async fn test_retry_on_429() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .respond_with(ResponseTemplate::new(429).set_body_string("rate limited"))
            .expect(1)
            .up_to_n_times(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .respond_with(ResponseTemplate::new(200).set_body_json(success_body()))
            .expect(1)
            .mount(&server)
            .await;

        let client = ClaudeClient::with_base_url("test-key".into(), server.uri(), test_config());
        let resp = client.generate(&test_request()).await.unwrap();
        assert_eq!(resp.content, "Hello back!");
    }

    #[tokio::test]
    async fn test_no_retry_on_400() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .respond_with(ResponseTemplate::new(400).set_body_string("bad request"))
            .expect(1)
            .mount(&server)
            .await;

        let client = ClaudeClient::with_base_url("test-key".into(), server.uri(), test_config());
        let err = client.generate(&test_request()).await.unwrap_err();
        assert!(err.to_string().contains("400"));
    }

    #[tokio::test]
    async fn test_auth_error_401() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .respond_with(ResponseTemplate::new(401).set_body_string("unauthorized"))
            .expect(1)
            .mount(&server)
            .await;

        let client = ClaudeClient::with_base_url("bad-key".into(), server.uri(), test_config());
        let err = client.generate(&test_request()).await.unwrap_err();
        match err {
            FearEngineError::Configuration(msg) => assert!(msg.contains("401")),
            other => panic!("expected Configuration error, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_timeout_handling() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(success_body())
                    .set_delay(Duration::from_secs(10)),
            )
            .mount(&server)
            .await;

        let mut config = test_config();
        config.timeout = Duration::from_millis(100);
        config.max_retries = 0;

        let client = ClaudeClient::with_base_url("test-key".into(), server.uri(), config);
        let err = client.generate(&test_request()).await.unwrap_err();
        assert!(
            matches!(err, FearEngineError::Timeout { .. }),
            "expected Timeout, got {err:?}"
        );
    }

    #[tokio::test]
    async fn test_malformed_response() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({"bad": "data"})),
            )
            .expect(1)
            .mount(&server)
            .await;

        let client = ClaudeClient::with_base_url("test-key".into(), server.uri(), test_config());
        let err = client.generate(&test_request()).await.unwrap_err();
        assert!(err.to_string().contains("malformed"));
    }

    #[tokio::test]
    async fn test_token_usage_parsing() {
        let server = MockServer::start().await;
        let body = serde_json::json!({
            "id": "msg_456",
            "type": "message",
            "role": "assistant",
            "model": "claude-sonnet-4-20250514",
            "content": [{"type": "text", "text": "ok"}],
            "stop_reason": "max_tokens",
            "usage": {"input_tokens": 150, "output_tokens": 75}
        });
        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .respond_with(ResponseTemplate::new(200).set_body_json(body))
            .expect(1)
            .mount(&server)
            .await;

        let client = ClaudeClient::with_base_url("test-key".into(), server.uri(), test_config());
        let resp = client.generate(&test_request()).await.unwrap();
        assert_eq!(resp.usage.input_tokens, 150);
        assert_eq!(resp.usage.output_tokens, 75);
        assert_eq!(resp.stop_reason, "max_tokens");
    }

    #[test]
    fn test_rate_limiter_basic() {
        let rl = TokenBucketRateLimiter::new(2, 1);
        assert!(rl.try_acquire());
        assert!(rl.try_acquire());
        assert!(!rl.try_acquire()); // bucket empty
    }

    #[test]
    fn test_retry_delay_exponential() {
        let client =
            ClaudeClient::with_base_url("k".into(), "http://localhost".into(), test_config());
        assert_eq!(client.retry_delay(1), Duration::from_millis(50));
        assert_eq!(client.retry_delay(2), Duration::from_millis(100));
        assert_eq!(client.retry_delay(3), Duration::from_millis(200)); // capped
    }
}
