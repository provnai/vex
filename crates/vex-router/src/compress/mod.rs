//! Prompt Compression - Reduce token count while preserving meaning

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum CompressionLevel {
    #[default]
    None,
    Light,
    Balanced,
    Aggressive,
}

pub struct PromptCompressor {
    level: CompressionLevel,
    stop_words: Vec<&'static str>,
}

impl PromptCompressor {
    pub fn new(level: CompressionLevel) -> Self {
        let stop_words = vec![
            "the", "a", "an", "is", "are", "was", "were", "be", "been", "being", "have", "has",
            "had", "do", "does", "did", "will", "would", "could", "should", "may", "might", "must",
            "shall", "can", "need", "dare", "to", "of", "in", "for", "on", "with", "at", "by",
            "from", "as", "into", "through", "during", "before", "after", "above", "below", "and",
            "but", "or", "nor", "so", "yet", "both", "either", "neither",
        ];

        Self { level, stop_words }
    }

    pub fn compress(&self, prompt: &str) -> CompressedPrompt {
        match self.level {
            CompressionLevel::None => CompressedPrompt {
                original: prompt.to_string(),
                compressed: prompt.to_string(),
                original_tokens: self.estimate_tokens(prompt),
                compressed_tokens: self.estimate_tokens(prompt),
                compression_ratio: 0.0,
            },
            CompressionLevel::Light => self.compress_light(prompt),
            CompressionLevel::Balanced => self.compress_balanced(prompt),
            CompressionLevel::Aggressive => self.compress_aggressive(prompt),
        }
    }

    fn compress_light(&self, prompt: &str) -> CompressedPrompt {
        let mut compressed = prompt.to_string();

        compressed = self.remove_extra_whitespace(&compressed);
        compressed = self.remove_filler_phrases(&compressed);

        let original_tokens = self.estimate_tokens(prompt);
        let compressed_tokens = self.estimate_tokens(&compressed);
        let ratio = if original_tokens > 0 {
            (original_tokens - compressed_tokens) as f64 / original_tokens as f64
        } else {
            0.0
        };

        CompressedPrompt {
            original: prompt.to_string(),
            compressed,
            original_tokens,
            compressed_tokens,
            compression_ratio: ratio,
        }
    }

    fn compress_balanced(&self, prompt: &str) -> CompressedPrompt {
        let mut compressed = prompt.to_string();

        compressed = self.remove_extra_whitespace(&compressed);
        compressed = self.remove_filler_phrases(&compressed);
        compressed = self.shorten_sentences(&compressed);
        compressed = self.remove_redundant_words(&compressed);

        let original_tokens = self.estimate_tokens(prompt);
        let compressed_tokens = self.estimate_tokens(&compressed);
        let ratio = if original_tokens > 0 {
            (original_tokens - compressed_tokens) as f64 / original_tokens as f64
        } else {
            0.0
        };

        CompressedPrompt {
            original: prompt.to_string(),
            compressed,
            original_tokens,
            compressed_tokens,
            compression_ratio: ratio,
        }
    }

    fn compress_aggressive(&self, prompt: &str) -> CompressedPrompt {
        let mut compressed = prompt.to_string();

        compressed = self.remove_extra_whitespace(&compressed);
        compressed = self.remove_filler_phrases(&compressed);
        compressed = self.shorten_sentences(&compressed);
        compressed = self.remove_redundant_words(&compressed);
        compressed = self.extract_key_information(&compressed);

        let original_tokens = self.estimate_tokens(prompt);
        let compressed_tokens = self.estimate_tokens(&compressed);
        let ratio = if original_tokens > 0 {
            (original_tokens - compressed_tokens) as f64 / original_tokens as f64
        } else {
            0.0
        };

        CompressedPrompt {
            original: prompt.to_string(),
            compressed,
            original_tokens,
            compressed_tokens,
            compression_ratio: ratio,
        }
    }

    fn remove_extra_whitespace(&self, text: &str) -> String {
        text.split_whitespace().collect::<Vec<_>>().join(" ")
    }

    fn remove_filler_phrases(&self, text: &str) -> String {
        let fillers = [
            "please ",
            "kindly ",
            "basically ",
            "actually ",
            "literally ",
            "really ",
            "just ",
            "simply ",
            "of course ",
            "as you know ",
            "you see ",
            "i was wondering ",
            "i wanted to ask ",
            "if you could ",
            "if possible ",
        ];

        let mut result = text.to_string();
        for filler in fillers {
            result = result.to_lowercase().replace(filler, "");
        }

        result
    }

    fn shorten_sentences(&self, text: &str) -> String {
        let abbreviations = [
            ("information", "info"),
            ("because", "bc"),
            ("without", "w/o"),
            ("with", "w/"),
            ("through", "thru"),
            ("approximately", "approx"),
            ("different", "diff"),
            ("example", "ex"),
            ("question", "q"),
            ("answer", "a"),
            ("number", "num"),
        ];

        let mut result = text.to_string();
        for (long, short) in abbreviations {
            result = result.replace(&format!(" {} ", long), &format!(" {} ", short));
            result = result.replace(&format!("{} ", long), &format!("{} ", short));
        }

        result
    }

    fn remove_redundant_words(&self, text: &str) -> String {
        let words: Vec<&str> = text.split_whitespace().collect();
        let mut result = Vec::new();

        for word in words {
            let word_lower = word.to_lowercase();
            let is_stop = self.stop_words.contains(&word_lower.as_str());

            if !is_stop {
                result.push(word);
            }
        }

        result.join(" ")
    }

    fn extract_key_information(&self, text: &str) -> String {
        let sentences: Vec<&str> = text
            .split(&['.', '!', '?'][..])
            .filter(|s| !s.trim().is_empty())
            .collect();

        if sentences.len() <= 2 {
            return text.to_string();
        }

        let important_markers = [
            "important",
            "critical",
            "key",
            "must",
            "require",
            "need",
            "task",
            "goal",
            "create",
            "build",
            "make",
            "write",
            "find",
            "get",
            "calculate",
            "solve",
        ];

        let mut important_sentences = Vec::new();

        for sentence in sentences {
            let sentence_lower = sentence.to_lowercase();
            if important_markers.iter().any(|m| sentence_lower.contains(m)) {
                important_sentences.push(sentence.trim());
            }
        }

        if important_sentences.is_empty() {
            important_sentences
                .iter()
                .take(2)
                .copied()
                .collect::<Vec<_>>()
                .join(". ")
        } else {
            important_sentences.join(". ")
        }
    }

    fn estimate_tokens(&self, text: &str) -> u32 {
        (text.split_whitespace().count() as f64 * 1.3) as u32
    }
}

impl Default for PromptCompressor {
    fn default() -> Self {
        Self::new(CompressionLevel::None)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressedPrompt {
    pub original: String,
    pub compressed: String,
    pub original_tokens: u32,
    pub compressed_tokens: u32,
    pub compression_ratio: f64,
}
