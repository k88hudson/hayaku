use crate::cli::Hayaku;
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
    local_template_dir: PathBuf,
    templates: HashMap<String, LocalTemplate>,
}

impl LocalTemplates {
    pub fn try_new(options: &Hayaku) -> Result<Self> {
        let local_template_dir = options.local_template_dir.clone().unwrap_or_else(|| {
            std::env::home_dir()
                .unwrap()
                .join(".hayaku")
                .join("templates")
        });

        if !local_template_dir.exists() {
            let should_create = cliclack::confirm(format!(
                "Template dir {} does not exist. Would you like to create it?",
                local_template_dir.display()
            ))
            .interact()?;

            if should_create {
                std::fs::create_dir_all(&local_template_dir)
                    .map_err(|e| anyhow!("Failed to create template directory: {}", e))?;
                cliclack::log::info("Directory was created!")?;
            } else {
                return Ok(Self {
                    local_template_dir,
                    templates: HashMap::new(),
                });
            }
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
            local_template_dir,
            templates,
        })
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
    use clap::Parser;

    fn hayaku_with_dir(dir: &std::path::Path) -> Hayaku {
        Hayaku::try_parse_from([
            "hayaku",
            "--local-template-dir",
            dir.to_str().unwrap(),
            "list",
        ])
        .expect("failed to parse test cli arguments")
    }

    #[test]
    fn uses_provided_template_directory() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let hayaku = hayaku_with_dir(dir.path());

        let templates = LocalTemplates::try_new(&hayaku).expect("init local templates");

        assert_eq!(templates.local_template_dir(), dir.path());
        assert!(templates.templates().is_empty());
    }

    #[test]
    fn discovers_template_directories() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let template_a = dir.path().join("alpha");
        let template_b = dir.path().join("beta");
        std::fs::create_dir_all(&template_a).expect("create alpha template");
        std::fs::create_dir_all(&template_b).expect("create beta template");
        std::fs::write(
            template_a.join("hayaku.toml"),
            "[template]\nname = \"alpha-template\"",
        )
        .expect("write config");

        let hayaku = hayaku_with_dir(dir.path());

        let templates = LocalTemplates::try_new(&hayaku).expect("init local templates");

        assert_eq!(templates.templates().len(), 2);
        assert!(templates.get("alpha-template").is_some());
        assert!(templates.get("beta").is_some());

        let alpha = templates.get("alpha-template").unwrap();
        assert_eq!(alpha.config.name, "alpha-template");
        assert_eq!(alpha.path, template_a);
    }

    #[test]
    fn empty_when_no_subdirectories() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let hayaku = hayaku_with_dir(dir.path());

        let templates = LocalTemplates::try_new(&hayaku).expect("init local templates");
        assert!(templates.templates().is_empty());
    }
}
