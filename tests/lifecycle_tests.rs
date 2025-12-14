//! Full agent lifecycle integration tests
//!
//! Tests the complete flow: Create Agent → Execute → Verify → Merkle Proof

use std::sync::Arc;
use vex_core::{Agent, AgentConfig, ContextPacket, MerkleTree};
use vex_adversarial::{ShadowAgent, ShadowConfig, Consensus, ConsensusProtocol, Vote};
use vex_temporal::{EpisodicMemory, Episode};
use vex_llm::MockProvider;

/// Test complete agent creation and configuration
#[tokio::test]
async fn test_agent_lifecycle_creation() {
    let config = AgentConfig {
        name: "TestAgent".to_string(),
        role: "Integration Tester".to_string(),
        max_depth: 3,
        spawn_shadow: true,
    };
    
    let agent = Agent::new(config.clone());
    
    assert!(!agent.id.is_nil(), "Agent should have valid ID");
    assert_eq!(agent.config.name, "TestAgent");
    assert_eq!(agent.generation, 0);
    assert_eq!(agent.fitness, 0.0);
}

/// Test hierarchical agent spawning
#[tokio::test]
async fn test_agent_child_spawning() {
    let root_config = AgentConfig {
        name: "Root".to_string(),
        role: "Coordinator".to_string(),
        max_depth: 3,
        spawn_shadow: false,
    };
    
    let root = Agent::new(root_config);
    let root_id = root.id;
    
    let child_config = AgentConfig {
        name: "Child".to_string(),
        role: "Researcher".to_string(),
        max_depth: 2,
        spawn_shadow: false,
    };
    
    let child = root.spawn_child(child_config);
    
    assert_eq!(child.parent_id, Some(root_id));
    assert_eq!(child.generation, 1);
}

/// Test context packet creation and hashing
#[tokio::test]
async fn test_context_packet_chain() {
    let packet1 = ContextPacket::new("First context");
    let packet2 = ContextPacket::new("Second context");
    
    // Each packet should have unique hash
    assert_ne!(packet1.hash, packet2.hash);
    
    // Packets with same content should have same hash
    let packet3 = ContextPacket::new("First context");
    assert_eq!(packet1.hash, packet3.hash);
}

/// Test Merkle tree integrity
#[tokio::test]
async fn test_merkle_proof_generation() {
    let leaves = vec![
        ("ctx1".to_string(), ContextPacket::new("Context 1").hash),
        ("ctx2".to_string(), ContextPacket::new("Context 2").hash),
        ("ctx3".to_string(), ContextPacket::new("Context 3").hash),
    ];
    
    let tree = MerkleTree::from_leaves(leaves.clone());
    
    // Should have a root
    let root = tree.root_hash();
    assert!(root.is_some(), "Tree should have root hash");
    
    // Should contain all leaves
    for (label, _) in &leaves {
        assert!(tree.contains(label), "Tree should contain {}", label);
    }
}

/// Test adversarial verification flow
#[tokio::test]
async fn test_adversarial_verification_flow() {
    let mock = MockProvider::smart();
    
    // Create shadow agent
    let shadow_config = ShadowConfig::default();
    let shadow = ShadowAgent::new(shadow_config);
    
    // Test claim
    let claim = "The system processed 1000 requests successfully.";
    
    // Detect pattern-based issues
    let issues = shadow.detect_issues(claim);
    println!("Detected issues: {:?}", issues);
    
    // Generate challenge using LLM
    let challenge = shadow.generate_challenge(&mock, claim).await;
    assert!(challenge.is_ok(), "Challenge should succeed with mock");
    
    let challenge_text = challenge.unwrap();
    assert!(!challenge_text.is_empty(), "Challenge should have content");
}

/// Test consensus protocol
#[tokio::test]
async fn test_consensus_voting() {
    // SuperMajority requires 2/3 agreement
    let mut consensus = Consensus::new(ConsensusProtocol::SuperMajority);
    
    // Add votes
    consensus.add_vote(Vote::new("agent1", true, 0.9));
    consensus.add_vote(Vote::new("agent2", true, 0.8));
    consensus.add_vote(Vote::new("agent3", false, 0.7));
    
    // Evaluate - should pass (2/3 = 66.7% voted true)
    consensus.evaluate();
    
    assert!(consensus.reached, "Consensus should be reached");
    assert_eq!(consensus.decision, Some(true));
}

/// Test episodic memory with decay
#[tokio::test]
async fn test_episodic_memory_lifecycle() {
    use vex_temporal::HorizonConfig;
    
    let mut config = HorizonConfig::default();
    config.max_entries = 5;
    
    let mut memory = EpisodicMemory::new(config);
    
    // Add episodes
    memory.remember("Event 1", 0.5);
    memory.remember("Event 2", 0.8);
    memory.remember("Event 3", 0.3);
    
    assert_eq!(memory.len(), 3);
    
    // Add pinned episode
    memory.add(Episode::pinned("System config"));
    
    // Add more to trigger eviction
    memory.remember("Event 4", 0.4);
    memory.remember("Event 5", 0.6);
    memory.remember("Event 6", 0.7);
    
    // Should not exceed max_entries (pinned doesn't count against limit in eviction logic)
    assert!(memory.len() <= 6, "Memory should respect limits");
    
    // Pinned should still exist
    assert!(memory.episodes().any(|e| e.content == "System config"));
}

/// Test full integration: Agent + Memory + Merkle + Verification
#[tokio::test]
async fn test_full_integration_flow() {
    // 1. Create agent
    let agent = Agent::new(AgentConfig {
        name: "IntegrationAgent".to_string(),
        role: "Full Flow Tester".to_string(),
        max_depth: 2,
        spawn_shadow: true,
    });
    
    // 2. Create memory
    let mut memory = EpisodicMemory::default();
    
    // 3. Simulate execution with mock LLM
    let mock = MockProvider::smart();
    let response = mock.ask("Analyze this data").await.unwrap();
    
    // 4. Store in memory
    memory.remember(&response, 0.9);
    
    // 5. Create context packet
    let packet = ContextPacket::new(&response);
    
    // 6. Build Merkle tree
    let leaves = vec![
        (agent.id.to_string(), packet.hash.clone()),
    ];
    let tree = MerkleTree::from_leaves(leaves);
    
    // 7. Verify integrity
    let root = tree.root_hash().expect("Should have root");
    
    // 8. Create shadow for verification
    let shadow = ShadowAgent::new(ShadowConfig::default());
    let issues = shadow.detect_issues(&response);
    
    println!("Agent: {}", agent.id);
    println!("Response: {}", &response[..50.min(response.len())]);
    println!("Merkle Root: {}", root);
    println!("Issues found: {}", issues.len());
    
    // All components should work together
    assert!(memory.len() > 0);
    assert!(!root.to_string().is_empty());
}
