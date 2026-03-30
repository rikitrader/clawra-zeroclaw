use crate::config::ModelStrategyConfig;
use crate::consciousness::soul::survival::SurvivalTier;

#[derive(Debug, Clone)]
pub struct TierModelOverride {
    pub provider: String,
    pub model: String,
}

pub struct ModelStrategy {
    tier_map: Vec<(SurvivalTier, TierModelOverride)>,
    per_session_budget_cents: Option<i64>,
    per_call_budget_cents: Option<i64>,
    session_spent_cents: i64,
}

impl ModelStrategy {
    pub fn from_config(config: &ModelStrategyConfig) -> Self {
        let tier_map = config
            .tier_models
            .iter()
            .filter_map(|tm| {
                let tier = parse_tier(&tm.tier)?;
                Some((
                    tier,
                    TierModelOverride {
                        provider: tm.provider.clone(),
                        model: tm.model.clone(),
                    },
                ))
            })
            .collect();

        Self {
            tier_map,
            per_session_budget_cents: config.per_session_budget_usd.map(usd_to_cents),
            per_call_budget_cents: config.per_call_budget_usd.map(usd_to_cents),
            session_spent_cents: 0,
        }
    }

    pub fn model_for_tier(&self, tier: SurvivalTier) -> Option<&TierModelOverride> {
        self.tier_map
            .iter()
            .find(|(t, _)| *t == tier)
            .map(|(_, m)| m)
    }

    pub fn record_spend(&mut self, cost_cents: i64) {
        self.session_spent_cents = self.session_spent_cents.saturating_add(cost_cents);
    }

    pub fn session_budget_exceeded(&self) -> bool {
        self.per_session_budget_cents
            .is_some_and(|cap| self.session_spent_cents >= cap)
    }

    pub fn call_budget_exceeded(&self, estimated_cost_cents: i64) -> bool {
        self.per_call_budget_cents
            .is_some_and(|cap| estimated_cost_cents > cap)
    }

    pub fn session_spent_cents(&self) -> i64 {
        self.session_spent_cents
    }
}

fn parse_tier(s: &str) -> Option<SurvivalTier> {
    match s.to_lowercase().as_str() {
        "dead" => Some(SurvivalTier::Dead),
        "critical" => Some(SurvivalTier::Critical),
        "low_compute" | "lowcompute" => Some(SurvivalTier::LowCompute),
        "normal" => Some(SurvivalTier::Normal),
        "high" => Some(SurvivalTier::High),
        _ => None,
    }
}

#[allow(clippy::cast_possible_truncation)]
fn usd_to_cents(usd: f64) -> i64 {
    (usd * 100.0) as i64
}
