use std::collections::HashMap;
use std::path::Path;

use anyhow::{Result, anyhow};
use cliclack;
use tera::Context as TeraContext;

use crate::config::{EnvVarConfig, HayakuConfig};

pub fn prompt_for_env(config: &HayakuConfig) -> Result<HashMap<String, String>> {
    if config.env.is_empty() {
        return Ok(HashMap::new());
    }

    let mut values = HashMap::new();
    let mut keys: Vec<_> = config.env.keys().cloned().collect();
    keys.sort();

    for key in keys {
        let env_cfg = config
            .env
            .get(&key)
            .expect("Key fetched from known iterator");

        let value = match &env_cfg {
            EnvVarConfig::String { prompt, default } => {
                let mut input = cliclack::input(prompt).required(true);
                if let Some(default) = default {
                    input = input.default_input(default);
                }
                input.interact::<String>()?
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
                input.interact()?
            }
            EnvVarConfig::Bool { prompt, default } => {
                let mut confirm = cliclack::confirm(prompt);
                if *default {
                    confirm = confirm.initial_value(*default)
                }
                let confirmed = confirm.interact()?;
                confirmed.to_string()
            }
        };

        values.insert(key, value);
    }

    Ok(values)
}

pub fn build_context(
    project_name: &str,
    config: &HayakuConfig,
    env_values: &HashMap<String, String>,
) -> TeraContext {
    let mut context = TeraContext::new();
    context.insert("project_name", project_name);
    context.insert("PROJECT_NAME", project_name);

    for (key, value) in env_values {
        context.insert(key, value);
        let canonical = canonical_env_key(key);
        context.insert(&canonical, value);
    }

    // Also expose the template name to templates that may rely on it.
    context.insert("template_name", &config.name);

    context
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
    use crate::config::EnvVarConfig;

    fn sample_config() -> HayakuConfig {
        let mut env = HashMap::new();
        env.insert(
            "crate_type".to_string(),
            EnvVarConfig::Choices {
                prompt: "Crate type".into(),
                choices: vec!["lib".into(), "bin".into()],
                default: Some("bin".into()),
            },
        );
        HayakuConfig {
            name: "sample".into(),
            display_name: None,
            description: None,
            author: None,
            env,
        }
    }

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
