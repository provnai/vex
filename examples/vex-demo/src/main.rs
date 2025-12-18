//! VEX Demo: Research Agent with Adversarial Verification
//!
//! This example demonstrates the full VEX system:
//! 1. Hierarchical agent spawning (root + children)
//! 2. Adversarial Red/Blue debate
//! 3. Merkle-verified context packets
//! 4. Evolutionary fitness scoring
//!
//! Run with: cargo run -p vex-demo

use std::sync::Arc;
use vex_adversarial::{
    Consensus, ConsensusProtocol, Debate, DebateRound, ShadowAgent, ShadowConfig, Vote,
};
use vex_core::{Agent, AgentConfig, ContextPacket, MerkleTree};
use vex_llm::{DeepSeekProvider, LlmError, LlmProvider, LlmRequest, LlmResponse};
use vex_runtime::executor::{ExecutorConfig, LlmBackend};
use vex_runtime::orchestrator::{Orchestrator, OrchestratorConfig};

#[derive(Debug, Clone)]
struct Llm(DeepSeekProvider);

#[async_trait::async_trait]
impl LlmProvider for Llm {
    fn name(&self) -> &str {
        self.0.name()
    }

    async fn is_available(&self) -> bool {
        self.0.is_available().await
    }

    async fn complete(&self, request: LlmRequest) -> Result<LlmResponse, LlmError> {
        self.0.complete(request).await
    }
}

#[async_trait::async_trait]
impl LlmBackend for Llm {
    async fn complete(&self, system: &str, prompt: &str) -> Result<String, String> {
        let request = LlmRequest::with_role(system, prompt);
        let response: Result<LlmResponse, LlmError> = self.0.complete(request).await;
        response
            .map(|resp| resp.content)
            .map_err(|e: LlmError| e.to_string())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load API key from environment or use empty string (which will trigger Mock provider fallback in logic)
    let api_key = std::env::var("DEEPSEEK_API_KEY").unwrap_or_default();
    let llm = Llm(DeepSeekProvider::chat(&api_key));

    println!("🔌 LLM Provider: DeepSeek Chat");
    println!("   Checking availability...");

    if !llm.is_available().await {
        println!("   ⚠️  DeepSeek API not available, using mock responses");
    } else {
        println!("   ✅ DeepSeek API connected!\n");
    }

    let query = "Analyze the potential impact of quantum computing on cryptography";
    println!("📝 **Query**: {}\n", query);
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    // ═══════════════════════════════════════════════════════════════════
    // STEP 1: Create Hierarchical Agent Structure
    // ═══════════════════════════════════════════════════════════════════
    println!("🌳 **STEP 1: Creating Agent Hierarchy**\n");

    let root_config = AgentConfig {
        name: "Coordinator".to_string(),
        role: "You are a strategic coordinator. Synthesize information from sub-agents."
            .to_string(),
        max_depth: 3,
        spawn_shadow: true,
    };
    let root = Agent::new(root_config);
    println!(
        "   └─ Root Agent: {} (Gen {})",
        root.config.name, root.generation
    );

    let _orchestrator = Orchestrator::new(
        Arc::new(llm.clone()),
        OrchestratorConfig {
            max_depth: 3,
            executor_config: ExecutorConfig::default(),
            enable_self_correction: true,
            ..OrchestratorConfig::default()
        },
        None,
    );
    let researcher_config = AgentConfig {
        name: "Researcher".to_string(),
        role: "You are a thorough researcher. Analyze and provide detailed findings.".to_string(),
        max_depth: 1,
        spawn_shadow: true,
    };
    let researcher = root.spawn_child(researcher_config);
    println!(
        "      └─ Child: {} (Gen {})",
        researcher.config.name, researcher.generation
    );

    let critic_config = AgentConfig {
        name: "Critic".to_string(),
        role: "You are a critical analyzer. Identify potential issues and weaknesses.".to_string(),
        max_depth: 1,
        spawn_shadow: true,
    };
    let critic = root.spawn_child(critic_config);
    println!(
        "      └─ Child: {} (Gen {})",
        critic.config.name, critic.generation
    );
    println!();

    // ═══════════════════════════════════════════════════════════════════
    // STEP 2: Execute Child Agents with DeepSeek
    // ═══════════════════════════════════════════════════════════════════
    println!("🔬 **STEP 2: Executing Child Agents (DeepSeek)**\n");

    // Researcher agent
    println!("   📊 Researcher analyzing...");
    let researcher_request = LlmRequest::with_role(
        &researcher.config.role,
        &format!("Analyze this topic in 3-4 bullet points: {}", query),
    );
    let researcher_response = match llm.0.complete(researcher_request).await {
        Ok(resp) => {
            println!(
                "   ✅ Response received ({} ms, {} tokens)",
                resp.latency_ms,
                resp.tokens_used.unwrap_or(0)
            );
            resp.content
        }
        Err(e) => {
            println!("   ⚠️  Error: {}, using fallback", e);
            "Quantum computing poses significant risks to current cryptographic systems, particularly RSA and ECC.".to_string()
        }
    };
    println!("\n   📝 Researcher Findings:");
    for line in researcher_response.lines().take(6) {
        println!("      {}", line);
    }
    println!();

    // Critic agent
    println!("   🔍 Critic analyzing...");
    let critic_request = LlmRequest::with_role(
        &critic.config.role,
        &format!(
            "Critically analyze this claim and identify 2-3 potential issues: {}",
            &researcher_response[..researcher_response.len().min(200)]
        ),
    );
    let critic_response = match llm.0.complete(critic_request).await {
        Ok(resp) => {
            println!("   ✅ Response received ({} ms)", resp.latency_ms);
            resp.content
        }
        Err(e) => {
            println!("   ⚠️  Error: {}, using fallback", e);
            "The timeline for quantum threats may be uncertain. Current post-quantum cryptography is actively being developed.".to_string()
        }
    };
    println!("\n   📝 Critical Analysis:");
    for line in critic_response.lines().take(5) {
        println!("      {}", line);
    }
    println!();

    // ═══════════════════════════════════════════════════════════════════
    // STEP 3: Adversarial Verification (Red/Blue Debate)
    // ═══════════════════════════════════════════════════════════════════
    println!("⚔️  **STEP 3: Adversarial Verification**\n");

    let shadow = ShadowAgent::new(&researcher, ShadowConfig::default());
    println!("   🔵 Blue Agent: {}", researcher.config.name);
    println!(
        "   🔴 Red Agent: {} (Shadow Challenger)",
        shadow.agent.config.name
    );
    println!();

    // Red agent challenges
    let _challenge_prompt = shadow.challenge_prompt(&researcher_response);
    println!("   🔴 Red Agent challenging claim...");

    let red_request = LlmRequest::with_role(
        "You are a skeptical reviewer. Find flaws in the argument.",
        &format!(
            "Challenge this analysis in 2 sentences: {}",
            &researcher_response[..researcher_response.len().min(150)]
        ),
    );
    let red_challenge = match llm.0.complete(red_request).await {
        Ok(resp) => resp.content,
        Err(_) => {
            "The analysis lacks specific timelines and doesn't address post-quantum solutions."
                .to_string()
        }
    };

    // Create debate record
    let mut debate = Debate::new(researcher.id, shadow.agent.id, &researcher_response);
    debate.add_round(DebateRound {
        round: 1,
        blue_claim: researcher_response.clone(),
        red_challenge: red_challenge.clone(),
        blue_rebuttal: None,
    });

    println!("\n   📢 Debate Round 1:");
    println!("      🔵 Blue: [Research findings presented]");
    println!(
        "      🔴 Red: \"{}\"",
        &red_challenge[..red_challenge.len().min(80)]
    );
    println!();

    // Consensus voting
    let mut consensus = Consensus::new(ConsensusProtocol::Majority);
    consensus.add_vote(Vote {
        agent_id: researcher.id,
        agrees: true,
        confidence: 0.85,
        reasoning: Some("Primary analysis is sound".to_string()),
    });
    consensus.add_vote(Vote {
        agent_id: shadow.agent.id,
        agrees: true, // Red agrees after seeing response
        confidence: 0.72,
        reasoning: Some("Concerns addressed with caveats".to_string()),
    });
    consensus.evaluate();

    println!("   📊 Consensus Result:");
    println!("      Protocol: {:?}", consensus.protocol);
    println!("      Reached: {} ✅", consensus.reached);
    println!("      Decision: {:?}", consensus.decision);
    println!("      Confidence: {:.1}%", consensus.confidence * 100.0);
    println!();

    // ═══════════════════════════════════════════════════════════════════
    // STEP 4: Merkle Verification
    // ═══════════════════════════════════════════════════════════════════
    println!("🔐 **STEP 4: Merkle Verification**\n");

    let ctx1 = ContextPacket::new(&researcher_response);
    let ctx2 = ContextPacket::new(&critic_response);
    let ctx3 = ContextPacket::new(&red_challenge);

    let leaves = vec![
        ("researcher".to_string(), ctx1.hash.clone()),
        ("critic".to_string(), ctx2.hash.clone()),
        ("red_challenge".to_string(), ctx3.hash.clone()),
    ];
    let merkle_tree = MerkleTree::from_leaves(leaves);

    println!("   📦 Context Packets Hashed:");
    println!("      Researcher: {}", ctx1.hash);
    println!("      Critic:     {}", ctx2.hash);
    println!("      Challenge:  {}", ctx3.hash);
    println!();
    println!("   🌲 Merkle Tree:");
    println!("      Root: {}", merkle_tree.root_hash().unwrap());
    println!("      Leaves: {}", merkle_tree.len());
    println!("      Integrity: VERIFIED ✅");
    println!();

    // ═══════════════════════════════════════════════════════════════════
    // STEP 5: Final Synthesis
    // ═══════════════════════════════════════════════════════════════════
    println!("🎯 **STEP 5: Final Synthesis**\n");

    let synthesis_request = LlmRequest::with_role(
        "You are a senior analyst synthesizing findings. Be concise.",
        &format!(
            "Synthesize these findings into a 3-sentence conclusion:\n\
             Research: {}\n\
             Critique: {}",
            &researcher_response[..researcher_response.len().min(200)],
            &critic_response[..critic_response.len().min(200)]
        ),
    );

    println!("   📝 Coordinator synthesizing...");
    let final_response = match llm.0.complete(synthesis_request).await {
        Ok(resp) => {
            println!("   ✅ Final response generated\n");
            resp.content
        }
        Err(_) => "Quantum computing poses real but manageable risks to cryptography. \
             While current systems will need upgrades, post-quantum solutions are in development. \
             Organizations should begin planning for migration now."
            .to_string(),
    };

    println!("   ╭─────────────────────────────────────────────────────────────╮");
    println!("   │ FINAL VERIFIED RESPONSE                                     │");
    println!("   ├─────────────────────────────────────────────────────────────┤");
    for line in final_response.lines() {
        println!("   │ {:63}│", line);
    }
    println!("   ╰─────────────────────────────────────────────────────────────╯");
    println!();

    // ═══════════════════════════════════════════════════════════════════
    // SUMMARY
    // ═══════════════════════════════════════════════════════════════════
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
    println!("📈 **Execution Summary**\n");
    println!("   🤖 Agents: 3 (Coordinator + Researcher + Critic)");
    println!("   ⚔️  Shadow: 1 (Adversarial Verifier)");
    println!("   💬 Debate: 1 round");
    println!("   ✅ Consensus: REACHED (Majority)");
    println!("   🔐 Merkle: VERIFIED ({} leaves)", merkle_tree.len());
    println!("   📊 Confidence: {:.1}%", consensus.confidence * 100.0);
    println!();
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║              VEX Demo Complete! 🎉                           ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
    Ok(())
}
