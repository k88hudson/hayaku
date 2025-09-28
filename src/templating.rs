use crate::config::HayakuConfig;
use anyhow::{Context as AnyhowContext, Result};
use ignore::WalkBuilder;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use tera::{Context as TeraContext, Tera};

pub fn create_project(
    template_dir: &Path,
    dest_dir: &Path,
    _template_config: &HayakuConfig,
) -> Result<()> {
    let mut tera = Tera::default();
    let mut context = TeraContext::new();

    let project_name = dest_dir
        .file_name()
        .and_then(|c| c.to_str())
        .ok_or_else(|| {
            anyhow::anyhow!(
                "Unable to determine project name from destination {}",
                dest_dir.display()
            )
        })?;
    context.insert("project_name", project_name);
    context.insert("PROJECT_NAME", project_name);

    if !dest_dir.exists() {
        fs::create_dir_all(dest_dir).with_context(|| {
            format!(
                "Failed to create destination directory {}",
                dest_dir.display()
            )
        })?;
    }

    let mut walker = WalkBuilder::new(template_dir);
    walker
        .git_ignore(true)
        .hidden(false)
        .filter_entry(|entry| entry.file_name() != OsStr::new(".git"));

    for entry in walker.build() {
        let entry = entry?;
        if !entry.file_type().map(|ft| ft.is_file()).unwrap_or(false) {
            continue;
        }
        let rel_path = entry.path().strip_prefix(template_dir)?;
        let dest_rel: PathBuf = rel_path.to_path_buf();
        let dest_path = dest_dir.join(&dest_rel);

        render_from_template_file(entry.path(), &dest_path, &mut tera, &context)?;
    }
    Ok(())
}

fn render_from_template_file(
    template_file: &Path,
    dest_path: &Path,
    tera: &mut Tera,
    context: &TeraContext,
) -> Result<()> {
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
    use std::fs;

    fn config(id: &str) -> HayakuConfig {
        HayakuConfig {
            name: id.to_string(),
            display_name: None,
            description: None,
            author: None,
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

        create_project(template_dir.path(), &dest_dir, &config("some_template")).unwrap();

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

        create_project(template_dir.path(), &dest_dir, &config("demo")).unwrap();

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
}
