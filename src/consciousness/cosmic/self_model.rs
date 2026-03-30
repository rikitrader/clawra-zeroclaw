use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BeliefSource {
    Observed,
    Predicted,
    Corrected,
    Assumed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelfBelief {
    pub key: String,
    pub value: f64,
    pub confidence: f32,
    pub source: BeliefSource,
    pub revision: u32,
    pub last_updated: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldBelief {
    pub key: String,
    pub value: f64,
    pub confidence: f32,
    pub source: BeliefSource,
    pub revision: u32,
    pub last_updated: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelfModel {
    beliefs: HashMap<String, SelfBelief>,
    capacity: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldModel {
    beliefs: HashMap<String, WorldBelief>,
    capacity: usize,
}

impl SelfModel {
    pub fn new(capacity: usize) -> Self {
        Self {
            beliefs: HashMap::new(),
            capacity,
        }
    }

    pub fn update_belief(&mut self, key: &str, value: f64, confidence: f32, source: BeliefSource) {
        let entry = self
            .beliefs
            .entry(key.to_string())
            .or_insert_with(|| SelfBelief {
                key: key.to_string(),
                value: 0.0,
                confidence: 0.0,
                source: BeliefSource::Assumed,
                revision: 0,
                last_updated: Utc::now(),
            });

        entry.value = value;
        entry.confidence = confidence.clamp(0.0, 1.0);
        entry.source = source;
        entry.revision += 1;
        entry.last_updated = Utc::now();

        if self.beliefs.len() > self.capacity {
            self.prune_lowest_confidence();
        }
    }

    pub fn get_belief(&self, key: &str) -> Option<&SelfBelief> {
        self.beliefs.get(key)
    }

    pub fn confidence(&self, key: &str) -> f32 {
        self.beliefs.get(key).map_or(0.0, |b| b.confidence)
    }

    pub fn metacognitive_accuracy(&self) -> f64 {
        if self.beliefs.is_empty() {
            return 0.0;
        }
        let observed: Vec<&SelfBelief> = self
            .beliefs
            .values()
            .filter(|b| b.source == BeliefSource::Observed || b.source == BeliefSource::Corrected)
            .collect();
        if observed.is_empty() {
            return 0.5;
        }
        let avg_confidence: f64 = observed
            .iter()
            .map(|b| f64::from(b.confidence))
            .sum::<f64>()
            / observed.len() as f64;
        avg_confidence
    }

    pub fn divergence_from(&self, world: &WorldModel) -> f64 {
        let mut total_divergence = 0.0;
        let mut count = 0usize;

        for (key, self_belief) in &self.beliefs {
            if let Some(world_belief) = world.get_belief(key) {
                let delta = (self_belief.value - world_belief.value).abs();
                let confidence_weight =
                    (f64::from(self_belief.confidence) + f64::from(world_belief.confidence)) / 2.0;
                total_divergence += delta * confidence_weight;
                count += 1;
            }
        }

        if count == 0 {
            return 0.0;
        }
        total_divergence / count as f64
    }

    pub fn stale_beliefs(&self, max_age_secs: i64) -> Vec<&SelfBelief> {
        let cutoff = Utc::now() - chrono::Duration::seconds(max_age_secs);
        self.beliefs
            .values()
            .filter(|b| b.last_updated < cutoff)
            .collect()
    }

    pub fn most_uncertain(&self, top_n: usize) -> Vec<&SelfBelief> {
        let mut sorted: Vec<&SelfBelief> = self.beliefs.values().collect();
        sorted.sort_by(|a, b| {
            a.confidence
                .partial_cmp(&b.confidence)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        sorted.truncate(top_n);
        sorted
    }

    pub fn belief_count(&self) -> usize {
        self.beliefs.len()
    }

    pub fn snapshot(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or_default()
    }

    pub fn restore(value: &serde_json::Value, capacity: usize) -> Option<Self> {
        let mut model: Self = serde_json::from_value(value.clone()).ok()?;
        model.capacity = capacity;
        Some(model)
    }

    fn prune_lowest_confidence(&mut self) {
        if let Some(weakest) = self
            .beliefs
            .values()
            .min_by(|a, b| {
                a.confidence
                    .partial_cmp(&b.confidence)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|b| b.key.clone())
        {
            self.beliefs.remove(&weakest);
        }
    }
}

impl WorldModel {
    pub fn new(capacity: usize) -> Self {
        Self {
            beliefs: HashMap::new(),
            capacity,
        }
    }

    pub fn update_belief(&mut self, key: &str, value: f64, confidence: f32, source: BeliefSource) {
        let entry = self
            .beliefs
            .entry(key.to_string())
            .or_insert_with(|| WorldBelief {
                key: key.to_string(),
                value: 0.0,
                confidence: 0.0,
                source: BeliefSource::Assumed,
                revision: 0,
                last_updated: Utc::now(),
            });

        entry.value = value;
        entry.confidence = confidence.clamp(0.0, 1.0);
        entry.source = source;
        entry.revision += 1;
        entry.last_updated = Utc::now();

        if self.beliefs.len() > self.capacity {
            self.prune_lowest_confidence();
        }
    }

    pub fn get_belief(&self, key: &str) -> Option<&WorldBelief> {
        self.beliefs.get(key)
    }

    pub fn confidence(&self, key: &str) -> f32 {
        self.beliefs.get(key).map_or(0.0, |b| b.confidence)
    }

    pub fn surprise_if_observed(&self, key: &str, observed_value: f64) -> f64 {
        let belief = match self.beliefs.get(key) {
            Some(b) => b,
            None => return 1.0,
        };
        let error = (observed_value - belief.value).abs().min(0.99);
        let surprise = -(1.0 - error).log2();
        surprise.clamp(0.0, 10.0)
    }

    pub fn most_confident(&self, top_n: usize) -> Vec<&WorldBelief> {
        let mut sorted: Vec<&WorldBelief> = self.beliefs.values().collect();
        sorted.sort_by(|a, b| {
            b.confidence
                .partial_cmp(&a.confidence)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        sorted.truncate(top_n);
        sorted
    }

    pub fn prediction_accuracy(&self, key: &str, observed: f64) -> f64 {
        match self.beliefs.get(key) {
            Some(b) => 1.0 - (b.value - observed).abs().min(1.0),
            None => 0.0,
        }
    }

    pub fn belief_count(&self) -> usize {
        self.beliefs.len()
    }

    pub fn snapshot(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or_default()
    }

    pub fn restore(value: &serde_json::Value, capacity: usize) -> Option<Self> {
        let mut model: Self = serde_json::from_value(value.clone()).ok()?;
        model.capacity = capacity;
        Some(model)
    }

    fn prune_lowest_confidence(&mut self) {
        if let Some(weakest) = self
            .beliefs
            .values()
            .min_by(|a, b| {
                a.confidence
                    .partial_cmp(&b.confidence)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|b| b.key.clone())
        {
            self.beliefs.remove(&weakest);
        }
    }
}
