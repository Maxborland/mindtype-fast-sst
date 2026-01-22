//! Yandex GPT provider implementation

use crate::error::LlmError;
use crate::provider::{default_summary_prompt, SummaryProvider, SummaryRequest, SummaryResponse};
use async_trait::async_trait;
use serde::Deserialize;
use tracing::debug;

const API_URL: &str = "https://llm.api.cloud.yandex.net/foundationModels/v1/completion";

#[derive(Debug, Deserialize)]
struct YandexResponse {
    result: YandexResult,
}

#[derive(Debug, Deserialize)]
struct YandexResult {
    alternatives: Vec<Alternative>,
    usage: Usage,
    #[serde(rename = "modelVersion")]
    model_version: String,
}

#[derive(Debug, Deserialize)]
struct Alternative {
    message: YandexMessage,
}

#[derive(Debug, Deserialize)]
struct YandexMessage {
    text: String,
}

#[derive(Debug, Deserialize)]
struct Usage {
    #[serde(rename = "inputTextTokens")]
    input_text_tokens: String,
    #[serde(rename = "completionTokens")]
    completion_tokens: String,
}

#[derive(Debug, Deserialize)]
struct ErrorResponse {
    error: ApiError,
}

#[derive(Debug, Deserialize)]
struct ApiError {
    message: String,
}

/// Yandex GPT provider
pub struct YandexProvider {
    api_key: String,
    folder_id: String,
    model: String,
    client: reqwest::Client,
}

impl YandexProvider {
    /// Create new provider
    /// api_key is the IAM token or API key
    /// folder_id is the Yandex Cloud folder ID
    pub fn new(api_key: &str, folder_id: &str, model: Option<&str>) -> Self {
        Self {
            api_key: api_key.to_string(),
            folder_id: folder_id.to_string(),
            model: model.unwrap_or("yandexgpt-lite").to_string(),
            client: reqwest::Client::new(),
        }
    }

    /// Set model to use
    pub fn with_model(mut self, model: &str) -> Self {
        self.model = model.to_string();
        self
    }

    fn model_uri(&self) -> String {
        format!("gpt://{}/{}/latest", self.folder_id, self.model)
    }
}

#[async_trait]
impl SummaryProvider for YandexProvider {
    fn name(&self) -> &'static str {
        "Yandex GPT"
    }

    fn requires_api_key(&self) -> bool {
        true
    }

    fn is_configured(&self) -> bool {
        !self.api_key.is_empty() && !self.folder_id.is_empty()
    }

    async fn validate_credentials(&self) -> Result<bool, LlmError> {
        // Try a minimal request
        let response = self
            .client
            .post(API_URL)
            .header("Authorization", format!("Api-Key {}", self.api_key))
            .header("x-folder-id", &self.folder_id)
            .json(&serde_json::json!({
                "modelUri": self.model_uri(),
                "completionOptions": {
                    "maxTokens": 1
                },
                "messages": [{"role": "user", "text": "Hi"}]
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

        debug!("Sending request to Yandex GPT ({})", self.model);

        let response = self
            .client
            .post(API_URL)
            .header("Authorization", format!("Api-Key {}", self.api_key))
            .header("x-folder-id", &self.folder_id)
            .json(&serde_json::json!({
                "modelUri": self.model_uri(),
                "completionOptions": {
                    "stream": false,
                    "temperature": 0.3,
                    "maxTokens": request.max_tokens.unwrap_or(2048).to_string()
                },
                "messages": [{"role": "user", "text": full_prompt}]
            }))
            .send()
            .await?;

        let status = response.status();

        if !status.is_success() {
            let text = response.text().await?;
            return Err(LlmError::ApiError(text));
        }

        let body: YandexResponse = response.json().await?;

        let summary = body
            .result
            .alternatives
            .first()
            .map(|a| a.message.text.clone())
            .ok_or_else(|| LlmError::ApiError("No response from model".to_string()))?;

        Ok(SummaryResponse {
            summary,
            input_tokens: body.result.usage.input_text_tokens.parse().ok(),
            output_tokens: body.result.usage.completion_tokens.parse().ok(),
            model: format!("{}/{}", self.model, body.result.model_version),
        })
    }

    fn available_models(&self) -> Vec<String> {
        vec![
            "yandexgpt-lite".to_string(),
            "yandexgpt".to_string(),
            "yandexgpt-32k".to_string(),
        ]
    }

    fn default_model(&self) -> &str {
        "yandexgpt-lite"
    }
}
