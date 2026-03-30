use std::collections::{HashMap, HashSet, VecDeque};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CosmicNode {
    pub id: String,
    pub content: String,
    pub category: String,
    pub embedding: Vec<f32>,
    pub created_at: DateTime<Utc>,
    pub access_count: u64,
    pub activation: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CosmicEdge {
    pub from: String,
    pub to: String,
    pub strength: f32,
    pub edge_type: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CosmicMemoryGraph {
    nodes: HashMap<String, CosmicNode>,
    edges: Vec<CosmicEdge>,
    max_nodes: usize,
}

impl CosmicMemoryGraph {
    pub fn new(max_nodes: usize) -> Self {
        Self {
            nodes: HashMap::new(),
            edges: Vec::new(),
            max_nodes,
        }
    }

    pub fn insert_node(
        &mut self,
        id: String,
        content: String,
        category: String,
        embedding: Vec<f32>,
    ) -> bool {
        if self.nodes.len() >= self.max_nodes && !self.nodes.contains_key(&id) {
            return false;
        }
        let node = CosmicNode {
            id: id.clone(),
            content,
            category,
            embedding,
            created_at: Utc::now(),
            access_count: 0,
            activation: 0.0,
        };
        self.nodes.insert(id, node);
        true
    }

    pub fn insert_edge(&mut self, from: String, to: String, strength: f32, edge_type: String) {
        let edge = CosmicEdge {
            from,
            to,
            strength,
            edge_type,
            created_at: Utc::now(),
        };
        self.edges.push(edge);
    }

    pub fn strengthen_or_insert_edge(&mut self, from: &str, to: &str, delta: f32, edge_type: &str) {
        if let Some(edge) = self
            .edges
            .iter_mut()
            .find(|e| e.from == from && e.to == to && e.edge_type == edge_type)
        {
            edge.strength = (edge.strength + delta).clamp(0.0, 1.0);
        } else {
            self.insert_edge(
                from.to_string(),
                to.to_string(),
                delta.clamp(0.0, 1.0),
                edge_type.to_string(),
            );
        }
    }

    pub fn get_node(&self, id: &str) -> Option<&CosmicNode> {
        self.nodes.get(id)
    }

    pub fn get_node_mut(&mut self, id: &str) -> Option<&mut CosmicNode> {
        self.nodes.get_mut(id)
    }

    pub fn neighbors(&self, id: &str) -> Vec<(&CosmicEdge, &CosmicNode)> {
        let mut result = Vec::new();
        for edge in &self.edges {
            if edge.from == id {
                if let Some(node) = self.nodes.get(&edge.to) {
                    result.push((edge, node));
                }
            } else if edge.to == id {
                if let Some(node) = self.nodes.get(&edge.from) {
                    result.push((edge, node));
                }
            }
        }
        result
    }

    pub fn strongest_path(&self, from: &str, to: &str) -> Option<Vec<String>> {
        if !self.nodes.contains_key(from) || !self.nodes.contains_key(to) {
            return None;
        }
        if from == to {
            return Some(vec![from.to_string()]);
        }

        let mut best_min_strength: HashMap<String, f32> = HashMap::new();
        best_min_strength.insert(from.to_string(), f32::INFINITY);

        let mut parent: HashMap<String, String> = HashMap::new();

        let mut queue: VecDeque<(String, f32)> = VecDeque::new();
        queue.push_back((from.to_string(), f32::INFINITY));

        while let Some((current, path_min)) = queue.pop_front() {
            if path_min
                < best_min_strength
                    .get(&current)
                    .copied()
                    .unwrap_or(f32::NEG_INFINITY)
            {
                continue;
            }

            for edge in &self.edges {
                let (neighbor_id, strength) = if edge.from == current {
                    (&edge.to, edge.strength)
                } else if edge.to == current {
                    (&edge.from, edge.strength)
                } else {
                    continue;
                };

                let new_min = path_min.min(strength);
                let prev_best = best_min_strength
                    .get(neighbor_id.as_str())
                    .copied()
                    .unwrap_or(f32::NEG_INFINITY);

                if new_min > prev_best {
                    best_min_strength.insert(neighbor_id.clone(), new_min);
                    parent.insert(neighbor_id.clone(), current.clone());
                    queue.push_back((neighbor_id.clone(), new_min));
                }
            }
        }

        if !parent.contains_key(to) {
            return None;
        }

        let mut path = vec![to.to_string()];
        let mut current = to.to_string();
        while current != from {
            let prev = parent.get(&current)?.clone();
            path.push(prev.clone());
            current = prev;
        }
        path.reverse();
        Some(path)
    }

    pub fn prune_weakest(&mut self, keep: usize) {
        if self.nodes.len() <= keep {
            return;
        }

        let mut entries: Vec<(String, u64)> = self
            .nodes
            .iter()
            .map(|(id, node)| (id.clone(), node.access_count))
            .collect();
        entries.sort_by(|a, b| a.1.cmp(&b.1));

        let remove_count = self.nodes.len() - keep;
        let to_remove: HashSet<String> = entries
            .into_iter()
            .take(remove_count)
            .map(|(id, _)| id)
            .collect();

        for id in &to_remove {
            self.nodes.remove(id);
        }
        self.edges
            .retain(|e| !to_remove.contains(&e.from) && !to_remove.contains(&e.to));
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    pub fn top_activated(&self, top_n: usize) -> Vec<&CosmicNode> {
        let mut ranked: Vec<&CosmicNode> =
            self.nodes.values().filter(|n| n.activation > 0.0).collect();
        ranked.sort_by(|a, b| {
            b.activation
                .partial_cmp(&a.activation)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        ranked.truncate(top_n);
        ranked
    }

    pub fn full_snapshot(&self) -> serde_json::Value {
        let nodes: Vec<&CosmicNode> = self.nodes.values().collect();
        serde_json::json!({
            "max_nodes": self.max_nodes,
            "nodes": nodes,
            "edges": self.edges,
        })
    }

    pub fn restore(data: &serde_json::Value) -> Option<Self> {
        #[allow(clippy::cast_possible_truncation)]
        let max_nodes = data.get("max_nodes")?.as_u64()? as usize;
        let nodes_vec: Vec<CosmicNode> =
            serde_json::from_value(data.get("nodes")?.clone()).ok()?;
        let edges: Vec<CosmicEdge> =
            serde_json::from_value(data.get("edges")?.clone()).ok()?;
        let mut nodes = HashMap::new();
        for n in nodes_vec {
            nodes.insert(n.id.clone(), n);
        }
        Some(Self {
            nodes,
            edges,
            max_nodes,
        })
    }

    pub fn hub_nodes(&self, top_n: usize) -> Vec<&CosmicNode> {
        let mut degree: HashMap<&str, usize> = HashMap::new();
        for edge in &self.edges {
            *degree.entry(edge.from.as_str()).or_default() += 1;
            *degree.entry(edge.to.as_str()).or_default() += 1;
        }

        let mut ranked: Vec<(&str, usize)> = degree.into_iter().collect();
        ranked.sort_by(|a, b| b.1.cmp(&a.1));

        ranked
            .into_iter()
            .take(top_n)
            .filter_map(|(id, _)| self.nodes.get(id))
            .collect()
    }
}

pub fn spreading_activation(
    graph: &mut CosmicMemoryGraph,
    seed_ids: &[String],
    initial_energy: f32,
    decay: f32,
    max_hops: u32,
) {
    for node in graph.nodes.values_mut() {
        node.activation = 0.0;
    }

    let mut queue: VecDeque<(String, f32, u32)> = VecDeque::new();
    for seed in seed_ids {
        if let Some(node) = graph.nodes.get_mut(seed) {
            node.activation = initial_energy;
            node.access_count += 1;
            queue.push_back((seed.clone(), initial_energy, 0));
        }
    }

    let mut visited: HashSet<String> = seed_ids.iter().cloned().collect();

    while let Some((current_id, energy, hop)) = queue.pop_front() {
        if hop >= max_hops {
            continue;
        }

        let neighbor_ids: Vec<(String, f32)> = graph
            .edges
            .iter()
            .filter_map(|edge| {
                if edge.from == current_id {
                    Some((edge.to.clone(), edge.strength))
                } else if edge.to == current_id {
                    Some((edge.from.clone(), edge.strength))
                } else {
                    None
                }
            })
            .collect();

        for (neighbor_id, strength) in neighbor_ids {
            let spread = energy * decay * strength;
            if spread < 0.001 {
                continue;
            }

            if let Some(node) = graph.nodes.get_mut(&neighbor_id) {
                node.activation += spread;
                node.access_count += 1;

                if visited.insert(neighbor_id.clone()) {
                    queue.push_back((neighbor_id, spread, hop + 1));
                }
            }
        }
    }
}
