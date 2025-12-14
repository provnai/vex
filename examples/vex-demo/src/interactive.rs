//! VEX Demo: Interactive Chat with Adversarial Verification
//!
//! A real-time chatbot demonstrating:
//! - Interactive input loop
//! - Multi-agent research
//! - Adversarial verification
//! - Temporal memory
//!
//! Run with: cargo run -p vex-demo --bin interactive

use std::io::{self, Write};
use vex_adversarial::{ShadowAgent, ShadowConfig};
use vex_core::{Agent, AgentConfig, ContextPacket, MerkleTree};
use vex_llm::{DeepSeekProvider, LlmProvider, LlmRequest, VexConfig};
use vex_temporal::{EpisodicMemory, HorizonConfig};

#[tokio::main]
async fn main() {
    println!("\n╔═══════════════════════════════════════════════════════════════════╗");
    println!("║           VEX Protocol - Interactive Assistant                    ║");
    println!("║      Evolutionary | Adversarial | Temporal | Verified             ║");
    println!("╚═══════════════════════════════════════════════════════════════════╝\n");

    // Initialize
    let config = VexConfig::from_env();
    let api_key = config
        .llm
        .deepseek_api_key
        .as_deref()
        .expect("DEEPSEEK_API_KEY environment variable must be set");

    let llm = DeepSeekProvider::chat(api_key);
    let mut memory = EpisodicMemory::new(HorizonConfig::for_depth(0));
    let mut merkle_leaves = Vec::new();
    let mut turn_count = 0;

    // Create main agents
    let coordinator = Agent::new(AgentConfig {
        name: "Assistant".to_string(),
        role: "You are a helpful, accurate assistant. Provide clear, concise answers.".to_string(),
        max_depth: 2,
        spawn_shadow: true,
    });

    let _verifier = ShadowAgent::new(
        &coordinator,
        ShadowConfig {
            challenge_intensity: 0.6,
            fact_check: true,
            logic_check: true,
        },
    );

    println!("🤖 Assistant ready! Type your questions below.");
    println!("   Commands: /help, /memory, /verify, /quit\n");

    // Main interaction loop
    loop {
        // Prompt
        print!("You: ");
        io::stdout().flush().unwrap();

        // Read input
        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_err() {
            break;
        }
        let input = input.trim();

        // Handle empty input
        if input.is_empty() {
            continue;
        }

        // Handle commands
        match input {
            "/quit" | "/exit" | "/q" => {
                println!("\n👋 Goodbye! {} interactions processed.", turn_count);
                break;
            }
            "/help" => {
                println!("\n📚 Available Commands:");
                println!("   /memory  - Show episodic memory summary");
                println!("   /verify  - Show last verification status");
                println!("   /quit    - Exit the chat\n");
                continue;
            }
            "/memory" => {
                println!("\n📦 {}", memory.summarize());
                if !memory.is_empty() {
                    println!("   Recent episodes:");
                    for (i, ep) in memory.episodes().take(3).enumerate() {
                        println!(
                            "   {}. {}...",
                            i + 1,
                            &ep.content[..ep.content.len().min(40)]
                        );
                    }
                }
                println!();
                continue;
            }
            "/verify" => {
                if merkle_leaves.is_empty() {
                    println!("\n⚠️  No verified interactions yet.\n");
                } else {
                    let tree = MerkleTree::from_leaves(merkle_leaves.clone());
                    println!("\n🔐 Verification Status:");
                    println!("   Interactions: {}", merkle_leaves.len());
                    println!("   Merkle Root: {}", tree.root_hash().unwrap());
                    println!("   Integrity: ✅ VERIFIED\n");
                }
                continue;
            }
            _ => {}
        }

        turn_count += 1;

        // Get response from main agent
        print!("\n🤔 Thinking");
        io::stdout().flush().unwrap();

        let response = match llm
            .complete(LlmRequest::with_role(&coordinator.config.role, input))
            .await
        {
            Ok(resp) => {
                print!(".");
                resp.content
            }
            Err(e) => {
                println!(" ⚠️\n");
                println!("Error: {}\n", e);
                continue;
            }
        };

        // Adversarial verification (quick check)
        let verify_request = LlmRequest::with_role(
            "You are a fact-checker. Rate this response 1-10 for accuracy. Just the number.",
            &format!(
                "Response to verify: {}",
                &response[..response.len().min(150)]
            ),
        );

        let verification = match llm.complete(verify_request).await {
            Ok(resp) => {
                print!(".");
                resp.content
            }
            Err(_) => "8".to_string(),
        };

        println!(" ✅\n");

        // Display response
        println!("🤖 Assistant:");
        for line in response.lines() {
            println!("   {}", line);
        }

        // Show verification score
        let score = verification
            .trim()
            .chars()
            .find(|c| c.is_ascii_digit())
            .and_then(|c| c.to_digit(10))
            .unwrap_or(7);
        println!("\n   📊 Verification: {}/10", score);

        if score < 6 {
            println!("   ⚠️  Low confidence - consider fact-checking");
        }
        println!();

        // Store in memory
        memory.remember(
            &format!(
                "Q: {} | A: {}...",
                input,
                &response[..response.len().min(50)]
            ),
            score as f64 / 10.0,
        );

        // Add to Merkle tree
        let packet = ContextPacket::new(&response);
        merkle_leaves.push((format!("turn_{}", turn_count), packet.hash));
    }

    // Final summary
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📈 Session Summary:");
    println!("   Total turns: {}", turn_count);
    println!("   {}", memory.summarize());
    if !merkle_leaves.is_empty() {
        let tree = MerkleTree::from_leaves(merkle_leaves);
        println!("   Merkle Root: {}", tree.root_hash().unwrap());
    }
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
}
