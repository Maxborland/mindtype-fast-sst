//! Provider trait definition

use crate::error::LlmError;
use async_trait::async_trait;

/// Request for summarization
#[derive(Debug, Clone)]
pub struct SummaryRequest {
    /// The text to summarize
    pub text: String,
    /// Custom prompt/instructions
    pub prompt: Option<String>,
    /// Target language for summary (optional)
    pub language: Option<String>,
    /// Maximum tokens for response
    pub max_tokens: Option<u32>,
}

impl SummaryRequest {
    /// Create a new summary request
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            prompt: None,
            language: None,
            max_tokens: None,
        }
    }

    /// Set custom prompt
    pub fn with_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.prompt = Some(prompt.into());
        self
    }

    /// Set target language
    pub fn with_language(mut self, language: impl Into<String>) -> Self {
        self.language = Some(language.into());
        self
    }

    /// Set max tokens
    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }
}

/// Response from summarization
#[derive(Debug, Clone)]
pub struct SummaryResponse {
    /// The generated summary
    pub summary: String,
    /// Tokens used for input
    pub input_tokens: Option<u32>,
    /// Tokens used for output
    pub output_tokens: Option<u32>,
    /// Model used
    pub model: String,
}

/// Trait for summary providers
#[async_trait]
pub trait SummaryProvider: Send + Sync {
    /// Provider name
    fn name(&self) -> &'static str;

    /// Whether this provider requires an API key
    fn requires_api_key(&self) -> bool;

    /// Check if provider is configured and ready
    fn is_configured(&self) -> bool;

    /// Validate credentials with the API
    async fn validate_credentials(&self) -> Result<bool, LlmError>;

    /// Summarize text
    async fn summarize(&self, request: SummaryRequest) -> Result<SummaryResponse, LlmError>;

    /// Get available models
    fn available_models(&self) -> Vec<String>;

    /// Get default model
    fn default_model(&self) -> &str;
}

/// Default summarization prompt
pub fn default_summary_prompt(language: Option<&str>) -> String {
    let lang_instruction = language
        .map(|l| format!(" Respond in {}.", l))
        .unwrap_or_default();

    format!(
        "Summarize the following transcription into clear, well-structured notes. \
         Include key points, decisions, and action items if applicable. \
         Use bullet points for clarity.{}\n\n",
        lang_instruction
    )
}
