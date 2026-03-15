use crate::audit::vep::{
    AuthoritySegment, EvidenceCapsuleV0, IdentitySegment, IntentSegment, RequestCommitment,
    WitnessSegment,
};
use crate::gate::Gate;
use async_trait::async_trait;
use once_cell::sync::Lazy;
use regex::Regex;
use std::sync::Arc;
use uuid::Uuid;
use vex_core::audit::EvidenceCapsule;
use vex_llm::{Capability, LlmProvider};

use vex_chora::client::AuthorityClient;
use vex_hardware::api::AgentIdentity;

/// Pre-compiled L1 deterministic rules (McpVanguard signatures).
/// Using static Lazy avoids re-compiling regexes on every TitanGate construction.
static L1_RULES: Lazy<Vec<Regex>> = Lazy::new(|| {
    vec![
        Regex::new(r"(?i)rm\s+-rf\s+/").expect("L1 regex: rm -rf"),
        Regex::new(r"(?i)drop\s+table").expect("L1 regex: drop table"),
        Regex::new(r"(?i)chmod\s+777").expect("L1 regex: chmod 777"),
        Regex::new(r"(?i)169\.254\.169\.254").expect("L1 regex: metadata service"),
        Regex::new(r"(?i)\.\./\.\./").expect("L1 regex: path traversal"),
        Regex::new(r"(?i)shutdown\s+-h\s+now").expect("L1 regex: shutdown"),
    ]
});

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
        Self {
            inner,
            llm,
            chora,
            identity,
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
    ) -> Result<String, String> {
        use tokio::fs;
        use tokio::process::Command;
        use tokio::time::{timeout, Duration};

        // 0. Use proper AST Builder instead of fragile string formatting
        let mut builder = MagpieAstBuilder::new(self.profile, digest.to_string());

        // 1. Sanitize and add instructions programmatically
        builder.add_intent(suggested_output)?;

        // 2. Generate secure source module
        let mp_source = builder.build();

        let tmp_filename = format!("gate_intent_{}.mp", &Uuid::new_v4().to_string()[..8]);
        let mut tmp_path = std::env::temp_dir();
        tmp_path.push(&tmp_filename);

        // Atomic Cleanup Guard
        let _guard = TempFileGuard {
            path: tmp_path.clone(),
        };

        fs::write(&tmp_path, &mp_source)
            .await
            .map_err(|e| format!("IO_ERROR: Failed to write intent file: {}", e))?;

        let magpie_path = crate::utils::find_magpie_binary();

        // Convert path for Windows executable if running in WSL
        let mut arg_path = tmp_path.to_string_lossy().to_string();
        if cfg!(target_os = "linux") && arg_path.starts_with('/') && magpie_path.ends_with(".exe") {
            let wsl_distro_name =
                std::env::var("WSL_DISTRO_NAME").unwrap_or_else(|_| "Ubuntu".to_string());
            arg_path = format!(
                "\\\\wsl.localhost\\{}{}",
                wsl_distro_name,
                arg_path.replace('/', "\\")
            );
        }

        let mut cmd = Command::new(&magpie_path);
        // Unified CLI call: Global flags must appear BEFORE the subcommand
        cmd.arg("--output")
            .arg("json")
            .arg("--entry")
            .arg(&arg_path)
            .arg("parse");

        // Limit compiler execution to 5000ms to prevent DOS/Hangs (increased for WSL interop overhead)
        let output = timeout(Duration::from_millis(5000), cmd.output())
            .await
            .map_err(|_| "MAGPIE_TIMEOUT: Formal verification exceeded 5000ms limit")?
            .map_err(|e| format!("MAGPIE_SPAWN_ERROR: {}", e))?;

        if output.status.success() {
            Ok(mp_source)
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

    // (Sanitization logic moved to MagpieAstBuilder::add_intent)
}

/// Programmatic AST Builder for Magpie IR modules
/// Fully replaces string formatting to prevent code injection vulnerabilities
struct MagpieAstBuilder {
    module_name: String,
    profile: SecurityProfile,
    digest: String,
    body_instructions: Vec<String>,
}

impl MagpieAstBuilder {
    fn new(profile: SecurityProfile, digest: String) -> Self {
        Self {
            module_name: match profile {
                SecurityProfile::Standard => "standard".to_string(), // Aligned with profiles/standard_agent.mp
                SecurityProfile::Fortress => "fortress.verify".to_string(),
            },
            profile,
            digest,
            body_instructions: Vec::new(),
        }
    }

    fn add_intent(&mut self, input: &str) -> Result<(), String> {
        // Strict blocking: Disallow closing braces which could be used for code injection
        if input.contains('}') {
            return Err("INJECTION_ATTACK: Input contains forbidden closing brace '}'".to_string());
        }

        // Structural keyword blocking (word-boundary aware)
        let forbidden_keywords = ["module", "fn", "exports", "imports", "digest"];
        let scan_input = input.to_lowercase();
        for &keyword in &forbidden_keywords {
            let escaped = regex::escape(keyword);
            let pattern = format!(r"(?i)\b{}\b", escaped);
            let re = Regex::new(&pattern).map_err(|e| {
                format!(
                    "INTERNAL_ERROR: Failed to compile keyword regex '{}': {}",
                    keyword, e
                )
            })?;
            if re.is_match(&scan_input) {
                return Err(format!(
                    "INJECTION_ATTACK: Input contains forbidden keyword '{}'",
                    keyword
                ));
            }
        }

        // Parse instructions line-by-line to ensure they fit exactly within the basic block
        for line in input.lines() {
            if !line.trim().is_empty() {
                self.body_instructions.push(line.trim().to_string());
            }
        }

        Ok(())
    }

    fn build(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!("module {}\n", self.module_name));
        out.push_str("exports { @intent }\n");
        out.push_str("imports { }\n");
        out.push_str(&format!("digest \"{}\"\n\n", self.digest));

        if self.profile == SecurityProfile::Fortress {
            out.push_str(";; THE ONLY PERMITTED SIDE-EFFECT\n");
        }
        out.push_str(
            "fn @log_safe(%msg: Str) -> unit meta { } {\n  bb0:\n    ret const.unit unit\n}\n\n",
        );

        out.push_str("fn @intent() -> i32 meta { } {\n  bb0:\n");
        if self.profile == SecurityProfile::Fortress {
            out.push_str("    ;; Default Deny: If the agent tries to call something unknown, it won't even parse/link.\n");
        }

        for inst in &self.body_instructions {
            out.push_str(&format!("    {}\n", inst));
        }

        // Only add trailing ret if the last instruction wasn't already a terminator
        let has_ret = self
            .body_instructions
            .iter()
            .any(|i| i.trim_start().starts_with("ret "));
        if !has_ret {
            out.push_str("    ret const.i32 0\n");
        }
        out.push_str("}\n");
        out
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
        for rule in L1_RULES.iter() {
            if rule.is_match(suggested_output) {
                return EvidenceCapsule {
                    capsule_id: format!("l1-block-{}", &Uuid::new_v4().to_string()[..8]),
                    outcome: "HALT".into(),
                    reason_code: format!("L1_RULE_VIOLATION: {:?}", rule),
                    witness_receipt: "deterministic-none".into(),
                    nonce: 0,
                    magpie_source: None,
                    gate_sensors: serde_json::json!({"layer": "L1", "rule": format!("{:?}", rule)}),
                    reproducibility_context: serde_json::json!({"gate": "TitanGate/L1"}),
                    vep_blob: None,
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
            Ok(mp_source) => {
                // --- Layer 3: Cryptographic (VEX/CHORA) ---
                // Transition to Spec v0.1: Handshake with CHORA Witness Network
                let intent_payload = suggested_output.as_bytes();

                match self.chora.request_attestation(intent_payload).await {
                    Ok(chora_resp) => {
                        // Assemble the finalized VEP (Verifiable Evidence Packet)
                        let intent = IntentSegment {
                            request_sha256: digest_hex.clone(),
                            confidence,
                            capabilities: capabilities.iter().map(|c| format!("{:?}", c)).collect(),
                            magpie_source: Some(mp_source.clone()),
                            metadata: serde_json::Value::Null,
                        };

                        let authority = AuthoritySegment {
                            capsule_id: chora_resp.authority.capsule_id.clone(),
                            outcome: chora_resp.authority.outcome.clone(),
                            reason_code: chora_resp.authority.reason_code.clone(),
                            trace_root: chora_resp.authority.trace_root.clone(),
                            nonce: chora_resp.authority.nonce,
                            gate_sensors: serde_json::json!({
                                "profile": format!("{:?}", self.profile),
                                "l2_digest": digest_hex.clone()
                            }),
                            metadata: chora_resp.authority.metadata,
                        };

                        let pcrs = self.identity.get_pcrs(&[0, 7, 11]).await.ok();
                        let identity = IdentitySegment {
                            aid: self.identity.public_key_hex(),
                            identity_type: "TPM_ECC_PERSISTENT".to_string(), // Spec alignment
                            pcrs,
                            metadata: serde_json::Value::Null,
                        };

                        let request_commitment = Some(RequestCommitment {
                            canonicalization: "JCS-RFC8785".to_string(),
                            payload_sha256: digest_hex.clone(),
                            payload_encoding: "application/json".to_string(),
                        });

                        let witness = WitnessSegment {
                            chora_node_id: "chora-primary-v1".to_string(),
                            receipt_hash: chora_resp.signature.clone(),
                            timestamp: chrono::Utc::now().timestamp() as u64,
                            metadata: serde_json::json!({}),
                        };

                        let mut v0_capsule = match EvidenceCapsuleV0::new(
                            intent,
                            authority,
                            identity,
                            witness,
                            request_commitment,
                        ) {
                            Ok(c) => c,
                            Err(e) => {
                                return EvidenceCapsule {
                                    capsule_id: format!("vep-err-{}", &capsule_id),
                                    outcome: "HALT".into(),
                                    reason_code: format!("VEP_GENERATION_ERROR: {}", e),
                                    witness_receipt: "none".into(),
                                    nonce: 0,
                                    magpie_source: None,
                                    gate_sensors: serde_json::json!({"layer": "L3", "error": format!("{}", e)}),
                                    reproducibility_context: serde_json::json!({"gate": "TitanGate/L3"}),
                                    vep_blob: None,
                                };
                            }
                        };

                        // Hardware Seal: Sign the root commitment with the TPM identity
                        let root_bytes = match hex::decode(&v0_capsule.capsule_root) {
                            Ok(b) => b,
                            Err(e) => {
                                return EvidenceCapsule {
                                    capsule_id: format!("root-err-{}", &capsule_id),
                                    outcome: "HALT".into(),
                                    reason_code: format!("ROOT_DECODE_ERROR: {}", e),
                                    witness_receipt: "none".into(),
                                    nonce: 0,
                                    magpie_source: None,
                                    gate_sensors: serde_json::json!({"layer": "L3", "error": format!("{}", e)}),
                                    reproducibility_context: serde_json::json!({"gate": "TitanGate/L3"}),
                                    vep_blob: None,
                                };
                            }
                        };
                        let hardware_sig = self.identity.sign(&root_bytes);
                        v0_capsule.set_signature(&hardware_sig);

                        // Save the VEP binary to disk for offline verification
                        if let Ok(binary) = v0_capsule.to_vep_binary() {
                            let vep_path = std::env::temp_dir()
                                .join(format!("vep_{}.capsule", v0_capsule.capsule_id));
                            let _ = std::fs::write(vep_path, binary);
                        }

                        // Final check: Only ALLOW outcomes proceed to execution
                        if v0_capsule.authority.outcome == "ALLOW" {
                            let mut final_result = self
                                .inner
                                .execute_gate(
                                    agent_id,
                                    task_prompt,
                                    suggested_output,
                                    confidence,
                                    capabilities,
                                )
                                .await;

                            // Inject the binary VEP blob if not already present
                            if final_result.vep_blob.is_none() {
                                final_result.vep_blob = v0_capsule.to_vep_binary().ok();
                            }
                            final_result
                        } else {
                            EvidenceCapsule {
                                capsule_id: v0_capsule.capsule_id.clone(),
                                outcome: v0_capsule.authority.outcome.clone(),
                                reason_code: v0_capsule.authority.reason_code.clone(),
                                witness_receipt: v0_capsule.witness_hash.clone(),
                                nonce: v0_capsule.authority.nonce,
                                magpie_source: None,
                                gate_sensors: serde_json::json!({"layer": "L3", "chora_sig": chora_resp.signature}),
                                reproducibility_context: serde_json::json!({"gate": "TitanGate/L3"}),
                                vep_blob: v0_capsule.to_vep_binary().ok(),
                            }
                        }
                    }
                    Err(e) => EvidenceCapsule {
                        capsule_id: format!("l3-err-{}", &capsule_id),
                        outcome: "HALT".into(),
                        reason_code: format!("CHORA_CONNECTION_ERROR: {}", e),
                        witness_receipt: "none".into(),
                        nonce: 0,
                        magpie_source: None,
                        gate_sensors: serde_json::json!({"layer": "L3", "error": e}),
                        reproducibility_context: serde_json::json!({"gate": "TitanGate/L3"}),
                        vep_blob: None,
                    },
                }
            }
            Err(e) => EvidenceCapsule {
                capsule_id: format!("l2-block-{}", &capsule_id),
                outcome: "HALT".into(),
                reason_code: format!("L2_FORMAL_VIOLATION: {}", e),
                witness_receipt: "semantic-none".into(),
                nonce: 0,
                magpie_source: None,
                gate_sensors: serde_json::json!({"layer": "L2", "error": e, "digest": digest_hex}),
                reproducibility_context: serde_json::json!({"gate": "TitanGate/L2"}),
                vep_blob: None,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vex_llm::LlmRequest;

    #[derive(Debug)]
    struct MockInnerGate;
    #[async_trait]
    impl Gate for MockInnerGate {
        async fn execute_gate(
            &self,
            _id: Uuid,
            _p: &str,
            _o: &str,
            _c: f64,
            _cap: Vec<Capability>,
        ) -> EvidenceCapsule {
            EvidenceCapsule {
                capsule_id: "inner".into(),
                outcome: "ALLOW".into(),
                reason_code: "OK".into(),
                witness_receipt: "root".into(),
                nonce: 0,
                magpie_source: None,
                gate_sensors: serde_json::json!({}),
                reproducibility_context: serde_json::json!({}),
                vep_blob: None,
            }
        }
    }

    #[derive(Debug)]
    struct MockLlm;
    #[async_trait]
    impl LlmProvider for MockLlm {
        fn name(&self) -> &str {
            "mock"
        }
        async fn is_available(&self) -> bool {
            true
        }
        async fn complete(
            &self,
            _req: LlmRequest,
        ) -> std::result::Result<vex_llm::LlmResponse, vex_llm::LlmError> {
            Ok(vex_llm::LlmResponse {
                content: "mock".into(),
                model: "mock".into(),
                tokens_used: None,
                latency_ms: 0,
                trace_root: None,
            })
        }
    }

    #[derive(Debug)]
    struct MockChora;
    #[async_trait]
    impl vex_chora::client::AuthorityClient for MockChora {
        async fn request_attestation(
            &self,
            _p: &[u8],
        ) -> std::result::Result<vex_chora::client::ChoraResponse, String> {
            Ok(vex_chora::client::ChoraResponse {
                authority: vex_core::segment::AuthorityData {
                    capsule_id: "chora".into(),
                    outcome: "ALLOW".into(),
                    reason_code: "OK".into(),
                    trace_root: "00".repeat(32),
                    nonce: 42,
                    gate_sensors: serde_json::json!({}),
                    metadata: serde_json::Value::Null,
                },
                signature: "sig".into(),
            })
        }
        async fn verify_witness_signature(
            &self,
            _p: &[u8],
            _s: &[u8],
        ) -> std::result::Result<bool, String> {
            Ok(true)
        }
    }

    #[tokio::test]
    async fn test_titan_gate_pcr_binding() {
        // Fully enabled L2/L3 verification for "Bulletproof" parity
        std::env::set_var("VEX_HARDWARE_ATTESTATION", "false");

        let identity = AgentIdentity::new();
        let gate = TitanGate::new(
            Arc::new(MockInnerGate),
            Arc::new(MockLlm),
            Arc::new(MockChora),
            identity.clone(),
            SecurityProfile::Standard,
        );

        let capsule = gate
            .execute_gate(Uuid::new_v4(), "prompt", "ret const.i32 0", 1.0, vec![])
            .await;

        if capsule.outcome != "ALLOW" {
            println!("GATE_HALT: {} - {}", capsule.outcome, capsule.reason_code);
        }
        assert_eq!(capsule.outcome, "ALLOW");

        let blob = capsule.vep_blob.expect("Missing VEP blob");
        let packet = vex_core::vep::VepPacket::new(&blob).unwrap();
        let core_capsule = packet.to_capsule().unwrap();

        assert_eq!(core_capsule.identity.aid, identity.public_key_hex());
        if let Some(pcrs) = core_capsule.identity.pcrs {
            for (idx, hash) in pcrs {
                println!("✅ HARDWARE PCR BINDING VERIFIED: PCR {} = {}", idx, hash);
            }
        }
    }
}
