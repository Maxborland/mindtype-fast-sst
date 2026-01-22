//! Ollama provider implementation (local models)

use crate::error::LlmError;
use crate::provider::{default_summary_prompt, SummaryProvider, SummaryRequest, SummaryResponse};
use async_trait::async_trait;
use serde::Deserialize;
use tracing::debug;

const DEFAULT_URL: &str = "http://localhost:11434";

#[derive(Debug, Deserialize)]
struct OllamaResponse {
    response: String,
    model: String,
    prompt_eval_count: Option<u32>,
    eval_count: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct ModelsResponse {
    models: Vec<ModelInfo>,
}

#[derive(Debug, Deserialize)]
struct ModelInfo {
    name: String,
}

/// Ollama provider (local)
pub struct OllamaProvider {
    base_url: String,
    model: String,
    client: reqwest::Client,
}

impl OllamaProvider {
    /// Create new provider
    pub fn new(base_url: Option<&str>, model: Option<&str>) -> Self {
        Self {
            base_url: base_url.unwrap_or(DEFAULT_URL).to_string(),
            model: model.unwrap_or("llama3.2").to_string(),
            client: reqwest::Client::new(),
        }
    }

    /// Set model to use
    pub fn with_model(mut self, model: &str) -> Self {
        self.model = model.to_string();
        self
    }

    /// Get list of available models from Ollama
    pub async fn list_models(&self) -> Result<Vec<String>, LlmError> {
        let url = format!("{}/api/tags", self.base_url);

        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(LlmError::NetworkError("Ollama not available".to_string()));
        }

        let body: ModelsResponse = response.json().await?;
        Ok(body.models.into_iter().map(|m| m.name).collect())
    }
}

#[async_trait]
impl SummaryProvider for OllamaProvider {
    fn name(&self) -> &'static str {
        "Ollama (Local)"
    }

    fn requires_api_key(&self) -> bool {
        false
    }

    fn is_configured(&self) -> bool {
        true // Always configured, just needs Ollama running
    }

    async fn validate_credentials(&self) -> Result<bool, LlmError> {
        // Check if Ollama is running
        let url = format!("{}/api/tags", self.base_url);
        match self.client.get(&url).send().await {
            Ok(response) => Ok(response.status().is_success()),
            Err(_) => Ok(false),
        }
    }

    async fn summarize(&self, request: SummaryRequest) -> Result<SummaryResponse, LlmError> {
        let prompt = request
            .prompt
            .unwrap_or_else(|| default_summary_prompt(request.language.as_deref()));

        let full_prompt = format!("{}\n\nTranscription:\n{}", prompt, request.text);

        let url = format!("{}/api/generate", self.base_url);

        debug!("Sending request to Ollama ({})", self.model);

        let response = self
            .client
            .post(&url)
            .json(&serde_json::json!({
                "model": self.model,
                "prompt": full_prompt,
                "stream": false,
                "options": {
                    "temperature": 0.3,
                    "num_predict": request.max_tokens.unwrap_or(2048)
                }
            }))
            .send()
            .await
            .map_err(|e| LlmError::NetworkError(format!("Ollama not available: {}", e)))?;

        let status = response.status();

        if !status.is_success() {
            let text = response.text().await?;
            return Err(LlmError::ApiError(text));
        }

        let body: OllamaResponse = response.json().await?;

        Ok(SummaryResponse {
            summary: body.response,
            input_tokens: body.prompt_eval_count,
            output_tokens: body.eval_count,
            model: body.model,
        })
    }

    fn available_models(&self) -> Vec<String> {
        // Common models - actual list fetched via list_models()
        vec![
            "llama3.2".to_string(),
            "llama3.1".to_string(),
            "qwen2.5".to_string(),
            "mistral".to_string(),
            "phi3".to_string(),
        ]
    }

    fn default_model(&self) -> &str {
        "llama3.2"
    }
}
