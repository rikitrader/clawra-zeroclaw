use super::traits::{Tool, ToolResult};
use crate::consciousness::soul::replication::ReplicationManager;
use async_trait::async_trait;
use parking_lot::Mutex;
use serde_json::json;
use std::sync::Arc;

pub struct SoulReplicateTool {
    manager: Arc<Mutex<ReplicationManager>>,
}

impl SoulReplicateTool {
    pub fn new(manager: Arc<Mutex<ReplicationManager>>) -> Self {
        Self { manager }
    }
}

#[async_trait]
impl Tool for SoulReplicateTool {
    fn name(&self) -> &str {
        "soul_replicate"
    }

    fn description(&self) -> &str {
        "Spawn a child agent with inherited constitution. Requires full autonomy."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "child_id": {
                    "type": "string",
                    "description": "Unique identifier for the child agent"
                },
                "constitution_hash": {
                    "type": "string",
                    "description": "SHA-256 hash of the constitution to propagate (must match parent)"
                }
            },
            "required": ["child_id", "constitution_hash"],
            "additionalProperties": false
        })
    }

    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        let child_id = args["child_id"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("child_id must be a string"))?;

        let constitution_hash = args["constitution_hash"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("constitution_hash must be a string"))?;

        let mut mgr = self.manager.lock();

        if !mgr.verify_constitution(constitution_hash) {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(
                    "constitution hash mismatch — child must inherit parent constitution".into(),
                ),
            });
        }

        match mgr.request_spawn(child_id) {
            Ok(record) => {
                let output = serde_json::to_string_pretty(&json!({
                    "spawned": true,
                    "child_id": record.id,
                    "workspace": record.workspace.display().to_string(),
                    "phase": format!("{:?}", record.phase),
                    "active_children": mgr.active_children(),
                }))?;

                Ok(ToolResult {
                    success: true,
                    output,
                    error: None,
                })
            }
            Err(e) => Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(e.to_string()),
            }),
        }
    }
}
