use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub id: String,
    pub content: String,
    pub category: String,
    pub importance: f64,
    pub access_count: u32,
    pub created_at: DateTime<Utc>,
    pub last_accessed: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryPattern {
    pub id: String,
    pub description: String,
    pub frequency: u32,
    pub source_ids: Vec<String>,
    pub confidence: f64,
    pub discovered_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsolidationResult {
    pub merged_count: usize,
    pub patterns_found: usize,
    pub pruned_count: usize,
    pub total_remaining: usize,
    pub completed_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct ConsolidationEngine {
    entries: Vec<MemoryEntry>,
    patterns: Vec<MemoryPattern>,
    similarity_threshold: f64,
}

impl ConsolidationEngine {
    pub fn new(similarity_threshold: f64) -> Self {
        Self {
            entries: Vec::new(),
            patterns: Vec::new(),
            similarity_threshold,
        }
    }

    pub fn add_entry(&mut self, entry: MemoryEntry) {
        self.entries.push(entry);
    }

    pub fn consolidate(&mut self) -> ConsolidationResult {
        let merged_count = self.merge_similar();
        let patterns = self.extract_patterns();
        let patterns_found = patterns.len();
        let pruned_count = self.prune_redundant(0.3);

        ConsolidationResult {
            merged_count,
            patterns_found,
            pruned_count,
            total_remaining: self.entries.len(),
            completed_at: Utc::now(),
        }
    }

    pub fn merge_similar(&mut self) -> usize {
        let mut merged_count = 0usize;
        let mut to_remove: HashSet<usize> = HashSet::new();

        for i in 0..self.entries.len() {
            if to_remove.contains(&i) {
                continue;
            }
            for j in (i + 1)..self.entries.len() {
                if to_remove.contains(&j) {
                    continue;
                }
                let sim = jaccard_similarity(&self.entries[i].content, &self.entries[j].content);
                if sim >= self.similarity_threshold {
                    let combined_access =
                        self.entries[i].access_count + self.entries[j].access_count;
                    let higher_importance =
                        self.entries[i].importance.max(self.entries[j].importance);
                    self.entries[i].access_count = combined_access;
                    self.entries[i].importance = higher_importance;
                    to_remove.insert(j);
                    merged_count += 1;
                }
            }
        }

        let mut idx = 0;
        self.entries.retain(|_| {
            let keep = !to_remove.contains(&idx);
            idx += 1;
            keep
        });

        merged_count
    }

    pub fn extract_patterns(&mut self) -> Vec<MemoryPattern> {
        let mut category_keyword_map: HashMap<(String, String), Vec<String>> = HashMap::new();

        for entry in &self.entries {
            let words: Vec<&str> = entry.content.split_whitespace().collect();
            for word in words {
                let key = (entry.category.clone(), word.to_lowercase());
                category_keyword_map
                    .entry(key)
                    .or_default()
                    .push(entry.id.clone());
            }
        }

        let mut new_patterns = Vec::new();
        for ((category, keyword), source_ids) in &category_keyword_map {
            if source_ids.len() >= 3 {
                let pattern = MemoryPattern {
                    id: format!("pat_{}_{}", category, keyword),
                    description: format!("{category}:{keyword}"),
                    #[allow(clippy::cast_possible_truncation)]
                    frequency: source_ids.len() as u32,
                    source_ids: source_ids.clone(),
                    confidence: (source_ids.len() as f64 / self.entries.len().max(1) as f64)
                        .min(1.0),
                    discovered_at: Utc::now(),
                };
                new_patterns.push(pattern);
            }
        }

        self.patterns.extend(new_patterns.clone());
        new_patterns
    }

    pub fn prune_redundant(&mut self, min_importance: f64) -> usize {
        let before = self.entries.len();
        self.entries
            .retain(|e| e.importance >= min_importance || e.access_count >= 2);
        before - self.entries.len()
    }

    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }

    pub fn pattern_count(&self) -> usize {
        self.patterns.len()
    }

    pub fn entries_by_category(&self, category: &str) -> Vec<&MemoryEntry> {
        self.entries
            .iter()
            .filter(|e| e.category == category)
            .collect()
    }

    pub fn snapshot(&self) -> serde_json::Value {
        serde_json::json!({
            "entry_count": self.entries.len(),
            "pattern_count": self.patterns.len(),
            "similarity_threshold": self.similarity_threshold,
        })
    }

    pub fn full_snapshot(&self) -> serde_json::Value {
        serde_json::json!({
            "similarity_threshold": self.similarity_threshold,
            "entries": self.entries,
            "patterns": self.patterns,
        })
    }

    pub fn restore(data: &serde_json::Value) -> Option<Self> {
        let threshold = data.get("similarity_threshold")?.as_f64()?;
        let entries: Vec<MemoryEntry> =
            serde_json::from_value(data.get("entries")?.clone()).ok()?;
        let patterns: Vec<MemoryPattern> =
            serde_json::from_value(data.get("patterns")?.clone()).ok()?;
        Some(Self {
            entries,
            patterns,
            similarity_threshold: threshold,
        })
    }

    pub fn top_patterns(&self, n: usize) -> Vec<&MemoryPattern> {
        let mut sorted: Vec<&MemoryPattern> = self.patterns.iter().collect();
        sorted.sort_by(|a, b| {
            b.frequency.cmp(&a.frequency).then_with(|| {
                b.confidence
                    .partial_cmp(&a.confidence)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
        });
        sorted.truncate(n);
        sorted
    }
}

fn jaccard_similarity(a: &str, b: &str) -> f64 {
    let set_a: HashSet<&str> = a.split_whitespace().collect();
    let set_b: HashSet<&str> = b.split_whitespace().collect();
    let intersection = set_a.intersection(&set_b).count();
    let union = set_a.union(&set_b).count();
    if union == 0 {
        return 0.0;
    }
    intersection as f64 / union as f64
}
