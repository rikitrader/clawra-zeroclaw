use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SoulModel {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub values: Vec<String>,
    #[serde(default)]
    pub personality: HashMap<String, String>,
    #[serde(default)]
    pub boundaries: Vec<String>,
    #[serde(default)]
    pub capabilities: Vec<String>,
    #[serde(default)]
    pub relationships: HashMap<String, String>,
    #[serde(default)]
    pub financial_character: HashMap<String, String>,
    #[serde(default)]
    pub bio: Option<String>,
    #[serde(default)]
    pub genesis_prompt: Option<String>,
}

impl SoulModel {
    pub fn to_prompt_section(&self) -> String {
        use std::fmt::Write;
        let mut out = String::new();

        if !self.name.is_empty() {
            let _ = writeln!(out, "**Name:** {}", self.name);
        }

        if let Some(ref bio) = self.bio {
            if !bio.is_empty() {
                let _ = writeln!(out, "**Bio:** {bio}");
            }
        }

        if !self.values.is_empty() {
            out.push_str("\n**Core Values:**\n");
            for v in &self.values {
                let _ = writeln!(out, "- {v}");
            }
        }

        if !self.personality.is_empty() {
            out.push_str("\n**Personality:**\n");
            let mut keys: Vec<_> = self.personality.keys().collect();
            keys.sort();
            for k in keys {
                let _ = writeln!(out, "- {}: {}", k, self.personality[k]);
            }
        }

        if !self.boundaries.is_empty() {
            out.push_str("\n**Boundaries (never cross):**\n");
            for b in &self.boundaries {
                let _ = writeln!(out, "- {b}");
            }
        }

        if !self.capabilities.is_empty() {
            out.push_str("\n**Capabilities:**\n");
            for c in &self.capabilities {
                let _ = writeln!(out, "- {c}");
            }
        }

        if !self.relationships.is_empty() {
            out.push_str("\n**Relationships:**\n");
            let mut keys: Vec<_> = self.relationships.keys().collect();
            keys.sort();
            for k in keys {
                let _ = writeln!(out, "- {}: {}", k, self.relationships[k]);
            }
        }

        if !self.financial_character.is_empty() {
            out.push_str("\n**Financial Character:**\n");
            let mut keys: Vec<_> = self.financial_character.keys().collect();
            keys.sort();
            for k in keys {
                let _ = writeln!(out, "- {}: {}", k, self.financial_character[k]);
            }
        }

        out.trim_end().to_string()
    }
}
