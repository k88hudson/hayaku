mod cli;
mod config;
mod env;
mod git;
mod hayaku_context;
pub use hayaku_context::Hayaku;
mod templating;

fn main() {
    cli::run().unwrap_or_else(|err| {
        eprintln!("Error: {}", err);
        std::process::exit(1);
    });
}
