use std::collections::{HashMap, VecDeque};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SignalSource {
    Channel,
    Tool,
    Peripheral,
    Internal,
    System,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputSignal {
    pub source: SignalSource,
    pub content: String,
    pub raw_salience: f64,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SalienceScore {
    pub signal_id: usize,
    pub score: f64,
    pub novelty: f64,
    pub urgency: f64,
    pub relevance: f64,
    pub compressed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThalamusSnapshot {
    pub signals_processed: usize,
    pub signals_passed: usize,
    pub signals_filtered: usize,
    pub avg_salience: f64,
    pub attention_threshold: f64,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct SensoryThalamus {
    attention_threshold: f64,
    habituation_map: HashMap<String, usize>,
    habituation_decay: f64,
    recent_signals: VecDeque<SalienceScore>,
    capacity: usize,
    signals_processed: usize,
    signals_passed: usize,
}

impl SensoryThalamus {
    pub fn new(attention_threshold: f64, capacity: usize) -> Self {
        Self {
            attention_threshold,
            habituation_map: HashMap::new(),
            habituation_decay: 0.3,
            recent_signals: VecDeque::new(),
            capacity,
            signals_processed: 0,
            signals_passed: 0,
        }
    }

    pub fn process_signal(&mut self, signal: &InputSignal) -> Option<SalienceScore> {
        self.signals_processed += 1;
        self.habituate(&signal.content.clone());
        let scored = self.score_salience(signal);
        if scored.score >= self.attention_threshold {
            self.signals_passed += 1;
            while self.recent_signals.len() >= self.capacity {
                self.recent_signals.pop_front();
            }
            self.recent_signals.push_back(scored.clone());
            Some(scored)
        } else {
            None
        }
    }

    pub fn score_salience(&self, signal: &InputSignal) -> SalienceScore {
        let novelty = self.novelty_score(&signal.content);
        let urgency = match signal.source {
            SignalSource::Internal => (signal.raw_salience * 0.5 + 0.2).min(1.0),
            SignalSource::System => signal.raw_salience * 0.8,
            _ => signal.raw_salience * 0.5,
        };
        let relevance = signal.raw_salience * 0.6;
        let score = signal.raw_salience * 0.3 + novelty * 0.3 + urgency * 0.2 + relevance * 0.2;
        let score = score.clamp(0.0, 1.0);

        SalienceScore {
            signal_id: self.signals_processed,
            score,
            novelty,
            urgency,
            relevance,
            compressed: signal.content.len() > 500,
        }
    }

    pub fn adjust_threshold(&mut self, arousal: f64) {
        let arousal = arousal.clamp(0.0, 1.0);
        self.attention_threshold = 0.5 - arousal * 0.35;
    }

    pub fn habituate(&mut self, pattern: &str) {
        *self.habituation_map.entry(pattern.to_string()).or_insert(0) += 1;
    }

    pub fn decay_habituation(&mut self) {
        self.habituation_map.retain(|_, count| {
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let decayed = (*count as f64 * (1.0 - self.habituation_decay))
                .round()
                .max(0.0) as usize;
            *count = decayed;
            decayed > 0
        });
    }

    pub fn novelty_score(&self, content: &str) -> f64 {
        let count = self.habituation_map.get(content).copied().unwrap_or(0);
        1.0 / (1.0 + count as f64)
    }

    pub fn snapshot(&self) -> ThalamusSnapshot {
        let avg_salience = if self.recent_signals.is_empty() {
            0.0
        } else {
            self.recent_signals.iter().map(|s| s.score).sum::<f64>()
                / self.recent_signals.len() as f64
        };
        ThalamusSnapshot {
            signals_processed: self.signals_processed,
            signals_passed: self.signals_passed,
            signals_filtered: self.signals_processed - self.signals_passed,
            avg_salience,
            attention_threshold: self.attention_threshold,
            timestamp: Utc::now(),
        }
    }

    pub fn filter_rate(&self) -> f64 {
        if self.signals_processed == 0 {
            return 0.0;
        }
        (self.signals_processed - self.signals_passed) as f64 / self.signals_processed as f64
    }

    pub fn is_saturated(&self) -> bool {
        if self.signals_processed <= 100 {
            return false;
        }
        let pass_rate = self.signals_passed as f64 / self.signals_processed as f64;
        pass_rate > 0.8
    }

    pub fn full_snapshot(&self) -> serde_json::Value {
        serde_json::json!({
            "attention_threshold": self.attention_threshold,
            "habituation_decay": self.habituation_decay,
            "capacity": self.capacity,
            "signals_processed": self.signals_processed,
            "signals_passed": self.signals_passed,
            "habituation_map": self.habituation_map,
        })
    }

    pub fn restore(data: &serde_json::Value) -> Option<Self> {
        let attention_threshold = data.get("attention_threshold")?.as_f64()?;
        let habituation_decay = data.get("habituation_decay")?.as_f64()?;
        #[allow(clippy::cast_possible_truncation)]
        let capacity = data.get("capacity")?.as_u64()? as usize;
        #[allow(clippy::cast_possible_truncation)]
        let signals_processed = data.get("signals_processed")?.as_u64()? as usize;
        #[allow(clippy::cast_possible_truncation)]
        let signals_passed = data.get("signals_passed")?.as_u64()? as usize;
        let habituation_map: HashMap<String, usize> = serde_json::from_value(
            data.get("habituation_map").cloned().unwrap_or_default(),
        )
        .ok()?;
        Some(Self {
            attention_threshold,
            habituation_decay,
            capacity,
            signals_processed,
            signals_passed,
            habituation_map,
            recent_signals: VecDeque::new(),
        })
    }
}
