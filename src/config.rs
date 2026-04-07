use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub claude_args: Vec<String>,
    pub claude_bin: String,
    pub claude_dir: Option<PathBuf>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            claude_args: Vec::new(),
            claude_bin: "claude".to_string(),
            claude_dir: None,
        }
    }
}

impl Config {
    pub fn load_from(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }

    pub fn load() -> Result<Self> {
        let path = Self::config_path();
        Self::load_from(&path)
    }

    pub fn config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("~/.config"))
            .join("claudeman")
            .join("config.toml")
    }

    pub fn claude_dir(&self) -> PathBuf {
        self.claude_dir
            .clone()
            .unwrap_or_else(|| {
                dirs::home_dir()
                    .unwrap_or_else(|| PathBuf::from("~"))
                    .join(".claude")
            })
    }

    pub fn index_dir() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("~/.config"))
            .join("claudeman")
            .join("index")
    }
}
