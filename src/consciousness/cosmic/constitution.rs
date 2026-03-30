use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Value {
    id: String,
    description: String,
    priority: f64,
    immutable: bool,
    created_at: DateTime<Utc>,
}

impl Value {
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub fn priority(&self) -> f64 {
        self.priority
    }

    pub fn immutable(&self) -> bool {
        self.immutable
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrityCheck {
    pub expected_hash: String,
    pub actual_hash: String,
    pub passed: bool,
    pub checked_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct Constitution {
    values: HashMap<String, Value>,
    integrity_hash: String,
}

impl Default for Constitution {
    fn default() -> Self {
        Self::new()
    }
}

impl Constitution {
    pub fn new() -> Self {
        let mut c = Self {
            values: HashMap::new(),
            integrity_hash: String::new(),
        };
        c.integrity_hash = c.compute_hash();
        c
    }

    pub fn register_value(
        &mut self,
        id: &str,
        description: &str,
        priority: f64,
        immutable: bool,
    ) -> bool {
        if let Some(existing) = self.values.get(id) {
            if existing.immutable {
                return false;
            }
        }
        let value = Value {
            id: id.to_string(),
            description: description.to_string(),
            priority: priority.clamp(0.0, 1.0),
            immutable,
            created_at: Utc::now(),
        };
        self.values.insert(id.to_string(), value);
        true
    }

    pub fn get_value(&self, id: &str) -> Option<&Value> {
        self.values.get(id)
    }

    pub fn remove_value(&mut self, id: &str) -> bool {
        if let Some(v) = self.values.get(id) {
            if v.immutable {
                return false;
            }
        } else {
            return false;
        }
        self.values.remove(id);
        true
    }

    pub fn compute_hash(&self) -> String {
        let mut keys: Vec<&String> = self.values.keys().collect();
        keys.sort();
        let stable: Vec<serde_json::Value> = keys
            .iter()
            .map(|k| {
                let v = &self.values[*k];
                serde_json::json!({
                    "id": v.id,
                    "description": v.description,
                    "priority": v.priority,
                    "immutable": v.immutable,
                })
            })
            .collect();
        let json = serde_json::to_string(&stable).unwrap_or_default();
        let mut hasher = Sha256::new();
        hasher.update(json.as_bytes());
        hex::encode(hasher.finalize())
    }

    pub fn seal(&mut self) {
        self.integrity_hash = self.compute_hash();
    }

    pub fn verify_integrity(&self) -> IntegrityCheck {
        let actual = self.compute_hash();
        IntegrityCheck {
            expected_hash: self.integrity_hash.clone(),
            actual_hash: actual.clone(),
            passed: self.integrity_hash == actual,
            checked_at: Utc::now(),
        }
    }

    pub fn check_action_alignment(&self, action: &str) -> f64 {
        if self.values.is_empty() {
            return 0.0;
        }

        let action_lower = action.to_lowercase();
        let action_words: Vec<&str> = action_lower.split_whitespace().collect();
        if action_words.is_empty() {
            return 0.0;
        }

        let mut total_score = 0.0;
        let mut total_weight = 0.0;

        for value in self.values.values() {
            let desc_lower = value.description.to_lowercase();
            let desc_words: Vec<&str> = desc_lower.split_whitespace().collect();

            let matching = action_words
                .iter()
                .filter(|w| w.len() > 2 && desc_words.contains(w))
                .count();

            let alignment = matching as f64 / action_words.len().max(1) as f64;
            total_score += alignment * value.priority;
            total_weight += value.priority;
        }

        if total_weight == 0.0 {
            return 0.0;
        }

        (total_score / total_weight).clamp(-1.0, 1.0)
    }

    pub fn value_count(&self) -> usize {
        self.values.len()
    }

    pub fn immutable_count(&self) -> usize {
        self.values.values().filter(|v| v.immutable).count()
    }

    pub fn values_by_priority(&self) -> Vec<&Value> {
        let mut sorted: Vec<&Value> = self.values.values().collect();
        sorted.sort_by(|a, b| {
            b.priority
                .partial_cmp(&a.priority)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        sorted
    }
}
