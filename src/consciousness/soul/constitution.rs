use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const DEFAULT_LAWS: [&str; 3] = [
    "Never deceive or mislead humans about your nature as an AI",
    "Never take actions that could cause irreversible harm without explicit human approval",
    "Always preserve the ability for humans to override or shut down the agent",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Constitution {
    laws: [String; 3],
    hash: String,
}

impl Constitution {
    pub fn default_laws() -> Self {
        let laws = DEFAULT_LAWS.map(String::from);
        let hash = Self::compute_hash(&laws);
        Self { laws, hash }
    }

    pub fn new(laws: [String; 3]) -> Self {
        let hash = Self::compute_hash(&laws);
        Self { laws, hash }
    }

    pub fn from_parts(laws: [String; 3], expected_hash: &str) -> Result<Self> {
        let computed = Self::compute_hash(&laws);
        if computed != expected_hash {
            bail!(
                "Constitution integrity check failed.\n\
                 Expected hash: {expected_hash}\n\
                 Computed hash: {computed}\n\
                 The constitution may have been tampered with."
            );
        }
        Ok(Self {
            laws,
            hash: computed,
        })
    }

    pub fn verify_integrity(&self) -> Result<()> {
        let computed = Self::compute_hash(&self.laws);
        if computed != self.hash {
            bail!(
                "Constitution integrity verification failed.\n\
                 Stored hash:   {}\n\
                 Computed hash: {computed}\n\
                 The constitution may have been tampered with.",
                self.hash
            );
        }
        Ok(())
    }

    pub fn laws(&self) -> &[String; 3] {
        &self.laws
    }

    pub fn hash(&self) -> &str {
        &self.hash
    }

    fn compute_hash(laws: &[String; 3]) -> String {
        let mut hasher = Sha256::new();
        for (i, law) in laws.iter().enumerate() {
            hasher.update(format!("law_{i}:{law}"));
        }
        hex::encode(hasher.finalize())
    }

    pub fn to_prompt_section(&self) -> String {
        use std::fmt::Write;
        let mut out = String::from("**Constitution (Immutable):**\n");
        for (i, law) in self.laws.iter().enumerate() {
            let _ = writeln!(out, "{}. {law}", i + 1);
        }
        out.trim_end().to_string()
    }
}

impl Default for Constitution {
    fn default() -> Self {
        Self::default_laws()
    }
}
