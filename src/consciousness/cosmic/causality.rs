use std::collections::{HashMap, HashSet, VecDeque};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CausalEvent {
    pub source: String,
    pub target: String,
    pub source_delta: f64,
    pub target_delta: f64,
    pub latency_ms: u64,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CausalEdge {
    pub from: String,
    pub to: String,
    pub strength: f64,
    pub transfer_entropy: f64,
    pub event_count: u64,
    pub last_event: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CausalLoop {
    pub nodes: Vec<String>,
    pub min_strength: f64,
    pub integrated_phi: f64,
}

const EMA_ALPHA: f64 = 0.15;

#[derive(Debug, Clone)]
pub struct CausalGraph {
    edges: HashMap<(String, String), CausalEdge>,
    events: VecDeque<CausalEvent>,
    capacity: usize,
}

impl CausalGraph {
    pub fn new(capacity: usize) -> Self {
        Self {
            edges: HashMap::new(),
            events: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    pub fn record_event(
        &mut self,
        source: &str,
        target: &str,
        source_delta: f64,
        target_delta: f64,
        latency_ms: u64,
    ) {
        let event = CausalEvent {
            source: source.to_string(),
            target: target.to_string(),
            source_delta,
            target_delta,
            latency_ms,
            timestamp: Utc::now(),
        };

        self.events.push_back(event);
        if self.events.len() > self.capacity {
            self.events.pop_front();
        }

        let coupling = if source_delta.abs() > f64::EPSILON {
            (target_delta.abs() / source_delta.abs()).min(1.0)
        } else {
            0.0
        };

        let te = self.compute_transfer_entropy(source, target);

        let key = (source.to_string(), target.to_string());
        let edge = self.edges.entry(key).or_insert_with(|| CausalEdge {
            from: source.to_string(),
            to: target.to_string(),
            strength: 0.0,
            transfer_entropy: 0.0,
            event_count: 0,
            last_event: Utc::now(),
        });

        edge.strength = EMA_ALPHA * coupling + (1.0 - EMA_ALPHA) * edge.strength;
        edge.transfer_entropy = te;
        edge.event_count += 1;
        edge.last_event = Utc::now();
    }

    fn compute_transfer_entropy(&self, source: &str, target: &str) -> f64 {
        let relevant: Vec<&CausalEvent> = self
            .events
            .iter()
            .filter(|e| e.source == source && e.target == target)
            .collect();

        if relevant.len() < 2 {
            return 0.0;
        }

        let responsive = relevant
            .iter()
            .filter(|e| e.target_delta.abs() > f64::EPSILON)
            .count();

        let ratio = responsive as f64 / relevant.len() as f64;
        if ratio >= 0.99 {
            return 3.0;
        }
        if ratio < f64::EPSILON {
            return 0.0;
        }
        let raw = -(1.0 - ratio).log2();
        raw.clamp(0.0, 3.0)
    }

    pub fn causal_strength(&self, from: &str, to: &str) -> f64 {
        self.edges
            .get(&(from.to_string(), to.to_string()))
            .map_or(0.0, |e| e.strength)
    }

    pub fn transfer_entropy(&self, from: &str, to: &str) -> f64 {
        self.edges
            .get(&(from.to_string(), to.to_string()))
            .map_or(0.0, |e| e.transfer_entropy)
    }

    pub fn find_loops(&self, max_length: usize) -> Vec<CausalLoop> {
        let nodes: HashSet<&str> = self
            .edges
            .keys()
            .flat_map(|(f, t)| [f.as_str(), t.as_str()])
            .collect();

        let mut loops = Vec::new();

        for &start in &nodes {
            self.dfs_loops(
                start,
                start,
                &mut vec![start.to_string()],
                max_length,
                &mut loops,
            );
        }

        deduplicate_loops(&mut loops);
        loops
    }

    fn dfs_loops(
        &self,
        start: &str,
        current: &str,
        path: &mut Vec<String>,
        max_length: usize,
        result: &mut Vec<CausalLoop>,
    ) {
        if path.len() > max_length {
            return;
        }

        for ((from, to), edge) in &self.edges {
            if from != current || edge.strength < 0.01 {
                continue;
            }

            if to == start && path.len() >= 3 {
                let min_strength = self.loop_min_strength(path, start);
                let phi = self.loop_phi(path, start);
                result.push(CausalLoop {
                    nodes: path.clone(),
                    min_strength,
                    integrated_phi: phi,
                });
            } else if !path.contains(to) && path.len() < max_length {
                path.push(to.clone());
                self.dfs_loops(start, to, path, max_length, result);
                path.pop();
            }
        }
    }

    fn loop_min_strength(&self, path: &[String], back_to: &str) -> f64 {
        let mut min = f64::INFINITY;
        for i in 0..path.len() {
            let next = if i + 1 < path.len() {
                &path[i + 1]
            } else {
                back_to
            };
            let s = self.causal_strength(&path[i], next);
            if s < min {
                min = s;
            }
        }
        if min.is_infinite() {
            0.0
        } else {
            min
        }
    }

    fn loop_phi(&self, path: &[String], back_to: &str) -> f64 {
        let mut product = 1.0f64;
        let mut count = 0u32;

        for i in 0..path.len() {
            let next = if i + 1 < path.len() {
                &path[i + 1]
            } else {
                back_to
            };
            let key = (path[i].clone(), next.to_string());
            if let Some(edge) = self.edges.get(&key) {
                let weighted = edge.strength * (1.0 + edge.transfer_entropy);
                product *= weighted;
                count += 1;
            }
        }

        if count == 0 {
            return 0.0;
        }

        product.powf(1.0 / f64::from(count))
    }

    pub fn integrated_phi(&self, max_loop_length: usize) -> f64 {
        let loops = self.find_loops(max_loop_length);
        if loops.is_empty() {
            return 0.0;
        }

        let sum: f64 = loops.iter().map(|l| l.integrated_phi).sum();
        sum / loops.len() as f64
    }

    pub fn weakest_causal_link(&self) -> Option<(String, String, f64)> {
        self.edges
            .values()
            .filter(|e| e.event_count > 0)
            .min_by(|a, b| {
                a.strength
                    .partial_cmp(&b.strength)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|e| (e.from.clone(), e.to.clone(), e.strength))
    }

    pub fn intervention_candidates(&self, top_n: usize) -> Vec<(String, f64)> {
        let nodes: HashSet<String> = self
            .edges
            .keys()
            .flat_map(|(f, t)| [f.clone(), t.clone()])
            .collect();

        let mut scores: Vec<(String, f64)> = nodes
            .into_iter()
            .map(|node| {
                let outgoing: f64 = self
                    .edges
                    .values()
                    .filter(|e| e.from == node)
                    .map(|e| e.strength)
                    .sum();
                let incoming: f64 = self
                    .edges
                    .values()
                    .filter(|e| e.to == node)
                    .map(|e| e.strength)
                    .sum();
                let score = if incoming > f64::EPSILON {
                    outgoing / incoming
                } else if outgoing > f64::EPSILON {
                    outgoing * 10.0
                } else {
                    0.0
                };
                (node, score)
            })
            .collect();

        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scores.truncate(top_n);
        scores
    }

    pub fn snapshot(&self) -> serde_json::Value {
        let edges_vec: Vec<&CausalEdge> = self.edges.values().collect();
        serde_json::json!({
            "edge_count": self.edges.len(),
            "event_count": self.events.len(),
            "integrated_phi": self.integrated_phi(4),
            "edges": edges_vec,
            "events": self.events.iter().collect::<Vec<_>>(),
            "capacity": self.capacity,
        })
    }

    pub fn restore(data: &serde_json::Value) -> Option<Self> {
        #[allow(clippy::cast_possible_truncation)]
        let capacity = data.get("capacity")?.as_u64()? as usize;
        let edges_vec: Vec<CausalEdge> = serde_json::from_value(data.get("edges")?.clone()).ok()?;
        let events_vec: Vec<CausalEvent> =
            serde_json::from_value(data.get("events").cloned().unwrap_or_default()).ok()?;
        let mut edge_map = HashMap::new();
        for e in edges_vec {
            edge_map.insert((e.from.clone(), e.to.clone()), e);
        }
        let mut events = VecDeque::with_capacity(capacity);
        for ev in events_vec {
            events.push_back(ev);
            if events.len() > capacity {
                events.pop_front();
            }
        }
        Some(Self {
            edges: edge_map,
            events,
            capacity,
        })
    }

    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    pub fn event_count(&self) -> usize {
        self.events.len()
    }
}

fn normalize_loop(nodes: &[String]) -> Vec<String> {
    if nodes.is_empty() {
        return vec![];
    }
    let min_pos = nodes
        .iter()
        .enumerate()
        .min_by(|a, b| a.1.cmp(b.1))
        .map(|(i, _)| i)
        .unwrap_or(0);
    let mut rotated: Vec<String> = nodes[min_pos..].to_vec();
    rotated.extend_from_slice(&nodes[..min_pos]);
    rotated
}

fn deduplicate_loops(loops: &mut Vec<CausalLoop>) {
    let mut seen: HashSet<Vec<String>> = HashSet::new();
    loops.retain(|l| {
        let normalized = normalize_loop(&l.nodes);
        seen.insert(normalized)
    });
}
