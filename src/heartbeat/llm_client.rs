//! LLM client abstraction for heartbeat job execution.
//!
//! Supports two providers:
//! - **Ollama** (primary, local): POST /api/generate with stream:false
//! - **Anthropic** (fallback, remote): POST /v1/messages with x-api-key header
//!
//! T-10-01: Never log API key values. Use tracing::debug for request URLs
//! but redact the x-api-key header value.
//! T-10-05: Non-localhost Ollama endpoints logged at warn level.

use std::fmt;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

use crate::config::global::LlmConfig;

// ---------------------------------------------------------------------------
// Error types
// ---------------------------------------------------------------------------

/// Errors from LLM provider operations.
#[derive(Debug)]
pub enum LlmError {
    /// Network/connection failure.
    Connection(String),
    /// Non-2xx response from the API.
    ApiError(u16, String),
    /// Response body could not be parsed.
    ParseError(String),
    /// No provider could be configured (e.g., missing API key).
    NoProvider(String),
    /// Requested model not available on the provider.
    ModelNotFound(String),
}

impl fmt::Display for LlmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LlmError::Connection(msg) => write!(f, "Connection error: {}", msg),
            LlmError::ApiError(status, msg) => write!(f, "API error ({}): {}", status, msg),
            LlmError::ParseError(msg) => write!(f, "Parse error: {}", msg),
            LlmError::NoProvider(msg) => write!(f, "No provider: {}", msg),
            LlmError::ModelNotFound(msg) => write!(f, "Model not found: {}", msg),
        }
    }
}

// ---------------------------------------------------------------------------
// LLM Provider
// ---------------------------------------------------------------------------

/// Unified response from an LLM provider.
#[derive(Debug)]
pub struct LlmResponse {
    /// The generated text.
    pub text: String,
    /// Model that produced the response.
    pub model: String,
    /// Input tokens consumed (if reported).
    pub input_tokens: Option<u64>,
    /// Output tokens generated (if reported).
    pub output_tokens: Option<u64>,
}

/// LLM provider variants with their connection configuration.
///
/// Debug output redacts the Anthropic API key (T-10-01).
pub enum LlmProvider {
    /// Local Ollama instance.
    Ollama {
        /// API endpoint (e.g., "http://localhost:11434").
        endpoint: String,
        /// Model name (e.g., "llama3.2").
        model: String,
    },
    /// Anthropic Messages API.
    Anthropic {
        /// API key (from ANTHROPIC_API_KEY env var, per D-11).
        api_key: String,
        /// Model name (e.g., "claude-haiku-4-5").
        model: String,
        /// Maximum tokens for response.
        max_tokens: u32,
    },
}

impl fmt::Debug for LlmProvider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LlmProvider::Ollama { endpoint, model } => f
                .debug_struct("Ollama")
                .field("endpoint", endpoint)
                .field("model", model)
                .finish(),
            LlmProvider::Anthropic {
                model, max_tokens, ..
            } => f
                .debug_struct("Anthropic")
                .field("api_key", &"[REDACTED]")
                .field("model", model)
                .field("max_tokens", max_tokens)
                .finish(),
        }
    }
}

impl LlmProvider {
    /// Build an LlmProvider from global configuration.
    ///
    /// For Anthropic, checks `ANTHROPIC_API_KEY` environment variable first
    /// (per D-11). Returns `Err(LlmError::NoProvider)` if Anthropic is the
    /// default provider but no API key is available.
    pub fn from_config(config: &LlmConfig) -> Result<Self, LlmError> {
        match config.default_provider.as_str() {
            "ollama" => {
                // T-10-05: Warn if endpoint is not localhost
                if !config.ollama.endpoint.contains("localhost")
                    && !config.ollama.endpoint.contains("127.0.0.1")
                {
                    warn!(
                        "Ollama endpoint is non-local: {}",
                        config.ollama.endpoint
                    );
                }

                Ok(LlmProvider::Ollama {
                    endpoint: config.ollama.endpoint.clone(),
                    model: config.ollama.model.clone(),
                })
            }
            "anthropic" => {
                let api_key = std::env::var("ANTHROPIC_API_KEY").map_err(|_| {
                    LlmError::NoProvider(
                        "ANTHROPIC_API_KEY environment variable not set".to_string(),
                    )
                })?;

                Ok(LlmProvider::Anthropic {
                    api_key,
                    model: config.anthropic.model.clone(),
                    max_tokens: config.anthropic.max_tokens,
                })
            }
            other => Err(LlmError::NoProvider(format!(
                "Unknown provider: '{}'. Expected 'ollama' or 'anthropic'.",
                other
            ))),
        }
    }

    /// Generate a response from the configured LLM provider.
    ///
    /// Dispatches to the appropriate API endpoint based on the provider variant.
    pub fn generate(
        &self,
        client: &reqwest::blocking::Client,
        prompt: &str,
    ) -> Result<LlmResponse, LlmError> {
        match self {
            LlmProvider::Ollama { endpoint, model } => {
                let url = format!("{}/api/generate", endpoint);
                debug!("Ollama generate request to {}", url);

                let request = OllamaGenerateRequest {
                    model: model.clone(),
                    prompt: prompt.to_string(),
                    stream: false,
                    options: None,
                };

                let resp = client
                    .post(&url)
                    .json(&request)
                    .send()
                    .map_err(|e| LlmError::Connection(e.to_string()))?;

                if !resp.status().is_success() {
                    let status = resp.status().as_u16();
                    let body = resp.text().unwrap_or_default();
                    return Err(LlmError::ApiError(status, body));
                }

                let body: OllamaGenerateResponse = resp
                    .json()
                    .map_err(|e| LlmError::ParseError(e.to_string()))?;

                Ok(LlmResponse {
                    text: body.response,
                    model: body.model,
                    input_tokens: body.prompt_eval_count,
                    output_tokens: body.eval_count,
                })
            }
            LlmProvider::Anthropic {
                api_key,
                model,
                max_tokens,
            } => {
                let url = "https://api.anthropic.com/v1/messages";
                debug!("Anthropic request to {}", url);
                // T-10-01: Never log the API key value

                let request = AnthropicRequest {
                    model: model.clone(),
                    max_tokens: *max_tokens,
                    messages: vec![AnthropicMessage {
                        role: "user".to_string(),
                        content: prompt.to_string(),
                    }],
                };

                let resp = client
                    .post(url)
                    .header("x-api-key", api_key)
                    .header("anthropic-version", "2023-06-01")
                    .header("content-type", "application/json")
                    .json(&request)
                    .send()
                    .map_err(|e| LlmError::Connection(e.to_string()))?;

                if !resp.status().is_success() {
                    let status = resp.status().as_u16();
                    let body = resp.text().unwrap_or_default();
                    return Err(LlmError::ApiError(status, body));
                }

                let body: AnthropicResponse = resp
                    .json()
                    .map_err(|e| LlmError::ParseError(e.to_string()))?;

                let text = body
                    .content
                    .iter()
                    .filter_map(|block| block.text.as_deref())
                    .collect::<Vec<_>>()
                    .join("\n");

                Ok(LlmResponse {
                    text,
                    model: body.model,
                    input_tokens: Some(body.usage.input_tokens),
                    output_tokens: Some(body.usage.output_tokens),
                })
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Ollama API types
// ---------------------------------------------------------------------------

/// Request body for Ollama POST /api/generate.
#[derive(Debug, Serialize)]
pub struct OllamaGenerateRequest {
    /// Model name to use.
    pub model: String,
    /// The prompt to generate from.
    pub prompt: String,
    /// Must be false for non-streaming (single JSON response).
    pub stream: bool,
    /// Optional generation parameters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<OllamaOptions>,
}

/// Optional parameters for Ollama generation.
#[derive(Debug, Serialize)]
pub struct OllamaOptions {
    /// Sampling temperature (0.0-1.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    /// Maximum number of tokens to generate.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub num_predict: Option<u32>,
}

/// Response body from Ollama POST /api/generate (non-streaming).
#[derive(Debug, Deserialize)]
pub struct OllamaGenerateResponse {
    /// The complete generated text.
    pub response: String,
    /// Whether generation is complete (always true in non-streaming).
    pub done: bool,
    /// Output token count (approximate).
    #[serde(default)]
    pub eval_count: Option<u64>,
    /// Input/prompt token count (approximate).
    #[serde(default)]
    pub prompt_eval_count: Option<u64>,
    /// Model that generated the response.
    pub model: String,
}

/// Response body from Ollama GET /api/tags.
#[derive(Debug, Deserialize)]
pub struct OllamaTagsResponse {
    /// List of available models.
    pub models: Vec<OllamaModel>,
}

/// A single model entry from Ollama /api/tags.
#[derive(Debug, Deserialize)]
pub struct OllamaModel {
    /// Model name (e.g., "llama3.2:latest").
    pub name: String,
    /// Model identifier.
    pub model: String,
    /// Model file size in bytes.
    pub size: u64,
    /// Model details.
    pub details: OllamaModelDetails,
}

/// Details about an Ollama model.
#[derive(Debug, Deserialize)]
pub struct OllamaModelDetails {
    /// Human-readable parameter count (e.g., "27.8B").
    pub parameter_size: String,
    /// Quantization level (e.g., "Q4_K_M").
    pub quantization_level: String,
}

// ---------------------------------------------------------------------------
// Anthropic API types
// ---------------------------------------------------------------------------

/// Request body for Anthropic POST /v1/messages.
#[derive(Debug, Serialize)]
pub struct AnthropicRequest {
    /// Model name (e.g., "claude-haiku-4-5").
    pub model: String,
    /// Maximum tokens for the response (required by Anthropic).
    pub max_tokens: u32,
    /// Message array (single user message for heartbeat).
    pub messages: Vec<AnthropicMessage>,
}

/// A single message in the Anthropic messages array.
#[derive(Debug, Serialize)]
pub struct AnthropicMessage {
    /// Message role ("user").
    pub role: String,
    /// Message content.
    pub content: String,
}

/// Response body from Anthropic POST /v1/messages.
#[derive(Debug, Deserialize)]
pub struct AnthropicResponse {
    /// Response content blocks.
    pub content: Vec<AnthropicContentBlock>,
    /// Model that generated the response.
    pub model: String,
    /// Reason for stopping generation.
    pub stop_reason: Option<String>,
    /// Token usage statistics.
    pub usage: AnthropicUsage,
}

/// A single content block in the Anthropic response.
#[derive(Debug, Deserialize)]
pub struct AnthropicContentBlock {
    /// Block type (e.g., "text").
    #[serde(rename = "type")]
    pub content_type: String,
    /// Text content (present when content_type is "text").
    pub text: Option<String>,
}

/// Token usage statistics from Anthropic.
#[derive(Debug, Deserialize)]
pub struct AnthropicUsage {
    /// Tokens in the input prompt.
    pub input_tokens: u64,
    /// Tokens in the generated output.
    pub output_tokens: u64,
}

/// Error response from Anthropic API.
#[derive(Debug, Deserialize)]
pub struct AnthropicErrorResponse {
    /// Error details.
    pub error: AnthropicErrorDetail,
}

/// Error detail from Anthropic API.
#[derive(Debug, Deserialize)]
pub struct AnthropicErrorDetail {
    /// Error type (e.g., "authentication_error").
    #[serde(rename = "type")]
    pub error_type: String,
    /// Human-readable error message.
    pub message: String,
}

// ---------------------------------------------------------------------------
// Health check and model listing
// ---------------------------------------------------------------------------

/// Check if Ollama is running and reachable.
///
/// Sends a GET request to the endpoint with a 2-second timeout.
/// Returns true if the response is 200 OK.
pub fn check_ollama_health(
    client: &reqwest::blocking::Client,
    endpoint: &str,
) -> bool {
    match client
        .get(endpoint)
        .timeout(Duration::from_secs(2))
        .send()
    {
        Ok(resp) => resp.status().is_success(),
        Err(_) => false,
    }
}

/// List available models on an Ollama instance.
///
/// Sends GET /api/tags and returns model names.
pub fn list_ollama_models(
    client: &reqwest::blocking::Client,
    endpoint: &str,
) -> Result<Vec<String>, LlmError> {
    let url = format!("{}/api/tags", endpoint);
    debug!("Listing Ollama models from {}", url);

    let resp = client
        .get(&url)
        .timeout(Duration::from_secs(5))
        .send()
        .map_err(|e| LlmError::Connection(e.to_string()))?;

    if !resp.status().is_success() {
        let status = resp.status().as_u16();
        let body = resp.text().unwrap_or_default();
        return Err(LlmError::ApiError(status, body));
    }

    let body: OllamaTagsResponse = resp
        .json()
        .map_err(|e| LlmError::ParseError(e.to_string()))?;

    Ok(body.models.into_iter().map(|m| m.name).collect())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ollama_generate_request_serializes_with_stream_false() {
        let req = OllamaGenerateRequest {
            model: "llama3.2".to_string(),
            prompt: "Hello world".to_string(),
            stream: false,
            options: None,
        };

        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains(r#""stream":false"#));
        assert!(json.contains(r#""model":"llama3.2""#));
        assert!(json.contains(r#""prompt":"Hello world""#));
        assert!(!json.contains("options")); // skipped when None
    }

    #[test]
    fn test_ollama_generate_request_with_options() {
        let req = OllamaGenerateRequest {
            model: "llama3.2".to_string(),
            prompt: "Hello".to_string(),
            stream: false,
            options: Some(OllamaOptions {
                temperature: Some(0.7),
                num_predict: Some(2048),
            }),
        };

        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains(r#""temperature":0.7"#));
        assert!(json.contains(r#""num_predict":2048"#));
    }

    #[test]
    fn test_ollama_generate_response_deserializes() {
        let json = r#"{
            "model": "qwen3.6:27b",
            "created_at": "2026-05-18T12:00:00Z",
            "response": "The answer is 42.",
            "done": true,
            "done_reason": "stop",
            "total_duration": 12345678,
            "load_duration": 1234567,
            "prompt_eval_count": 100,
            "prompt_eval_duration": 5000000,
            "eval_count": 200,
            "eval_duration": 10000000
        }"#;

        let resp: OllamaGenerateResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.response, "The answer is 42.");
        assert!(resp.done);
        assert_eq!(resp.eval_count, Some(200));
        assert_eq!(resp.prompt_eval_count, Some(100));
        assert_eq!(resp.model, "qwen3.6:27b");
    }

    #[test]
    fn test_ollama_generate_response_missing_optional_fields() {
        let json = r#"{
            "model": "llama3.2",
            "response": "Hello!",
            "done": true
        }"#;

        let resp: OllamaGenerateResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.response, "Hello!");
        assert!(resp.done);
        assert_eq!(resp.eval_count, None);
        assert_eq!(resp.prompt_eval_count, None);
    }

    #[test]
    fn test_ollama_tags_response_deserializes() {
        let json = r#"{
            "models": [
                {
                    "name": "qwen3.6:27b",
                    "model": "qwen3.6:27b",
                    "modified_at": "2026-05-18T18:25:38Z",
                    "size": 17420432739,
                    "digest": "a50eda8ed977",
                    "details": {
                        "format": "gguf",
                        "family": "qwen35",
                        "families": ["qwen35"],
                        "parameter_size": "27.8B",
                        "quantization_level": "Q4_K_M"
                    }
                }
            ]
        }"#;

        let resp: OllamaTagsResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.models.len(), 1);
        assert_eq!(resp.models[0].name, "qwen3.6:27b");
        assert_eq!(resp.models[0].size, 17420432739);
        assert_eq!(resp.models[0].details.parameter_size, "27.8B");
        assert_eq!(resp.models[0].details.quantization_level, "Q4_K_M");
    }

    #[test]
    fn test_anthropic_request_serializes() {
        let req = AnthropicRequest {
            model: "claude-haiku-4-5".to_string(),
            max_tokens: 2048,
            messages: vec![AnthropicMessage {
                role: "user".to_string(),
                content: "Hello, Claude!".to_string(),
            }],
        };

        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains(r#""model":"claude-haiku-4-5""#));
        assert!(json.contains(r#""max_tokens":2048"#));
        assert!(json.contains(r#""role":"user""#));
        assert!(json.contains(r#""content":"Hello, Claude!""#));
    }

    #[test]
    fn test_anthropic_response_deserializes() {
        let json = r#"{
            "id": "msg_abc123",
            "type": "message",
            "role": "assistant",
            "content": [
                {
                    "type": "text",
                    "text": "[INFO] Everything looks good."
                }
            ],
            "model": "claude-haiku-4-5",
            "stop_reason": "end_turn",
            "usage": {
                "input_tokens": 150,
                "output_tokens": 42
            }
        }"#;

        let resp: AnthropicResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.content.len(), 1);
        assert_eq!(resp.content[0].content_type, "text");
        assert_eq!(
            resp.content[0].text.as_deref(),
            Some("[INFO] Everything looks good.")
        );
        assert_eq!(resp.model, "claude-haiku-4-5");
        assert_eq!(resp.stop_reason.as_deref(), Some("end_turn"));
        assert_eq!(resp.usage.input_tokens, 150);
        assert_eq!(resp.usage.output_tokens, 42);
    }

    #[test]
    fn test_anthropic_error_response_deserializes() {
        let json = r#"{
            "error": {
                "type": "authentication_error",
                "message": "Invalid API key"
            }
        }"#;

        let resp: AnthropicErrorResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.error.error_type, "authentication_error");
        assert_eq!(resp.error.message, "Invalid API key");
    }

    #[test]
    fn test_llm_provider_from_config_ollama() {
        let config = LlmConfig {
            default_provider: "ollama".to_string(),
            ollama: crate::config::global::OllamaConfig {
                endpoint: "http://localhost:11434".to_string(),
                model: "llama3.2".to_string(),
            },
            anthropic: crate::config::global::AnthropicConfig::default(),
            heartbeat_concurrency: 1,
            heartbeat_retention: 10,
        };

        let provider = LlmProvider::from_config(&config).unwrap();
        match provider {
            LlmProvider::Ollama { endpoint, model } => {
                assert_eq!(endpoint, "http://localhost:11434");
                assert_eq!(model, "llama3.2");
            }
            _ => panic!("Expected Ollama provider"),
        }
    }

    /// Tests both the with-key and no-key paths for Anthropic in sequence
    /// to avoid env var race conditions with parallel test execution.
    #[test]
    fn test_llm_provider_from_config_anthropic_env_key_handling() {
        // Use a unique env var to avoid race conditions with real ANTHROPIC_API_KEY
        // Test the from_config logic by temporarily setting/unsetting

        // Phase 1: Test with key set
        // Save any existing value
        let original = std::env::var("ANTHROPIC_API_KEY").ok();

        std::env::set_var("ANTHROPIC_API_KEY", "sk-ant-test-key");

        let config = LlmConfig {
            default_provider: "anthropic".to_string(),
            ollama: crate::config::global::OllamaConfig::default(),
            anthropic: crate::config::global::AnthropicConfig {
                model: "claude-haiku-4-5".to_string(),
                max_tokens: 2048,
            },
            heartbeat_concurrency: 1,
            heartbeat_retention: 10,
        };

        let provider = LlmProvider::from_config(&config).unwrap();
        match provider {
            LlmProvider::Anthropic {
                api_key,
                model,
                max_tokens,
            } => {
                assert_eq!(api_key, "sk-ant-test-key");
                assert_eq!(model, "claude-haiku-4-5");
                assert_eq!(max_tokens, 2048);
            }
            _ => panic!("Expected Anthropic provider"),
        }

        // Phase 2: Test without key
        std::env::remove_var("ANTHROPIC_API_KEY");

        let config_no_key = LlmConfig {
            default_provider: "anthropic".to_string(),
            ollama: crate::config::global::OllamaConfig::default(),
            anthropic: crate::config::global::AnthropicConfig::default(),
            heartbeat_concurrency: 1,
            heartbeat_retention: 10,
        };

        let result = LlmProvider::from_config(&config_no_key);
        assert!(result.is_err());
        match result.unwrap_err() {
            LlmError::NoProvider(msg) => {
                assert!(msg.contains("ANTHROPIC_API_KEY"));
            }
            _ => panic!("Expected NoProvider error"),
        }

        // Restore original value if it existed
        match original {
            Some(val) => std::env::set_var("ANTHROPIC_API_KEY", val),
            None => std::env::remove_var("ANTHROPIC_API_KEY"),
        }
    }

    #[test]
    fn test_llm_provider_from_config_unknown_provider() {
        let config = LlmConfig {
            default_provider: "openai".to_string(),
            ollama: crate::config::global::OllamaConfig::default(),
            anthropic: crate::config::global::AnthropicConfig::default(),
            heartbeat_concurrency: 1,
            heartbeat_retention: 10,
        };

        let result = LlmProvider::from_config(&config);
        assert!(result.is_err());
        match result.unwrap_err() {
            LlmError::NoProvider(msg) => {
                assert!(msg.contains("openai"));
            }
            _ => panic!("Expected NoProvider error"),
        }
    }

    #[test]
    fn test_check_ollama_health_returns_false_for_unreachable() {
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_millis(100))
            .build()
            .unwrap();

        // Use port 1 which should be unreachable
        let healthy = check_ollama_health(&client, "http://127.0.0.1:1");
        assert!(!healthy);
    }

    #[test]
    fn test_llm_error_display() {
        let err = LlmError::Connection("timeout".to_string());
        assert_eq!(format!("{}", err), "Connection error: timeout");

        let err = LlmError::ApiError(404, "not found".to_string());
        assert_eq!(format!("{}", err), "API error (404): not found");

        let err = LlmError::NoProvider("no key".to_string());
        assert_eq!(format!("{}", err), "No provider: no key");
    }
}
