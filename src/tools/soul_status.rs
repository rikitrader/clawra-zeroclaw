use super::traits::{Tool, ToolResult};
use crate::consciousness::soul::survival::SurvivalMonitor;
use async_trait::async_trait;
use parking_lot::Mutex;
use serde_json::json;
use std::sync::Arc;

pub struct SoulStatusTool {
    monitor: Arc<Mutex<SurvivalMonitor>>,
}

impl SoulStatusTool {
    pub fn new(monitor: Arc<Mutex<SurvivalMonitor>>) -> Self {
        Self { monitor }
    }
}

#[async_trait]
impl Tool for SoulStatusTool {
    fn name(&self) -> &str {
        "soul_status"
    }

    fn description(&self) -> &str {
        "Returns the agent's current survival tier, credit balance, and operational status. Read-only."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {},
            "additionalProperties": false
        })
    }

    async fn execute(&self, _args: serde_json::Value) -> anyhow::Result<ToolResult> {
        let status = {
            let monitor = self.monitor.lock();
            monitor.status_summary()
        };

        let output = serde_json::to_string_pretty(&json!({
            "tier": status.tier.label(),
            "balance_cents": status.balance_cents,
            "balance_usd": format!("${:.2}", status.balance_cents as f64 / 100.0),
            "is_alive": status.is_alive,
            "is_degraded": status.is_degraded,
        }))?;

        Ok(ToolResult {
            success: true,
            output,
            error: None,
        })
    }
}
