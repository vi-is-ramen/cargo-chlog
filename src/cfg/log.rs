//! Configuration structure for changelog generation.

use serde::Deserialize;

/// Configuration for changelog generation.
#[derive(Debug, Deserialize, PartialEq)]
pub struct LogConfig {
    /// Whether to include commit URLs in the changelog.
    #[serde(default)]
    pub include_commit_url: bool,

    /// Whether to include commit hashes in the changelog.
    #[serde(default)]
    pub include_commit_hash: bool,

    /// Whether to separate change lists by scope using subheadings.
    #[serde(default)]
    pub separate_scope_lists: bool,

    /// Whether to collect and display a list of contributors.
    #[serde(default)]
    pub collect_thanks: bool,

    /// The subtitle text for the contributors/thanks section.
    pub thanks_subtitle: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_log_config_defaults() {
        let toml_str = "";
        let log: LogConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(log.include_commit_url, false);
        assert_eq!(log.include_commit_hash, false);
        assert_eq!(log.separate_scope_lists, false);
        assert_eq!(log.collect_thanks, false);
        assert_eq!(log.thanks_subtitle, None);
    }

    #[test]
    fn test_deserialize_log_config_custom() {
        let toml_str = r#"
            include_commit_url = true
            include_commit_hash = true
            separate_scope_lists = true
            collect_thanks = true
            thanks_subtitle = "Contributors"
        "#;
        let log: LogConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(log.include_commit_url, true);
        assert_eq!(log.include_commit_hash, true);
        assert_eq!(log.separate_scope_lists, true);
        assert_eq!(log.collect_thanks, true);
        assert_eq!(log.thanks_subtitle, Some("Contributors".to_string()));
    }
}
