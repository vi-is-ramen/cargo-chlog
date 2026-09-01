//! Configuration parsing module.

mod pat;
use std::path::Path;

pub use pat::*;

mod commits;
pub use commits::*;

mod log;
pub use log::*;
use serde::Deserialize;

/// Root configuration structure for the cargo-chlog tool.
#[derive(Deserialize, Debug)]
pub struct Config {
    /// Configuration related to commit linting and filtering.
    pub commits: Commits,
    /// Configuration related to changelog generation.
    pub log: LogConfig,
}

/// Parses the configuration from the given file path.
///
/// If the file cannot be read or parsed, it invokes `std::process::exit`
/// with a non-zero status code and prints a detailed error message.
///
/// # Arguments
///
/// * `path` - The path to the TOML configuration file.
///
/// # Returns
///
/// A parsed [`Config`] instance.
pub fn parse_from<P>(path: P) -> Config
where
    P: AsRef<Path>,
{
    let source = match std::fs::read_to_string(&path) {
        Ok(source) => source,
        Err(err) => {
            crate::error(&format!("failed to read config file: {}", err));
            std::process::exit(1);
        }
    };
    let config: Result<Config, _> = toml::from_str(&source);

    match config {
        Err(err) => {
            eprintln!(
                "{}",
                crate::ANS_RENDERER.render(&[ans::Group::with_title(
                    ans::Level::ERROR.primary_title("failed to parse config")
                )
                .element(
                    ans::Snippet::source(source)
                        .path(AsRef::<Path>::as_ref(&path).to_str())
                        .annotation(
                            ans::AnnotationKind::Primary
                                .span(err.span().unwrap())
                                .label(err.message())
                        ),
                ),])
            );
            std::process::exit(1);
        }
        Ok(config) => config,
    }
}

/// Parses the configuration from the "./Chlog.toml" file in the current directory.
///
/// If the file cannot be read or parsed, it invokes `std::process::exit`
/// with a non-zero status code.
///
/// # Returns
///
/// A parsed [`Config`] instance.
pub fn parse() -> Config {
    parse_from("Chlog.toml")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    fn create_temp_toml(content: &str) -> PathBuf {
        let dir = std::env::temp_dir();
        let path = dir.join(format!("cargo-chlog-test-{}.toml", std::process::id()));
        fs::write(&path, content).unwrap();
        path
    }

    #[test]
    fn test_parse_from_valid() {
        let content = r#"
            [commits.types]
            feat = "Feature"

            [log]
            include_commit_url = true
        "#;
        let path = create_temp_toml(content);
        let config = parse_from(&path);
        assert_eq!(config.commits.types["feat"], "Feature");
        assert_eq!(config.log.include_commit_url, true);
        let _ = fs::remove_file(path);
    }
}
