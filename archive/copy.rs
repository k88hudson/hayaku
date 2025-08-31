use anyhow::{Context as AnyhowContext, Result};
use ignore::{WalkBuilder, overrides::OverrideBuilder};
use tera::Context as TeraContext;

use crate::{config::HayakuConfig, templating};

pub fn copy_local(
    src: &std::path::Path,
    dest: &std::path::Path,
    context: &TeraContext,
) -> Result<()> {
    if !dest.exists() {
        std::fs::create_dir_all(dest)?;
    }

    let mut overrides = OverrideBuilder::new(".");
    overrides.add("!.git/").unwrap();
    let overrides = overrides.build().unwrap();

    for entry in WalkBuilder::new(src)
        .hidden(false)
        .overrides(overrides)
        .build()
    {
        let entry = entry?;

        if entry.file_type().map(|ft| ft.is_file()).unwrap_or(false) {
            let src_path = entry.path();
            let relative_path = src_path.strip_prefix(src)?;
            let is_tera = matches!(
                src_path.extension().and_then(|ext| ext.to_str()),
                Some(ext) if ext.eq_ignore_ascii_case("tera")
            );
            let output_relpath = if is_tera {
                relative_path.with_extension("")
            } else {
                relative_path.to_path_buf()
            };

            println!("=> {}", output_relpath.display());

            let dest_path = dest.join(&output_relpath);

            if let Some(parent) = dest_path.parent() {
                std::fs::create_dir_all(parent)?;
            }

            let rendered = templating::render_file(src_path, context)?;
            std::fs::write(&dest_path, rendered)
                .with_context(|| format!("Failed to write file {}", dest_path.display()))?;
        }
    }
    Ok(())
}

pub fn render_templates_in_dir(
    template_dir: &std::path::Path,
    dest_dir: &std::path::Path,
    template_config: &HayakuConfig,
) -> Result<()> {
    let mut tera_files = Vec::new();

    for entry in WalkBuilder::new(root).hidden(false).build() {
        let entry = entry?;
        if entry.file_type().map(|ft| ft.is_file()).unwrap_or(false) {
            tera_files.push(entry.into_path());
        }
    }

    for src_path in tera_files {
        let relative_path = src_path.strip_prefix(root)?;
        let is_tera = matches!(
            src_path.extension().and_then(|ext| ext.to_str()),
            Some(ext) if ext.eq_ignore_ascii_case("tera")
        );
        let output_relpath = if is_tera {
            relative_path.with_extension("")
        } else {
            relative_path.to_path_buf()
        };
        let dest_path = root.join(&output_relpath);
        if let Some(parent) = dest_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let rendered = templating::render_file(&src_path, context)?;
        std::fs::write(&dest_path, rendered)
            .with_context(|| format!("Failed to write file {}", dest_path.display()))?;

        if dest_path != src_path {
            std::fs::remove_file(&src_path)
                .with_context(|| format!("Failed to remove template {}", src_path.display()))?;
        }

        println!("=> {}", output_relpath.display());
    }

    Ok(())
}
