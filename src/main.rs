//! The entry point for the `cargo-chlog` binary.

use cargo_chlog::{cfg, cli, error, git, lint, log};
use clap::Parser;

/// The main entry point for the `cargo-chlog` application.
///
/// Parses CLI arguments, loads the configuration, fetches the Git history,
/// and dispatches the execution to either the `check` or `log` subcommand.
fn main() {
    let cli = cli::Cli::parse();

    // 1. Parse the configuration
    let config = match cli.config {
        Some(path) => cfg::parse_from(path),
        None => cfg::parse(),
    };

    // 2. Fetch the commit history
    let commits = match git::history(cli.since.as_deref()) {
        Ok(c) => c,
        Err(e) => {
            error(&format!("Failed to fetch commit history: {}", e));
            std::process::exit(1);
        }
    };

    // 3. Execute the requested command
    match cli.command {
        cli::Command::Check => match lint::lint(commits, &config) {
            Ok(_) => {
                println!("Success: All commits follow Conventional Commits specification.");
            }
            Err(e) => {
                error(&format!("Linting failed: {}", e));
                std::process::exit(1);
            }
        },
        cli::Command::Log => {
            // We must pass the commits through the linter first to ensure
            // they adhere to conventions and are properly structured.
            match lint::lint(commits, &config) {
                Ok(linted_commits) => {
                    let markdown = log::generate_markdown(&linted_commits, &config.log);
                    println!("{}", markdown);
                }
                Err(e) => {
                    error(&format!("Linting failed: {}", e));
                    std::process::exit(1);
                }
            }
        }
    }
}
