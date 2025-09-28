use crate::config::TemplateConfig;
use crate::env;
use crate::hayaku_context::TemplateOrigin;
use crate::templating;
use crate::{Hayaku, hayaku_context::HayakuSettings};
use anyhow::{Result, anyhow, bail};
use clap::{Parser, Subcommand, ValueEnum, command};
use cliclack;
use owo_colors::OwoColorize;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "Hayaku",
    bin_name = "hayaku",
    author = "k88hudson <k88hudson@gmail.com>",
    version = env!("CARGO_PKG_VERSION"),
)]
pub struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    #[command(name = "create", about = "Create a new project")]
    Create(CreateOptions),

    #[command(name = "list", about = "List available templates")]
    List,

    #[command(name = "edit", about = "Edit templates")]
    Edit,

    #[command(name = "init", about = "Set up hayaku")]
    Init,
}

#[derive(ValueEnum, Clone, Debug, Default)]
pub enum Source {
    #[default]
    Local,
    TemplateDir,
}

#[derive(Parser, Debug)]
pub struct CreateOptions {
    /// The path where the new project should be created
    #[arg(value_name = "PATH")]
    project_path: Option<String>,

    /// The template to use for the new project
    /// Templates must be in your local hayaku template directory,
    /// or one of the hayaku built-in templates.
    #[arg(short, long, conflicts_with_all = ["template_dir"])]
    template: Option<Option<String>>,

    /// A directory containing a hayaku template
    #[arg(long, conflicts_with_all = ["template"])]
    template_dir: Option<Option<PathBuf>>,

    /// Overwrite existing files in the destination directory
    #[arg(short, long)]
    force: bool,
}

// fn validate_github_repo(repo: &str) -> Result<()> {
//     if !repo.contains("/") {
//         bail!("GitHub repository must be in the form owner/repo");
//     }
//     Ok(())
// }

fn validate_directory(path: &PathBuf) -> Result<()> {
    if !path.is_dir() {
        bail!("The path {} is not a directory", path.display());
    }
    Ok(())
}

fn create(create_options: &CreateOptions) -> Result<()> {
    let hayaku = Hayaku::try_new()?;

    let template_message = if hayaku.no_local_templates() {
        "No local templates; using built-in templates only".to_string()
    } else {
        format!(
            "Loaded templates from {}",
            hayaku.local_template_dir().display()
        )
    };

    cliclack::log::info(format!("hayaku!\n{}", template_message.dimmed()))?;

    let project_path_str = create_options
        .project_path
        .as_ref()
        .map(|s| Ok(s.clone()))
        .unwrap_or_else(|| {
            cliclack::input("Directory for the new project")
                .validate(|val: &String| {
                    if val.is_empty() {
                        Err("Value is required")
                    } else {
                        Ok(())
                    }
                })
                .interact()
        })?;

    let dest_path = PathBuf::from(&project_path_str);
    if dest_path.exists() && !create_options.force {
        let should_overwrite = cliclack::confirm(format!(
            "Directory {} already exists. Overwrite?",
            dest_path.display()
        ))
        .interact()?;
        if should_overwrite {
            std::fs::remove_dir_all(&dest_path)?;
        } else {
            return Err(anyhow::anyhow!("Aborted by user"));
        }
    }

    let template_path: PathBuf = if let Some(template_dir) = &create_options.template_dir {
        if let Some(cli_defined) = template_dir {
            validate_directory(cli_defined)?;
            cli_defined.clone()
        } else {
            // Ask for the directory
            let dir: String = cliclack::input("Template directory")
                .validate(|val: &String| validate_directory(&PathBuf::from(val)))
                .interact()?;
            PathBuf::from(dir)
        }
    } else {
        let template_items: Vec<(String, String, String)> = hayaku
            .all_templates()
            .into_iter()
            .map(|t| {
                let id = t.config.name.clone();
                let display_name = t
                    .config
                    .display_name
                    .clone()
                    .unwrap_or_else(|| t.config.name.clone());
                let label = match t.origin {
                    TemplateOrigin::BuiltIn => format!("{display_name} [built-in]"),
                    TemplateOrigin::Local => display_name,
                };
                let description = t
                    .config
                    .description
                    .clone()
                    .unwrap_or_else(|| "".to_string());
                (id, label, description)
            })
            .collect();

        let selection: String = cliclack::select(format!(
            "Choose a template: {}",
            "(Type to search)".dimmed()
        ))
        .items(&template_items)
        .filter_mode()
        .interact()?;

        let selected_template = hayaku
            .get(&selection)
            .ok_or_else(|| anyhow!("Selected template '{}' not found", selection))?;
        selected_template.path.clone()
    };

    let template_config = TemplateConfig::try_from_dir(&template_path)?;

    let project_name = env::project_name_from_path(&dest_path)?;
    let context = env::build_context(&project_name, &template_config, &hayaku)?;

    templating::create_project(&template_path, &dest_path, &context)?;
    cliclack::log::success(format!(
        "{} Your project {} is ready.",
        "Success!".green(),
        project_path_str.bold()
    ))?;
    Ok(())
}

pub fn run() -> Result<()> {
    let cli = Cli::try_parse()?;
    let hayaku = Hayaku::try_new()?;
    match cli.command {
        Commands::Init => {}
        _ => {
            if !hayaku.hayaku_dir().exists() {
                cliclack::log::warning(
                    "Consider running hayaku init to set up your local hayaku templates directory"
                        .yellow(),
                )?;
            }
        }
    }

    match cli.command {
        Commands::Create(ref create_options) => create(create_options),
        Commands::Init => init(),
        Commands::List => {
            let visible_built_ins: Vec<_> = hayaku
                .built_in_templates()
                .values()
                .filter(|template| !hayaku.templates().contains_key(&template.config.name))
                .collect();

            if visible_built_ins.is_empty() {
                cliclack::log::info("No built-in templates found.")?;
            } else {
                cliclack::log::info(format!(
                    "Built-in {}\n{}",
                    "(Templates that ship with hayaku)".dimmed(),
                    visible_built_ins
                        .iter()
                        .map(|t| format!(
                            "· {} {}",
                            t.config.name.bold(),
                            t.config
                                .description
                                .clone()
                                .unwrap_or_else(|| "".to_string())
                        ))
                        .collect::<Vec<_>>()
                        .join("\n")
                ))?;
            }
            if hayaku.templates().is_empty() {
                cliclack::log::info(format!(
                    "No local templates found in {}",
                    hayaku.local_template_dir().display()
                ))?;
            } else {
                cliclack::log::info(format!(
                    "Local templates {}\n{}",
                    hayaku.local_template_dir().display().dimmed(),
                    hayaku
                        .templates()
                        .values()
                        .map(|t| format!(
                            "· {} {}",
                            t.config.name.bold(),
                            t.config
                                .description
                                .clone()
                                .unwrap_or_else(|| "".to_string())
                        ))
                        .collect::<Vec<_>>()
                        .join("\n")
                ))?;
            }
            Ok(())
        }
        Commands::Edit => {
            let local_templates = Hayaku::try_new()?;
            std::process::Command::new("code")
                .arg(local_templates.hayaku_dir())
                .status()
                .map_err(|e| anyhow::anyhow!("Failed to open code editor: {}", e))?;

            Ok(())
        }
    }
}

fn init() -> Result<()> {
    let hayaku = Hayaku::try_new()?;
    if hayaku.local_template_dir().exists() {
        cliclack::log::success(format!(
            "Template directory found: {}",
            hayaku.local_template_dir().display()
        ))?;
    } else if cliclack::confirm(format!(
        "Create your hayaku template directory at {}?",
        hayaku.local_template_dir().display()
    ))
    .interact()?
    {
        std::fs::create_dir_all(hayaku.local_template_dir())?;
    }

    if hayaku.settings_config_path().exists() {
        cliclack::log::success(format!(
            "Global settings found: {}",
            hayaku.settings_config_path().display()
        ))?;
    } else {
        cliclack::log::info(format!(
            "Creating a global settings file at {}",
            hayaku.settings_config_path().display()
        ))?;
        let default_license = cliclack::select("Choose a default license for new projects")
            .items(&[
                ("MIT OR Apache-2.0", "MIT or Apache-2.0", ""),
                ("Apache-2.0", "Apache License 2.0", ""),
                ("MIT", "MIT License", ""),
                ("MPL-2.0", "Mozilla Public License 2.0", ""),
                ("GPL-3.0", "GNU General Public License v3.0", ""),
            ])
            .initial_value("MIT OR Apache-2.0")
            .interact()?;

        let settings = HayakuSettings {
            global_env: Some(std::collections::HashMap::from([(
                "LICENSE".to_string(),
                toml::Value::String(default_license.to_string()),
            )])),
        };
        settings.write_to_file(hayaku.settings_config_path())?;
    }

    cliclack::log::success("Done! Next, try this to generate a new project:\nhayaku create")?;

    Ok(())
}
