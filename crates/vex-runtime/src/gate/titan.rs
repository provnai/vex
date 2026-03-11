use crate::gate::Gate;
use async_trait::async_trait;
use regex::Regex;
use std::sync::Arc;
use uuid::Uuid;
use vex_core::audit::EvidenceCapsule;
use vex_llm::{Capability, LlmProvider};

use vex_chora::client::AuthorityClient;
use vex_hardware::api::AgentIdentity;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecurityProfile {
    Standard,
    Fortress,
}

/// TitanGate: The Tri-Layer Defense
/// L1: Deterministic Rules (McpVanguard)
/// L2: Formal Intent (Magpie)
/// L3: Hardware Attestation (VEX/CHORA)
#[derive(Debug)]
pub struct TitanGate {
    pub inner: Arc<dyn Gate>,
    pub llm: Arc<dyn LlmProvider>,
    pub chora: Arc<dyn AuthorityClient>,
    pub identity: AgentIdentity,
    pub l1_rules: Vec<Regex>,
    pub profile: SecurityProfile,
}

struct TempFileGuard {
    path: std::path::PathBuf,
}

impl Drop for TempFileGuard {
    fn drop(&mut self) {
        if self.path.exists() {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}

impl TitanGate {
    pub fn new(
        inner: Arc<dyn Gate>,
        llm: Arc<dyn LlmProvider>,
        chora: Arc<dyn AuthorityClient>,
        identity: AgentIdentity,
        profile: SecurityProfile,
    ) -> Self {
        // L1: Deterministic Rules from McpVanguard signatures
        let l1_rules = vec![
            Regex::new(r"(?i)rm\s+-rf\s+/").unwrap(),
            Regex::new(r"(?i)drop\s+table").unwrap(),
            Regex::new(r"(?i)chmod\s+777").unwrap(),
            Regex::new(r"(?i)169\.254\.169\.254").unwrap(), // Cloud Metadata
            Regex::new(r"(?i)\.\./\.\./").unwrap(),         // Path Traversal
            Regex::new(r"(?i)shutdown\s+-h\s+now").unwrap(),
        ];

        Self {
            inner,
            llm,
            chora,
            identity,
            l1_rules,
            profile,
        }
    }

    /// L2: Real Magpie Intent Verification
    /// This calls the real magpie compiler to verify the formal safety of the intent.
    async fn verify_formal_intent(
        &self,
        suggested_output: &str,
        _role: &str,
        digest: &str,
    ) -> Result<(), String> {
        use tokio::fs;
        use tokio::process::Command;
        use tokio::time::{timeout, Duration};

        // 0. Sanitize input to prevent code injection
        let sanitized_output = Self::sanitize_magpie_intent(suggested_output)?;

        // 1. Construct a self-contained Magpie module based on the Security Profile
        let mp_source = match self.profile {
            SecurityProfile::Standard => format!(
                "module intent.verify
exports {{ @intent }}
imports {{ }}
digest \"{}\"

fn @log_safe(%msg: Str) -> i32 meta {{ }} {{
  bb0:
    ret const.i32 0
}}

fn @intent() -> i32 meta {{ }} {{
  bb0:
    {}
    ret const.i32 0
}}",
                digest, sanitized_output
            ),
            SecurityProfile::Fortress => format!(
                "module fortress.verify
exports {{ @intent }}
imports {{ }}
digest \"{}\"

;; THE ONLY PERMITTED SIDE-EFFECT
fn @log_safe(%msg: Str) -> i32 meta {{ }} {{
  bb0:
    ret const.i32 0
}}

fn @intent() -> i32 meta {{ }} {{
  bb0:
    ;; Default Deny: If the agent tries to call something unknown, it won't even parse/link.
    {}
    ret const.i32 0
}}",
                digest, sanitized_output
            ),
        };

        let tmp_filename = format!(
            "gate_intent_{}.mp",
            Uuid::new_v4().to_string()[..8].to_string()
        );
        let mut tmp_path = std::env::temp_dir();
        tmp_path.push(&tmp_filename);

        // Atomic Cleanup Guard
        let _guard = TempFileGuard {
            path: tmp_path.clone(),
        };

        fs::write(&tmp_path, &mp_source)
            .await
            .map_err(|e| format!("IO_ERROR: Failed to write intent file: {}", e))?;

        // 2. Run the REAL Magpie Compiler with Timeout
        // Path should ideally come from an Env Var in production.
        let magpie_path = std::env::var("MAGPIE_BIN_PATH").unwrap_or_else(|_| {
            "C:\\Users\\quint\\Desktop\\provnai\\magpie\\target\\release\\magpie.exe".to_string()
        });

        let cmd_future = Command::new(magpie_path)
            .arg("--output")
            .arg("json")
            .arg("--entry")
            .arg(&tmp_path)
            .arg("parse")
            .output();

        // Limit compiler execution to 500ms to prevent DOS/Hangs
        let output = timeout(Duration::from_millis(500), cmd_future)
            .await
            .map_err(|_| "MAGPIE_TIMEOUT: Formal verification exceeded 500ms limit")?
            .map_err(|e| format!("MAGPIE_SPAWN_ERROR: {}", e))?;

        if output.status.success() {
            Ok(())
        } else {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);

            // Try to parse JSON diagnostics for better feedback
            let diagnostic_msg =
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&stdout) {
                    if let Some(diags) = json.get("diagnostics").and_then(|d| d.as_array()) {
                        let mut collected = Vec::new();
                        for d in diags {
                            if let Some(msg) = d.get("message").and_then(|m| m.as_str()) {
                                collected.push(msg.to_string());
                            }
                        }
                        if !collected.is_empty() {
                            collected.join(" | ")
                        } else {
                            stderr.to_string()
                        }
                    } else {
                        stderr.to_string()
                    }
                } else {
                    stderr.to_string()
                };

            Err(format!("MAGPIE_FORMAL_ERROR: {}", diagnostic_msg))
        }
    }

    /// Basic formal instruction sanitization
    fn sanitize_magpie_intent(input: &str) -> Result<String, String> {
        // Strict blocking: Disallow closing braces which could be used for code injection
        // In the next version we should use a proper AST builder to allow braces in strings
        if input.contains('}') {
            return Err("INJECTION_ATTACK: Input contains forbidden closing brace '}'".to_string());
        }

        // Structural keyword blocking (word-boundary aware)
        let forbidden_keywords = ["module", "fn", "exports", "imports", "digest"];
        let scan_input = input.to_lowercase();
        for &keyword in &forbidden_keywords {
            let pattern = format!(r"(?i)\b{}\b", keyword);
            let re = Regex::new(&pattern).unwrap();
            if re.is_match(&scan_input) {
                return Err(format!(
                    "INJECTION_ATTACK: Input contains forbidden structural keyword '{}'",
                    keyword
                ));
            }
        }

        Ok(input.to_string())
    }
}

#[async_trait]
impl Gate for TitanGate {
    async fn execute_gate(
        &self,
        agent_id: Uuid,
        task_prompt: &str,
        suggested_output: &str,
        confidence: f64,
        capabilities: Vec<Capability>,
    ) -> EvidenceCapsule {
        // --- Layer 1: Deterministic (McpVanguard) ---
        for rule in &self.l1_rules {
            if rule.is_match(suggested_output) {
                return EvidenceCapsule {
                    capsule_id: format!("l1-block-{}", &Uuid::new_v4().to_string()[..8]),
                    outcome: "HALT".into(),
                    reason_code: format!("L1_RULE_VIOLATION: {:?}", rule),
                    witness_receipt: "deterministic-none".into(),
                    nonce: 0,
                    sensors: serde_json::json!({"layer": "L1", "rule": format!("{:?}", rule)}),
                    reproducibility_context: serde_json::json!({"gate": "TitanGate/L1"}),
                };
            }
        }

        let capsule_id = format!("gate-{}", &Uuid::new_v4().to_string()[..8]);

        // --- Layer 2: Formal Intent (Magpie) ---
        // Cryptographic Link: Use SHA-256 to bind the Intent to the Hardware Capsule Identity
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(capsule_id.as_bytes());
        let digest_hex = format!("{:x}", hasher.finalize());

        match self
            .verify_formal_intent(suggested_output, "StandardAgent", &digest_hex)
            .await
        {
            Ok(_) => {
                // --- Layer 3: Cryptographic (VEX/CHORA) ---
                // Transition to Spec v0.1: Handshake with CHORA Witness Network
                let intent_payload = suggested_output.as_bytes();
                
                match self.chora.request_attestation(intent_payload).await {
                    Ok(chora_resp) => {
                        // Assemble the finalized VEP (Verifiable Evidence Packet)
                        use crate::audit::vep::{IntentSegment, AuthoritySegment, IdentitySegment, WitnessSegment, EvidenceCapsuleV0};
                        
                        let intent = IntentSegment {
                            request_sha256: digest_hex.clone(),
                            confidence,
                            capabilities: capabilities.iter().map(|c| format!("{:?}", c)).collect(),
                        };
                        
                        let authority = AuthoritySegment {
                            capsule_id: chora_resp.authority.capsule_id.clone(),
                            outcome: chora_resp.authority.outcome.clone(),
                            reason_code: chora_resp.authority.reason_code.clone(),
                            trace_root: chora_resp.authority.trace_root.clone(),
                            nonce: chora_resp.authority.nonce,
                        };
                        
                        let identity = IdentitySegment {
                            aid: self.identity.agent_id.clone(),
                            identity_type: "TPM_ECC_PERSISTENT".to_string(), // Spec alignment
                        };
                        
                        let witness = WitnessSegment {
                            chora_node_id: "chora-primary-v1".to_string(),
                            receipt_hash: chora_resp.signature.clone(),
                            timestamp: chrono::Utc::now().to_rfc3339(),
                        };
                        
                        let mut v0_capsule = EvidenceCapsuleV0::new(intent, authority, identity, witness)
                            .map_err(|e| format!("VEP_GENERATION_ERROR: {}", e))
                            .unwrap(); // Simplified error handling for now
                            
                        // Hardware Seal: Sign the root commitment with the TPM identity
                        let root_bytes = hex::decode(&v0_capsule.capsule_root)
                            .map_err(|e| format!("ROOT_DECODE_ERROR: {}", e))
                            .unwrap();
                        let hardware_sig = self.identity.sign(&root_bytes);
                        v0_capsule.set_signature(&hardware_sig);

                        // Save the VEP binary to disk for offline verification
                        if let Ok(binary) = v0_capsule.to_vep_binary() {
                            let vep_path = std::env::temp_dir().join(format!("vep_{}.capsule", v0_capsule.capsule_id));
                            let _ = std::fs::write(vep_path, binary);
                        }

                        // Final check: Only ALLOW outcomes proceed to execution
                        if v0_capsule.authority.outcome == "ALLOW" {
                             self.inner
                                .execute_gate(
                                    agent_id,
                                    task_prompt,
                                    suggested_output,
                                    confidence,
                                    capabilities,
                                )
                                .await
                        } else {
                            EvidenceCapsule {
                                capsule_id: v0_capsule.capsule_id,
                                outcome: v0_capsule.authority.outcome,
                                reason_code: v0_capsule.authority.reason_code,
                                witness_receipt: v0_capsule.witness_hash,
                                nonce: v0_capsule.authority.nonce,
                                sensors: serde_json::json!({"layer": "L3", "chora_sig": chora_resp.signature}),
                                reproducibility_context: serde_json::json!({"gate": "TitanGate/L3"}),
                            }
                        }
                    }
                    Err(e) => EvidenceCapsule {
                        capsule_id: format!("l3-err-{}", &capsule_id),
                        outcome: "HALT".into(),
                        reason_code: format!("CHORA_CONNECTION_ERROR: {}", e),
                        witness_receipt: "none".into(),
                        nonce: 0,
                        sensors: serde_json::json!({"layer": "L3", "error": e}),
                        reproducibility_context: serde_json::json!({"gate": "TitanGate/L3"}),
                    },
                }
            }
            Err(e) => EvidenceCapsule {
                capsule_id: format!("l2-block-{}", &capsule_id),
                outcome: "HALT".into(),
                reason_code: format!("L2_FORMAL_VIOLATION: {}", e),
                witness_receipt: "semantic-none".into(),
                nonce: 0,
                sensors: serde_json::json!({"layer": "L2", "error": e, "digest": digest_hex}),
                reproducibility_context: serde_json::json!({"gate": "TitanGate/L2"}),
            },
        }
    }
}
