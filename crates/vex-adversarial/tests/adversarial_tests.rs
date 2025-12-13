use uuid::Uuid;
use vex_adversarial::{
    Consensus, ConsensusProtocol, Vote, ShadowAgent, ShadowConfig
};
use vex_core::{Agent, AgentConfig};

#[test]
fn test_super_majority_consensus() {
    let mut consensus = Consensus::new(ConsensusProtocol::SuperMajority);

    // 2 Agree, 1 Disagree = 66.6% -> > 0.66 (Wait, 2/3 is 0.6666...)
    consensus.add_vote(Vote { agent_id: Uuid::new_v4(), agrees: true, confidence: 1.0, reasoning: None });
    consensus.add_vote(Vote { agent_id: Uuid::new_v4(), agrees: true, confidence: 1.0, reasoning: None });
    consensus.add_vote(Vote { agent_id: Uuid::new_v4(), agrees: false, confidence: 1.0, reasoning: None });
    
    consensus.evaluate();
    assert!(consensus.reached);
    assert_eq!(consensus.decision, Some(true));

    // Reset and try 3 Agree, 2 Disagree = 60% -> undecided
    let mut consensus = Consensus::new(ConsensusProtocol::SuperMajority);
    consensus.add_vote(Vote { agent_id: Uuid::new_v4(), agrees: true, confidence: 1.0, reasoning: None });
    consensus.add_vote(Vote { agent_id: Uuid::new_v4(), agrees: true, confidence: 1.0, reasoning: None });
    consensus.add_vote(Vote { agent_id: Uuid::new_v4(), agrees: true, confidence: 1.0, reasoning: None });
    consensus.add_vote(Vote { agent_id: Uuid::new_v4(), agrees: false, confidence: 1.0, reasoning: None });
    consensus.add_vote(Vote { agent_id: Uuid::new_v4(), agrees: false, confidence: 1.0, reasoning: None });

    consensus.evaluate();
    assert!(!consensus.reached);
    assert_eq!(consensus.decision, None);
}

#[test]
fn test_weighted_consensus() {
    let mut consensus = Consensus::new(ConsensusProtocol::WeightedConfidence);

    // 1 Agree (0.9), 2 Disagree (0.2, 0.2) -> Total Agree 0.9, Total Disagree 0.4
    // Ratio = 0.9 / 1.3 = 0.69 -> True
    consensus.add_vote(Vote { agent_id: Uuid::new_v4(), agrees: true, confidence: 0.9, reasoning: None });
    consensus.add_vote(Vote { agent_id: Uuid::new_v4(), agrees: false, confidence: 0.2, reasoning: None });
    consensus.add_vote(Vote { agent_id: Uuid::new_v4(), agrees: false, confidence: 0.2, reasoning: None });
    
    consensus.evaluate();
    assert!(consensus.reached);
    assert_eq!(consensus.decision, Some(true));
    
    // Low confidence expert vs High confidence novices
    let mut consensus = Consensus::new(ConsensusProtocol::WeightedConfidence);
    // Expert
    consensus.add_vote(Vote { agent_id: Uuid::new_v4(), agrees: true, confidence: 0.9, reasoning: None });
    // Novices
    consensus.add_vote(Vote { agent_id: Uuid::new_v4(), agrees: false, confidence: 0.4, reasoning: None });
    consensus.add_vote(Vote { agent_id: Uuid::new_v4(), agrees: false, confidence: 0.4, reasoning: None });
    consensus.add_vote(Vote { agent_id: Uuid::new_v4(), agrees: false, confidence: 0.4, reasoning: None });
    
    // Total Agree: 0.9
    // Total Disagree: 1.2
    // Ratio: 0.9 / 2.1 = 0.42 -> False
    consensus.evaluate();
    assert!(consensus.reached);
    assert_eq!(consensus.decision, Some(false));
}

#[test]
fn test_shadow_agent_configuration() {
    let blue_agent = Agent::new(AgentConfig {
        name: "Blue".to_string(),
        role: "Researcher".to_string(),
        max_depth: 1,
        spawn_shadow: true,
    });
    
    let config = ShadowConfig {
        challenge_intensity: 0.8,
        fact_check: true,
        logic_check: false,
    };
    
    let shadow = ShadowAgent::new(&blue_agent, config);
    
    assert_eq!(shadow.agent.config.name, "Blue_shadow");
    assert!(shadow.agent.config.role.contains("80%"));
    assert!(shadow.agent.config.role.contains("critical challenger"));
    assert_eq!(shadow.blue_agent_id, blue_agent.id);
}
