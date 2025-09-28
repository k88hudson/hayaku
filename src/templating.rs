use anyhow::{Context as AnyhowContext, Result};
use ignore::WalkBuilder;
use ignore::overrides::OverrideBuilder;
use std::fs;
use std::path::{Path, PathBuf};
use tera::{Context as TeraContext, Tera};

pub fn create_project(template_dir: &Path, dest_dir: &Path, context: &TeraContext) -> Result<()> {
    let mut tera = Tera::default();

    if !dest_dir.exists() {
        fs::create_dir_all(dest_dir).with_context(|| {
            format!(
                "Failed to create destination directory {}",
                dest_dir.display()
            )
        })?;
    }

    let mut overrides = OverrideBuilder::new(".");
    overrides.add("!**/.git")?;
    overrides.add("!**/hayaku.toml")?;
    let overrides = overrides.build()?;

    let mut walker = WalkBuilder::new(template_dir);
    walker.git_ignore(true).hidden(false).overrides(overrides);

    for entry in walker.build() {
        let entry = entry?;
        if !entry.file_type().map(|ft| ft.is_file()).unwrap_or(false) {
            continue;
        }
        let rel_path = entry.path().strip_prefix(template_dir)?;
        let dest_rel: PathBuf = rel_path.to_path_buf();
        let dest_path = dest_dir.join(&dest_rel);

        render_from_template_file(entry.path(), &dest_path, &mut tera, context)?;
    }
    Ok(())
}

fn process_dest_path(dest_path: &Path, context: &TeraContext) -> PathBuf {
    let components = dest_path.components().map(|comp| {
        let comp_str = comp.as_os_str().to_string_lossy();
        if comp_str.starts_with('[') && comp_str.ends_with(']') {
            let var_name = &comp_str[1..comp_str.len() - 1];
            if let Some(value) = context.get(var_name) {
                if let Some(s) = value.as_str() {
                    return PathBuf::from(s);
                }
            }
        }
        PathBuf::from(comp.as_os_str())
    });
    components.collect::<PathBuf>()
}

fn render_from_template_file(
    template_file: &Path,
    dest_path: &Path,
    tera: &mut Tera,
    context: &TeraContext,
) -> Result<()> {
    let mut dest_path = process_dest_path(dest_path, context);

    if dest_path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.eq_ignore_ascii_case("tera"))
        .unwrap_or(false)
    {
        dest_path.set_extension("");
    }

    if let Some(parent) = dest_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create parent directory {}", parent.display()))?;
    }
    let contents = fs::read_to_string(template_file)?;
    let rendered = tera.render_str(&contents, &context).map_err(|e| {
        anyhow::anyhow!(
            "Failed to render template file {}:\n{:?}",
            template_file.display(),
            e
        )
    })?;

    fs::write(&dest_path, rendered)
        .with_context(|| format!("Failed to write rendered file {}", dest_path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{collections::HashMap, fs, path::Path};

    use crate::config::HayakuConfig;
    use crate::env;

    fn config(id: &str) -> HayakuConfig {
        HayakuConfig {
            name: id.to_string(),
            display_name: None,
            description: None,
            author: None,
            env: HashMap::new(),
        }
    }

    fn write_template(dir: &Path, rel: &str, contents: &[u8]) {
        let path = dir.join(rel);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, contents).unwrap();
    }

    #[test]
    fn renders_template_variables() {
        let template_dir = tempfile::tempdir().unwrap();
        let dest_dir = tempfile::tempdir().unwrap().path().join("demo");

        write_template(template_dir.path(), "file.txt", b"Hello {{ project_name }}");
        write_template(
            template_dir.path(),
            "nested/config.toml",
            b"name = \"{{ PROJECT_NAME }}\"",
        );

        let env_values = HashMap::new();
        let context = env::build_context("demo", &config("some_template"), &env_values);

        create_project(template_dir.path(), &dest_dir, &context).unwrap();

        assert_eq!(
            fs::read_to_string(dest_dir.join("file.txt")).unwrap(),
            "Hello demo"
        );
        assert_eq!(
            fs::read_to_string(dest_dir.join("nested/config.toml")).unwrap(),
            "name = \"demo\""
        );
    }

    #[test]
    fn create_project_renders_and_respects_ignores() {
        let template_dir = tempfile::tempdir().unwrap();
        let dest_dir = tempfile::tempdir().unwrap().path().join("demo");

        fs::create_dir_all(template_dir.path().join("nested")).unwrap();
        fs::create_dir_all(template_dir.path().join(".git")).unwrap();

        write_template(template_dir.path(), ".gitignore", b"ignored.txt\n");
        write_template(template_dir.path(), "file.txt", b"Hello {{ project_name }}");
        write_template(
            template_dir.path(),
            "nested/config.toml",
            b"name = \"{{ PROJECT_NAME }}\"",
        );
        write_template(template_dir.path(), "ignored.txt", b"nope");
        write_template(template_dir.path(), ".git/config", b"secret");

        let env_values = HashMap::new();
        let context = env::build_context("demo", &config("demo"), &env_values);

        create_project(template_dir.path(), &dest_dir, &context).unwrap();

        assert_eq!(
            fs::read_to_string(dest_dir.join("file.txt")).unwrap(),
            "Hello demo"
        );
        assert_eq!(
            fs::read_to_string(dest_dir.join("nested/config.toml")).unwrap(),
            "name = \"demo\""
        );
        assert!(!dest_dir.join("ignored.txt").exists());
        assert!(!dest_dir.join(".git").exists());
    }

    #[test]
    fn process_dest_path_substitutes_with_context() {
        let mut context = TeraContext::new();
        context.insert("project_name", "demo");

        let dest = Path::new("output/[PROJECT_NAME]/config.toml");
        let resolved = super::process_dest_path(dest, &context);

        assert_eq!(resolved, Path::new("output/demo/config.toml"));
    }
}
