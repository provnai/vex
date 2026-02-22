//! Shadow agent spawning and management

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use vex_core::{Agent, AgentConfig};

/// Configuration for shadow (adversarial) agents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShadowConfig {
    /// How aggressively the shadow should challenge
    pub challenge_intensity: f64,
    /// Whether to focus on factual accuracy
    pub fact_check: bool,
    /// Whether to check for logical consistency
    pub logic_check: bool,
}

impl Default for ShadowConfig {
    fn default() -> Self {
        Self {
            challenge_intensity: 0.7,
            fact_check: true,
            logic_check: true,
        }
    }
}

/// A shadow agent that challenges its paired Blue agent
#[derive(Debug, Clone)]
pub struct ShadowAgent {
    /// The underlying agent
    pub agent: Agent,
    /// Shadow-specific configuration
    pub config: ShadowConfig,
    /// ID of the Blue agent this shadow is paired with
    pub blue_agent_id: Uuid,
}

impl ShadowAgent {
    /// Create a new shadow agent for the given Blue agent
    pub fn new(blue_agent: &Agent, config: ShadowConfig) -> Self {
        let agent_config = AgentConfig {
            name: format!("{}_shadow", blue_agent.config.name),
            role: format!(
                "You are a critical challenger. Your job is to find flaws, \
                 inconsistencies, and potential errors in the following claim. \
                 Challenge intensity: {:.0}%",
                config.challenge_intensity * 100.0
            ),
            max_depth: 0,        // Shadows don't spawn children
            spawn_shadow: false, // Shadows don't have their own shadows
        };

        let mut agent = blue_agent.spawn_child(agent_config);
        agent.shadow_id = None;

        Self {
            agent,
            config,
            blue_agent_id: blue_agent.id,
        }
    }

    /// Generate a challenge prompt for the given claim with enhanced heuristics
    pub fn challenge_prompt(&self, claim: &str) -> String {
        let mut challenge_types = Vec::new();

        if self.config.fact_check {
            challenge_types.push("factual accuracy");
        }
        if self.config.logic_check {
            challenge_types.push("logical consistency");
        }

        // Detect potential issues using pattern-based heuristics
        let detected_issues = self.detect_issues(claim);

        // Build targeted challenge based on detected issues
        let issue_guidance = if detected_issues.is_empty() {
            String::from("Look for hidden assumptions, unstated premises, and edge cases.")
        } else {
            format!(
                "Pay special attention to these potential issues: {}",
                detected_issues.join("; ")
            )
        };

        format!(
            "Critically analyze the following claim for {}:\n\n\
             \"{}\"\n\n\
             {}\n\n\
            For each issue found:
            1. State the specific problem
            2. Explain why it matters
            3. Suggest how it could be verified or corrected

            If any issues are found, start your response with the marker: [CHALLENGE]
            If no issues are found, start your response with the marker: [CLEAN]

            If [CLEAN], explain what makes the claim robust.",
            challenge_types.join(" and "),
            claim,
            issue_guidance
        )
    }

    /// Detect potential areas of interest in a claim using pattern-based guidance
    /// Returns a list of focus areas for the shadow agent to scrutinize.
    pub fn detect_issues(&self, claim: &str) -> Vec<String> {
        let mut areas_of_interest = Vec::new();
        let claim_lower = claim.to_lowercase();

        // Check for over-generalization
        if claim_lower.contains("always")
            || claim_lower.contains("never")
            || claim_lower.contains("all ")
            || claim_lower.contains("none ")
        {
            areas_of_interest.push("Over-generalization: Verify if universal claims 'always'/'never' hold true for all edge cases.".to_string());
        }

        // Check for quantificational ambiguity
        if claim_lower.contains("many")
            || claim_lower.contains("some")
            || claim_lower.contains("significant")
        {
            areas_of_interest.push("Quantification: Assess if vague terms like 'many' or 'significant' hide a lack of specific data.".to_string());
        }

        // Check for evidence-free causality
        if claim_lower.contains("because") || claim_lower.contains("therefore") {
            areas_of_interest.push("Causality: Scrutinize the link between the premise and the conclusion for logical leaps.".to_string());
        }

        // Check for unsubstantiated numbers
        if claim_lower.contains("%") || claim_lower.contains("percent") {
            areas_of_interest.push("Statistics: Determine if percentages are sourced or if they are illustrative placeholders.".to_string());
        }

        // Check for certainty bias (loaded language)
        let certainty_markers = ["obvious", "clearly", "undeniable", "proven", "fact"];
        for word in certainty_markers {
            if claim_lower.contains(word) {
                areas_of_interest.push(format!("Certainty Bias: The use of '{}' may indicate a claim that assumes its own conclusion.", word));
                break;
            }
        }

        // Check for linguistic complexity
        let avg_words_per_sentence = claim.split_whitespace().count() / claim.matches('.').count().max(1);
        if avg_words_per_sentence > 30 {
            areas_of_interest.push("Complexity: The high sentence length may obscure specific errors or contradictions.".to_string());
        }

        areas_of_interest
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shadow_creation() {
        let blue = Agent::new(AgentConfig::default());
        let shadow = ShadowAgent::new(&blue, ShadowConfig::default());

        assert_eq!(shadow.blue_agent_id, blue.id);
        assert!(!shadow.agent.config.spawn_shadow);
    }

    #[test]
    fn test_detect_issues_universal_claims() {
        let blue = Agent::new(AgentConfig::default());
        let shadow = ShadowAgent::new(&blue, ShadowConfig::default());

        let issues = shadow.detect_issues("This method always works without fail.");
        assert!(issues.iter().any(|i| i.contains("Over-generalization")));
    }

    #[test]
    fn test_detect_issues_statistics() {
        let blue = Agent::new(AgentConfig::default());
        let shadow = ShadowAgent::new(&blue, ShadowConfig::default());

        let issues = shadow.detect_issues("Studies show 90% of users prefer this approach.");
        assert!(issues.iter().any(|i| i.contains("Statistics")));
    }

    #[test]
    fn test_detect_issues_loaded_language() {
        let blue = Agent::new(AgentConfig::default());
        let shadow = ShadowAgent::new(&blue, ShadowConfig::default());

        let issues = shadow.detect_issues("It is obvious that the solution is correct.");
        assert!(issues.iter().any(|i| i.contains("Certainty Bias")));
    }

    #[test]
    fn test_detect_issues_clean_claim() {
        let blue = Agent::new(AgentConfig::default());
        let shadow = ShadowAgent::new(&blue, ShadowConfig::default());

        // A clean, specific claim with no detected patterns
        let issues = shadow.detect_issues("The API returns a 200 status code.");
        // May still detect some issues, but should be fewer
        assert!(issues.len() <= 2);
    }
}
