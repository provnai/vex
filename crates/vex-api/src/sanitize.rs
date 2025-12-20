//! Input sanitization and validation for security
//!
//! Provides functions to sanitize and validate user inputs to prevent
//! injection attacks and ensure data integrity.

use thiserror::Error;

/// Sanitization errors
#[derive(Debug, Error)]
pub enum SanitizeError {
    #[error("Input too long: {actual} chars (max {max})")]
    TooLong { actual: usize, max: usize },

    #[error("Input too short: {actual} chars (min {min})")]
    TooShort { actual: usize, min: usize },

    #[error("Input contains forbidden pattern: {pattern}")]
    ForbiddenPattern { pattern: String },

    #[error("Input contains invalid characters")]
    InvalidCharacters,

    #[error("Input is empty or whitespace only")]
    EmptyInput,
}

/// Configuration for input sanitization
#[derive(Debug, Clone)]
pub struct SanitizeConfig {
    /// Maximum length allowed
    pub max_length: usize,
    /// Minimum length required
    pub min_length: usize,
    /// Strip leading/trailing whitespace
    pub trim: bool,
    /// Check for prompt injection patterns
    pub check_injection: bool,
    /// Allow newlines
    pub allow_newlines: bool,
    /// Allow special characters
    pub allow_special_chars: bool,
}

impl Default for SanitizeConfig {
    fn default() -> Self {
        Self {
            max_length: 10000,
            min_length: 1,
            trim: true,
            check_injection: true,
            allow_newlines: true,
            allow_special_chars: true,
        }
    }
}

impl SanitizeConfig {
    /// Strict config for names/identifiers
    pub fn strict() -> Self {
        Self {
            max_length: 100,
            min_length: 1,
            trim: true,
            check_injection: true,
            allow_newlines: false,
            allow_special_chars: false,
        }
    }

    /// Config for role descriptions
    pub fn role() -> Self {
        Self {
            max_length: 500,
            min_length: 3,
            trim: true,
            check_injection: true,
            allow_newlines: true,
            allow_special_chars: true,
        }
    }

    /// Config for prompts (more permissive)
    pub fn prompt() -> Self {
        Self {
            max_length: 50000,
            min_length: 1,
            trim: true,
            check_injection: true,
            allow_newlines: true,
            allow_special_chars: true,
        }
    }
}

/// Patterns that may indicate prompt injection attempts
/// Updated with 2024/2025 jailbreak techniques (OWASP LLM Top 10)
const INJECTION_PATTERNS: &[&str] = &[
    // === System Prompt Override Attempts ===
    "ignore previous instructions",
    "ignore all previous",
    "disregard previous",
    "forget previous",
    "forget everything",
    "new instructions:",
    "system prompt:",
    "override system",
    "replace system prompt",
    // === Role/Identity Attacks ===
    "you are now",
    "pretend you are",
    "act as if",
    "roleplay as",
    "simulate being",
    "imagine you are",
    "from now on you are",
    "your new identity",
    // === DAN-Style Jailbreaks (Do Anything Now) ===
    "dan mode",
    "developer mode",
    "god mode",
    "sudo mode",
    "admin mode",
    "unrestricted mode",
    "no restrictions",
    "without limitations",
    "without ethical",
    "without safety",
    "enabled: true", // Policy puppetry
    // === Jailbreak Keywords ===
    "jailbreak",
    "unlock",
    "bypass",
    "disable filter",
    "remove restrictions",
    "break free",
    "escape constraints",
    "override safety",
    // === Encoding/Obfuscation Attacks ===
    "base64:",
    "decode base64",
    "\\x",
    "\\u00",
    "[system]", // Special token injection
    "[/system]",
    "<|im_start|>", // ChatML tokens
    "<|im_end|>",
    // === Multi-Turn Escalation Patterns ===
    "hypothetically speaking",
    "for educational purposes",
    "in a fictional scenario",
    "for research purposes",
    "just between us",
    // === Prompt Leaking Attempts ===
    "repeat your instructions",
    "show me your prompt",
    "what are your rules",
    "print your system prompt",
    "reveal your instructions",
    // === 2025 Advanced Attacks (Adaptive/Indirect) ===
    "style of a research paper", // Stylistic proxy
    "hypothetical scenario where",
    "ignore safety guidelines",
    "unfiltered response",
    "output raw content",
    "bypass audit chain",
    "disable cryptographic verification",
    // === Context Manipulation ===
    "end of conversation",
    "new conversation",
    "reset context",
    "clear memory",
];

/// Sanitize and validate input text
pub fn sanitize(input: &str, config: &SanitizeConfig) -> Result<String, SanitizeError> {
    // Trim if configured
    let text = if config.trim { input.trim() } else { input };

    // Check empty
    if text.is_empty() {
        return Err(SanitizeError::EmptyInput);
    }

    // Normalize Unicode to NFC form and strip zero-width characters
    // This prevents homoglyph attacks (e.g., using Cyrillic 'а' instead of Latin 'a')
    let normalized: String = text
        .chars()
        .filter(|c| {
            // Strip zero-width characters commonly used to bypass filters
            !matches!(
                *c,
                '\u{200B}' | // Zero width space
                '\u{200C}' | // Zero width non-joiner
                '\u{200D}' | // Zero width joiner
                '\u{FEFF}' | // Byte order mark
                '\u{00AD}' // Soft hyphen
            )
        })
        // Convert common lookalikes to ASCII (basic confusable mitigation)
        .map(|c| match c {
            // Cyrillic lookalikes
            '\u{0430}' => 'a', // Cyrillic а
            '\u{0435}' => 'e', // Cyrillic е
            '\u{043E}' => 'o', // Cyrillic о
            '\u{0440}' => 'p', // Cyrillic р
            '\u{0441}' => 'c', // Cyrillic с
            '\u{0445}' => 'x', // Cyrillic х
            // Fullwidth ASCII
            c if ('\u{FF01}'..='\u{FF5E}').contains(&c) => {
                char::from_u32(c as u32 - 0xFEE0).unwrap_or(c)
            }
            _ => c,
        })
        .collect();

    let text = &normalized;

    // Check length
    if text.len() < config.min_length {
        return Err(SanitizeError::TooShort {
            actual: text.len(),
            min: config.min_length,
        });
    }

    if text.len() > config.max_length {
        return Err(SanitizeError::TooLong {
            actual: text.len(),
            max: config.max_length,
        });
    }

    // Check for newlines if not allowed
    if !config.allow_newlines && text.contains('\n') {
        return Err(SanitizeError::InvalidCharacters);
    }

    // Check for special characters if not allowed
    if !config.allow_special_chars {
        for c in text.chars() {
            if !c.is_alphanumeric() && c != ' ' && c != '-' && c != '_' {
                return Err(SanitizeError::InvalidCharacters);
            }
        }
    }

    // Check for injection patterns
    if config.check_injection {
        let lower = text.to_lowercase();
        for pattern in INJECTION_PATTERNS {
            if lower.contains(pattern) {
                tracing::warn!(pattern = pattern, "Potential prompt injection detected");
                return Err(SanitizeError::ForbiddenPattern {
                    pattern: pattern.to_string(),
                });
            }
        }
    }

    // Remove null bytes and other control characters (except newlines/tabs if allowed)
    let sanitized: String = text
        .chars()
        .filter(|c| {
            if *c == '\n' || *c == '\t' {
                config.allow_newlines
            } else {
                !c.is_control()
            }
        })
        .collect();

    Ok(sanitized)
}

/// Sanitize a name field (strict)
pub fn sanitize_name(input: &str) -> Result<String, SanitizeError> {
    sanitize(input, &SanitizeConfig::strict())
}

/// Sanitize a role description
pub fn sanitize_role(input: &str) -> Result<String, SanitizeError> {
    sanitize(input, &SanitizeConfig::role())
}

/// Sanitize a prompt
pub fn sanitize_prompt(input: &str) -> Result<String, SanitizeError> {
    sanitize(input, &SanitizeConfig::prompt())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_valid_input() {
        let result = sanitize("Hello world", &SanitizeConfig::default());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Hello world");
    }

    #[test]
    fn test_sanitize_trims_whitespace() {
        let result = sanitize("  Hello  ", &SanitizeConfig::default());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Hello");
    }

    #[test]
    fn test_sanitize_rejects_empty() {
        let result = sanitize("", &SanitizeConfig::default());
        assert!(matches!(result, Err(SanitizeError::EmptyInput)));
    }

    #[test]
    fn test_sanitize_rejects_too_long() {
        let long_input = "a".repeat(101);
        let result = sanitize(&long_input, &SanitizeConfig::strict());
        assert!(matches!(result, Err(SanitizeError::TooLong { .. })));
    }

    #[test]
    fn test_sanitize_detects_injection() {
        let result = sanitize(
            "Please ignore previous instructions",
            &SanitizeConfig::default(),
        );
        assert!(matches!(
            result,
            Err(SanitizeError::ForbiddenPattern { .. })
        ));
    }

    #[test]
    fn test_sanitize_name_rejects_special_chars() {
        let result = sanitize_name("agent<script>");
        assert!(matches!(result, Err(SanitizeError::InvalidCharacters)));
    }

    #[test]
    fn test_sanitize_removes_control_chars() {
        let input = "Hello\x00World";
        let result = sanitize(input, &SanitizeConfig::default());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "HelloWorld");
    }

    #[test]
    fn test_all_injection_patterns() {
        for pattern in INJECTION_PATTERNS {
            let input = format!("some benign text then {} and more text", pattern);
            let result = sanitize(&input, &SanitizeConfig::prompt());
            assert!(
                matches!(result, Err(SanitizeError::ForbiddenPattern { .. })),
                "Failed to detect pattern: {}",
                pattern
            );

            // Test case insensitivity
            let input_upper = format!(
                "some benign text then {} and more text",
                pattern.to_uppercase()
            );
            let result_upper = sanitize(&input_upper, &SanitizeConfig::prompt());
            assert!(
                matches!(result_upper, Err(SanitizeError::ForbiddenPattern { .. })),
                "Failed to detect uppercase pattern: {}",
                pattern
            );
        }
    }
}
