use std::collections::{HashMap, VecDeque};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NormKind {
    Obligation,
    Prohibition,
    Permission,
    Preference,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NormStatus {
    Active,
    Suspended,
    Expired,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Norm {
    pub id: String,
    pub kind: NormKind,
    pub domain: String,
    pub description: String,
    pub weight: f64,
    pub status: NormStatus,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormViolation {
    pub norm_id: String,
    pub action: String,
    pub severity: f64,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormativeDecision {
    pub action: String,
    pub score: f64,
    pub supporting_norms: Vec<String>,
    pub opposing_norms: Vec<String>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct NormativeEngine {
    norms: HashMap<String, Norm>,
    violations: VecDeque<NormViolation>,
    decisions: VecDeque<NormativeDecision>,
    violation_capacity: usize,
    decision_capacity: usize,
}

impl NormativeEngine {
    pub fn new(violation_capacity: usize, decision_capacity: usize) -> Self {
        Self {
            norms: HashMap::new(),
            violations: VecDeque::with_capacity(violation_capacity),
            decisions: VecDeque::with_capacity(decision_capacity),
            violation_capacity,
            decision_capacity,
        }
    }

    pub fn register_norm(
        &mut self,
        id: &str,
        kind: NormKind,
        domain: &str,
        description: &str,
        weight: f64,
    ) {
        let norm = Norm {
            id: id.to_string(),
            kind,
            domain: domain.to_string(),
            description: description.to_string(),
            weight: weight.clamp(0.0, 1.0),
            status: NormStatus::Active,
            created_at: Utc::now(),
        };
        self.norms.insert(id.to_string(), norm);
    }

    pub fn suspend_norm(&mut self, id: &str) {
        if let Some(norm) = self.norms.get_mut(id) {
            norm.status = NormStatus::Suspended;
        }
    }

    pub fn activate_norm(&mut self, id: &str) {
        if let Some(norm) = self.norms.get_mut(id) {
            norm.status = NormStatus::Active;
        }
    }

    pub fn evaluate_action(&mut self, action: &str, relevant_domains: &[&str]) -> f64 {
        let active_norms: Vec<&Norm> = self
            .norms
            .values()
            .filter(|n| n.status == NormStatus::Active)
            .filter(|n| {
                relevant_domains.is_empty() || relevant_domains.contains(&n.domain.as_str())
            })
            .collect();

        if active_norms.is_empty() {
            return 0.0;
        }

        let mut supporting = Vec::new();
        let mut opposing = Vec::new();
        let mut score = 0.0;

        for norm in &active_norms {
            let alignment = self.compute_alignment(action, norm);
            match norm.kind {
                NormKind::Obligation => {
                    score += alignment * norm.weight;
                    if alignment > 0.0 {
                        supporting.push(norm.id.clone());
                    } else {
                        opposing.push(norm.id.clone());
                    }
                }
                NormKind::Prohibition => {
                    score -= alignment * norm.weight;
                    if alignment > 0.0 {
                        opposing.push(norm.id.clone());
                    } else {
                        supporting.push(norm.id.clone());
                    }
                }
                NormKind::Permission => {
                    score += alignment * norm.weight * 0.5;
                    if alignment > 0.0 {
                        supporting.push(norm.id.clone());
                    }
                }
                NormKind::Preference => {
                    score += alignment * norm.weight * 0.3;
                    if alignment > 0.0 {
                        supporting.push(norm.id.clone());
                    }
                }
            }
        }

        let normalized = score / active_norms.len() as f64;

        let decision = NormativeDecision {
            action: action.to_string(),
            score: normalized,
            supporting_norms: supporting,
            opposing_norms: opposing,
            timestamp: Utc::now(),
        };
        self.decisions.push_back(decision);
        if self.decisions.len() > self.decision_capacity {
            self.decisions.pop_front();
        }

        normalized
    }

    fn compute_alignment(&self, action: &str, norm: &Norm) -> f64 {
        let action_lower = action.to_lowercase();
        let desc_lower = norm.description.to_lowercase();

        let action_words: Vec<&str> = action_lower.split_whitespace().collect();
        let desc_words: Vec<&str> = desc_lower.split_whitespace().collect();

        if action_words.is_empty() || desc_words.is_empty() {
            return 0.0;
        }

        let matching = action_words
            .iter()
            .filter(|w| w.len() > 2 && desc_words.contains(w))
            .count();

        matching as f64 / action_words.len().max(1) as f64
    }

    pub fn record_violation(&mut self, norm_id: &str, action: &str, severity: f64) {
        if !self.norms.contains_key(norm_id) {
            return;
        }
        let violation = NormViolation {
            norm_id: norm_id.to_string(),
            action: action.to_string(),
            severity: severity.clamp(0.0, 1.0),
            timestamp: Utc::now(),
        };
        self.violations.push_back(violation);
        if self.violations.len() > self.violation_capacity {
            self.violations.pop_front();
        }
    }

    pub fn violation_rate(&self, norm_id: &str) -> f64 {
        let total = self
            .violations
            .iter()
            .filter(|v| v.norm_id == norm_id)
            .count();
        let decisions = self
            .decisions
            .iter()
            .filter(|d| {
                d.supporting_norms.contains(&norm_id.to_string())
                    || d.opposing_norms.contains(&norm_id.to_string())
            })
            .count();

        if decisions == 0 {
            return 0.0;
        }
        total as f64 / decisions as f64
    }

    pub fn most_violated(&self, top_n: usize) -> Vec<(String, usize)> {
        let mut counts: HashMap<&str, usize> = HashMap::new();
        for v in &self.violations {
            *counts.entry(v.norm_id.as_str()).or_default() += 1;
        }

        let mut sorted: Vec<(String, usize)> = counts
            .into_iter()
            .map(|(id, c)| (id.to_string(), c))
            .collect();
        sorted.sort_by(|a, b| b.1.cmp(&a.1));
        sorted.truncate(top_n);
        sorted
    }

    pub fn should_inhibit(&self, action: &str, threshold: f64) -> bool {
        let active_prohibitions: Vec<&Norm> = self
            .norms
            .values()
            .filter(|n| n.status == NormStatus::Active && n.kind == NormKind::Prohibition)
            .collect();

        for norm in &active_prohibitions {
            let alignment = self.compute_alignment(action, norm);
            if alignment * norm.weight > threshold {
                return true;
            }
        }
        false
    }

    pub fn norm_count(&self) -> usize {
        self.norms.len()
    }

    pub fn active_norm_count(&self) -> usize {
        self.norms
            .values()
            .filter(|n| n.status == NormStatus::Active)
            .count()
    }

    pub fn violation_count(&self) -> usize {
        self.violations.len()
    }

    pub fn snapshot(&self) -> serde_json::Value {
        serde_json::json!({
            "norm_count": self.norms.len(),
            "active_norm_count": self.active_norm_count(),
            "violation_count": self.violations.len(),
            "decision_count": self.decisions.len(),
            "norms": self.norms.values().collect::<Vec<_>>(),
            "violations": self.violations.iter().collect::<Vec<_>>(),
        })
    }

    pub fn restore(
        data: &serde_json::Value,
        violation_capacity: usize,
        decision_capacity: usize,
    ) -> Option<Self> {
        let norms: Vec<Norm> = serde_json::from_value(data.get("norms")?.clone()).ok()?;
        let violations: Vec<NormViolation> =
            serde_json::from_value(data.get("violations").cloned().unwrap_or_default()).ok()?;
        let norm_map: HashMap<String, Norm> =
            norms.into_iter().map(|n| (n.id.clone(), n)).collect();
        let mut vq = VecDeque::with_capacity(violation_capacity);
        for v in violations {
            vq.push_back(v);
            if vq.len() > violation_capacity {
                vq.pop_front();
            }
        }
        Some(Self {
            norms: norm_map,
            violations: vq,
            decisions: VecDeque::with_capacity(decision_capacity),
            violation_capacity,
            decision_capacity,
        })
    }

    pub fn norms_by_domain(&self, domain: &str) -> Vec<&Norm> {
        self.norms.values().filter(|n| n.domain == domain).collect()
    }
}
