//! MindType Text Processor
//!
//! Text processing utilities for transcription cleanup:
//! - Filler word removal (Russian and English)
//! - Text normalization (numbers, dates, time, currency)

mod fillers;
mod normalizer;
mod error;

pub use fillers::FillerRemover;
pub use normalizer::TextNormalizer;
pub use error::ProcessorError;

/// Text processing configuration
#[derive(Debug, Clone)]
pub struct ProcessorConfig {
    /// Remove filler words
    pub remove_fillers: bool,
    /// Preserve context when removing fillers (keeps meaningful uses)
    pub filler_preserve_context: bool,
    /// Normalize numbers (word to digit)
    pub normalize_numbers: bool,
    /// Normalize dates
    pub normalize_dates: bool,
    /// Normalize time
    pub normalize_time: bool,
    /// Normalize currency
    pub normalize_currency: bool,
}

impl Default for ProcessorConfig {
    fn default() -> Self {
        Self {
            remove_fillers: true,
            filler_preserve_context: true,
            normalize_numbers: true,
            normalize_dates: true,
            normalize_time: true,
            normalize_currency: true,
        }
    }
}

/// Main text processor that combines filler removal and normalization
pub struct TextProcessor {
    config: ProcessorConfig,
    filler_remover: FillerRemover,
    normalizer: TextNormalizer,
}

impl TextProcessor {
    /// Create a new text processor with default config
    pub fn new() -> Self {
        Self::with_config(ProcessorConfig::default())
    }

    /// Create a new text processor with custom config
    pub fn with_config(config: ProcessorConfig) -> Self {
        Self {
            filler_remover: FillerRemover::new(config.filler_preserve_context),
            normalizer: TextNormalizer::new(),
            config,
        }
    }

    /// Process text: remove fillers and normalize
    pub fn process(&self, text: &str, language: Option<&str>) -> String {
        let mut result = text.to_string();

        // Detect language if not provided
        let lang = language.unwrap_or_else(|| detect_language(text));

        // Remove fillers first
        if self.config.remove_fillers {
            result = self.filler_remover.remove(&result, lang);
        }

        // Normalize text
        if self.config.normalize_numbers
            || self.config.normalize_dates
            || self.config.normalize_time
            || self.config.normalize_currency
        {
            result = self.normalizer.normalize(&result, lang, &self.config);
        }

        result
    }
}

impl Default for TextProcessor {
    fn default() -> Self {
        Self::new()
    }
}

/// Detect language from text based on character frequency
pub fn detect_language(text: &str) -> &'static str {
    let cyrillic_count = text.chars().filter(|c| matches!(c, '\u{0400}'..='\u{04FF}')).count();
    let latin_count = text.chars().filter(|c| c.is_ascii_alphabetic()).count();

    if cyrillic_count > latin_count {
        "ru"
    } else {
        "en"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_language() {
        assert_eq!(detect_language("Hello world"), "en");
        assert_eq!(detect_language("Привет мир"), "ru");
        assert_eq!(detect_language("Привет world"), "ru"); // Mixed, more cyrillic
    }

    #[test]
    fn test_processor_basic() {
        let processor = TextProcessor::new();

        // English filler removal
        let result = processor.process("I, uh, think we should, um, proceed", Some("en"));
        assert!(!result.contains("uh"));
        assert!(!result.contains("um"));

        // Russian filler removal
        let result = processor.process("Ну, эээ, давайте обсудим, типа, бюджет", Some("ru"));
        assert!(!result.contains("эээ"));
        assert!(!result.contains("типа"));
    }
}
