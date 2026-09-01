//! Configuration structures related to commits.

use crate::cfg::Pat;
use serde::Deserialize;
use std::collections::HashMap;

/// A pattern for matching commit messages.
///
/// Sub-patterns behave as an "and" operator; i.e., the pattern matches if all
/// defined sub-patterns match.
#[derive(Debug, Deserialize, Clone)]
pub struct CommitPattern {
    /// Pattern to match against the commit's brief description.
    pub brief: Option<Pat>,
    /// Pattern to match against the commit's scope.
    pub scope: Option<Pat>,
    /// Pattern to match against the commit's body.
    pub body: Option<Pat>,
    /// Pattern to match against the commit's footers.
    pub footer: Option<Pat>,
}

/// Configuration for commits.
#[derive(Debug, Deserialize, Clone)]
pub struct Commits {
    /// List of commit types and their corresponding descriptions.
    ///
    /// Commits linting will fail if a commit type is not found in this map.
    pub types: HashMap<String, String>,

    /// List of patterns defining which commits to ignore during linting and changelog generation.
    pub ignore: Option<Vec<CommitPattern>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_commits() {
        let toml_str = r#"
            [types]
            feat = "Feature"
            fix = "Fix"

            [[ignore]]
            brief = { regex = "^WIP:" }
        "#;
        let commits: Commits = toml::from_str(toml_str).unwrap();
        assert_eq!(commits.types.len(), 2);
        assert_eq!(commits.types["feat"], "Feature");
        assert_eq!(commits.ignore.unwrap().len(), 1);
    }
}
