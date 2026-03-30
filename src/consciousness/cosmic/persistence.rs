use std::collections::HashMap;
use std::io::Read as _;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub const SNAPSHOT_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CosmicSnapshot {
    pub modules: HashMap<String, serde_json::Value>,
    pub version: u32,
    pub saved_at: DateTime<Utc>,
}

#[derive(Debug, thiserror::Error)]
pub enum PersistenceError {
    #[error("io: {0}")]
    Io(String),
    #[error("serialization: {0}")]
    Serialization(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("corruption: {0}")]
    Corruption(String),
}

#[derive(Debug, Clone, Serialize)]
pub struct SnapshotInfo {
    pub version: u32,
    pub saved_at: DateTime<Utc>,
    pub module_count: usize,
    pub total_bytes: u64,
    pub integrity_ok: bool,
}

#[derive(Debug, Clone)]
pub struct CosmicPersistence {
    base_dir: PathBuf,
}

impl CosmicPersistence {
    pub fn new(base_dir: impl AsRef<Path>) -> Self {
        Self {
            base_dir: base_dir.as_ref().to_path_buf(),
        }
    }

    pub fn save_module(
        &self,
        name: &str,
        data: &serde_json::Value,
    ) -> Result<(), PersistenceError> {
        std::fs::create_dir_all(&self.base_dir).map_err(|e| PersistenceError::Io(e.to_string()))?;
        let path = self.base_dir.join(format!("{name}.json"));
        let tmp_path = self.base_dir.join(format!("{name}.json.tmp"));
        let json = serde_json::to_string(data)
            .map_err(|e| PersistenceError::Serialization(e.to_string()))?;
        std::fs::write(&tmp_path, &json).map_err(|e| PersistenceError::Io(e.to_string()))?;
        std::fs::rename(&tmp_path, &path).map_err(|e| PersistenceError::Io(e.to_string()))
    }

    pub fn load_module(&self, name: &str) -> Result<serde_json::Value, PersistenceError> {
        let path = self.base_dir.join(format!("{name}.json"));
        if !path.exists() {
            return Err(PersistenceError::NotFound(format!("{name}.json")));
        }
        let raw =
            std::fs::read_to_string(&path).map_err(|e| PersistenceError::Io(e.to_string()))?;
        serde_json::from_str(&raw).map_err(|e| PersistenceError::Corruption(e.to_string()))
    }

    pub fn save_all(&self, snapshot: &CosmicSnapshot) -> Result<(), PersistenceError> {
        std::fs::create_dir_all(&self.base_dir).map_err(|e| PersistenceError::Io(e.to_string()))?;
        if self.base_dir.join("_snapshot_meta.json").exists() {
            let _ = self.rotate_backup(3);
            self.prune_old_backups(3);
        }
        for (name, data) in &snapshot.modules {
            self.save_module(name, data)?;
        }
        let meta = serde_json::json!({
            "version": snapshot.version,
            "saved_at": snapshot.saved_at,
            "module_names": snapshot.modules.keys().collect::<Vec<_>>(),
        });
        let meta_json = serde_json::to_string_pretty(&meta)
            .map_err(|e| PersistenceError::Serialization(e.to_string()))?;
        let meta_tmp = self.base_dir.join("_snapshot_meta.json.tmp");
        let meta_final = self.base_dir.join("_snapshot_meta.json");
        std::fs::write(&meta_tmp, meta_json).map_err(|e| PersistenceError::Io(e.to_string()))?;
        std::fs::rename(&meta_tmp, &meta_final).map_err(|e| PersistenceError::Io(e.to_string()))?;

        let current: Vec<&str> = snapshot.modules.keys().map(String::as_str).collect();
        self.prune_stale_modules(&current);

        Ok(())
    }

    pub fn load_all(&self) -> Result<CosmicSnapshot, PersistenceError> {
        let meta_path = self.base_dir.join("_snapshot_meta.json");
        if !meta_path.exists() {
            return Err(PersistenceError::NotFound(
                "_snapshot_meta.json".to_string(),
            ));
        }
        let meta_raw =
            std::fs::read_to_string(&meta_path).map_err(|e| PersistenceError::Io(e.to_string()))?;
        let meta: serde_json::Value = serde_json::from_str(&meta_raw)
            .map_err(|e| PersistenceError::Corruption(e.to_string()))?;

        #[allow(clippy::cast_possible_truncation)]
        let version = meta["version"].as_u64().unwrap_or(0) as u32;

        if version > SNAPSHOT_VERSION {
            return Err(PersistenceError::Corruption(format!(
                "snapshot version {version} is newer than supported {SNAPSHOT_VERSION}"
            )));
        }

        if version < SNAPSHOT_VERSION {
            tracing::info!(
                from = version,
                to = SNAPSHOT_VERSION,
                "Migrating snapshot from v{version} to v{SNAPSHOT_VERSION}"
            );
        }

        let saved_at: DateTime<Utc> = meta["saved_at"]
            .as_str()
            .and_then(|s| s.parse().ok())
            .unwrap_or_else(Utc::now);

        let module_names: Vec<String> = meta["module_names"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        let mut modules = HashMap::new();
        for name in &module_names {
            match self.load_module(name) {
                Ok(data) => {
                    modules.insert(name.clone(), data);
                }
                Err(PersistenceError::NotFound(_)) => {
                    tracing::warn!(module = %name, "Module file missing — skipping");
                }
                Err(e) => return Err(e),
            }
        }

        Ok(CosmicSnapshot {
            modules,
            version,
            saved_at,
        })
    }

    pub fn list_modules(&self) -> Result<Vec<String>, PersistenceError> {
        if !self.base_dir.exists() {
            return Ok(Vec::new());
        }
        let entries =
            std::fs::read_dir(&self.base_dir).map_err(|e| PersistenceError::Io(e.to_string()))?;
        let mut names = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|e| PersistenceError::Io(e.to_string()))?;
            let file_name = entry.file_name();
            let name = file_name.to_string_lossy();
            if name.ends_with(".json") && !name.starts_with('_') {
                names.push(name.trim_end_matches(".json").to_string());
            }
        }
        names.sort();
        Ok(names)
    }

    pub fn prune_stale_modules(&self, current_modules: &[&str]) -> usize {
        let mut pruned = 0;
        if let Ok(entries) = std::fs::read_dir(&self.base_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                if !name_str.ends_with(".json") || name_str.starts_with('_') {
                    continue;
                }
                let module_name = name_str.trim_end_matches(".json");
                if !current_modules.contains(&module_name)
                    && std::fs::remove_file(entry.path()).is_ok()
                {
                    tracing::info!(module = %module_name, "Pruned stale module file");
                    pruned += 1;
                }
            }
        }
        pruned
    }

    pub fn rotate_backup(&self, max_backups: usize) -> Result<(), PersistenceError> {
        let meta_path = self.base_dir.join("_snapshot_meta.json");
        if !meta_path.exists() {
            return Ok(());
        }
        for i in (1..max_backups).rev() {
            let older = self.base_dir.join(format!("_backup_{i}"));
            let newer = self.base_dir.join(format!("_backup_{}", i + 1));
            if older.exists() {
                if i + 1 > max_backups {
                    let _ = std::fs::remove_dir_all(&older);
                } else {
                    let _ = std::fs::rename(&older, &newer);
                }
            }
        }
        let backup_dir = self.base_dir.join("_backup_1");
        let _ = std::fs::remove_dir_all(&backup_dir);
        std::fs::create_dir_all(&backup_dir).map_err(|e| PersistenceError::Io(e.to_string()))?;
        if let Ok(entries) = std::fs::read_dir(&self.base_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                if name_str.ends_with(".json") {
                    let dest = backup_dir.join(&*name_str);
                    let _ = std::fs::copy(entry.path(), dest);
                }
            }
        }
        let stale_limit = max_backups + 1;
        let stale_dir = self.base_dir.join(format!("_backup_{stale_limit}"));
        if stale_dir.exists() {
            let _ = std::fs::remove_dir_all(&stale_dir);
        }
        Ok(())
    }

    pub fn prune_old_backups(&self, max_backups: usize) -> usize {
        let mut pruned = 0;
        for i in (max_backups + 1)..100 {
            let backup_dir = self.base_dir.join(format!("_backup_{i}"));
            if backup_dir.exists() {
                if std::fs::remove_dir_all(&backup_dir).is_ok() {
                    pruned += 1;
                }
            } else {
                break;
            }
        }
        pruned
    }

    pub fn compute_checksum(path: &Path) -> Result<String, PersistenceError> {
        use std::hash::{DefaultHasher, Hash, Hasher};
        let mut file =
            std::fs::File::open(path).map_err(|e| PersistenceError::Io(e.to_string()))?;
        let mut buf = Vec::new();
        file.read_to_end(&mut buf)
            .map_err(|e| PersistenceError::Io(e.to_string()))?;
        let mut hasher = DefaultHasher::new();
        buf.hash(&mut hasher);
        Ok(format!("{:016x}", hasher.finish()))
    }

    pub fn verify_integrity(&self) -> Result<bool, PersistenceError> {
        let meta_path = self.base_dir.join("_snapshot_meta.json");
        if !meta_path.exists() {
            return Err(PersistenceError::NotFound(
                "_snapshot_meta.json".to_string(),
            ));
        }
        let meta_raw =
            std::fs::read_to_string(&meta_path).map_err(|e| PersistenceError::Io(e.to_string()))?;
        let meta: serde_json::Value = serde_json::from_str(&meta_raw)
            .map_err(|e| PersistenceError::Corruption(e.to_string()))?;

        let module_names: Vec<String> = meta["module_names"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        for name in &module_names {
            let path = self.base_dir.join(format!("{name}.json"));
            if !path.exists() {
                return Ok(false);
            }
            let raw =
                std::fs::read_to_string(&path).map_err(|e| PersistenceError::Io(e.to_string()))?;
            if serde_json::from_str::<serde_json::Value>(&raw).is_err() {
                return Ok(false);
            }
        }
        Ok(true)
    }

    pub fn snapshot_info(&self) -> Result<SnapshotInfo, PersistenceError> {
        let meta_path = self.base_dir.join("_snapshot_meta.json");
        if !meta_path.exists() {
            return Err(PersistenceError::NotFound(
                "_snapshot_meta.json".to_string(),
            ));
        }
        let meta_raw =
            std::fs::read_to_string(&meta_path).map_err(|e| PersistenceError::Io(e.to_string()))?;
        let meta: serde_json::Value = serde_json::from_str(&meta_raw)
            .map_err(|e| PersistenceError::Corruption(e.to_string()))?;

        #[allow(clippy::cast_possible_truncation)]
        let version = meta["version"].as_u64().unwrap_or(0) as u32;
        let saved_at: DateTime<Utc> = meta["saved_at"]
            .as_str()
            .and_then(|s| s.parse().ok())
            .unwrap_or_else(Utc::now);
        let module_names: Vec<String> = meta["module_names"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        let mut total_bytes: u64 = 0;
        for name in &module_names {
            let path = self.base_dir.join(format!("{name}.json"));
            if let Ok(m) = std::fs::metadata(&path) {
                total_bytes += m.len();
            }
        }
        if let Ok(m) = std::fs::metadata(&meta_path) {
            total_bytes += m.len();
        }

        Ok(SnapshotInfo {
            version,
            saved_at,
            module_count: module_names.len(),
            total_bytes,
            integrity_ok: self.verify_integrity().unwrap_or(false),
        })
    }

    pub fn delete_module(&self, name: &str) -> Result<(), PersistenceError> {
        let path = self.base_dir.join(format!("{name}.json"));
        if !path.exists() {
            return Err(PersistenceError::NotFound(format!("{name}.json")));
        }
        std::fs::remove_file(&path).map_err(|e| PersistenceError::Io(e.to_string()))
    }
}

#[allow(clippy::too_many_arguments)]
pub fn gather_snapshot(
    modulator: &crate::consciousness::cosmic::EmotionalModulator,
    drift: &crate::consciousness::cosmic::DriftDetector,
    thalamus: &crate::consciousness::cosmic::SensoryThalamus,
    workspace: &crate::consciousness::cosmic::GlobalWorkspace,
    self_model: &crate::consciousness::cosmic::SelfModel,
    world_model: &crate::consciousness::cosmic::WorldModel,
    consolidation: &crate::consciousness::cosmic::ConsolidationEngine,
    normative: &crate::consciousness::cosmic::NormativeEngine,
    causal: &crate::consciousness::cosmic::CausalGraph,
    graph: &crate::consciousness::cosmic::CosmicMemoryGraph,
) -> CosmicSnapshot {
    let mut modules = HashMap::new();

    modules.insert("modulation".to_string(), modulator.full_snapshot());
    modules.insert("drift".to_string(), drift.snapshot());
    modules.insert("thalamus".to_string(), thalamus.full_snapshot());
    modules.insert("workspace".to_string(), workspace.full_snapshot());
    modules.insert("self_model".to_string(), self_model.snapshot());
    modules.insert("world_model".to_string(), world_model.snapshot());
    modules.insert("consolidation".to_string(), consolidation.full_snapshot());
    modules.insert("normative".to_string(), normative.snapshot());
    modules.insert("causal".to_string(), causal.snapshot());
    modules.insert("graph".to_string(), graph.full_snapshot());

    CosmicSnapshot {
        modules,
        version: SNAPSHOT_VERSION,
        saved_at: Utc::now(),
    }
}
