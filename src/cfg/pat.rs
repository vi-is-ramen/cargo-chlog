//! Pattern matching structures for commit message filtering.

use serde::Deserialize;

/// Specifies how to find a string within a text.
#[derive(Debug, Deserialize, PartialEq, Clone)]
pub enum Find {
    /// Regex pattern to find.
    #[serde(rename = "regex")]
    Regex(String),

    /// Exact string to find.
    #[serde(untagged)]
    Exact(String),
}

/// A pattern for matching string values.
#[derive(Debug, Deserialize, PartialEq, Clone)]
pub enum Pat {
    /// Prefix for matching.
    #[serde(rename = "prefix")]
    Prefix(String),

    /// Suffix for matching.
    #[serde(rename = "suffix")]
    Suffix(String),

    /// Invert the match.
    #[serde(rename = "not")]
    Not(Box<Self>),

    /// Find instead of matching.
    #[serde(rename = "find")]
    Find(Find),

    /// Regular expression pattern for matching.
    #[serde(rename = "regex")]
    Regex(String),

    /// Exact string for matching.
    #[serde(untagged)]
    Exact(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_pat_prefix() {
        let toml_str = r#"prefix = "feat""#;
        let pat: Pat = toml::from_str(toml_str).unwrap();
        assert_eq!(pat, Pat::Prefix("feat".to_string()));
    }

    #[test]
    fn test_deserialize_pat_regex() {
        let toml_str = r#"regex = ".*""#;
        let pat: Pat = toml::from_str(toml_str).unwrap();
        assert_eq!(pat, Pat::Regex(".*".to_string()));
    }

    #[test]
    fn test_deserialize_find_regex() {
        let toml_str = r#"regex = ".*""#;
        let find: Find = toml::from_str(toml_str).unwrap();
        assert_eq!(find, Find::Regex(".*".to_string()));
    }
}
