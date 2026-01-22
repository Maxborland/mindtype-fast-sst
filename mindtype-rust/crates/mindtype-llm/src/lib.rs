//! MindType LLM Integration
//!
//! Provides AI summarization through various providers.

mod error;
mod provider;

// Providers
mod anthropic;
mod gemini;
mod mindtype_cloud;
mod ollama;
mod openai;
mod openrouter;
mod yandex;

pub use error::LlmError;
pub use provider::{SummaryProvider, SummaryRequest, SummaryResponse};

// Re-export providers
pub use anthropic::AnthropicProvider;
pub use gemini::GeminiProvider;
pub use mindtype_cloud::MindTypeCloudProvider;
pub use ollama::OllamaProvider;
pub use openai::OpenAiProvider;
pub use openrouter::OpenRouterProvider;
pub use yandex::YandexProvider;

/// Available LLM provider types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderType {
    /// MindType Cloud (uses credits)
    MindTypeCloud,
    /// OpenAI (GPT-4o, GPT-4o-mini)
    OpenAi,
    /// Anthropic (Claude)
    Anthropic,
    /// Google Gemini
    Gemini,
    /// OpenRouter (200+ models)
    OpenRouter,
    /// Yandex GPT
    Yandex,
    /// Ollama (local)
    Ollama,
}

impl ProviderType {
    /// Whether this provider requires an API key
    pub fn requires_api_key(&self) -> bool {
        !matches!(self, ProviderType::MindTypeCloud | ProviderType::Ollama)
    }

    /// Display name
    pub fn name(&self) -> &'static str {
        match self {
            ProviderType::MindTypeCloud => "MindType Cloud",
            ProviderType::OpenAi => "OpenAI",
            ProviderType::Anthropic => "Anthropic",
            ProviderType::Gemini => "Google Gemini",
            ProviderType::OpenRouter => "OpenRouter",
            ProviderType::Yandex => "Yandex GPT",
            ProviderType::Ollama => "Ollama (Local)",
        }
    }

    /// All provider types
    pub fn all() -> &'static [ProviderType] {
        &[
            ProviderType::MindTypeCloud,
            ProviderType::OpenAi,
            ProviderType::Anthropic,
            ProviderType::Gemini,
            ProviderType::OpenRouter,
            ProviderType::Yandex,
            ProviderType::Ollama,
        ]
    }
}

impl std::fmt::Display for ProviderType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_type_requires_api_key() {
        assert!(!ProviderType::MindTypeCloud.requires_api_key());
        assert!(!ProviderType::Ollama.requires_api_key());
        assert!(ProviderType::OpenAi.requires_api_key());
        assert!(ProviderType::Anthropic.requires_api_key());
        assert!(ProviderType::Gemini.requires_api_key());
        assert!(ProviderType::OpenRouter.requires_api_key());
        assert!(ProviderType::Yandex.requires_api_key());
    }

    #[test]
    fn test_provider_type_name() {
        assert_eq!(ProviderType::MindTypeCloud.name(), "MindType Cloud");
        assert_eq!(ProviderType::OpenAi.name(), "OpenAI");
        assert_eq!(ProviderType::Anthropic.name(), "Anthropic");
        assert_eq!(ProviderType::Gemini.name(), "Google Gemini");
        assert_eq!(ProviderType::OpenRouter.name(), "OpenRouter");
        assert_eq!(ProviderType::Yandex.name(), "Yandex GPT");
        assert_eq!(ProviderType::Ollama.name(), "Ollama (Local)");
    }

    #[test]
    fn test_provider_type_all() {
        let all = ProviderType::all();
        assert_eq!(all.len(), 7);
        assert!(all.contains(&ProviderType::MindTypeCloud));
        assert!(all.contains(&ProviderType::OpenAi));
        assert!(all.contains(&ProviderType::Anthropic));
        assert!(all.contains(&ProviderType::Gemini));
        assert!(all.contains(&ProviderType::OpenRouter));
        assert!(all.contains(&ProviderType::Yandex));
        assert!(all.contains(&ProviderType::Ollama));
    }

    #[test]
    fn test_provider_type_display() {
        assert_eq!(format!("{}", ProviderType::OpenAi), "OpenAI");
        assert_eq!(format!("{}", ProviderType::Anthropic), "Anthropic");
    }

    #[test]
    fn test_provider_type_equality() {
        assert_eq!(ProviderType::OpenAi, ProviderType::OpenAi);
        assert_ne!(ProviderType::OpenAi, ProviderType::Anthropic);
    }

    #[test]
    fn test_provider_type_clone() {
        let original = ProviderType::Gemini;
        let cloned = original.clone();
        assert_eq!(original, cloned);
    }
}
