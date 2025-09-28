use std::path::Path;

use anyhow::{Result, anyhow};
use cliclack;
use serde::{Deserialize, Serialize};
use tera::Context as TeraContext;

use crate::{config::TemplateConfig, hayaku_context::Hayaku};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EnvVarConfig {
    String {
        prompt: String,
        default: Option<String>,
    },
    Choices {
        prompt: String,
        choices: Vec<String>,
        default: Option<String>,
    },
    Bool {
        prompt: String,
        #[serde(default)]
        default: bool,
    },
}

fn add_config_env_to_context(config: &TemplateConfig, context: &mut TeraContext) -> Result<()> {
    if config.env.is_empty() {
        return Ok(());
    }
    for (raw_key, env_cfg) in config.env.iter() {
        let key = canonical_env_key(raw_key);
        match &env_cfg {
            EnvVarConfig::String { prompt, default } => {
                let mut input = cliclack::input(prompt).required(true);
                if let Some(default) = default {
                    input = input.default_input(default);
                }
                let result = input.interact::<String>()?;
                context.insert(&key, &result);
            }
            EnvVarConfig::Choices {
                prompt,
                choices,
                default,
            } => {
                let choices_tuple: Vec<(String, String, String)> = choices
                    .iter()
                    .map(|c| (c.clone(), c.clone(), c.clone()))
                    .collect();
                let mut input = cliclack::select(prompt).items(&choices_tuple);
                if let Some(default) = default {
                    input = input.initial_value(default.clone())
                }
                let result = input.interact()?;
                context.insert(&key, &result);
            }
            EnvVarConfig::Bool { prompt, default } => {
                let mut confirm = cliclack::confirm(prompt);
                if *default {
                    confirm = confirm.initial_value(*default)
                }
                let result = confirm.interact()?;
                context.insert(&key, &result);
            }
        };
    }
    Ok(())
}

pub fn build_context(
    project_name: &str,
    config: &TemplateConfig,
    hayaku: &Hayaku,
) -> Result<TeraContext> {
    let mut context = TeraContext::new();
    context.insert("project_name", project_name);
    context.insert("PROJECT_NAME", project_name);
    context.insert("template_name", &config.name);
    context.insert("TEMPLATE_NAME", &config.name);

    let global_settings = hayaku.parse_settings()?;
    if let Some(global_env) = global_settings.global_env {
        for (key, value) in global_env.iter() {
            context.insert(&canonical_env_key(key), value);
        }
    }

    add_config_env_to_context(config, &mut context)?;

    Ok(context)
}

pub fn canonical_env_key(raw: &str) -> String {
    raw.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_uppercase()
            } else {
                '_'
            }
        })
        .collect()
}

pub fn project_name_from_path(dest_path: &Path) -> Result<String> {
    dest_path
        .file_name()
        .and_then(|name| name.to_str())
        .map(|s| s.to_string())
        .ok_or_else(|| {
            anyhow!(
                "Unable to determine project name from destination {}",
                dest_path.display()
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonicalizes_env_keys() {
        assert_eq!(canonical_env_key("crate_type"), "CRATE_TYPE");
        assert_eq!(canonical_env_key("with-hyphen"), "WITH_HYPHEN");
    }

    #[test]
    fn project_name_extraction() {
        let path = Path::new("/tmp/example");
        assert_eq!(project_name_from_path(path).unwrap(), "example");
    }
}
