use std::collections::{HashMap, VecDeque};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriftSample {
    pub subsystem: String,
    pub value: f64,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriftAlert {
    pub subsystem: String,
    pub drift_magnitude: f64,
    pub direction: f64,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriftReport {
    pub alerts: Vec<DriftAlert>,
    pub max_drift: f64,
    pub drifting_count: usize,
    pub total_subsystems: usize,
    pub generated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct DriftDetector {
    samples: HashMap<String, VecDeque<DriftSample>>,
    window_size: usize,
    threshold: f64,
}

impl DriftDetector {
    pub fn new(window_size: usize, threshold: f64) -> Self {
        Self {
            samples: HashMap::new(),
            window_size,
            threshold,
        }
    }

    pub fn record_sample(&mut self, subsystem: &str, value: f64) {
        let window = self.samples.entry(subsystem.to_string()).or_default();
        window.push_back(DriftSample {
            subsystem: subsystem.to_string(),
            value,
            timestamp: Utc::now(),
        });
        while window.len() > self.window_size {
            window.pop_front();
        }
    }

    pub fn detect_drift(&self, subsystem: &str) -> Option<DriftAlert> {
        let window = self.samples.get(subsystem)?;
        if window.len() < 2 {
            return None;
        }
        let mid = window.len() / 2;
        let first_half: f64 = window.iter().take(mid).map(|s| s.value).sum::<f64>() / mid as f64;
        let second_half: f64 =
            window.iter().skip(mid).map(|s| s.value).sum::<f64>() / (window.len() - mid) as f64;
        let direction = second_half - first_half;
        let magnitude = direction.abs();
        if magnitude > self.threshold {
            Some(DriftAlert {
                subsystem: subsystem.to_string(),
                drift_magnitude: magnitude,
                direction,
                timestamp: Utc::now(),
            })
        } else {
            None
        }
    }

    pub fn drift_report(&self) -> DriftReport {
        let mut alerts = Vec::new();
        let mut max_drift: f64 = 0.0;
        for subsystem in self.samples.keys() {
            if let Some(alert) = self.detect_drift(subsystem) {
                if alert.drift_magnitude > max_drift {
                    max_drift = alert.drift_magnitude;
                }
                alerts.push(alert);
            }
        }
        DriftReport {
            drifting_count: alerts.len(),
            total_subsystems: self.samples.len(),
            alerts,
            max_drift,
            generated_at: Utc::now(),
        }
    }

    pub fn is_drifting(&self, subsystem: &str) -> bool {
        self.detect_drift(subsystem).is_some()
    }

    pub fn subsystem_count(&self) -> usize {
        self.samples.len()
    }

    pub fn clear_subsystem(&mut self, subsystem: &str) {
        self.samples.remove(subsystem);
    }

    pub fn mean_value(&self, subsystem: &str) -> Option<f64> {
        let window = self.samples.get(subsystem)?;
        if window.is_empty() {
            return None;
        }
        Some(window.iter().map(|s| s.value).sum::<f64>() / window.len() as f64)
    }

    pub fn snapshot(&self) -> serde_json::Value {
        let samples: HashMap<&str, Vec<&DriftSample>> = self
            .samples
            .iter()
            .map(|(k, v)| (k.as_str(), v.iter().collect()))
            .collect();
        serde_json::json!({
            "window_size": self.window_size,
            "threshold": self.threshold,
            "samples": samples,
        })
    }

    pub fn restore(data: &serde_json::Value) -> Option<Self> {
        #[allow(clippy::cast_possible_truncation)]
        let window_size = data.get("window_size")?.as_u64()? as usize;
        let threshold = data.get("threshold")?.as_f64()?;
        let raw: HashMap<String, Vec<DriftSample>> =
            serde_json::from_value(data.get("samples")?.clone()).ok()?;
        let mut samples = HashMap::new();
        for (k, v) in raw {
            let mut deque = VecDeque::with_capacity(window_size);
            for s in v {
                deque.push_back(s);
                if deque.len() > window_size {
                    deque.pop_front();
                }
            }
            samples.insert(k, deque);
        }
        Some(Self {
            samples,
            window_size,
            threshold,
        })
    }
}
