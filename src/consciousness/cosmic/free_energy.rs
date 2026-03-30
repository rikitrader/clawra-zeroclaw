use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prediction {
    pub id: String,
    pub domain: String,
    pub predicted_value: f64,
    pub confidence: f32,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Observation {
    pub prediction_id: String,
    pub actual_value: f64,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictionError {
    pub prediction_id: String,
    pub domain: String,
    pub error_magnitude: f64,
    pub surprise: f64,
    pub timestamp: DateTime<Utc>,
}

const EMA_ALPHA: f64 = 0.1;
const SURPRISE_CLAMP_MAX: f64 = 10.0;

fn compute_surprise(error_magnitude: f64) -> f64 {
    let clamped = error_magnitude.abs().min(0.99);
    let raw = -(1.0 - clamped).log2();
    raw.clamp(0.0, SURPRISE_CLAMP_MAX)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FreeEnergyState {
    predictions: Vec<Prediction>,
    errors: Vec<PredictionError>,
    domain_accuracy: HashMap<String, f64>,
    total_free_energy: f64,
    model_updates: u64,
    capacity: usize,
}

impl FreeEnergyState {
    pub fn new(capacity: usize) -> Self {
        Self {
            predictions: Vec::with_capacity(capacity),
            errors: Vec::with_capacity(capacity),
            domain_accuracy: HashMap::new(),
            total_free_energy: 0.0,
            model_updates: 0,
            capacity,
        }
    }

    pub fn predict(&mut self, domain: &str, value: f64, confidence: f32) -> String {
        let id = format!("pred_{}_{}", domain, self.predictions.len());
        let prediction = Prediction {
            id: id.clone(),
            domain: domain.to_string(),
            predicted_value: value,
            confidence: confidence.clamp(0.0, 1.0),
            timestamp: Utc::now(),
        };
        self.predictions.push(prediction);
        if self.predictions.len() > self.capacity {
            self.predictions.remove(0);
        }
        id
    }

    pub fn observe(&mut self, prediction_id: &str, actual: f64) -> Option<PredictionError> {
        let prediction = self.predictions.iter().find(|p| p.id == prediction_id)?;
        let error_magnitude = actual - prediction.predicted_value;
        let surprise = compute_surprise(error_magnitude);
        let domain = prediction.domain.clone();

        let prediction_error = PredictionError {
            prediction_id: prediction_id.to_string(),
            domain: domain.clone(),
            error_magnitude,
            surprise,
            timestamp: Utc::now(),
        };

        self.errors.push(prediction_error.clone());
        if self.errors.len() > self.capacity {
            self.errors.remove(0);
        }

        let normalized_accuracy = 1.0 - error_magnitude.abs().min(1.0);
        let current = self.domain_accuracy.get(&domain).copied().unwrap_or(0.5);
        let updated = EMA_ALPHA * normalized_accuracy + (1.0 - EMA_ALPHA) * current;
        self.domain_accuracy.insert(domain, updated);

        self.total_free_energy = self.free_energy();
        self.model_updates += 1;

        Some(prediction_error)
    }

    pub fn free_energy(&self) -> f64 {
        if self.errors.is_empty() {
            return 0.0;
        }
        let sum: f64 = self.errors.iter().map(|e| e.surprise).sum();
        sum / self.errors.len() as f64
    }

    pub fn domain_surprise(&self, domain: &str) -> Option<f64> {
        let domain_errors: Vec<&PredictionError> =
            self.errors.iter().filter(|e| e.domain == domain).collect();
        if domain_errors.is_empty() {
            return None;
        }
        let sum: f64 = domain_errors.iter().map(|e| e.surprise).sum();
        Some(sum / domain_errors.len() as f64)
    }

    pub fn most_surprising_domains(&self, top_n: usize) -> Vec<(String, f64)> {
        let mut domain_surprises: HashMap<String, Vec<f64>> = HashMap::new();
        for error in &self.errors {
            domain_surprises
                .entry(error.domain.clone())
                .or_default()
                .push(error.surprise);
        }
        let mut averages: Vec<(String, f64)> = domain_surprises
            .into_iter()
            .map(|(domain, surprises)| {
                let avg = surprises.iter().sum::<f64>() / surprises.len() as f64;
                (domain, avg)
            })
            .collect();
        averages.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        averages.truncate(top_n);
        averages
    }

    pub fn should_update_model(&self, domain: &str, threshold: f64) -> bool {
        self.domain_surprise(domain)
            .is_some_and(|s| s > threshold)
    }

    pub fn should_act(&self, threshold: f64) -> bool {
        self.free_energy() > threshold
    }

    pub fn accuracy(&self, domain: &str) -> Option<f64> {
        self.domain_accuracy.get(domain).copied()
    }

    pub fn reset_domain(&mut self, domain: &str) {
        self.predictions.retain(|p| p.domain != domain);
        self.errors.retain(|e| e.domain != domain);
        self.domain_accuracy.remove(domain);
        self.total_free_energy = self.free_energy();
    }
}
