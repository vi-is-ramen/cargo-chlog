//! Command-line interface definitions using `clap`.

use clap::Parser;
use std::path::PathBuf;

/// Command-line interface arguments for `cargo-chlog`.
#[derive(Parser, Debug)]
#[clap(name = "cargo-chlog")]
pub struct Cli {
    /// Subcommand to execute.
    #[clap(subcommand)]
    pub command: Command,

    /// Path to the configuration file.
    ///
    /// By default, the configuration file is searched as "Chlog.toml" in the
    /// current working directory.
    #[clap(short, long, global = true)]
    pub config: Option<PathBuf>,

    /// Specification from which to start generating changelog entries.
    ///
    /// Allowed notations:
    /// * tag name: start from the specified tag;
    /// * hash: start from the specified commit;
    /// * `~N` or `HEAD~N`: start from the Nth commit before the current one.
    #[clap(short, long, global = true)]
    pub since: Option<String>,
}

/// Available subcommands for `cargo-chlog`.
#[derive(Parser, Debug, PartialEq)]
pub enum Command {
    /// Check the commit history to ensure it follows the changelog conventions.
    #[clap(name = "check")]
    Check,

    /// Generate a changelog from the commit history.
    #[clap(name = "log")]
    Log,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_check_command() {
        let args = vec!["cargo-chlog", "check"];
        let cli = Cli::try_parse_from(args)
            .map_err(|e| panic!("{e}"))
            .unwrap();
        assert_eq!(cli.command, Command::Check);
    }

    #[test]
    fn test_cli_log_command_with_since() {
        let args = vec!["cargo-chlog", "log", "--since", "v1.0.0"];
        let cli = Cli::try_parse_from(args)
            .map_err(|e| panic!("{e}"))
            .unwrap();
        assert_eq!(cli.command, Command::Log);
        assert_eq!(cli.since, Some("v1.0.0".to_string()));
    }

    #[test]
    fn test_cli_custom_config() {
        let args = vec!["cargo-chlog", "check", "--config", "custom.toml"];
        let cli = Cli::try_parse_from(args)
            .map_err(|e| panic!("{e}"))
            .unwrap();
        assert_eq!(cli.config, Some(PathBuf::from("custom.toml")));
    }
}
