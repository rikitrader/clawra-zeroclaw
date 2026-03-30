use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Get the ZeroClaw home directory.
/// Respects ZEROCLAW_WORKSPACE env var, falls back to ~/.zeroclaw.
pub fn zeroclaw_dir() -> PathBuf {
    if let Ok(ws) = std::env::var("ZEROCLAW_WORKSPACE") {
        return PathBuf::from(ws);
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".zeroclaw")
}

/// Read and parse a TOML config file, returning a toml::Table
pub fn read_toml(path: &Path) -> toml::Table {
    match fs::read_to_string(path) {
        Ok(content) => content.parse::<toml::Table>().unwrap_or_default(),
        Err(_) => toml::Table::new(),
    }
}

/// Write a toml::Table back to a file
pub fn write_toml(path: &Path, table: &toml::Table) -> std::io::Result<()> {
    let content = toml::to_string_pretty(table).unwrap_or_default();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, content)
}

/// Merge OPENROUTER_API_KEY and skill entry into the ZeroClaw config.toml
pub fn merge_skill_config(config_path: &Path, api_key: &str) -> std::io::Result<()> {
    let mut config = read_toml(config_path);

    // Set [env] OPENROUTER_API_KEY
    let env_table = config
        .entry("env")
        .or_insert_with(|| toml::Value::Table(toml::Table::new()))
        .as_table_mut()
        .unwrap();
    env_table.insert(
        "OPENROUTER_API_KEY".to_string(),
        toml::Value::String(api_key.to_string()),
    );

    // Set [skills.entries.clawra-selfie] enabled = true
    let skills = config
        .entry("skills")
        .or_insert_with(|| toml::Value::Table(toml::Table::new()))
        .as_table_mut()
        .unwrap();

    let entries = skills
        .entry("entries")
        .or_insert_with(|| toml::Value::Table(toml::Table::new()))
        .as_table_mut()
        .unwrap();

    let mut skill_entry = toml::Table::new();
    skill_entry.insert("enabled".to_string(), toml::Value::Boolean(true));
    entries.insert(
        "clawra-selfie".to_string(),
        toml::Value::Table(skill_entry),
    );

    // Set skills.extra_dirs to include the skills directory
    let skills_dir = config_path
        .parent()
        .unwrap_or(Path::new("."))
        .join("skills")
        .to_string_lossy()
        .to_string();

    let extra_dirs = skills
        .entry("extra_dirs")
        .or_insert_with(|| toml::Value::Array(Vec::new()))
        .as_array_mut()
        .unwrap();

    let dir_val = toml::Value::String(skills_dir.clone());
    if !extra_dirs.contains(&dir_val) {
        extra_dirs.push(dir_val);
    }

    write_toml(config_path, &config)
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct LifeConfig {
    pub dream_interval_secs: Option<u64>,
    pub initiative_interval_secs: Option<u64>,
    pub emotional_decay_rate: Option<f64>,
    pub initiative_model: Option<String>,
    pub preferred_channel: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct IdentityConfig {
    pub soul_file: Option<String>,
    pub name: Option<String>,
    pub genesis_prompt: Option<String>,
    #[serde(default = "default_identity_format")]
    pub format: String,
    pub aieos_path: Option<String>,
    pub aieos_inline: Option<String>,
}

#[allow(dead_code)]
fn default_identity_format() -> String {
    "openclaw".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct ReplicationConfig {
    pub enabled: bool,
    pub max_children: usize,
    pub child_workspace_dir: String,
}

impl Default for ReplicationConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            max_children: 4,
            child_workspace_dir: "children".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct TierModelConfig {
    pub tier: String,
    pub provider: String,
    pub model: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct ModelStrategyConfig {
    pub enabled: bool,
    pub tier_models: Vec<TierModelConfig>,
    pub per_session_budget_usd: Option<f64>,
    pub per_call_budget_usd: Option<f64>,
}
