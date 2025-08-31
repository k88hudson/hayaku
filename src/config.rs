use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct HayakuConfig {
    pub name: String,
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub author: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct ConfigToml {
    pub template: HayakuConfig,
}

impl HayakuConfig {
    pub fn default(dir: &str) -> Self {
        Self {
            name: dir.to_string(),
            display_name: None,
            description: None,
            author: None,
        }
    }

    pub fn try_from_dir(path: &std::path::Path) -> Result<Self> {
        if !path.is_dir() {
            return Err(anyhow::anyhow!(
                "Path {} is not a directory",
                path.display()
            ));
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
            Ok(config.template)
        } else {
            let dir_name = path.file_name().and_then(|c| c.to_str()).ok_or_else(|| {
                anyhow::anyhow!("Unable to determine directory name for {}", path.display())
            })?;
            Ok(HayakuConfig::default(dir_name))
        }
    }
}
