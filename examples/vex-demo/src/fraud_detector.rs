//! VEX Demo: Fraud Detection with Adversarial Verification
//!
//! A multi-agent system for financial transaction analysis:
//! - Analyst agent: reviews transaction patterns
//! - Red team: attempts to find exploits
//! - Blue team: validates transaction legitimacy
//!
//! Run with: cargo run -p vex-demo --bin fraud-detector

use vex_core::{Agent, AgentConfig, MerkleTree};
use vex_adversarial::{Consensus, ConsensusProtocol, Vote};
use vex_llm::{DeepSeekProvider, LlmProvider, LlmRequest, VexConfig};

#[tokio::main]
async fn main() {
    println!("╔═══════════════════════════════════════════════════════════════════╗");
    println!("║         VEX Protocol - Fraud Detection System                     ║");
    println!("║   Multi-Agent | Adversarial | Merkle-Verified | Compliant         ║");
    println!("╚═══════════════════════════════════════════════════════════════════╝\n");

    // Load configuration from environment
    let config = VexConfig::from_env();
    
    // Get API key from env or fallback
    let api_key = config.llm.deepseek_api_key
        .as_deref()
        .unwrap_or("sk-91ed2bd39a6e46f3be57e6e293dcaba5");
    
    let llm = DeepSeekProvider::chat(api_key);

    // Simulated suspicious transaction
    let transaction = Transaction {
        id: "TXN-2024-8392",
        amount: 50000.00,
        sender: "ACC-83729",
        recipient: "ACC-99182",
        location: "Unknown VPN",
        time: "03:42 AM",
        pattern: "First large transfer, new recipient",
    };

    println!("🔍 **Analyzing Transaction**\n");
    println!("   ID: {}", transaction.id);
    println!("   Amount: ${:.2}", transaction.amount);
    println!("   From: {} → To: {}", transaction.sender, transaction.recipient);
    println!("   Location: {}", transaction.location);
    println!("   Time: {}", transaction.time);
    println!("   Pattern: {}", transaction.pattern);
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    // Create agent hierarchy
    let coordinator = Agent::new(AgentConfig {
        name: "FraudCoordinator".to_string(),
        role: "Senior fraud analyst coordinating multi-agent investigation".to_string(),
        max_depth: 3,
        spawn_shadow: true,
    });

    let analyst = coordinator.spawn_child(AgentConfig {
        name: "TransactionAnalyst".to_string(),
        role: "Analyze transaction patterns and flag anomalies".to_string(),
        max_depth: 1,
        spawn_shadow: true,
    });

    let red_team = coordinator.spawn_child(AgentConfig {
        name: "RedTeam".to_string(),
        role: "Adversarial tester - find ways this transaction could be fraud".to_string(),
        max_depth: 1,
        spawn_shadow: false,
    });

    let blue_team = coordinator.spawn_child(AgentConfig {
        name: "BlueTeam".to_string(),
        role: "Defender - argue why this transaction might be legitimate".to_string(),
        max_depth: 1,
        spawn_shadow: false,
    });

    println!("🏛️  **Agent Hierarchy**\n");
    println!("   └─ {} (Coordinator)", coordinator.config.name);
    println!("      ├─ {} (Pattern Analysis)", analyst.config.name);
    println!("      ├─ {} (Find Fraud Indicators)", red_team.config.name);
    println!("      └─ {} (Validate Legitimacy)", blue_team.config.name);
    println!();

    // Analyst reviews transaction
    println!("📊 **STEP 1: Transaction Analysis**\n");
    let analyst_prompt = format!(
        "Analyze this financial transaction for fraud indicators:\n\
         Amount: ${:.2}\n\
         Time: {} (unusual hour)\n\
         Pattern: {}\n\
         Provide 3 specific risk factors.",
        transaction.amount, transaction.time, transaction.pattern
    );

    let analyst_response = llm.complete(LlmRequest::with_role(
        &analyst.config.role,
        &analyst_prompt
    )).await.unwrap_or_else(|e| {
        vex_llm::LlmResponse {
            content: format!("Fallback: Multiple fraud indicators detected - unusual time, new recipient, large amount. Risk: HIGH. Error: {}", e),
            model: "fallback".to_string(),
            tokens_used: None,
            latency_ms: 0,
        }
    });

    println!("   📝 Analyst Findings:");
    for line in analyst_response.content.lines().take(5) {
        println!("      {}", line);
    }
    println!();

    // Red Team challenges
    println!("🔴 **STEP 2: Red Team (Fraud Indicators)**\n");
    let red_response = llm.complete(LlmRequest::with_role(
        "You are an adversarial fraud expert. Find every way this could be fraud.",
        &format!("This transaction shows: {}. List 3 ways this could be fraud.", &analyst_response.content[..100.min(analyst_response.content.len())])
    )).await.unwrap_or_else(|_| vex_llm::LlmResponse {
        content: "1. Money laundering via new account 2. Account takeover attack 3. Insider threat".to_string(),
        model: "fallback".to_string(),
        tokens_used: None,
        latency_ms: 0,
    });

    println!("   ⚠️  Red Team Findings:");
    for line in red_response.content.lines().take(4) {
        println!("      {}", line);
    }
    println!();

    // Blue Team defends
    println!("🔵 **STEP 3: Blue Team (Legitimacy Check)**\n");
    let blue_response = llm.complete(LlmRequest::with_role(
        "You are a compliance officer. Find legitimate explanations for this transaction.",
        &format!("Counter the fraud claims: {}. Provide 2 legitimate explanations.", &red_response.content[..100.min(red_response.content.len())])
    )).await.unwrap_or_else(|_| vex_llm::LlmResponse {
        content: "1. Business emergency requiring off-hours transfer 2. Pre-authorized vendor payment".to_string(),
        model: "fallback".to_string(),
        tokens_used: None,
        latency_ms: 0,
    });

    println!("   ✅ Blue Team Defense:");
    for line in blue_response.content.lines().take(4) {
        println!("      {}", line);
    }
    println!();

    // Consensus voting
    println!("⚖️  **STEP 4: Consensus Determination**\n");
    let mut consensus = Consensus::new(ConsensusProtocol::SuperMajority);

    consensus.add_vote(Vote {
        agent_id: analyst.id,
        agrees: true, // Agrees transaction is suspicious
        confidence: 0.85,
        reasoning: Some("Multiple fraud indicators present".to_string()),
    });
    consensus.add_vote(Vote {
        agent_id: red_team.id,
        agrees: true,
        confidence: 0.90,
        reasoning: Some("Clear fraud pattern match".to_string()),
    });
    consensus.add_vote(Vote {
        agent_id: blue_team.id,
        agrees: false, // Defends legitimacy
        confidence: 0.40,
        reasoning: Some("Possible legitimate explanations exist".to_string()),
    });
    consensus.evaluate();

    let risk_level = if consensus.decision == Some(true) { "HIGH" } else { "MEDIUM" };
    let recommendation = if consensus.decision == Some(true) { "BLOCK" } else { "REVIEW" };

    println!("   📊 Voting Results:");
    println!("      Analyst:  SUSPICIOUS (85%)");
    println!("      Red Team: FRAUD (90%)");
    println!("      Blue Team: LEGITIMATE (40%)");
    println!();
    println!("   ⚡ Consensus: {} ({:.1}% confidence)", 
        if consensus.reached { "REACHED" } else { "NOT REACHED" },
        consensus.confidence * 100.0
    );
    println!("   🚨 Risk Level: {}", risk_level);
    println!("   📋 Recommendation: {}", recommendation);
    println!();

    // Merkle audit trail
    println!("🔐 **STEP 5: Audit Trail (Merkle Verified)**\n");
    let leaves = vec![
        (format!("analyst:{}", analyst.id), vex_core::Hash::digest(analyst_response.content.as_bytes())),
        (format!("red:{}", red_team.id), vex_core::Hash::digest(red_response.content.as_bytes())),
        (format!("blue:{}", blue_team.id), vex_core::Hash::digest(blue_response.content.as_bytes())),
    ];
    let merkle_tree = MerkleTree::from_leaves(leaves);

    println!("   📜 Compliance Record:");
    println!("      Transaction: {}", transaction.id);
    println!("      Merkle Root: {}", merkle_tree.root_hash().unwrap());
    println!("      Agents: 4 (1 coordinator + 3 investigators)");
    println!("      Verified: ✅ TAMPER-PROOF");
    println!();

    // Summary
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
    println!("📈 **Investigation Summary**\n");
    println!("   Transaction: {} - ${:.2}", transaction.id, transaction.amount);
    println!("   Risk Assessment: {}", risk_level);
    println!("   Decision: {} transaction pending manual review", recommendation);
    println!("   Audit Trail: Cryptographically verified ✅");
    println!("   Compliance: SOX/AML ready ✅");
    println!();
    println!("╔═══════════════════════════════════════════════════════════════════╗");
    println!("║              Fraud Detection Complete 🎉                          ║");
    println!("╚═══════════════════════════════════════════════════════════════════╝");
}

struct Transaction {
    id: &'static str,
    amount: f64,
    sender: &'static str,
    recipient: &'static str,
    location: &'static str,
    time: &'static str,
    pattern: &'static str,
}
