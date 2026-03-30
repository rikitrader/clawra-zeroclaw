use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubsystemState {
    pub name: String,
    pub value: f64,
    pub connections: Vec<String>,
    pub last_update: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrationSnapshot {
    pub phi: f64,
    pub subsystem_count: usize,
    pub edge_count: usize,
    pub hub_ratio: f64,
    pub clustering_coefficient: f64,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct IntegrationMeter {
    subsystems: HashMap<String, SubsystemState>,
}

impl Default for IntegrationMeter {
    fn default() -> Self {
        Self::new()
    }
}

impl IntegrationMeter {
    pub fn new() -> Self {
        Self {
            subsystems: HashMap::new(),
        }
    }

    pub fn register_subsystem(&mut self, name: &str, connections: Vec<String>) {
        self.subsystems.insert(
            name.to_string(),
            SubsystemState {
                name: name.to_string(),
                value: 0.0,
                connections,
                last_update: Utc::now(),
            },
        );
    }

    pub fn update_state(&mut self, name: &str, value: f64) {
        if let Some(state) = self.subsystems.get_mut(name) {
            state.value = value;
            state.last_update = Utc::now();
        }
    }

    pub fn compute_phi(&self) -> f64 {
        let pairs = self.connected_pairs();
        if pairs.is_empty() {
            return 0.0;
        }
        let total: f64 = pairs
            .iter()
            .map(|(a, b)| {
                let va = self.subsystems[a].value;
                let vb = self.subsystems[b].value;
                1.0 - (va - vb).abs()
            })
            .sum();
        total / pairs.len() as f64
    }

    pub fn hub_ratio(&self) -> f64 {
        if self.subsystems.is_empty() {
            return 0.0;
        }
        let mut degrees: Vec<usize> = self
            .subsystems
            .values()
            .map(|s| s.connections.len())
            .collect();
        degrees.sort_unstable_by(|a, b| b.cmp(a));

        let total_edges: usize = degrees.iter().sum();
        if total_edges == 0 {
            return 0.0;
        }

        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let top_count = (self.subsystems.len() as f64 * 0.2).ceil() as usize;
        let top_count = top_count.max(1);
        let top_edges: usize = degrees.iter().take(top_count).sum();

        top_edges as f64 / total_edges as f64
    }

    pub fn clustering_coefficient(&self) -> f64 {
        if self.subsystems.is_empty() {
            return 0.0;
        }

        let mut total = 0.0;
        let mut counted = 0usize;

        for state in self.subsystems.values() {
            let neighbors = &state.connections;
            let k = neighbors.len();
            if k < 2 {
                continue;
            }

            let mut triangles = 0usize;
            for i in 0..k {
                for j in (i + 1)..k {
                    if let Some(ni) = self.subsystems.get(&neighbors[i]) {
                        if ni.connections.contains(&neighbors[j]) {
                            triangles += 1;
                        }
                    }
                }
            }

            let possible = k * (k - 1) / 2;
            total += triangles as f64 / possible as f64;
            counted += 1;
        }

        if counted == 0 {
            return 0.0;
        }
        total / counted as f64
    }

    pub fn snapshot(&self) -> IntegrationSnapshot {
        IntegrationSnapshot {
            phi: self.compute_phi(),
            subsystem_count: self.subsystem_count(),
            edge_count: self.edge_count(),
            hub_ratio: self.hub_ratio(),
            clustering_coefficient: self.clustering_coefficient(),
            timestamp: Utc::now(),
        }
    }

    pub fn is_scale_free(&self) -> bool {
        self.hub_ratio() >= 0.6
    }

    pub fn weakest_link(&self) -> Option<(String, String, f64)> {
        let pairs = self.connected_pairs();
        pairs
            .into_iter()
            .map(|(a, b)| {
                let va = self.subsystems[&a].value;
                let vb = self.subsystems[&b].value;
                let mi = 1.0 - (va - vb).abs();
                (a, b, mi)
            })
            .min_by(|x, y| x.2.partial_cmp(&y.2).unwrap_or(std::cmp::Ordering::Equal))
    }

    pub fn subsystem_count(&self) -> usize {
        self.subsystems.len()
    }

    fn edge_count(&self) -> usize {
        let mut count = 0usize;
        for (name, state) in &self.subsystems {
            for conn in &state.connections {
                if conn > name && self.subsystems.contains_key(conn) {
                    count += 1;
                }
            }
        }
        count
    }

    fn connected_pairs(&self) -> Vec<(String, String)> {
        let mut pairs = Vec::new();
        for (name, state) in &self.subsystems {
            for conn in &state.connections {
                if conn > name && self.subsystems.contains_key(conn) {
                    pairs.push((name.clone(), conn.clone()));
                }
            }
        }
        pairs
    }
}
