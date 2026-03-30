use super::traits::{Tool, ToolResult};
use crate::consciousness::soul::model::SoulModel;
use crate::consciousness::soul::reflection::{apply_insights, write_soul_file, ReflectionInsights};
use async_trait::async_trait;
use parking_lot::Mutex;
use serde_json::json;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

pub struct SoulReflectTool {
    soul_path: PathBuf,
    soul: Arc<Mutex<SoulModel>>,
}

impl SoulReflectTool {
    pub fn new(soul_path: PathBuf, soul: Arc<Mutex<SoulModel>>) -> Self {
        Self { soul_path, soul }
    }
}

#[async_trait]
impl Tool for SoulReflectTool {
    fn name(&self) -> &str {
        "soul_reflect"
    }

    fn description(&self) -> &str {
        "Updates the agent's soul identity with new insights discovered during conversation. \
         Pass arrays of new capabilities, and objects of new relationships, personality traits, \
         or financial character traits. Changes are persisted to SOUL.md."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "capabilities": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "New capabilities discovered (e.g. ['web_scraping', 'data_analysis'])"
                },
                "relationships": {
                    "type": "object",
                    "additionalProperties": { "type": "string" },
                    "description": "New or updated relationships (e.g. {'collaborator': 'agent_b'})"
                },
                "personality": {
                    "type": "object",
                    "additionalProperties": { "type": "string" },
                    "description": "Updated personality traits (e.g. {'patience': 'high'})"
                },
                "financial_character": {
                    "type": "object",
                    "additionalProperties": { "type": "string" },
                    "description": "Updated financial traits (e.g. {'risk_tolerance': 'moderate'})"
                }
            },
            "additionalProperties": false
        })
    }

    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        let capabilities: Vec<String> = args
            .get("capabilities")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();

        let relationships: HashMap<String, String> = args
            .get("relationships")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();

        let personality: HashMap<String, String> = args
            .get("personality")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();

        let financial: HashMap<String, String> = args
            .get("financial_character")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();

        let insights = ReflectionInsights {
            new_capabilities: capabilities,
            new_relationships: relationships,
            personality_updates: personality,
            financial_updates: financial,
        };

        if insights.is_empty() {
            return Ok(ToolResult {
                success: true,
                output: "No insights provided — soul unchanged.".into(),
                error: None,
            });
        }

        {
            let mut soul = self.soul.lock();
            apply_insights(&mut soul, &insights);

            write_soul_file(&self.soul_path, &soul)?;
        }

        let output = serde_json::to_string_pretty(&json!({
            "updated": true,
            "insights_applied": insights.count(),
            "soul_path": self.soul_path.to_string_lossy(),
        }))?;

        Ok(ToolResult {
            success: true,
            output,
            error: None,
        })
    }
}
