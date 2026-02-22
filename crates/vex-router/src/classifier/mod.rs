//! Query Classifier - Simple complexity analysis

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QueryComplexity {
    pub score: f64,
    pub capabilities: Vec<String>,
    pub estimated_tokens: usize,
    pub confidence: f64,
}

#[derive(Debug)]
pub struct QueryClassifier;

impl QueryClassifier {
    pub fn new() -> Self {
        Self
    }
    
    pub fn classify(&self, query: &str) -> QueryComplexity {
        let query_lower = query.to_lowercase();
        let word_count = query_lower.split_whitespace().count();
        let estimated_tokens = (word_count as f64 * 1.3) as usize;
        
        let mut capabilities = vec!["general".to_string()];
        
        if query_lower.contains("code") || query_lower.contains("function") || query_lower.contains("implement") {
            capabilities.push("code".to_string());
        }
        if query_lower.contains("math") || query_lower.contains("calculate") {
            capabilities.push("math".to_string());
        }
        if query_lower.contains("analyze") || query_lower.contains("compare") {
            capabilities.push("analysis".to_string());
        }
        
        let score = self.calculate_complexity(query_lower.len(), word_count);
        
        QueryComplexity {
            score,
            capabilities,
            estimated_tokens,
            confidence: 0.7,
        }
    }
    
    fn calculate_complexity(&self, char_count: usize, word_count: usize) -> f64 {
        let mut score: f64 = 0.1;
        
        if word_count > 50 {
            score += 0.3;
        } else if word_count > 20 {
            score += 0.2;
        } else if word_count > 10 {
            score += 0.1;
        }
        
        if char_count > 500 {
            score += 0.2;
        }
        
        score = score.clamp(0.05, 1.0);
        
        score
    }
}

impl Default for QueryClassifier {
    fn default() -> Self {
        Self::new()
    }
}
