use std::collections::{HashMap, VecDeque};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SubsystemId {
    Memory,
    FreeEnergy,
    Causality,
    SelfModel,
    WorldModel,
    Normative,
    Modulation,
    Policy,
    Counterfactual,
    Consolidation,
    Drift,
    Constitution,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceEntry {
    pub subsystem: SubsystemId,
    pub activation: f64,
    pub priority: f64,
    pub last_broadcast: DateTime<Utc>,
    pub suppressed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BroadcastResult {
    pub active_subsystems: Vec<SubsystemId>,
    pub suppressed_subsystems: Vec<SubsystemId>,
    pub dominant: Option<SubsystemId>,
    pub coherence: f64,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictResolution {
    pub subsystem_a: SubsystemId,
    pub subsystem_b: SubsystemId,
    pub winner: SubsystemId,
    pub reason: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct GlobalWorkspace {
    entries: HashMap<SubsystemId, WorkspaceEntry>,
    broadcast_history: VecDeque<BroadcastResult>,
    conflict_log: VecDeque<ConflictResolution>,
    activation_threshold: f64,
    max_active: usize,
    history_capacity: usize,
}

impl GlobalWorkspace {
    pub fn new(activation_threshold: f64, max_active: usize, history_capacity: usize) -> Self {
        Self {
            entries: HashMap::new(),
            broadcast_history: VecDeque::with_capacity(history_capacity),
            conflict_log: VecDeque::with_capacity(history_capacity),
            activation_threshold,
            max_active,
            history_capacity,
        }
    }

    pub fn register_subsystem(&mut self, id: SubsystemId, priority: f64) {
        let entry = WorkspaceEntry {
            subsystem: id,
            activation: 0.0,
            priority: priority.clamp(0.0, 1.0),
            last_broadcast: Utc::now(),
            suppressed: false,
        };
        self.entries.insert(id, entry);
    }

    pub fn activate(&mut self, id: SubsystemId, level: f64) {
        if let Some(entry) = self.entries.get_mut(&id) {
            entry.activation = level.clamp(0.0, 1.0);
        }
    }

    pub fn suppress(&mut self, id: SubsystemId) {
        if let Some(entry) = self.entries.get_mut(&id) {
            entry.suppressed = true;
        }
    }

    pub fn unsuppress(&mut self, id: SubsystemId) {
        if let Some(entry) = self.entries.get_mut(&id) {
            entry.suppressed = false;
        }
    }

    pub fn compete(&mut self) -> BroadcastResult {
        let mut ranked: Vec<(SubsystemId, f64)> = self
            .entries
            .values()
            .filter(|e| !e.suppressed)
            .map(|e| (e.subsystem, e.activation * e.priority))
            .collect();
        ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let mut active = Vec::new();
        let mut suppressed = Vec::new();

        for (i, (id, _score)) in ranked.iter().enumerate() {
            let entry = self.entries.get(id).unwrap();
            if i < self.max_active && entry.activation >= self.activation_threshold {
                active.push(*id);
            } else {
                suppressed.push(*id);
            }
        }

        for (id, entry) in &mut self.entries {
            if active.contains(id) {
                entry.suppressed = false;
                entry.last_broadcast = Utc::now();
            } else {
                entry.suppressed = true;
            }
        }

        let already_suppressed: Vec<SubsystemId> = self
            .entries
            .values()
            .filter(|e| e.suppressed && !suppressed.contains(&e.subsystem))
            .map(|e| e.subsystem)
            .collect();
        suppressed.extend(already_suppressed);

        let coherence = self.coherence_score(&active);
        let dominant = active.first().copied();

        let result = BroadcastResult {
            active_subsystems: active,
            suppressed_subsystems: suppressed,
            dominant,
            coherence,
            timestamp: Utc::now(),
        };

        self.broadcast_history.push_back(result.clone());
        if self.broadcast_history.len() > self.history_capacity {
            self.broadcast_history.pop_front();
        }

        result
    }

    pub fn coherence_score(&self, active: &[SubsystemId]) -> f64 {
        if active.is_empty() {
            return 0.0;
        }
        let sum: f64 = active
            .iter()
            .filter_map(|id| self.entries.get(id))
            .map(|e| e.activation)
            .sum();
        sum / active.len() as f64
    }

    pub fn resolve_conflict(&mut self, a: SubsystemId, b: SubsystemId) -> SubsystemId {
        let score_a = self
            .entries
            .get(&a)
            .map(|e| e.activation * e.priority)
            .unwrap_or(0.0);
        let score_b = self
            .entries
            .get(&b)
            .map(|e| e.activation * e.priority)
            .unwrap_or(0.0);

        let winner = if score_a >= score_b { a } else { b };
        let reason = format!(
            "{:?} scored {:.4} vs {:?} scored {:.4}",
            a, score_a, b, score_b
        );

        let resolution = ConflictResolution {
            subsystem_a: a,
            subsystem_b: b,
            winner,
            reason,
            timestamp: Utc::now(),
        };

        self.conflict_log.push_back(resolution);
        if self.conflict_log.len() > self.history_capacity {
            self.conflict_log.pop_front();
        }

        winner
    }

    pub fn broadcast(&self) -> Vec<SubsystemId> {
        self.entries
            .values()
            .filter(|e| !e.suppressed && e.activation >= self.activation_threshold)
            .map(|e| e.subsystem)
            .collect()
    }

    pub fn dominant_subsystem(&self) -> Option<SubsystemId> {
        self.entries
            .values()
            .filter(|e| !e.suppressed && e.activation >= self.activation_threshold)
            .max_by(|a, b| {
                let sa = a.activation * a.priority;
                let sb = b.activation * b.priority;
                sa.partial_cmp(&sb).unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|e| e.subsystem)
    }

    pub fn decay_activations(&mut self, rate: f64) {
        let rate = rate.clamp(0.0, 1.0);
        for entry in self.entries.values_mut() {
            entry.activation *= 1.0 - rate;
        }
    }

    pub fn snapshot(&self) -> BroadcastResult {
        let active: Vec<SubsystemId> = self
            .entries
            .values()
            .filter(|e| !e.suppressed && e.activation >= self.activation_threshold)
            .map(|e| e.subsystem)
            .collect();

        let suppressed: Vec<SubsystemId> = self
            .entries
            .values()
            .filter(|e| e.suppressed || e.activation < self.activation_threshold)
            .map(|e| e.subsystem)
            .collect();

        let coherence = self.coherence_score(&active);
        let dominant = self.dominant_subsystem();

        BroadcastResult {
            active_subsystems: active,
            suppressed_subsystems: suppressed,
            dominant,
            coherence,
            timestamp: Utc::now(),
        }
    }

    pub fn active_count(&self) -> usize {
        self.entries
            .values()
            .filter(|e| !e.suppressed && e.activation >= self.activation_threshold)
            .count()
    }

    pub fn conflict_count(&self) -> usize {
        self.conflict_log.len()
    }

    pub fn full_snapshot(&self) -> serde_json::Value {
        let entries: Vec<&WorkspaceEntry> = self.entries.values().collect();
        serde_json::json!({
            "activation_threshold": self.activation_threshold,
            "max_active": self.max_active,
            "history_capacity": self.history_capacity,
            "entries": entries,
        })
    }

    pub fn restore(data: &serde_json::Value) -> Option<Self> {
        let activation_threshold = data.get("activation_threshold")?.as_f64()?;
        #[allow(clippy::cast_possible_truncation)]
        let max_active = data.get("max_active")?.as_u64()? as usize;
        #[allow(clippy::cast_possible_truncation)]
        let history_capacity = data.get("history_capacity")?.as_u64()? as usize;
        let entries_vec: Vec<WorkspaceEntry> =
            serde_json::from_value(data.get("entries")?.clone()).ok()?;
        let mut entries = HashMap::new();
        for e in entries_vec {
            entries.insert(e.subsystem, e);
        }
        Some(Self {
            entries,
            broadcast_history: VecDeque::with_capacity(history_capacity),
            conflict_log: VecDeque::with_capacity(history_capacity),
            activation_threshold,
            max_active,
            history_capacity,
        })
    }
}
