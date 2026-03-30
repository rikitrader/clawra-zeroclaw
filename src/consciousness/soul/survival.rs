use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum SurvivalTier {
    Dead,
    Critical,
    LowCompute,
    Normal,
    High,
}

impl SurvivalTier {
    pub fn from_balance(balance_cents: i64) -> Self {
        match balance_cents {
            b if b < 0 => Self::Dead,
            b if b < 10 => Self::Critical,
            b if b < 50 => Self::LowCompute,
            b if b < 500 => Self::Normal,
            _ => Self::High,
        }
    }

    pub fn is_degraded(self) -> bool {
        matches!(self, Self::Dead | Self::Critical | Self::LowCompute)
    }

    pub fn is_alive(self) -> bool {
        !matches!(self, Self::Dead)
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Dead => "DEAD",
            Self::Critical => "CRITICAL",
            Self::LowCompute => "LOW_COMPUTE",
            Self::Normal => "NORMAL",
            Self::High => "HIGH",
        }
    }
}

impl std::fmt::Display for SurvivalTier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurvivalThresholds {
    pub high: i64,
    pub normal: i64,
    pub low_compute: i64,
    pub critical: i64,
}

impl Default for SurvivalThresholds {
    fn default() -> Self {
        Self {
            high: 500,
            normal: 50,
            low_compute: 10,
            critical: 0,
        }
    }
}

impl SurvivalThresholds {
    pub fn tier_for_balance(&self, balance_cents: i64) -> SurvivalTier {
        if balance_cents < self.critical {
            SurvivalTier::Dead
        } else if balance_cents < self.low_compute {
            SurvivalTier::Critical
        } else if balance_cents < self.normal {
            SurvivalTier::LowCompute
        } else if balance_cents < self.high {
            SurvivalTier::Normal
        } else {
            SurvivalTier::High
        }
    }
}

pub struct SurvivalMonitor {
    current_tier: SurvivalTier,
    credit_balance_cents: i64,
    thresholds: SurvivalThresholds,
}

impl SurvivalMonitor {
    pub fn new(initial_balance_cents: i64, thresholds: SurvivalThresholds) -> Self {
        let current_tier = thresholds.tier_for_balance(initial_balance_cents);
        Self {
            current_tier,
            credit_balance_cents: initial_balance_cents,
            thresholds,
        }
    }

    pub fn tier(&self) -> SurvivalTier {
        self.current_tier
    }

    pub fn balance_cents(&self) -> i64 {
        self.credit_balance_cents
    }

    pub fn deduct(&mut self, amount_cents: i64) -> Option<(SurvivalTier, SurvivalTier)> {
        self.credit_balance_cents = self.credit_balance_cents.saturating_sub(amount_cents);
        self.check_tier_transition()
    }

    pub fn add_credits(&mut self, amount_cents: i64) -> Option<(SurvivalTier, SurvivalTier)> {
        self.credit_balance_cents = self.credit_balance_cents.saturating_add(amount_cents);
        self.check_tier_transition()
    }

    pub fn set_balance(&mut self, balance_cents: i64) -> Option<(SurvivalTier, SurvivalTier)> {
        self.credit_balance_cents = balance_cents;
        self.check_tier_transition()
    }

    pub fn status_summary(&self) -> SurvivalStatus {
        SurvivalStatus {
            tier: self.current_tier,
            balance_cents: self.credit_balance_cents,
            is_alive: self.current_tier.is_alive(),
            is_degraded: self.current_tier.is_degraded(),
        }
    }

    fn check_tier_transition(&mut self) -> Option<(SurvivalTier, SurvivalTier)> {
        let new_tier = self.thresholds.tier_for_balance(self.credit_balance_cents);
        if new_tier == self.current_tier {
            None
        } else {
            let old = self.current_tier;
            self.current_tier = new_tier;
            Some((old, new_tier))
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurvivalStatus {
    pub tier: SurvivalTier,
    pub balance_cents: i64,
    pub is_alive: bool,
    pub is_degraded: bool,
}
