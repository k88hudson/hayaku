use crate::env::EnvVarConfig;
use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TemplateSection {
    pub name: String,
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub author: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfigToml {
    pub template: TemplateSection,
    #[serde(default)]
    pub env: HashMap<String, EnvVarConfig>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemplateConfig {
    pub name: String,
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub author: Option<String>,
    pub env: HashMap<String, EnvVarConfig>,
}

impl TemplateConfig {
    pub fn default(dir: &str) -> Self {
        Self {
            name: dir.to_string(),
            display_name: None,
            description: None,
            author: None,
            env: HashMap::new(),
        }
    }

    pub fn try_from_dir(path: &std::path::Path) -> Result<Self> {
        if !path.is_dir() {
            return Err(anyhow!("Path {} is not a directory", path.display()));
        }

        let config_path = path.join("hayaku.toml");
        if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)
                .with_context(|| format!("Failed to read config file {}", config_path.display()))?;
            let config: ConfigToml = toml::from_str(&content).map_err(|err| {
                anyhow!(
                    "Failed to parse config file {}:\n{err}",
                    config_path.display()
                )
            })?;
            Ok(Self {
                name: config.template.name,
                display_name: config.template.display_name,
                description: config.template.description,
                author: config.template.author,
                env: config.env,
            })
        } else {
            let dir_name = path.file_name().and_then(|c| c.to_str()).ok_or_else(|| {
                anyhow!("Unable to determine directory name for {}", path.display())
            })?;
            Ok(TemplateConfig::default(dir_name))
        }
    }
}
