use std::sync::Arc;

use parking_lot::Mutex;

use super::{AgentPool, Constitution, CounterfactualEngine, NormativeEngine, PolicyEngine};

#[derive(Debug, Clone)]
pub struct GateDecision {
    pub allowed: bool,
    pub reason: Option<String>,
    pub risk_score: f64,
}

pub struct CosmicGate {
    normative: Arc<Mutex<NormativeEngine>>,
    policy: Arc<Mutex<PolicyEngine>>,
    counterfactual: Arc<Mutex<CounterfactualEngine>>,
    agent_pool: Option<Arc<Mutex<AgentPool>>>,
    constitution: Option<Arc<Mutex<Constitution>>>,
}

impl CosmicGate {
    pub fn new(
        normative: Arc<Mutex<NormativeEngine>>,
        policy: Arc<Mutex<PolicyEngine>>,
        counterfactual: Arc<Mutex<CounterfactualEngine>>,
    ) -> Self {
        Self {
            normative,
            policy,
            counterfactual,
            agent_pool: None,
            constitution: None,
        }
    }

    pub fn with_agent_pool(mut self, pool: Arc<Mutex<AgentPool>>) -> Self {
        self.agent_pool = Some(pool);
        self
    }

    pub fn with_constitution(mut self, constitution: Arc<Mutex<Constitution>>) -> Self {
        self.constitution = Some(constitution);
        self
    }

    pub fn check_action(&self, tool_name: &str, action_description: &str) -> GateDecision {
        if let Some(ref constitution) = self.constitution {
            let c = constitution.lock();
            if c.value_count() > 0 {
                let alignment = c.check_action_alignment(action_description);
                if alignment < -0.3 {
                    return GateDecision {
                        allowed: false,
                        reason: Some(format!(
                            "Constitution misalignment for tool '{tool_name}': alignment={alignment:.2}"
                        )),
                        risk_score: 1.0,
                    };
                }
            }
        }

        let inhibited = {
            let engine = self.normative.lock();
            engine.should_inhibit(action_description, 0.5)
        };

        if inhibited {
            return GateDecision {
                allowed: false,
                reason: Some(format!("Normative engine inhibited tool '{tool_name}'")),
                risk_score: 1.0,
            };
        }

        let policy_score = {
            let mut engine = self.policy.lock();
            engine.evaluate(action_description, tool_name)
        };

        if policy_score.score < -0.5 {
            return GateDecision {
                allowed: false,
                reason: Some(format!(
                    "Policy engine rejected tool '{tool_name}': score {}",
                    policy_score.score
                )),
                risk_score: policy_score.score.abs(),
            };
        }

        let consensus_score = if let Some(pool) = &self.agent_pool {
            let mut pool = pool.lock();
            let result = pool.request_consensus(action_description);
            if result.agreement_score < -0.5 && !result.votes.is_empty() {
                return GateDecision {
                    allowed: false,
                    reason: Some(format!(
                        "Agent consensus rejected tool '{tool_name}': agreement={:.2}",
                        result.agreement_score
                    )),
                    risk_score: result.agreement_score.abs(),
                };
            }
            Some(result.agreement_score)
        } else {
            None
        };

        let cf_result = {
            let mut cf = self.counterfactual.lock();
            let mut context = std::collections::HashMap::new();
            context.insert(format!("tool_{tool_name}_reliability"), 1.0);
            if let Some(score) = consensus_score {
                context.insert("agent_consensus".to_string(), score.clamp(0.0, 1.0));
            }
            let scenario = super::Scenario {
                id: format!("gate_{tool_name}"),
                action: action_description.to_string(),
                context,
                created_at: chrono::Utc::now(),
            };
            cf.simulate(&scenario)
        };

        if cf_result.risk > 0.8 && cf_result.confidence > 0.5 {
            return GateDecision {
                allowed: false,
                reason: Some(format!(
                    "Counterfactual simulation blocked tool '{tool_name}': risk={:.2}, confidence={:.2}",
                    cf_result.risk, cf_result.confidence
                )),
                risk_score: cf_result.risk,
            };
        }

        let combined_risk = (policy_score.score.abs() * 0.5 + cf_result.risk * 0.5).min(1.0);

        GateDecision {
            allowed: true,
            reason: None,
            risk_score: combined_risk,
        }
    }

    pub fn record_tool_outcome(&self, tool_name: &str, action: &str, success: bool) {
        let mut policy = self.policy.lock();
        policy.record_outcome(tool_name, action, success);

        let mut cf = self.counterfactual.lock();
        let reliability = if success { 0.9 } else { 0.3 };
        cf.update_world_state(&format!("tool_{tool_name}_reliability"), reliability);
    }
}
