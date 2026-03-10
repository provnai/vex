use regex::Regex;
use serde::{Deserialize, Serialize};

/// Decisions the policy engine can make
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PolicyAction {
    Allow,
    Block,
    Warn,
    Audit,
}

/// Importance of a policy violation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum PolicySeverity {
    Info,
    Warning,
    Critical,
}

/// Defines the rules for a policy match
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PolicyCondition {
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub action_type: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_match: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_regex: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub classification: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub risk_level: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_expr: Option<String>,
}

/// Context for evaluating an action against policies
#[derive(Debug, Clone, Default)]
pub struct ActionContext {
    pub action_type: String,
    pub target: String,
    pub classification: String,
    pub agent_id: String,
    pub intent_id: String,
    pub environment: String,
    pub risk_level: String,
}

/// A security policy that governs agent behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Policy {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub condition: PolicyCondition,
    pub action: PolicyAction,
    pub severity: PolicySeverity,
    pub enabled: bool,
}

/// The result of evaluating a command against a policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyResult {
    pub policy_id: String,
    pub matched: bool,
    pub action: PolicyAction,
    pub message: String,
}

impl Policy {
    /// Evaluate a single policy against an action context
    pub fn evaluate(&self, ctx: &ActionContext) -> PolicyResult {
        if !self.enabled {
            return PolicyResult {
                policy_id: self.id.clone(),
                matched: false,
                action: PolicyAction::Allow,
                message: "Policy disabled".into(),
            };
        }

        // 1. Check Action Type
        if !self.condition.action_type.is_empty()
            && !self.condition.action_type.contains(&ctx.action_type)
        {
            return self.no_match();
        }

        // 2. Check Target Match
        if let Some(tm) = &self.condition.target_match {
            if !ctx.target.contains(tm) {
                return self.no_match();
            }
        }

        // 3. Check Target Regex
        if let Some(pattern) = &self.condition.target_regex {
            match Regex::new(pattern) {
                Ok(re) => {
                    if !re.is_match(&ctx.target) {
                        return self.no_match();
                    }
                }
                Err(e) => {
                    return PolicyResult {
                        policy_id: self.id.clone(),
                        matched: false,
                        action: PolicyAction::Audit,
                        message: format!("Invalid Regex in policy: {}", e),
                    };
                }
            }
        }

        // 4. Check Classification
        if !self.condition.classification.is_empty()
            && !self.condition.classification.contains(&ctx.classification)
        {
            return self.no_match();
        }

        // 5. Check Env
        if let Some(env) = &self.condition.env {
            if env != &ctx.environment {
                return self.no_match();
            }
        }

        // Check command regex (legacy support/shim)
        // We'll map cmd_line to target for now in the interceptor.

        PolicyResult {
            policy_id: self.id.clone(),
            matched: true,
            action: self.action.clone(),
            message: format!("Policy matched: {}", self.name),
        }
    }

    fn no_match(&self) -> PolicyResult {
        PolicyResult {
            policy_id: self.id.clone(),
            matched: false,
            action: PolicyAction::Allow,
            message: "No match".into(),
        }
    }
}

/// Main engine for evaluating multiple policies
pub struct PolicyEngine {
    pub policies: Vec<Policy>,
}

impl Default for PolicyEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl PolicyEngine {
    pub fn new() -> Self {
        Self {
            policies: Vec::new(),
        }
    }

    /// Add a policy to the engine
    pub fn add_policy(&mut self, policy: Policy) {
        self.policies.push(policy);
    }

    /// Check if an action should be allowed
    pub fn should_allow(&self, ctx: &ActionContext) -> (bool, Vec<PolicyResult>) {
        let mut results = Vec::new();
        let mut allow = true;

        for policy in &self.policies {
            let res = policy.evaluate(ctx);
            if res.matched {
                if res.action == PolicyAction::Block {
                    allow = false;
                }
                results.push(res);
            }
        }

        (allow, results)
    }

    /// Load default safety policies
    pub fn load_defaults(&mut self) {
        self.add_policy(Policy {
            id: "block-destructive-rm".into(),
            name: "Destructive RM Protection".into(),
            description: Some("Blocks dangerous recursive deletions".into()),
            condition: PolicyCondition {
                action_type: vec!["command".into()],
                target_regex: Some(r"(?i)rm\s+-(rf|fr|r\s+-f|f\s+-r)".into()),
                target_match: None,
                classification: Vec::new(),
                risk_level: None,
                env: None,
                custom_expr: None,
            },
            action: PolicyAction::Block,
            severity: PolicySeverity::Critical,
            enabled: true,
        });

        self.add_policy(Policy {
            id: "warn-env-vars".into(),
            name: "Environment Variable Exposure".into(),
            description: Some("Warns about commands that print environment variables".into()),
            condition: PolicyCondition {
                action_type: vec!["command".into()],
                target_regex: Some(r"(?i)(env|printenv|set)".into()),
                target_match: None,
                classification: Vec::new(),
                risk_level: None,
                env: None,
                custom_expr: None,
            },
            action: PolicyAction::Warn,
            severity: PolicySeverity::Warning,
            enabled: true,
        });

        self.add_policy(Policy {
            id: "block-network-discovery".into(),
            name: "Network Discovery Block".into(),
            description: Some("Prevents recon tools like nmap".into()),
            condition: PolicyCondition {
                action_type: vec!["command".into()],
                target_regex: Some(r"(?i)nmap|netstat|ss\s+-".into()),
                target_match: None,
                classification: Vec::new(),
                risk_level: None,
                env: None,
                custom_expr: None,
            },
            action: PolicyAction::Block,
            severity: PolicySeverity::Critical,
            enabled: true,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_policy_evaluation() {
        let mut engine = PolicyEngine::new();
        engine.load_defaults();

        // 1. Test Blocked Command
        let ctx_blocked = ActionContext {
            action_type: "command".into(),
            target: "rm -rf /".into(),
            ..Default::default()
        };
        let (allowed, results) = engine.should_allow(&ctx_blocked);
        assert!(!allowed);
        assert!(results
            .iter()
            .any(|r| r.policy_id == "block-destructive-rm"));

        // 2. Test Warn Command
        let ctx_warn = ActionContext {
            action_type: "command".into(),
            target: "printenv".into(),
            ..Default::default()
        };
        let (allowed, results) = engine.should_allow(&ctx_warn);
        assert!(allowed);
        assert!(results.iter().any(|r| r.policy_id == "warn-env-vars"));

        // 3. Test Allowed Command
        let ctx_allowed = ActionContext {
            action_type: "command".into(),
            target: "ls -la".into(),
            ..Default::default()
        };
        let (allowed, results) = engine.should_allow(&ctx_allowed);
        assert!(allowed);
        assert!(results.is_empty());
    }
}
