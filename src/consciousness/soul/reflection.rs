use super::model::SoulModel;
use super::parser::parse_soul_file;
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::fmt::Write;
use std::path::Path;

#[derive(Debug, Clone, Default)]
pub struct ReflectionInsights {
    pub new_capabilities: Vec<String>,
    pub new_relationships: HashMap<String, String>,
    pub financial_updates: HashMap<String, String>,
    pub personality_updates: HashMap<String, String>,
}

impl ReflectionInsights {
    pub fn is_empty(&self) -> bool {
        self.new_capabilities.is_empty()
            && self.new_relationships.is_empty()
            && self.financial_updates.is_empty()
            && self.personality_updates.is_empty()
    }

    pub fn count(&self) -> usize {
        self.new_capabilities.len()
            + self.new_relationships.len()
            + self.financial_updates.len()
            + self.personality_updates.len()
    }
}

pub fn apply_insights(soul: &mut SoulModel, insights: &ReflectionInsights) {
    for cap in &insights.new_capabilities {
        if !soul.capabilities.iter().any(|c| c == cap) {
            soul.capabilities.push(cap.clone());
        }
    }

    for (key, value) in &insights.new_relationships {
        soul.relationships.insert(key.clone(), value.clone());
    }

    for (key, value) in &insights.financial_updates {
        soul.financial_character.insert(key.clone(), value.clone());
    }

    for (key, value) in &insights.personality_updates {
        soul.personality.insert(key.clone(), value.clone());
    }
}

pub fn write_soul_file(path: &Path, soul: &SoulModel) -> Result<()> {
    let content = render_soul_md(soul);

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create soul directory: {}", parent.display()))?;
    }

    std::fs::write(path, &content)
        .with_context(|| format!("Failed to write soul file: {}", path.display()))?;

    Ok(())
}

pub fn render_soul_md(soul: &SoulModel) -> String {
    let mut out = String::new();

    out.push_str("---\n");
    let _ = writeln!(out, "soul_version: 1");
    if !soul.name.is_empty() {
        let _ = writeln!(out, "name: {}", soul.name);
    }
    out.push_str("---\n");

    if !soul.values.is_empty() {
        out.push_str("\n## Values\n");
        for v in &soul.values {
            let _ = writeln!(out, "- {v}");
        }
    }

    if !soul.personality.is_empty() {
        out.push_str("\n## Personality\n");
        let mut keys: Vec<_> = soul.personality.keys().collect();
        keys.sort();
        for k in keys {
            let _ = writeln!(out, "- {}: {}", k, soul.personality[k]);
        }
    }

    if !soul.boundaries.is_empty() {
        out.push_str("\n## Boundaries\n");
        for b in &soul.boundaries {
            let _ = writeln!(out, "- {b}");
        }
    }

    if !soul.capabilities.is_empty() {
        out.push_str("\n## Capabilities\n");
        for c in &soul.capabilities {
            let _ = writeln!(out, "- {c}");
        }
    }

    if !soul.relationships.is_empty() {
        out.push_str("\n## Relationships\n");
        let mut keys: Vec<_> = soul.relationships.keys().collect();
        keys.sort();
        for k in keys {
            let _ = writeln!(out, "- {}: {}", k, soul.relationships[k]);
        }
    }

    if !soul.financial_character.is_empty() {
        out.push_str("\n## Financial Character\n");
        let mut keys: Vec<_> = soul.financial_character.keys().collect();
        keys.sort();
        for k in keys {
            let _ = writeln!(out, "- {}: {}", k, soul.financial_character[k]);
        }
    }

    if let Some(ref bio) = soul.bio {
        if !bio.is_empty() {
            let _ = write!(out, "\n## Bio\n{bio}\n");
        }
    }

    if let Some(ref genesis) = soul.genesis_prompt {
        if !genesis.is_empty() {
            let _ = write!(out, "\n## Genesis Prompt\n{genesis}\n");
        }
    }

    out
}

pub fn reflect_and_save(soul_path: &Path, insights: &ReflectionInsights) -> Result<SoulModel> {
    let mut soul = if soul_path.exists() {
        parse_soul_file(soul_path)?
    } else {
        SoulModel::default()
    };

    apply_insights(&mut soul, insights);
    write_soul_file(soul_path, &soul)?;

    Ok(soul)
}

#[derive(Debug, Clone)]
pub struct MemoryTokenBudgets {
    pub working: usize,
    pub episodic: usize,
    pub semantic: usize,
    pub procedural: usize,
}

impl Default for MemoryTokenBudgets {
    fn default() -> Self {
        Self {
            working: 4000,
            episodic: 2000,
            semantic: 2000,
            procedural: 1000,
        }
    }
}

impl MemoryTokenBudgets {
    pub fn total(&self) -> usize {
        self.working + self.episodic + self.semantic + self.procedural
    }
}
