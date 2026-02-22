//! Models module - Model pool and backend integrations

use crate::config::ModelConfig;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// A model in our pool
#[derive(Debug, Clone)]
pub struct Model {
    pub config: Arc<ModelConfig>,
    pub id: String,
}

impl Model {
    pub fn new(config: ModelConfig) -> Self {
        let id = config.id.clone();
        Self {
            config: Arc::new(config),
            id,
        }
    }
}

/// Model pool - manages available models
#[derive(Debug, Clone)]
pub struct ModelPool {
    pub models: Vec<Model>,
    by_id: HashMap<String, usize>,
}

impl ModelPool {
    pub fn new(configs: Vec<ModelConfig>) -> Self {
        let by_id: HashMap<String, usize> = configs
            .iter()
            .enumerate()
            .map(|(i, c)| (c.id.clone(), i))
            .collect();

        let models = configs.into_iter().map(Model::new).collect();

        Self { models, by_id }
    }

    pub fn get(&self, id: &str) -> Option<&Model> {
        self.by_id.get(id).and_then(|&i| self.models.get(i))
    }

    pub fn get_all(&self) -> &[Model] {
        &self.models
    }

    pub fn is_empty(&self) -> bool {
        self.models.is_empty()
    }

    pub fn len(&self) -> usize {
        self.models.len()
    }

    pub fn get_by_capability(&self, capability: &str) -> Vec<&Model> {
        self.models
            .iter()
            .filter(|m| {
                m.config
                    .capabilities
                    .iter()
                    .any(|c| format!("{:?}", c).contains(capability))
            })
            .collect()
    }

    pub fn get_cheapest(&self) -> Option<&Model> {
        self.models.iter().min_by(|a, b| {
            a.config
                .input_cost
                .partial_cmp(&b.config.input_cost)
                .unwrap()
        })
    }

    pub fn get_medium(&self) -> Option<&Model> {
        let mut models: Vec<_> = self.models.iter().collect();
        models.sort_by(|a, b| {
            a.config
                .input_cost
                .partial_cmp(&b.config.input_cost)
                .unwrap()
        });
        models.get(models.len() / 2).copied()
    }

    pub fn get_best(&self) -> Option<&Model> {
        self.models.iter().max_by(|a, b| {
            a.config
                .quality_score
                .partial_cmp(&b.config.quality_score)
                .unwrap()
        })
    }

    pub fn get_fastest(&self) -> Option<&Model> {
        self.models.iter().min_by_key(|m| m.config.latency_ms)
    }

    pub fn get_best_quality(&self) -> Option<&Model> {
        self.get_best()
    }

    /// Get models sorted by cost (ascending)
    pub fn get_sorted_by_cost(&self) -> Vec<&Model> {
        let mut models: Vec<_> = self.models.iter().collect();
        models.sort_by(|a, b| {
            a.config
                .input_cost
                .partial_cmp(&b.config.input_cost)
                .unwrap()
        });
        models
    }

    /// Get models sorted by quality (descending)
    pub fn get_sorted_by_quality(&self) -> Vec<&Model> {
        let mut models: Vec<_> = self.models.iter().collect();
        models.sort_by(|a, b| {
            b.config
                .quality_score
                .partial_cmp(&a.config.quality_score)
                .unwrap()
        });
        models
    }
}

impl Default for ModelPool {
    fn default() -> Self {
        use crate::config::default_models;
        Self::new(default_models())
    }
}

/// Request to a model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<Message>,
    pub temperature: Option<f64>,
    pub max_tokens: Option<u32>,
    pub stream: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

/// Response from a model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    pub id: String,
    pub model: String,
    pub choices: Vec<Choice>,
    pub usage: Usage,
    pub created: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Choice {
    pub index: u32,
    pub message: Message,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}
