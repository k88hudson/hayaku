use crate::config::TemplateConfig;
use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

#[derive(Eq, PartialEq, Debug, Clone, Copy)]
pub enum TemplateOrigin {
    Local,
    BuiltIn,
}

#[derive(Eq, PartialEq, Debug, Clone)]
pub struct TemplateEntry {
    pub config: TemplateConfig,
    pub path: PathBuf,
    pub origin: TemplateOrigin,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HayakuSettings {
    pub global_env: Option<HashMap<String, toml::Value>>,
}

impl HayakuSettings {
    pub fn write_to_file(&self, path: &Path) -> Result<()> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let toml_string = toml::to_string_pretty(self)
            .map_err(|err| anyhow!("Failed to serialize settings to TOML:\n{err}"))?;
        std::fs::write(path, toml_string)
            .map_err(|err| anyhow!("Failed to write settings to {}:\n{err}", path.display()))?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct Hayaku {
    hayaku_dir: PathBuf,
    local_template_dir: PathBuf,
    built_in_template_dir: PathBuf,
    settings_config_path: PathBuf,
    local_templates: HashMap<String, TemplateEntry>,
    built_in_templates: HashMap<String, TemplateEntry>,
}

impl Hayaku {
    const TEMPLATE_DIR: &str = "templates";
    const SETTINGS_FILE: &str = "hayaku.settings.toml";

    fn hayaku_dir_from_env() -> Result<PathBuf> {
        if let Ok(dir) = std::env::var("HAYAKU_DIRECTORY") {
            Ok(dir.into())
        } else {
            Ok(std::env::home_dir()
                .ok_or_else(|| anyhow!("Could not determine home directory"))?
                .join(".hayaku"))
        }
    }
    pub fn try_new_from_dir(hayaku_dir: &Path) -> Result<Self> {
        let local_template_dir = hayaku_dir.join(Self::TEMPLATE_DIR);
        let settings_config_path = hayaku_dir.join(Self::SETTINGS_FILE);
        let built_in_template_dir = built_in_templates_dir();

        let local_templates = load_templates_from_dir(&local_template_dir, TemplateOrigin::Local)?;

        let built_in_templates =
            load_templates_from_dir(&built_in_template_dir, TemplateOrigin::BuiltIn)?;

        Ok(Self {
            hayaku_dir: hayaku_dir.to_path_buf(),
            settings_config_path,
            local_template_dir,
            built_in_template_dir,
            local_templates,
            built_in_templates,
        })
    }
    pub fn try_new() -> Result<Self> {
        Self::try_new_from_dir(&Self::hayaku_dir_from_env()?)
    }

    pub fn settings_config_path(&self) -> &Path {
        &self.settings_config_path
    }

    pub fn parse_settings(&self) -> Result<HayakuSettings> {
        if self.settings_config_path.exists() {
            let raw = std::fs::read_to_string(&self.settings_config_path)?;
            let config: HayakuSettings = toml::from_str(&raw).map_err(|err| {
                anyhow!(
                    "Failed to parse settings file {}:\n{err}",
                    self.settings_config_path.display()
                )
            })?;
            Ok(config)
        } else {
            Ok(HayakuSettings::default())
        }
    }

    pub fn hayaku_dir(&self) -> &Path {
        &self.hayaku_dir
    }

    pub fn local_template_dir(&self) -> &Path {
        &self.local_template_dir
    }

    pub fn built_in_template_dir(&self) -> &Path {
        &self.built_in_template_dir
    }

    pub fn templates(&self) -> &HashMap<String, TemplateEntry> {
        &self.local_templates
    }

    pub fn built_in_templates(&self) -> &HashMap<String, TemplateEntry> {
        &self.built_in_templates
    }

    pub fn all_templates(&self) -> Vec<&TemplateEntry> {
        let mut combined: Vec<&TemplateEntry> = self.built_in_templates.values().collect();

        for local in self.local_templates.values() {
            if let Some(pos) = combined
                .iter()
                .position(|existing| existing.config.name == local.config.name)
            {
                combined.remove(pos);
            }
            combined.push(local);
        }

        combined.sort_by(|a, b| match (&a.origin, &b.origin) {
            (TemplateOrigin::Local, TemplateOrigin::BuiltIn) => std::cmp::Ordering::Greater,
            (TemplateOrigin::BuiltIn, TemplateOrigin::Local) => std::cmp::Ordering::Less,
            _ => {
                let a_name = a.config.display_name.as_ref().unwrap_or(&a.config.name);
                let b_name = b.config.display_name.as_ref().unwrap_or(&b.config.name);
                a_name.cmp(b_name)
            }
        });

        combined
    }

    pub fn get(&self, id: &str) -> Option<&TemplateEntry> {
        self.local_templates
            .get(id)
            .or_else(|| self.built_in_templates.get(id))
    }

    pub fn no_local_templates(&self) -> bool {
        self.local_templates.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uses_provided_template_directory() {
        let dir = tempfile::tempdir().expect("create temp dir");

        let templates = Hayaku::try_new_from_dir(dir.path()).expect("init local templates");

        assert_eq!(templates.hayaku_dir(), dir.path());
        assert_eq!(templates.local_template_dir(), dir.path().join("templates"));
        assert!(templates.templates().is_empty());
        assert_eq!(
            templates.built_in_template_dir().file_name(),
            Some(std::ffi::OsStr::new("built_in"))
        );
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

        let templates = Hayaku::try_new_from_dir(dir.path()).expect("init local templates");

        assert_eq!(templates.templates().len(), 2);
        assert!(templates.get("alpha-template").is_some());
        assert!(templates.get("beta").is_some());

        let alpha = templates.get("alpha-template").unwrap();
        assert_eq!(alpha.config.name, "alpha-template");
        assert_eq!(alpha.path, template_a);
    }

    #[test]
    fn loads_built_in_templates() {
        let dir = tempfile::tempdir().expect("create temp dir");

        let templates = Hayaku::try_new_from_dir(dir.path()).expect("init templates");

        assert!(!templates.built_in_templates().is_empty());
        assert!(
            templates
                .built_in_templates()
                .values()
                .all(|template| matches!(template.origin, TemplateOrigin::BuiltIn))
        );
    }

    #[test]
    fn local_templates_override_built_in() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let local_templates_path = dir.path().join("templates");
        std::fs::create_dir_all(&local_templates_path).expect("create local templates dir");

        let local_rust = local_templates_path.join("rust");
        std::fs::create_dir_all(&local_rust).expect("create local rust template");
        std::fs::write(
            local_rust.join("hayaku.toml"),
            "[template]\nname = \"rust\"\ndescription = \"Local rust template\"",
        )
        .expect("write local template config");

        let templates = Hayaku::try_new_from_dir(dir.path()).expect("init templates");

        let template = templates.get("rust").expect("rust template exists");
        assert_eq!(template.path, local_rust);
        assert!(matches!(template.origin, TemplateOrigin::Local));
    }

    #[test]
    fn empty_when_no_subdirectories() {
        let templates = Hayaku::try_new_from_dir(tempfile::tempdir().unwrap().path())
            .expect("init local templates");
        assert!(templates.templates().is_empty());
    }
}

fn load_templates_from_dir(
    dir: &Path,
    origin: TemplateOrigin,
) -> Result<HashMap<String, TemplateEntry>> {
    let mut templates = HashMap::new();

    if !dir.exists() {
        return Ok(templates);
    }

    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        if entry.path().is_dir() {
            let path = entry.path();
            let config = TemplateConfig::try_from_dir(&path)?;
            let id = config.name.clone();
            templates.insert(
                id,
                TemplateEntry {
                    config,
                    path,
                    origin,
                },
            );
        }
    }

    Ok(templates)
}

fn built_in_templates_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("built_in")
}
