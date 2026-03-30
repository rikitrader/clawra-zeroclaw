use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scenario {
    pub id: String,
    pub action: String,
    pub context: HashMap<String, f64>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationResult {
    pub scenario_id: String,
    pub predicted_outcome: f64,
    pub confidence: f64,
    pub risk: f64,
    pub affected_subsystems: Vec<String>,
    pub simulated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct CounterfactualEngine {
    world_beliefs: HashMap<String, f64>,
    simulations: Vec<SimulationResult>,
    max_scenarios: usize,
    history_capacity: usize,
}

impl CounterfactualEngine {
    pub fn new(max_scenarios: usize, history_capacity: usize) -> Self {
        Self {
            world_beliefs: HashMap::new(),
            simulations: Vec::new(),
            max_scenarios,
            history_capacity,
        }
    }

    pub fn update_world_state(&mut self, key: &str, value: f64) {
        self.world_beliefs.insert(key.to_string(), value);
    }

    pub fn simulate(&mut self, scenario: &Scenario) -> SimulationResult {
        let affected_subsystems: Vec<String> = scenario
            .context
            .keys()
            .filter(|k| self.world_beliefs.contains_key(*k))
            .cloned()
            .collect();

        let overlap = affected_subsystems.len();
        let total_context = scenario.context.len().max(1);
        let confidence = overlap as f64 / total_context as f64;

        let mut divergence_sum = 0.0;
        let mut divergence_count = 0usize;
        for (key, ctx_val) in &scenario.context {
            if let Some(world_val) = self.world_beliefs.get(key) {
                divergence_sum += (ctx_val - world_val).abs();
                divergence_count += 1;
            }
        }
        let risk = if divergence_count > 0 {
            (divergence_sum / divergence_count as f64).clamp(0.0, 1.0)
        } else {
            0.5
        };

        let mut alignment_sum = 0.0;
        let mut alignment_count = 0usize;
        for (key, ctx_val) in &scenario.context {
            if let Some(world_val) = self.world_beliefs.get(key) {
                alignment_sum += 1.0 - (ctx_val - world_val).abs().min(1.0);
                alignment_count += 1;
            }
        }
        let predicted_outcome = if alignment_count > 0 {
            alignment_sum / alignment_count as f64
        } else {
            0.5
        };

        let result = SimulationResult {
            scenario_id: scenario.id.clone(),
            predicted_outcome,
            confidence,
            risk,
            affected_subsystems,
            simulated_at: Utc::now(),
        };

        self.simulations.push(result.clone());
        if self.simulations.len() > self.history_capacity {
            self.simulations.remove(0);
        }

        result
    }

    pub fn compare_scenarios(&mut self, scenarios: &[Scenario]) -> Vec<SimulationResult> {
        let mut results: Vec<SimulationResult> =
            scenarios.iter().map(|s| self.simulate(s)).collect();
        results.sort_by(|a, b| {
            b.predicted_outcome
                .partial_cmp(&a.predicted_outcome)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        results
    }

    pub fn best_action(&mut self, scenarios: &[Scenario]) -> Option<SimulationResult> {
        let results = self.compare_scenarios(scenarios);
        results.into_iter().find(|r| r.risk < 0.7)
    }

    pub fn regret(&self, scenario_id: &str, actual_outcome: f64) -> Option<f64> {
        self.simulations
            .iter()
            .find(|s| s.scenario_id == scenario_id)
            .map(|s| (s.predicted_outcome - actual_outcome).abs())
    }

    pub fn simulation_count(&self) -> usize {
        self.simulations.len()
    }

    pub fn clear_history(&mut self) {
        self.simulations.clear();
    }
}
