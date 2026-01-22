//! Google Gemini provider implementation

use crate::error::LlmError;
use crate::provider::{default_summary_prompt, SummaryProvider, SummaryRequest, SummaryResponse};
use async_trait::async_trait;
use serde::Deserialize;
use tracing::debug;

const API_BASE: &str = "https://generativelanguage.googleapis.com/v1beta/models";

#[derive(Debug, Deserialize)]
struct GeminiResponse {
    candidates: Vec<Candidate>,
    #[serde(rename = "usageMetadata")]
    usage_metadata: Option<UsageMetadata>,
}

#[derive(Debug, Deserialize)]
struct Candidate {
    content: Content,
}

#[derive(Debug, Deserialize)]
struct Content {
    parts: Vec<Part>,
}

#[derive(Debug, Deserialize)]
struct Part {
    text: String,
}

#[derive(Debug, Deserialize)]
struct UsageMetadata {
    #[serde(rename = "promptTokenCount")]
    prompt_token_count: u32,
    #[serde(rename = "candidatesTokenCount")]
    candidates_token_count: u32,
}

#[derive(Debug, Deserialize)]
struct ErrorResponse {
    error: ApiError,
}

#[derive(Debug, Deserialize)]
struct ApiError {
    message: String,
}

/// Google Gemini provider
pub struct GeminiProvider {
    api_key: String,
    model: String,
    client: reqwest::Client,
}

impl GeminiProvider {
    /// Create new provider
    pub fn new(api_key: &str, model: Option<&str>) -> Self {
        Self {
            api_key: api_key.to_string(),
            model: model.unwrap_or("gemini-1.5-flash").to_string(),
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
impl SummaryProvider for GeminiProvider {
    fn name(&self) -> &'static str {
        "Google Gemini"
    }

    fn requires_api_key(&self) -> bool {
        true
    }

    fn is_configured(&self) -> bool {
        !self.api_key.is_empty()
    }

    async fn validate_credentials(&self) -> Result<bool, LlmError> {
        let url = format!("{}?key={}", API_BASE, self.api_key);
        let response = self.client.get(&url).send().await?;
        Ok(response.status().is_success())
    }

    async fn summarize(&self, request: SummaryRequest) -> Result<SummaryResponse, LlmError> {
        let prompt = request
            .prompt
            .unwrap_or_else(|| default_summary_prompt(request.language.as_deref()));

        let full_prompt = format!("{}\n\nTranscription:\n{}", prompt, request.text);

        let url = format!(
            "{}/{}:generateContent?key={}",
            API_BASE, self.model, self.api_key
        );

        debug!("Sending request to Gemini ({})", self.model);

        let response = self
            .client
            .post(&url)
            .json(&serde_json::json!({
                "contents": [{
                    "parts": [{"text": full_prompt}]
                }],
                "generationConfig": {
                    "maxOutputTokens": request.max_tokens.unwrap_or(2048),
                    "temperature": 0.3
                }
            }))
            .send()
            .await?;

        let status = response.status();

        if !status.is_success() {
            let error: ErrorResponse = response.json().await?;
            return Err(LlmError::ApiError(error.error.message));
        }

        let body: GeminiResponse = response.json().await?;

        let summary = body
            .candidates
            .first()
            .and_then(|c| c.content.parts.first())
            .map(|p| p.text.clone())
            .ok_or_else(|| LlmError::ApiError("No response from model".to_string()))?;

        Ok(SummaryResponse {
            summary,
            input_tokens: body.usage_metadata.as_ref().map(|u| u.prompt_token_count),
            output_tokens: body
                .usage_metadata
                .as_ref()
                .map(|u| u.candidates_token_count),
            model: self.model.clone(),
        })
    }

    fn available_models(&self) -> Vec<String> {
        vec![
            "gemini-1.5-flash".to_string(),
            "gemini-1.5-pro".to_string(),
            "gemini-1.0-pro".to_string(),
        ]
    }

    fn default_model(&self) -> &str {
        "gemini-1.5-flash"
    }
}
