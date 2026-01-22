//! Anthropic (Claude) provider implementation

use crate::error::LlmError;
use crate::provider::{default_summary_prompt, SummaryProvider, SummaryRequest, SummaryResponse};
use async_trait::async_trait;
use serde::Deserialize;
use tracing::debug;

const API_URL: &str = "https://api.anthropic.com/v1/messages";
const API_VERSION: &str = "2023-06-01";

#[derive(Debug, Deserialize)]
struct AnthropicResponse {
    content: Vec<ContentBlock>,
    model: String,
    usage: Usage,
}

#[derive(Debug, Deserialize)]
struct ContentBlock {
    text: String,
}

#[derive(Debug, Deserialize)]
struct Usage {
    input_tokens: u32,
    output_tokens: u32,
}

#[derive(Debug, Deserialize)]
struct ErrorResponse {
    error: ApiError,
}

#[derive(Debug, Deserialize)]
struct ApiError {
    message: String,
}

/// Anthropic provider
pub struct AnthropicProvider {
    api_key: String,
    model: String,
    client: reqwest::Client,
}

impl AnthropicProvider {
    /// Create new provider
    pub fn new(api_key: &str, model: Option<&str>) -> Self {
        Self {
            api_key: api_key.to_string(),
            model: model.unwrap_or("claude-3-5-haiku-latest").to_string(),
            client: reqwest::Client::new(),
        }
    }

    /// Set model to use
    pub fn with_model(mut self, model: &str) -> Self {
        self.model = model.to_string();
        self
    }
}

#[async_trait]
impl SummaryProvider for AnthropicProvider {
    fn name(&self) -> &'static str {
        "Anthropic"
    }

    fn requires_api_key(&self) -> bool {
        true
    }

    fn is_configured(&self) -> bool {
        !self.api_key.is_empty()
    }

    async fn validate_credentials(&self) -> Result<bool, LlmError> {
        // Anthropic doesn't have a simple validation endpoint
        // We'll try a minimal completion
        let response = self
            .client
            .post(API_URL)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", API_VERSION)
            .header("content-type", "application/json")
            .json(&serde_json::json!({
                "model": self.model,
                "max_tokens": 1,
                "messages": [{"role": "user", "content": "Hi"}]
            }))
            .send()
            .await?;

        Ok(response.status().is_success())
    }

    async fn summarize(&self, request: SummaryRequest) -> Result<SummaryResponse, LlmError> {
        let prompt = request
            .prompt
            .unwrap_or_else(|| default_summary_prompt(request.language.as_deref()));

        let full_prompt = format!("{}\n\nTranscription:\n{}", prompt, request.text);

        debug!("Sending request to Anthropic ({})", self.model);

        let response = self
            .client
            .post(API_URL)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", API_VERSION)
            .header("content-type", "application/json")
            .json(&serde_json::json!({
                "model": self.model,
                "max_tokens": request.max_tokens.unwrap_or(2048),
                "messages": [{"role": "user", "content": full_prompt}]
            }))
            .send()
            .await?;

        let status = response.status();

        if !status.is_success() {
            let error: ErrorResponse = response.json().await?;
            return Err(LlmError::ApiError(error.error.message));
        }

        let body: AnthropicResponse = response.json().await?;

        let summary = body
            .content
            .first()
            .map(|c| c.text.clone())
            .ok_or_else(|| LlmError::ApiError("No response from model".to_string()))?;

        Ok(SummaryResponse {
            summary,
            input_tokens: Some(body.usage.input_tokens),
            output_tokens: Some(body.usage.output_tokens),
            model: body.model,
        })
    }

    fn available_models(&self) -> Vec<String> {
        vec![
            "claude-3-5-sonnet-latest".to_string(),
            "claude-3-5-haiku-latest".to_string(),
            "claude-3-opus-latest".to_string(),
        ]
    }

    fn default_model(&self) -> &str {
        "claude-3-5-haiku-latest"
    }
}
