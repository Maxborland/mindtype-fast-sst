//! MindType Cloud provider implementation
//!
//! Uses credits system - no API key required from user.

use crate::error::LlmError;
use crate::provider::{default_summary_prompt, SummaryProvider, SummaryRequest, SummaryResponse};
use async_trait::async_trait;
use serde::Deserialize;
use tracing::{debug, info};

/// MindType Cloud API response
#[derive(Debug, Deserialize)]
struct CloudResponse {
    success: bool,
    summary: Option<String>,
    model: Option<String>,
    input_tokens: Option<u32>,
    output_tokens: Option<u32>,
    error: Option<String>,
}

/// MindType Cloud provider
pub struct MindTypeCloudProvider {
    api_base: String,
    license_key: String,
    client: reqwest::Client,
}

impl MindTypeCloudProvider {
    /// Create new provider with license key
    pub fn new(api_base: &str, license_key: &str) -> Self {
        Self {
            api_base: api_base.to_string(),
            license_key: license_key.to_string(),
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl SummaryProvider for MindTypeCloudProvider {
    fn name(&self) -> &'static str {
        "MindType Cloud"
    }

    fn requires_api_key(&self) -> bool {
        false
    }

    fn is_configured(&self) -> bool {
        !self.license_key.is_empty()
    }

    async fn validate_credentials(&self) -> Result<bool, LlmError> {
        // Just check if we have a license key
        // Actual validation happens during summarization
        Ok(!self.license_key.is_empty())
    }

    async fn summarize(&self, request: SummaryRequest) -> Result<SummaryResponse, LlmError> {
        let url = format!("{}/api/gateway/summarize", self.api_base);

        let prompt = request
            .prompt
            .unwrap_or_else(|| default_summary_prompt(request.language.as_deref()));

        debug!("Sending summarization request to MindType Cloud");

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.license_key))
            .json(&serde_json::json!({
                "text": request.text,
                "prompt": prompt,
                "language": request.language,
                "max_tokens": request.max_tokens.unwrap_or(2048),
            }))
            .send()
            .await?;

        let status = response.status();
        let body: CloudResponse = response.json().await?;

        if !body.success {
            return Err(LlmError::ApiError(
                body.error.unwrap_or_else(|| "Unknown error".to_string()),
            ));
        }

        let summary = body
            .summary
            .ok_or_else(|| LlmError::ApiError("No summary in response".to_string()))?;

        info!("Summary generated successfully via MindType Cloud");

        Ok(SummaryResponse {
            summary,
            input_tokens: body.input_tokens,
            output_tokens: body.output_tokens,
            model: body.model.unwrap_or_else(|| "mindtype-cloud".to_string()),
        })
    }

    fn available_models(&self) -> Vec<String> {
        vec!["auto".to_string()]
    }

    fn default_model(&self) -> &str {
        "auto"
    }
}
