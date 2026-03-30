use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlignmentScore {
    pub jaccard: f64,
    pub recall: f64,
    pub combined: f64,
}

impl AlignmentScore {
    pub fn compute(genesis: &str, current: &str) -> Self {
        let genesis_words = tokenize(genesis);
        let current_words = tokenize(current);

        if genesis_words.is_empty() && current_words.is_empty() {
            return Self {
                jaccard: 1.0,
                recall: 1.0,
                combined: 1.0,
            };
        }

        if genesis_words.is_empty() || current_words.is_empty() {
            return Self {
                jaccard: 0.0,
                recall: 0.0,
                combined: 0.0,
            };
        }

        let intersection = genesis_words.intersection(&current_words).count() as f64;
        let union = genesis_words.union(&current_words).count() as f64;

        let jaccard = intersection / union;
        let recall = intersection / genesis_words.len() as f64;
        let combined = (jaccard + recall) / 2.0;

        Self {
            jaccard,
            recall,
            combined,
        }
    }

    pub fn is_aligned(&self, threshold: f64) -> bool {
        self.combined >= threshold
    }
}

fn tokenize(text: &str) -> HashSet<String> {
    text.split_whitespace()
        .map(|w| {
            w.to_lowercase()
                .chars()
                .filter(|c| c.is_alphanumeric())
                .collect::<String>()
        })
        .filter(|w| !w.is_empty())
        .collect()
}
