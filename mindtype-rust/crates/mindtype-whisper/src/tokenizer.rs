//! Whisper tokenizer
//!
//! Handles encoding/decoding of tokens for Whisper model.
//! Uses a simplified vocabulary based on Whisper's tiktoken encoding.

use crate::error::WhisperError;
use std::collections::HashMap;
use std::path::Path;

/// Special tokens for Whisper
pub mod special_tokens {
    // These are the standard Whisper special token IDs
    pub const SOT: i64 = 50258;              // Start of transcript
    pub const EOT: i64 = 50257;              // End of transcript
    pub const TRANSLATE: i64 = 50358;        // Translate task
    pub const TRANSCRIBE: i64 = 50359;       // Transcribe task
    pub const NO_TIMESTAMPS: i64 = 50363;    // Suppress timestamps
    pub const NO_SPEECH: i64 = 50362;        // No speech detected

    // Language tokens (subset - add more as needed)
    pub const LANG_EN: i64 = 50259;
    pub const LANG_RU: i64 = 50299;
    pub const LANG_ES: i64 = 50262;
    pub const LANG_DE: i64 = 50261;
    pub const LANG_FR: i64 = 50265;
    pub const LANG_ZH: i64 = 50260;
    pub const LANG_JA: i64 = 50266;
    pub const LANG_KO: i64 = 50264;
}

/// Whisper tokenizer
pub struct WhisperTokenizer {
    /// Token ID to string mapping
    id_to_token: HashMap<i64, String>,
    /// String to token ID mapping
    token_to_id: HashMap<String, i64>,
    /// Language code to token ID mapping
    lang_to_id: HashMap<String, i64>,
}

impl WhisperTokenizer {
    /// Create a new tokenizer with embedded vocabulary
    pub fn new() -> Self {
        let mut id_to_token = HashMap::new();
        let mut token_to_id = HashMap::new();
        let lang_to_id = Self::create_language_map();

        // Add special tokens
        id_to_token.insert(special_tokens::SOT, "<|startoftranscript|>".to_string());
        id_to_token.insert(special_tokens::EOT, "<|endoftext|>".to_string());
        id_to_token.insert(special_tokens::TRANSCRIBE, "<|transcribe|>".to_string());
        id_to_token.insert(special_tokens::TRANSLATE, "<|translate|>".to_string());
        id_to_token.insert(special_tokens::NO_TIMESTAMPS, "<|notimestamps|>".to_string());
        id_to_token.insert(special_tokens::NO_SPEECH, "<|nospeech|>".to_string());

        // Build reverse mapping
        for (&id, token) in &id_to_token {
            token_to_id.insert(token.clone(), id);
        }

        Self {
            id_to_token,
            token_to_id,
            lang_to_id,
        }
    }

    /// Load tokenizer from vocab file (JSON format)
    pub fn from_file(vocab_path: &Path) -> Result<Self, WhisperError> {
        let content = std::fs::read_to_string(vocab_path)?;
        let vocab: HashMap<String, i64> = serde_json::from_str(&content)
            .map_err(|e| WhisperError::ModelLoadError(format!("Failed to parse vocab: {}", e)))?;

        let mut id_to_token = HashMap::new();
        let mut token_to_id = HashMap::new();

        for (token, id) in vocab {
            id_to_token.insert(id, token.clone());
            token_to_id.insert(token, id);
        }

        let lang_to_id = Self::create_language_map();

        Ok(Self {
            id_to_token,
            token_to_id,
            lang_to_id,
        })
    }

    /// Create language code to token ID mapping
    fn create_language_map() -> HashMap<String, i64> {
        let mut map = HashMap::new();
        map.insert("en".to_string(), special_tokens::LANG_EN);
        map.insert("ru".to_string(), special_tokens::LANG_RU);
        map.insert("es".to_string(), special_tokens::LANG_ES);
        map.insert("de".to_string(), special_tokens::LANG_DE);
        map.insert("fr".to_string(), special_tokens::LANG_FR);
        map.insert("zh".to_string(), special_tokens::LANG_ZH);
        map.insert("ja".to_string(), special_tokens::LANG_JA);
        map.insert("ko".to_string(), special_tokens::LANG_KO);
        map
    }

    /// Get start of transcript token
    pub fn sot_token(&self) -> i64 {
        special_tokens::SOT
    }

    /// Get end of transcript token
    pub fn eot_token(&self) -> i64 {
        special_tokens::EOT
    }

    /// Get transcribe task token
    pub fn transcribe_token(&self) -> i64 {
        special_tokens::TRANSCRIBE
    }

    /// Get no timestamps token
    pub fn no_timestamps_token(&self) -> i64 {
        special_tokens::NO_TIMESTAMPS
    }

    /// Get language token for a language code
    pub fn language_token(&self, lang: &str) -> Result<i64, WhisperError> {
        self.lang_to_id
            .get(lang)
            .copied()
            .ok_or_else(|| WhisperError::TranscriptionFailed(format!("Unknown language: {}", lang)))
    }

    /// Check if token is a special token (should not be decoded)
    pub fn is_special_token(&self, token_id: i64) -> bool {
        token_id >= 50257
    }

    /// Decode a sequence of tokens to text
    pub fn decode(&self, tokens: &[i64]) -> Result<String, WhisperError> {
        let mut result = String::new();

        for &token_id in tokens {
            // Skip special tokens
            if self.is_special_token(token_id) {
                continue;
            }

            // Look up token in vocabulary
            if let Some(token) = self.id_to_token.get(&token_id) {
                result.push_str(token);
            } else {
                // For tokens not in our simplified vocab, use a placeholder
                // In production, you'd have the full vocabulary loaded
                // For now, try to decode as UTF-8 bytes if it looks like a byte token
                if token_id < 256 {
                    if let Some(c) = char::from_u32(token_id as u32) {
                        result.push(c);
                    }
                }
            }
        }

        // Clean up the result
        let cleaned = result
            .replace("Ġ", " ")  // GPT-2 space encoding
            .replace("Ċ", "\n") // GPT-2 newline encoding
            .trim()
            .to_string();

        Ok(cleaned)
    }

    /// Decode a single token to string (for debugging)
    pub fn decode_token(&self, token_id: i64) -> String {
        self.id_to_token
            .get(&token_id)
            .cloned()
            .unwrap_or_else(|| format!("[{}]", token_id))
    }

    /// Get initial prompt tokens for a language
    pub fn initial_tokens(&self, language: Option<&str>) -> Vec<i64> {
        let mut tokens = vec![self.sot_token()];

        // Add language token if specified
        if let Some(lang) = language {
            if let Ok(lang_token) = self.language_token(lang) {
                tokens.push(lang_token);
            }
        }

        // Add transcribe task and no timestamps
        tokens.push(self.transcribe_token());
        tokens.push(self.no_timestamps_token());

        tokens
    }
}

impl Default for WhisperTokenizer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_special_tokens() {
        let tokenizer = WhisperTokenizer::new();
        assert!(tokenizer.is_special_token(special_tokens::SOT));
        assert!(tokenizer.is_special_token(special_tokens::EOT));
        assert!(!tokenizer.is_special_token(100));
    }

    #[test]
    fn test_initial_tokens() {
        let tokenizer = WhisperTokenizer::new();
        let tokens = tokenizer.initial_tokens(Some("en"));
        assert_eq!(tokens[0], special_tokens::SOT);
        assert!(tokens.contains(&special_tokens::LANG_EN));
        assert!(tokens.contains(&special_tokens::TRANSCRIBE));
    }
}
