use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NameStore {
    #[serde(default)]
    names: HashMap<String, String>,
}

impl NameStore {
    pub fn load_from(path: &Path) -> Result<Self> {
        if !path.exists() { return Ok(Self::default()); }
        let content = std::fs::read_to_string(path)?;
        let store: NameStore = toml::from_str(&content)?;
        Ok(store)
    }

    pub fn load() -> Result<Self> {
        Self::load_from(&Self::default_path())
    }

    pub fn save_to(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() { std::fs::create_dir_all(parent)?; }
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    pub fn save(&self) -> Result<()> { self.save_to(&Self::default_path()) }

    pub fn get(&self, session_id: &str) -> Option<&str> {
        self.names.get(session_id).map(|s| s.as_str())
    }

    pub fn set(&mut self, session_id: &str, name: &str) {
        self.names.insert(session_id.to_string(), name.to_string());
    }

    pub fn default_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("~/.config"))
            .join("claudeman")
            .join("names.toml")
    }
}
