//! OpenAI provider implementation

use crate::error::LlmError;
use crate::provider::{default_summary_prompt, SummaryProvider, SummaryRequest, SummaryResponse};
use async_trait::async_trait;
use serde::Deserialize;
use tracing::debug;

const API_URL: &str = "https://api.openai.com/v1/chat/completions";

#[derive(Debug, Deserialize)]
struct OpenAiResponse {
    choices: Vec<Choice>,
    usage: Option<Usage>,
    model: String,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: Message,
}

#[derive(Debug, Deserialize)]
struct Message {
    content: String,
}

#[derive(Debug, Deserialize)]
struct Usage {
    prompt_tokens: u32,
    completion_tokens: u32,
}

#[derive(Debug, Deserialize)]
struct ErrorResponse {
    error: ApiError,
}

#[derive(Debug, Deserialize)]
struct ApiError {
    message: String,
    #[serde(rename = "type")]
    error_type: Option<String>,
}

/// OpenAI provider
pub struct OpenAiProvider {
    api_key: String,
    model: String,
    client: reqwest::Client,
}

impl OpenAiProvider {
    /// Create new provider
    pub fn new(api_key: &str, model: Option<&str>) -> Self {
        Self {
            api_key: api_key.to_string(),
            model: model.unwrap_or("gpt-4o-mini").to_string(),
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
impl SummaryProvider for OpenAiProvider {
    fn name(&self) -> &'static str {
        "OpenAI"
    }

    fn requires_api_key(&self) -> bool {
        true
    }

    fn is_configured(&self) -> bool {
        !self.api_key.is_empty()
    }

    async fn validate_credentials(&self) -> Result<bool, LlmError> {
        // Try a minimal request to validate
        let response = self
            .client
            .get("https://api.openai.com/v1/models")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await?;

        Ok(response.status().is_success())
    }

    async fn summarize(&self, request: SummaryRequest) -> Result<SummaryResponse, LlmError> {
        let prompt = request
            .prompt
            .unwrap_or_else(|| default_summary_prompt(request.language.as_deref()));

        let full_prompt = format!("{}\n\nTranscription:\n{}", prompt, request.text);

        debug!("Sending request to OpenAI ({})", self.model);

        let response = self
            .client
            .post(API_URL)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&serde_json::json!({
                "model": self.model,
                "messages": [
                    {"role": "user", "content": full_prompt}
                ],
                "max_tokens": request.max_tokens.unwrap_or(2048),
                "temperature": 0.3,
            }))
            .send()
            .await?;

        let status = response.status();

        if !status.is_success() {
            let error: ErrorResponse = response.json().await?;
            return Err(LlmError::ApiError(error.error.message));
        }

        let body: OpenAiResponse = response.json().await?;

        let summary = body
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .ok_or_else(|| LlmError::ApiError("No response from model".to_string()))?;

        Ok(SummaryResponse {
            summary,
            input_tokens: body.usage.as_ref().map(|u| u.prompt_tokens),
            output_tokens: body.usage.as_ref().map(|u| u.completion_tokens),
            model: body.model,
        })
    }

    fn available_models(&self) -> Vec<String> {
        vec![
            "gpt-4o".to_string(),
            "gpt-4o-mini".to_string(),
            "gpt-4-turbo".to_string(),
            "gpt-3.5-turbo".to_string(),
        ]
    }

    fn default_model(&self) -> &str {
        "gpt-4o-mini"
    }
}
