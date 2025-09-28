mod cli;
mod config;
mod env;
mod git;
mod local_templates;
mod templating;

fn main() {
    cli::run().unwrap_or_else(|err| {
        eprintln!("Error: {}", err);
        std::process::exit(1);
    });
}
