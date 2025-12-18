//! A2A Agent Card
//!
//! The Agent Card is a JSON document that describes an agent's capabilities.
//! It's served at `/.well-known/agent.json` per the A2A spec.
//!
//! # Security
//!
//! - Agent Cards can be protected via mTLS
//! - Authentication requirements are declared in the card
//! - Capabilities are whitelisted

use serde::{Deserialize, Serialize};

/// A2A Agent Card structure
///
/// Describes this agent's capabilities to other agents.
/// Served at `/.well-known/agent.json`.
///
/// # Example
///
/// ```
/// use vex_api::a2a::AgentCard;
///
/// let card = AgentCard::new("vex-verifier")
///     .with_description("VEX adversarial verification agent")
///     .with_skill("verify", "Verify claims with adversarial debate");
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCard {
    /// Agent name (unique identifier)
    pub name: String,
    /// Human-readable description
    pub description: String,
    /// Protocol version
    pub version: String,
    /// Agent capabilities (skills)
    pub skills: Vec<Skill>,
    /// Authentication configuration
    pub authentication: AuthConfig,
    /// Provider information
    pub provider: ProviderInfo,
    /// Optional URL for agent documentation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub docs_url: Option<String>,
}

/// A skill/capability that this agent offers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    /// Skill identifier
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// Description of what this skill does
    pub description: String,
    /// JSON Schema for skill input
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_schema: Option<serde_json::Value>,
    /// JSON Schema for skill output
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_schema: Option<serde_json::Value>,
}

/// Authentication configuration for the agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    /// Supported authentication schemes
    pub schemes: Vec<String>,
    /// OAuth 2.0 token endpoint (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_endpoint: Option<String>,
    /// OpenID Connect discovery URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oidc_discovery: Option<String>,
}

/// Provider information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderInfo {
    /// Organization name
    pub organization: String,
    /// Contact URL or email
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contact: Option<String>,
}

impl AgentCard {
    /// Create a new agent card with minimal info
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: String::new(),
            version: "1.0".to_string(),
            skills: Vec::new(),
            authentication: AuthConfig::default(),
            provider: ProviderInfo {
                organization: "VEX".to_string(),
                contact: None,
            },
            docs_url: None,
        }
    }

    /// Set the agent description
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    /// Add a skill to the agent
    pub fn with_skill(mut self, id: impl Into<String>, description: impl Into<String>) -> Self {
        let id_str = id.into();
        self.skills.push(Skill {
            id: id_str.clone(),
            name: id_str,
            description: description.into(),
            input_schema: None,
            output_schema: None,
        });
        self
    }

    /// Add a skill with full details
    pub fn with_skill_full(mut self, skill: Skill) -> Self {
        self.skills.push(skill);
        self
    }

    /// Set documentation URL
    pub fn with_docs(mut self, url: impl Into<String>) -> Self {
        self.docs_url = Some(url.into());
        self
    }

    /// Set authentication config
    pub fn with_auth(mut self, auth: AuthConfig) -> Self {
        self.authentication = auth;
        self
    }

    /// Create the default VEX agent card
    pub fn vex_default() -> Self {
        Self::new("vex-agent")
            .with_description(
                "VEX Protocol agent with adversarial verification and cryptographic proofs",
            )
            .with_skill("verify", "Verify a claim using adversarial red/blue debate")
            .with_skill("hash", "Compute SHA-256 hash of content")
            .with_skill("merkle_root", "Get current Merkle root for audit chain")
            .with_docs("https://provnai.dev/docs")
            .with_auth(AuthConfig {
                schemes: vec!["bearer".to_string(), "api_key".to_string()],
                token_endpoint: None,
                oidc_discovery: None,
            })
    }
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            schemes: vec!["bearer".to_string()],
            token_endpoint: None,
            oidc_discovery: None,
        }
    }
}

impl Skill {
    /// Create a new skill
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: description.into(),
            input_schema: None,
            output_schema: None,
        }
    }

    /// Add input schema
    pub fn with_input_schema(mut self, schema: serde_json::Value) -> Self {
        self.input_schema = Some(schema);
        self
    }

    /// Add output schema
    pub fn with_output_schema(mut self, schema: serde_json::Value) -> Self {
        self.output_schema = Some(schema);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_card_new() {
        let card = AgentCard::new("test-agent");
        assert_eq!(card.name, "test-agent");
        assert_eq!(card.version, "1.0");
        assert!(card.skills.is_empty());
    }

    #[test]
    fn test_agent_card_builder() {
        let card = AgentCard::new("vex")
            .with_description("VEX verifier")
            .with_skill("verify", "Verify claims");

        assert_eq!(card.description, "VEX verifier");
        assert_eq!(card.skills.len(), 1);
        assert_eq!(card.skills[0].id, "verify");
    }

    #[test]
    fn test_vex_default() {
        let card = AgentCard::vex_default();
        assert_eq!(card.name, "vex-agent");
        assert!(card.skills.len() >= 3);
        assert!(card.docs_url.is_some());
    }

    #[test]
    fn test_serialization() {
        let card = AgentCard::vex_default();
        let json = serde_json::to_string_pretty(&card).unwrap();
        assert!(json.contains("vex-agent"));
        assert!(json.contains("verify"));
    }
}
