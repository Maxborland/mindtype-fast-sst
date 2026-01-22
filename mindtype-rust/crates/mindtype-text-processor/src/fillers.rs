//! Filler word removal for Russian and English

use once_cell::sync::Lazy;
use regex::Regex;

/// Russian filler words and phrases
static FILLERS_RU: Lazy<Vec<&str>> = Lazy::new(|| {
    vec![
        // Sound fillers
        "эээ", "ээ", "э-э-э", "э-э",
        "ммм", "мм", "м-м-м", "м-м",
        "ааа", "а-а-а",
        "угу", "ага",
        // Filler words
        "ну", "вот", "так", "того", "значит", "короче",
        "блин", "типа", "прикинь", "прикиньте", "слышь", "чё", "чо",
        // Filler phrases (longer ones first)
        "на самом деле", "собственно говоря", "как говорится",
        "честно говоря", "если честно", "грубо говоря", "скажем так",
        "в принципе", "в общем", "так сказать", "по факту",
        "то есть", "как бы", "по сути", "по идее", "в целом",
        "собственно",
    ]
});

/// English filler words and phrases
static FILLERS_EN: Lazy<Vec<&str>> = Lazy::new(|| {
    vec![
        // Sound fillers
        "uh", "uhh", "uhhh",
        "um", "umm", "ummm",
        "er", "err",
        "ah", "ahh",
        "hmm", "hm",
        // Filler words
        "like", "basically", "actually", "literally", "honestly",
        "seriously", "obviously", "apparently", "definitely",
        // Filler phrases (longer ones first)
        "at the end of the day", "as a matter of fact",
        "for what it's worth", "to be honest",
        "you know", "i mean", "you see",
        "kind of", "sort of",
    ]
});

/// Context patterns where words should NOT be removed (Russian)
static CONTEXT_EXCEPTIONS_RU: Lazy<Vec<(&str, Regex)>> = Lazy::new(|| {
    vec![
        ("ну", Regex::new(r"(?i)ну и что|ну-ну|ну да|ну нет").unwrap()),
        ("вот", Regex::new(r"(?i)вот это|вот так|вот именно").unwrap()),
        ("так", Regex::new(r"(?i)так и\b|так что|и так|так как|так же|вот так").unwrap()),
    ]
});

/// Context patterns where words should NOT be removed (English)
static CONTEXT_EXCEPTIONS_EN: Lazy<Vec<(&str, Regex)>> = Lazy::new(|| {
    vec![
        ("like", Regex::new(r"(?i)would like|looks like|feels like|sounds like|seems like|just like|like this|like that|like \d+").unwrap()),
    ]
});

/// Filler word remover
pub struct FillerRemover {
    preserve_context: bool,
}

impl FillerRemover {
    /// Create a new filler remover
    pub fn new(preserve_context: bool) -> Self {
        Self { preserve_context }
    }

    /// Remove filler words from text
    pub fn remove(&self, text: &str, language: &str) -> String {
        if text.is_empty() {
            return text.to_string();
        }

        let fillers: &[&str] = if language == "ru" {
            &FILLERS_RU
        } else {
            &FILLERS_EN
        };

        let exceptions = if language == "ru" {
            &*CONTEXT_EXCEPTIONS_RU
        } else {
            &*CONTEXT_EXCEPTIONS_EN
        };

        let mut result = text.to_string();

        // Process fillers from longest to shortest to handle phrases first
        for filler in fillers.iter() {
            result = self.remove_filler(&result, filler, exceptions);
        }

        // Clean up extra spaces and punctuation
        result = self.cleanup(&result, text);

        result
    }

    fn remove_filler(
        &self,
        text: &str,
        filler: &str,
        exceptions: &[(&str, Regex)],
    ) -> String {
        // Build pattern for the filler
        let escaped = regex::escape(filler);
        let pattern = if filler.contains(' ') {
            // Multi-word filler
            format!(r"(?i)(?:^|[\s,]){}(?:[\s,.]|$)", escaped)
        } else {
            // Single word filler
            format!(r"(?i)\b{}\b", escaped)
        };

        let re = match Regex::new(&pattern) {
            Ok(r) => r,
            Err(_) => return text.to_string(),
        };

        let mut result = text.to_string();
        let mut offset: i64 = 0;

        // Find all matches
        let matches: Vec<_> = re.find_iter(text).collect();

        for m in matches {
            let adjusted_start = (m.start() as i64 + offset) as usize;
            let adjusted_end = (m.end() as i64 + offset) as usize;

            // Check context exceptions
            if self.preserve_context && self.should_keep(filler, &result, adjusted_start, exceptions) {
                continue;
            }

            // Remove the filler
            let before = &result[..adjusted_start];
            let after = &result[adjusted_end..];

            // Handle surrounding punctuation
            let mut new_before = before.to_string();
            let mut new_after = after.to_string();

            // Remove trailing comma from before if filler was followed by comma
            if new_before.trim_end().ends_with(',') && !new_after.trim_start().starts_with(',') {
                new_before = new_before.trim_end().trim_end_matches(',').to_string();
                new_before.push(' ');
            }

            // Remove leading comma from after if needed
            if new_after.trim_start().starts_with(',') {
                new_after = new_after.trim_start().trim_start_matches(',').to_string();
            }

            let new_text = format!("{}{}", new_before, new_after);
            let len_diff = result.len() as i64 - new_text.len() as i64;
            offset -= len_diff;
            result = new_text;
        }

        result
    }

    fn should_keep(
        &self,
        filler: &str,
        text: &str,
        position: usize,
        exceptions: &[(&str, Regex)],
    ) -> bool {
        let filler_lower = filler.to_lowercase();

        for (word, pattern) in exceptions {
            if *word == filler_lower {
                // Check context around the position (handle UTF-8 boundaries)
                let mut start = position.saturating_sub(20);
                let mut end = (position + filler.len() + 20).min(text.len());

                // Adjust start to char boundary
                while start > 0 && !text.is_char_boundary(start) {
                    start -= 1;
                }

                // Adjust end to char boundary
                while end < text.len() && !text.is_char_boundary(end) {
                    end += 1;
                }

                let context = &text[start..end];

                if pattern.is_match(context) {
                    return true;
                }
            }
        }

        false
    }

    fn cleanup(&self, result: &str, original: &str) -> String {
        // Remove multiple spaces
        let re_spaces = Regex::new(r"\s+").unwrap();
        let mut cleaned = re_spaces.replace_all(result, " ").to_string();

        // Remove multiple commas
        let re_commas = Regex::new(r"\s*,\s*,\s*").unwrap();
        cleaned = re_commas.replace_all(&cleaned, ", ").to_string();

        // Remove leading/trailing comma
        let re_leading_comma = Regex::new(r"^\s*,\s*").unwrap();
        cleaned = re_leading_comma.replace(&cleaned, "").to_string();

        let re_trailing_comma = Regex::new(r"\s*,\s*$").unwrap();
        cleaned = re_trailing_comma.replace(&cleaned, "").to_string();

        cleaned = cleaned.trim().to_string();

        // Restore capitalization if original started with uppercase
        if !cleaned.is_empty() {
            let first_char_original = original.chars().next();
            let first_char_cleaned = cleaned.chars().next();

            if let (Some(orig), Some(clean)) = (first_char_original, first_char_cleaned) {
                if orig.is_uppercase() && clean.is_lowercase() {
                    let mut chars: Vec<char> = cleaned.chars().collect();
                    chars[0] = clean.to_uppercase().next().unwrap_or(clean);
                    cleaned = chars.into_iter().collect();
                }
            }
        }

        cleaned
    }

    /// Get list of fillers found in text
    pub fn get_fillers_found(&self, text: &str, language: &str) -> Vec<(String, usize)> {
        let fillers: &[&str] = if language == "ru" {
            &FILLERS_RU
        } else {
            &FILLERS_EN
        };

        let exceptions = if language == "ru" {
            &*CONTEXT_EXCEPTIONS_RU
        } else {
            &*CONTEXT_EXCEPTIONS_EN
        };

        let mut found = Vec::new();

        for filler in fillers.iter() {
            let escaped = regex::escape(filler);
            let pattern = format!(r"(?i)\b{}\b", escaped);

            if let Ok(re) = Regex::new(&pattern) {
                for m in re.find_iter(text) {
                    if !self.preserve_context || !self.should_keep(filler, text, m.start(), exceptions) {
                        found.push((m.as_str().to_string(), m.start()));
                    }
                }
            }
        }

        found.sort_by_key(|(_, pos)| *pos);
        found
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_russian_fillers() {
        let remover = FillerRemover::new(true);

        let result = remover.remove("Ну, эээ, давайте обсудим, типа, бюджет", "ru");
        assert!(!result.to_lowercase().contains("эээ"));
        assert!(!result.to_lowercase().contains("типа"));
        assert!(result.contains("бюджет"));
    }

    #[test]
    fn test_english_fillers() {
        let remover = FillerRemover::new(true);

        let result = remover.remove("I, um, think we should, like, proceed", "en");
        assert!(!result.to_lowercase().contains(" um"));
        // "like" might be kept in some contexts
        assert!(result.contains("proceed"));
    }

    #[test]
    fn test_context_preservation() {
        let remover = FillerRemover::new(true);

        // "would like" should be preserved
        let result = remover.remove("I would like to proceed", "en");
        assert!(result.contains("like"));

        // "ну да" should be preserved
        let result = remover.remove("Ну да, согласен", "ru");
        assert!(result.contains("Ну да") || result.contains("ну да"));
    }

    #[test]
    fn test_multi_word_fillers() {
        let remover = FillerRemover::new(true);

        let result = remover.remove("На самом деле, это важно", "ru");
        assert!(!result.to_lowercase().contains("на самом деле"));
        assert!(result.contains("важно"));
    }
}
