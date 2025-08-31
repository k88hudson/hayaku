use anyhow::{Context as AnyhowContext, Result};
use ignore::WalkBuilder;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use tera::{Context as TeraContext, Tera};

use crate::config::HayakuConfig;

pub fn create_project(
    template_dir: &Path,
    dest_dir: &Path,
    template_config: &HayakuConfig,
) -> Result<()> {
    let mut tera = Tera::default();
    let mut context = TeraContext::new();
    context.insert("project_name", &template_config.name);
    context.insert("PROJECT_NAME", &template_config.name);

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
            "Failed to render template file {}:\n{}",
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

    fn context_with_name() -> TeraContext {
        let mut ctx = TeraContext::new();
        ctx.insert("name", "World");
        ctx
    }

    #[test]
    fn renders_template_variables() {
        let ctx = context_with_name();
        let rendered = render_bytes(b"Hello {{ name }}".to_vec(), &ctx).unwrap();
        assert_eq!(String::from_utf8(rendered).unwrap(), "Hello World");
    }

    #[test]
    fn skips_templating_when_directive_present() {
        let ctx = context_with_name();
        let rendered = render_bytes(
            b"// hayaku: skip-templating\nHello {{ name }}".to_vec(),
            &ctx,
        )
        .unwrap();
        assert_eq!(String::from_utf8(rendered).unwrap(), "Hello {{ name }}");
    }

    #[test]
    fn skip_directive_handles_windows_newline() {
        let ctx = context_with_name();
        let rendered = render_bytes(
            b"// hayaku: skip-templating\r\nHello {{ name }}".to_vec(),
            &ctx,
        )
        .unwrap();
        assert_eq!(String::from_utf8(rendered).unwrap(), "Hello {{ name }}");
    }

    #[test]
    fn skip_directive_requires_comment_prefix() {
        let ctx = context_with_name();
        let rendered =
            render_bytes(b"hayaku: skip-templating\nHello {{ name }}".to_vec(), &ctx).unwrap();
        assert_eq!(
            String::from_utf8(rendered).unwrap(),
            "hayaku: skip-templating\nHello World"
        );
    }

    #[test]
    fn passes_through_binary_content() {
        let ctx = TeraContext::new();
        let bytes = vec![0u8, 159, 255, 42];
        let rendered = render_bytes(bytes.clone(), &ctx).unwrap();
        assert_eq!(rendered, bytes);
    }

    #[test]
    fn create_project_renders_and_respects_ignores() {
        let template_dir = tempfile::tempdir().unwrap();
        let dest_dir = tempfile::tempdir().unwrap();

        fs::create_dir_all(template_dir.path().join("nested")).unwrap();
        fs::create_dir_all(template_dir.path().join(".git")).unwrap();

        fs::write(template_dir.path().join(".gitignore"), "ignored.txt\n").unwrap();
        fs::write(
            template_dir.path().join("file.txt"),
            "Hello {{ project_name }}",
        )
        .unwrap();
        fs::write(
            template_dir.path().join("nested").join("config.toml.tera"),
            "name = \"{{ PROJECT_NAME }}\"",
        )
        .unwrap();
        fs::write(template_dir.path().join("ignored.txt"), "nope").unwrap();
        fs::write(template_dir.path().join(".git").join("config"), "secret").unwrap();

        let config = HayakuConfig {
            name: "demo".to_string(),
            display_name: None,
            description: None,
            author: None,
        };

        create_project(template_dir.path(), dest_dir.path(), &config).unwrap();

        let rendered = fs::read_to_string(dest_dir.path().join("file.txt")).unwrap();
        assert_eq!(rendered, "Hello demo");

        let nested_rendered =
            fs::read_to_string(dest_dir.path().join("nested").join("config.toml")).unwrap();
        assert_eq!(nested_rendered, "name = \"demo\"");

        assert!(!dest_dir.path().join("ignored.txt").exists());
        assert!(!dest_dir.path().join(".git").exists());
    }
}
