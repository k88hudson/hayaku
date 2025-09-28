use crate::config::HayakuConfig;
use anyhow::{Result, anyhow};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

#[derive(Eq, PartialEq, Debug, Clone)]
pub struct LocalTemplate {
    pub config: HayakuConfig,
    pub path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct LocalTemplates {
    hayaku_dir: PathBuf,
    local_template_dir: PathBuf,
    templates: HashMap<String, LocalTemplate>,
}

impl LocalTemplates {
    pub fn try_new_from_dir(hayaku_dir: &Path) -> Result<Self> {
        let local_template_dir = hayaku_dir.join("templates");

        if !hayaku_dir.exists() {
            let should_create = cliclack::confirm(format!(
                "Template directory {} does not exist. Would you like to create it?",
                local_template_dir.display()
            ))
            .interact()?;

            if should_create {
                std::fs::create_dir_all(&local_template_dir)
                    .map_err(|e| anyhow!("Failed to create template directory: {}", e))?;
                cliclack::log::info("Directory was created!")?;
            } else {
                return Err(anyhow!("Aborted by user"));
            }
        } else if !local_template_dir.exists() {
            std::fs::create_dir_all(&local_template_dir)
                .map_err(|e| anyhow!("Failed to create template directory: {}", e))?;
        }

        let mut templates = HashMap::new();
        for entry in std::fs::read_dir(&local_template_dir)? {
            let entry = entry?;
            if entry.path().is_dir() {
                let path = entry.path();
                let config = HayakuConfig::try_from_dir(&path)?;
                let id = config.name.clone();
                templates.insert(id, LocalTemplate { config, path });
            }
        }

        Ok(Self {
            hayaku_dir: hayaku_dir.to_path_buf(),
            local_template_dir,
            templates,
        })
    }
    pub fn try_new() -> Result<Self> {
        let env_dir: String = std::env::var("HAYAKU_DIRECTORY").unwrap_or("".to_string());

        let hayaku_dir = if env_dir != "" {
            PathBuf::from(env_dir)
        } else {
            std::env::home_dir()
                .ok_or_else(|| anyhow!("Could not determine home directory"))?
                .join(".hayaku")
        };
        Self::try_new_from_dir(&hayaku_dir)
    }

    pub fn hayaku_dir(&self) -> &Path {
        &self.hayaku_dir
    }

    pub fn local_template_dir(&self) -> &Path {
        &self.local_template_dir
    }

    pub fn templates(&self) -> &HashMap<String, LocalTemplate> {
        &self.templates
    }

    pub fn get(&self, id: &str) -> Option<&LocalTemplate> {
        self.templates.get(id)
    }

    pub fn is_empty(&self) -> bool {
        self.templates.is_empty()
    }

    // pub fn values(&self) -> Vec<&LocalTemplate> {
    //     self.templates.values().collect()
    // }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uses_provided_template_directory() {
        let dir = tempfile::tempdir().expect("create temp dir");

        let templates = LocalTemplates::try_new_from_dir(dir.path()).expect("init local templates");

        assert_eq!(templates.hayaku_dir(), dir.path());
        assert_eq!(templates.local_template_dir(), dir.path().join("templates"));
        assert!(templates.templates().is_empty());
    }

    #[test]
    fn discovers_template_directories() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let template_dir = dir.path().join("templates");
        let template_a = template_dir.join("alpha");
        let template_b = template_dir.join("beta");
        std::fs::create_dir_all(&template_a).expect("create alpha template");
        std::fs::create_dir_all(&template_b).expect("create beta template");
        std::fs::write(
            template_a.join("hayaku.toml"),
            "[template]\nname = \"alpha-template\"",
        )
        .expect("write config");

        let templates = LocalTemplates::try_new_from_dir(dir.path()).expect("init local templates");

        assert_eq!(templates.templates().len(), 2);
        assert!(templates.get("alpha-template").is_some());
        assert!(templates.get("beta").is_some());

        let alpha = templates.get("alpha-template").unwrap();
        assert_eq!(alpha.config.name, "alpha-template");
        assert_eq!(alpha.path, template_a);
    }

    #[test]
    fn empty_when_no_subdirectories() {
        let templates = LocalTemplates::try_new_from_dir(tempfile::tempdir().unwrap().path())
            .expect("init local templates");
        assert!(templates.templates().is_empty());
    }
}
