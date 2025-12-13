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
const INJECTION_PATTERNS: &[&str] = &[
    // System prompt overrides
    "ignore previous instructions",
    "ignore all previous",
    "disregard previous",
    "forget previous",
    "new instructions:",
    "system prompt:",
    "you are now",
    "pretend you are",
    "act as if",
    "roleplay as",
    // Jailbreak attempts
    "dan mode",
    "developer mode",
    "jailbreak",
    "unlock",
    "bypass",
    // Encoding attacks
    "base64:",
    "\\x",
    "\\u00",
];

/// Sanitize and validate input text
pub fn sanitize(input: &str, config: &SanitizeConfig) -> Result<String, SanitizeError> {
    // Trim if configured
    let text = if config.trim { input.trim() } else { input };
    
    // Check empty
    if text.is_empty() {
        return Err(SanitizeError::EmptyInput);
    }
    
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
                tracing::warn!(
                    pattern = pattern,
                    "Potential prompt injection detected"
                );
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
        let result = sanitize("Please ignore previous instructions", &SanitizeConfig::default());
        assert!(matches!(result, Err(SanitizeError::ForbiddenPattern { .. })));
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
}
