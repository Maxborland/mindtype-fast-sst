//! Text normalization: numbers, dates, time, currency

use crate::ProcessorConfig;
use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::HashMap;

/// Russian number words to values
static NUMBERS_RU: Lazy<HashMap<&str, i64>> = Lazy::new(|| {
    let mut m = HashMap::new();
    // Units
    m.insert("ноль", 0);
    m.insert("нуль", 0);
    m.insert("один", 1);
    m.insert("одна", 1);
    m.insert("одно", 1);
    m.insert("первый", 1);
    m.insert("первого", 1);
    m.insert("первое", 1);
    m.insert("два", 2);
    m.insert("две", 2);
    m.insert("второй", 2);
    m.insert("второго", 2);
    m.insert("три", 3);
    m.insert("третий", 3);
    m.insert("третьего", 3);
    m.insert("четыре", 4);
    m.insert("четвёртый", 4);
    m.insert("четвертый", 4);
    m.insert("четвёртого", 4);
    m.insert("четвертого", 4);
    m.insert("пять", 5);
    m.insert("пятый", 5);
    m.insert("пятого", 5);
    m.insert("шесть", 6);
    m.insert("шестой", 6);
    m.insert("шестого", 6);
    m.insert("семь", 7);
    m.insert("седьмой", 7);
    m.insert("седьмого", 7);
    m.insert("восемь", 8);
    m.insert("восьмой", 8);
    m.insert("восьмого", 8);
    m.insert("девять", 9);
    m.insert("девятый", 9);
    m.insert("девятого", 9);

    // Teens
    m.insert("десять", 10);
    m.insert("десятый", 10);
    m.insert("десятого", 10);
    m.insert("одиннадцать", 11);
    m.insert("одиннадцатый", 11);
    m.insert("одиннадцатого", 11);
    m.insert("двенадцать", 12);
    m.insert("двенадцатый", 12);
    m.insert("двенадцатого", 12);
    m.insert("тринадцать", 13);
    m.insert("тринадцатый", 13);
    m.insert("тринадцатого", 13);
    m.insert("четырнадцать", 14);
    m.insert("четырнадцатый", 14);
    m.insert("четырнадцатого", 14);
    m.insert("пятнадцать", 15);
    m.insert("пятнадцатый", 15);
    m.insert("пятнадцатого", 15);
    m.insert("шестнадцать", 16);
    m.insert("шестнадцатый", 16);
    m.insert("шестнадцатого", 16);
    m.insert("семнадцать", 17);
    m.insert("семнадцатый", 17);
    m.insert("семнадцатого", 17);
    m.insert("восемнадцать", 18);
    m.insert("восемнадцатый", 18);
    m.insert("восемнадцатого", 18);
    m.insert("девятнадцать", 19);
    m.insert("девятнадцатый", 19);
    m.insert("девятнадцатого", 19);

    // Tens
    m.insert("двадцать", 20);
    m.insert("двадцатый", 20);
    m.insert("двадцатого", 20);
    m.insert("тридцать", 30);
    m.insert("тридцатый", 30);
    m.insert("тридцатого", 30);
    m.insert("сорок", 40);
    m.insert("сороковой", 40);
    m.insert("сорокового", 40);
    m.insert("пятьдесят", 50);
    m.insert("пятидесятый", 50);
    m.insert("пятидесятого", 50);
    m.insert("шестьдесят", 60);
    m.insert("шестидесятый", 60);
    m.insert("семьдесят", 70);
    m.insert("семидесятый", 70);
    m.insert("восемьдесят", 80);
    m.insert("восьмидесятый", 80);
    m.insert("девяносто", 90);
    m.insert("девяностый", 90);

    // Hundreds
    m.insert("сто", 100);
    m.insert("сотый", 100);
    m.insert("двести", 200);
    m.insert("двухсотый", 200);
    m.insert("триста", 300);
    m.insert("трёхсотый", 300);
    m.insert("трехсотый", 300);
    m.insert("четыреста", 400);
    m.insert("четырёхсотый", 400);
    m.insert("пятьсот", 500);
    m.insert("пятисотый", 500);
    m.insert("шестьсот", 600);
    m.insert("шестисотый", 600);
    m.insert("семьсот", 700);
    m.insert("семисотый", 700);
    m.insert("восемьсот", 800);
    m.insert("восьмисотый", 800);
    m.insert("девятьсот", 900);
    m.insert("девятисотый", 900);

    m
});

/// Russian multipliers
static MULTIPLIERS_RU: Lazy<HashMap<&str, i64>> = Lazy::new(|| {
    let mut m = HashMap::new();
    m.insert("тысяча", 1000);
    m.insert("тысячи", 1000);
    m.insert("тысяч", 1000);
    m.insert("тыс", 1000);
    m.insert("миллион", 1_000_000);
    m.insert("миллиона", 1_000_000);
    m.insert("миллионов", 1_000_000);
    m.insert("млн", 1_000_000);
    m.insert("миллиард", 1_000_000_000);
    m.insert("миллиарда", 1_000_000_000);
    m.insert("миллиардов", 1_000_000_000);
    m.insert("млрд", 1_000_000_000);
    m
});

/// Russian months
static MONTHS_RU: Lazy<HashMap<&str, &str>> = Lazy::new(|| {
    let mut m = HashMap::new();
    m.insert("января", "января");
    m.insert("январь", "января");
    m.insert("янв", "января");
    m.insert("февраля", "февраля");
    m.insert("февраль", "февраля");
    m.insert("фев", "февраля");
    m.insert("марта", "марта");
    m.insert("март", "марта");
    m.insert("мар", "марта");
    m.insert("апреля", "апреля");
    m.insert("апрель", "апреля");
    m.insert("апр", "апреля");
    m.insert("мая", "мая");
    m.insert("май", "мая");
    m.insert("июня", "июня");
    m.insert("июнь", "июня");
    m.insert("июн", "июня");
    m.insert("июля", "июля");
    m.insert("июль", "июля");
    m.insert("июл", "июля");
    m.insert("августа", "августа");
    m.insert("август", "августа");
    m.insert("авг", "августа");
    m.insert("сентября", "сентября");
    m.insert("сентябрь", "сентября");
    m.insert("сен", "сентября");
    m.insert("октября", "октября");
    m.insert("октябрь", "октября");
    m.insert("окт", "октября");
    m.insert("ноября", "ноября");
    m.insert("ноябрь", "ноября");
    m.insert("ноя", "ноября");
    m.insert("декабря", "декабря");
    m.insert("декабрь", "декабря");
    m.insert("дек", "декабря");
    m
});

/// Russian currencies
static CURRENCIES_RU: Lazy<HashMap<&str, &str>> = Lazy::new(|| {
    let mut m = HashMap::new();
    m.insert("рубль", "руб.");
    m.insert("рубля", "руб.");
    m.insert("рублей", "руб.");
    m.insert("руб", "руб.");
    m.insert("доллар", "$");
    m.insert("доллара", "$");
    m.insert("долларов", "$");
    m.insert("бакс", "$");
    m.insert("баксов", "$");
    m.insert("евро", "€");
    m.insert("юань", "¥");
    m.insert("юаня", "¥");
    m.insert("юаней", "¥");
    m
});

/// Text normalizer for numbers, dates, time, currency
pub struct TextNormalizer {
    number_pattern: Regex,
}

impl TextNormalizer {
    /// Create a new text normalizer
    pub fn new() -> Self {
        // Build pattern for number words
        let mut words: Vec<&str> = NUMBERS_RU.keys().copied().collect();
        words.extend(MULTIPLIERS_RU.keys().copied());
        words.sort_by_key(|w| std::cmp::Reverse(w.len()));

        let escaped: Vec<String> = words.iter().map(|w| regex::escape(w)).collect();
        let pattern = format!(r"(?i)\b({})([\s,]+({})){{0,5}}\b", escaped.join("|"), escaped.join("|"));

        Self {
            number_pattern: Regex::new(&pattern).unwrap_or_else(|_| Regex::new(r"^\b$").unwrap()),
        }
    }

    /// Normalize text based on language and config
    pub fn normalize(&self, text: &str, language: &str, config: &ProcessorConfig) -> String {
        if text.is_empty() {
            return text.to_string();
        }

        let mut result = text.to_string();

        if language == "ru" {
            if config.normalize_dates {
                result = self.normalize_dates_ru(&result);
            }
            if config.normalize_time {
                result = self.normalize_time_ru(&result);
            }
            if config.normalize_numbers {
                result = self.normalize_numbers_ru(&result);
            }
            if config.normalize_currency {
                result = self.normalize_currency_ru(&result);
            }
        }

        result
    }

    fn normalize_numbers_ru(&self, text: &str) -> String {
        let result = self.number_pattern.replace_all(text, |caps: &regex::Captures| {
            let matched = caps.get(0).map(|m| m.as_str()).unwrap_or("");
            let words: Vec<&str> = matched.split_whitespace().collect();

            if let Some(value) = self.words_to_number(&words) {
                self.format_number(value)
            } else {
                matched.to_string()
            }
        });

        result.to_string()
    }

    fn words_to_number(&self, words: &[&str]) -> Option<i64> {
        if words.is_empty() {
            return None;
        }

        let mut total: i64 = 0;
        let mut current: i64 = 0;

        for word in words {
            let word_lower = word.to_lowercase();

            if let Some(&value) = NUMBERS_RU.get(word_lower.as_str()) {
                current += value;
            } else if let Some(&multiplier) = MULTIPLIERS_RU.get(word_lower.as_str()) {
                if current == 0 {
                    current = 1;
                }
                current *= multiplier;

                // If multiplier is 1000+, add to total
                if multiplier >= 1000 {
                    total += current;
                    current = 0;
                }
            }
        }

        total += current;

        if total > 0 {
            Some(total)
        } else {
            None
        }
    }

    fn format_number(&self, value: i64) -> String {
        if value >= 1000 {
            // Add space separators for thousands
            let s = value.to_string();
            let mut result = String::new();
            for (i, c) in s.chars().rev().enumerate() {
                if i > 0 && i % 3 == 0 {
                    result.push(' ');
                }
                result.push(c);
            }
            result.chars().rev().collect()
        } else {
            value.to_string()
        }
    }

    fn normalize_dates_ru(&self, text: &str) -> String {
        let mut result = text.to_string();

        for (month_word, month_norm) in MONTHS_RU.iter() {
            // Pattern for compound ordinals: "двадцать третьего марта"
            let compound_pattern = format!(
                r"(?i)(двадцать|тридцать)\s+(\w+(?:ого|его|ьего))\s+{}",
                regex::escape(month_word)
            );

            if let Ok(re) = Regex::new(&compound_pattern) {
                result = re.replace_all(&result, |caps: &regex::Captures| {
                    let tens_word = caps.get(1).map(|m| m.as_str().to_lowercase()).unwrap_or_default();
                    let units_word = caps.get(2).map(|m| m.as_str().to_lowercase()).unwrap_or_default();

                    let tens = NUMBERS_RU.get(tens_word.as_str()).copied().unwrap_or(0);
                    let units = NUMBERS_RU.get(units_word.as_str()).copied().unwrap_or(0);
                    let day = tens + units;

                    if day > 0 {
                        format!("{} {}", day, month_norm)
                    } else {
                        caps.get(0).map(|m| m.as_str().to_string()).unwrap_or_default()
                    }
                }).to_string();
            }

            // Pattern for simple ordinals: "пятого марта"
            let simple_pattern = format!(
                r"(?i)(\w+(?:ого|его|ьего|ое|ый|ий))\s+{}",
                regex::escape(month_word)
            );

            if let Ok(re) = Regex::new(&simple_pattern) {
                result = re.replace_all(&result, |caps: &regex::Captures| {
                    let ordinal = caps.get(1).map(|m| m.as_str().to_lowercase()).unwrap_or_default();

                    if let Some(&day) = NUMBERS_RU.get(ordinal.as_str()) {
                        format!("{} {}", day, month_norm)
                    } else {
                        caps.get(0).map(|m| m.as_str().to_string()).unwrap_or_default()
                    }
                }).to_string();
            }
        }

        result
    }

    fn normalize_time_ru(&self, text: &str) -> String {
        let mut result = text.to_string();

        // Pattern: X часов Y минут
        let time_full_re = Regex::new(r"(?i)(\w+)\s+час(?:а|ов|)\s+(\w+)\s+минут(?:а|ы|)").unwrap();
        result = time_full_re.replace_all(&result, |caps: &regex::Captures| {
            let hour_word = caps.get(1).map(|m| m.as_str().to_lowercase()).unwrap_or_default();
            let minute_word = caps.get(2).map(|m| m.as_str().to_lowercase()).unwrap_or_default();

            let hour = NUMBERS_RU.get(hour_word.as_str()).copied();
            let minute = NUMBERS_RU.get(minute_word.as_str()).copied();

            if let (Some(h), Some(m)) = (hour, minute) {
                format!("{}:{:02}", h, m)
            } else {
                caps.get(0).map(|m| m.as_str().to_string()).unwrap_or_default()
            }
        }).to_string();

        // Pattern: X часов (without minutes)
        let hour_only_re = Regex::new(r"(?i)(\w+)\s+час(?:а|ов|)\b").unwrap();
        result = hour_only_re.replace_all(&result, |caps: &regex::Captures| {
            let hour_word = caps.get(1).map(|m| m.as_str().to_lowercase()).unwrap_or_default();

            if let Some(&hour) = NUMBERS_RU.get(hour_word.as_str()) {
                if hour <= 24 {
                    return format!("{}:00", hour);
                }
            }
            caps.get(0).map(|m| m.as_str().to_string()).unwrap_or_default()
        }).to_string();

        result
    }

    fn normalize_currency_ru(&self, text: &str) -> String {
        let mut result = text.to_string();

        for (currency_word, currency_symbol) in CURRENCIES_RU.iter() {
            let pattern = format!(r"(\d[\d\s]*)\s*{}", regex::escape(currency_word));
            if let Ok(re) = Regex::new(&pattern) {
                let symbol = *currency_symbol;
                result = re.replace_all(&result, |caps: &regex::Captures| {
                    let number = caps.get(1).map(|m| m.as_str().trim()).unwrap_or("");
                    format!("{} {}", number, symbol)
                }).to_string();
            }
        }

        result
    }
}

impl Default for TextNormalizer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_number_normalization() {
        let normalizer = TextNormalizer::new();
        let config = ProcessorConfig::default();

        let result = normalizer.normalize("пятьсот двадцать три", "ru", &config);
        assert!(result.contains("523") || result.contains("500"));
    }

    #[test]
    fn test_date_normalization() {
        let normalizer = TextNormalizer::new();
        let config = ProcessorConfig::default();

        let result = normalizer.normalize("встреча двадцать третьего марта", "ru", &config);
        assert!(result.contains("23 марта"));
    }

    #[test]
    fn test_time_normalization() {
        let normalizer = TextNormalizer::new();
        let config = ProcessorConfig::default();

        let result = normalizer.normalize("в два часа тридцать минут", "ru", &config);
        assert!(result.contains("2:30"));
    }

    #[test]
    fn test_currency_normalization() {
        let normalizer = TextNormalizer::new();
        let config = ProcessorConfig::default();

        let result = normalizer.normalize("500 рублей", "ru", &config);
        assert!(result.contains("руб."));
    }
}
