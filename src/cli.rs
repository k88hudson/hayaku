use crate::config::HayakuConfig;
use crate::local_templates::LocalTemplates;
use crate::templating;
use anyhow::{Result, anyhow, bail};
use clap::{Parser, Subcommand, ValueEnum, command};
use cliclack;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "Hayaku",
    bin_name = "hayaku",
    author = "k88hudson <k88hudson@gmail.com>",
    version = env!("CARGO_PKG_VERSION"),
)]
pub struct Hayaku {
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
}

#[derive(ValueEnum, Clone, Debug, Default)]
pub enum Source {
    #[default]
    Local,
    GitHub,
    TemplateDir,
}

#[derive(Parser, Debug)]
pub struct CreateOptions {
    /// The path where the new project should be created
    #[arg(long, short)]
    project_path: Option<String>,

    /// The template to use for the new project
    /// Templates must be in your local hayaku template directory,
    /// or one of the hayaku built-in templates.
    #[arg(short, long, conflicts_with_all = ["github", "template_dir"])]
    template: Option<Option<String>>,

    /// A Github repository in the form owner/repo
    #[arg(short, long,  conflicts_with_all = ["template", "template_dir"])]
    github: Option<Option<String>>,

    /// A directory containing a hayaku template
    #[arg(long, conflicts_with_all = ["template", "github"])]
    template_dir: Option<Option<PathBuf>>,

    /// Overwrite existing files in the destination directory
    #[arg(short, long)]
    force: bool,
}

fn validate_github_repo(repo: &str) -> Result<()> {
    if !repo.contains("/") {
        bail!("GitHub repository must be in the form owner/repo");
    }
    Ok(())
}

fn validate_directory(path: &PathBuf) -> Result<()> {
    if !path.is_dir() {
        bail!("The path {} is not a directory", path.display());
    }
    Ok(())
}

fn create(create_options: &CreateOptions) -> Result<()> {
    cliclack::intro("hayaku!")?;

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
    } else if let Some(github) = &create_options.github {
        if let Some(repo) = github {
            validate_github_repo(repo)?;
            PathBuf::new() // Placeholder, will be handled as GitHub template
        } else {
            // Ask for the repository name
            // let repo_name: String = cliclack::input("Repository name in the form owner/repo")
            //     .validate(|val: &String| validate_github_repo(val))
            //     .interact()?;
            PathBuf::new() // Placeholder, will be handled as GitHub template
        }
    } else {
        let local_templates = LocalTemplates::try_new()?;
        if local_templates.is_empty() {
            bail!("No local templates found. Please add a template first.");
        }
        let selection: String = cliclack::select("Select a template")
            .items(
                &local_templates
                    .templates()
                    .values()
                    .into_iter()
                    .map(|t| {
                        let HayakuConfig {
                            name: id,
                            display_name,
                            description,
                            ..
                        } = &t.config;
                        (
                            id.clone(),
                            display_name.clone().unwrap_or(id.clone()),
                            description.clone().unwrap_or("".to_string()),
                        )
                    })
                    .collect::<Vec<(String, String, String)>>(),
            )
            .interact()?;

        let selected_template = local_templates
            .get(&selection)
            .ok_or_else(|| anyhow!("Selected template '{}' not found", selection))?;
        selected_template.path.clone()
    };

    let template_config = HayakuConfig::try_from_dir(&template_path)?;
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

    templating::create_project(&template_path, &dest_path, &template_config)?;
    cliclack::log::info(format!(
        "Project created successfully! Now run:\ncd {} ",
        &project_path_str
    ))?;
    Ok(())
}

pub fn run() -> Result<()> {
    let cli = Hayaku::try_parse()?;

    match cli.command {
        Commands::Create(ref create_options) => create(create_options),
        Commands::List => {
            let local_templates = LocalTemplates::try_new()?;

            cliclack::log::info(format!(
                "Available templates in {}:",
                local_templates.local_template_dir().display()
            ))?;
            for (_, template) in local_templates.templates() {
                cliclack::log::info(format!(
                    "{}: {}",
                    template.config.name,
                    template
                        .config
                        .description
                        .as_ref()
                        .unwrap_or(&"".to_string())
                ))?;
            }
            Ok(())
        }
        Commands::Edit => {
            let local_templates = LocalTemplates::try_new()?;
            std::process::Command::new("code")
                .arg(local_templates.hayaku_dir())
                .status()
                .map_err(|e| anyhow::anyhow!("Failed to open code editor: {}", e))?;

            Ok(())
        }
    }
}
