//! Module for linting Git commits against the Conventional Commits specification.

use crate::{cfg, git};
use regex::Regex;

/// Represents a single commit that has been successfully parsed and linted.
///
/// Contains structured data extracted from the commit message, such as the type,
/// scope, breaking change flag, brief description, body, and footers.
#[derive(Debug, Clone, PartialEq)]
pub struct LintedCommit {
    /// Full (40-character) commit hash.
    pub hash: String,
    /// Author's display name, e.g., `"Jane Doe <jane@example.com>"`.
    pub author: String,
    /// URL to this commit on the remote hosting service, if available.
    pub url: Option<String>,
    /// Mapped type from config (e.g., "Feature" instead of "feat").
    pub ty: String,
    /// Optional scope of the commit.
    pub scope: Option<String>,
    /// Whether the commit introduces a breaking change.
    pub breaking: bool,
    /// Brief description of the commit.
    pub brief: String,
    /// Optional body of the commit message.
    pub body: Option<String>,
    /// List of footers in the commit message, as `(token, value)` pairs.
    pub footers: Vec<(String, String)>,
}

/// Errors that can occur during the linting process.
#[derive(Debug, PartialEq)]
pub enum LintError {
    /// The commit message does not follow the Conventional Commits format.
    InvalidFormat {
        /// The hash of the offending commit.
        hash: String,
        /// A descriptive message explaining the format violation.
        message: String,
    },
    /// The commit uses a type that is not defined in the configuration.
    UnknownType {
        /// The hash of the offending commit.
        hash: String,
        /// The unrecognized type string.
        ty: String,
    },
}

impl std::fmt::Display for LintError {
    /// Formats the lint error into a user-friendly string.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LintError::InvalidFormat { hash, message } => {
                write!(
                    f,
                    "Commit {} does not follow Conventional Commits format: {}",
                    hash, message
                )
            }
            LintError::UnknownType { hash, ty } => {
                write!(
                    f,
                    "Commit {} has unknown type '{}' (not defined in config)",
                    hash, ty
                )
            }
        }
    }
}

impl std::error::Error for LintError {}

/// Internal representation of a parsed commit before applying ignore patterns
/// and mapping types.
struct ParsedCommit {
    /// The raw type string from the commit message (e.g., "feat").
    _ty: String,
    /// The optional scope.
    scope: Option<String>,
    /// The breaking change flag (`!`).
    _breaking: bool,
    /// The brief description.
    brief: String,
    /// The body of the commit message.
    body: Option<String>,
    /// The list of footers.
    footers: Vec<(String, String)>,
}

/// Lints a list of Git commits against the provided configuration.
///
/// Parses each commit message to ensure it follows the Conventional Commits
/// specification. It applies ignore patterns from the configuration and maps
/// the commit types to their configured descriptions.
///
/// # Arguments
///
/// * `commits` - A vector of raw [`git::Commit`] instances.
/// * `config` - The parsed [`cfg::Config`] containing rules and type mappings.
///
/// # Returns
///
/// A `Result` containing a vector of [`LintedCommit`] instances if all commits
/// pass the lint checks, or a [`LintError`] if any commit fails.
pub fn lint(
    commits: Vec<git::Commit>,
    config: &cfg::Config,
) -> Result<Vec<LintedCommit>, LintError> {
    let mut linted = Vec::new();

    // Precompile regexes for the header and footer lines
    let header_re = Regex::new(
        r"^(?P<type>[a-zA-Z0-9_-]+)(?:\((?P<scope>[^)]+)\))?(?P<breaking>!)?:\s*(?P<brief>.+)$",
    )
    .unwrap();
    let footer_re =
        Regex::new(r"^(?P<token>[a-zA-Z0-9_-]+|BREAKING CHANGE)(?:[:#]\s*)(?P<value>.+)$").unwrap();

    for commit in commits {
        let clean_message = commit.message.replace("\r\n", "\n");
        let mut lines = clean_message.split('\n');
        let first_line = lines.next().unwrap_or("");

        let caps = match header_re.captures(first_line) {
            Some(c) => c,
            None => {
                return Err(LintError::InvalidFormat {
                    hash: commit.hash.clone(),
                    message: format!(
                        "Header '{}' does not match '<type>[(<scope>)][!]: <description>'",
                        first_line
                    ),
                });
            }
        };

        let ty = caps.name("type").unwrap().as_str().to_string();
        let scope = caps.name("scope").map(|m| m.as_str().to_string());
        let breaking_flag = caps.name("breaking").is_some();
        let brief = caps.name("brief").unwrap().as_str().trim().to_string();

        let mapped_ty = match config.commits.types.get(&ty) {
            Some(mapped) => mapped.clone(),
            None => {
                return Err(LintError::UnknownType {
                    hash: commit.hash.clone(),
                    ty: ty.clone(),
                });
            }
        };

        let rest_lines: Vec<&str> = lines.collect();

        // Split the remaining message into paragraphs (separated by blank lines)
        let mut paragraphs = Vec::new();
        let mut current_para = Vec::new();

        for line in rest_lines {
            if line.trim().is_empty() {
                if !current_para.is_empty() {
                    paragraphs.push(current_para);
                    current_para = Vec::new();
                }
            } else {
                current_para.push(line);
            }
        }
        if !current_para.is_empty() {
            paragraphs.push(current_para);
        }

        let mut footers = Vec::new();
        let mut body_paragraphs = paragraphs.clone();

        // Extract footers from the bottom up. Any trailing paragraph that consists
        // entirely of valid footer lines is treated as a footer block.
        while let Some(last_para) = body_paragraphs.last() {
            let mut all_footers = true;
            for line in last_para {
                if !footer_re.is_match(line) {
                    all_footers = false;
                    break;
                }
            }

            if all_footers && !last_para.is_empty() {
                let para = body_paragraphs.pop().unwrap();
                let mut para_footers = Vec::new();
                for line in para {
                    let caps = footer_re.captures(line).unwrap();
                    let token = caps.name("token").unwrap().as_str().to_string();
                    let value = caps.name("value").unwrap().as_str().to_string();
                    para_footers.push((token, value));
                }
                footers.extend(para_footers);
            } else {
                break;
            }
        }

        footers.reverse(); // Restore original chronological order

        let body = if body_paragraphs.is_empty() {
            None
        } else {
            Some(
                body_paragraphs
                    .into_iter()
                    .map(|p| p.join("\n"))
                    .collect::<Vec<_>>()
                    .join("\n\n"),
            )
        };

        let parsed = ParsedCommit {
            _ty: ty,
            scope: scope.clone(),
            _breaking: breaking_flag,
            brief: brief.clone(),
            body: body.clone(),
            footers: footers.clone(),
        };

        // Check ignore patterns
        let mut ignored = false;
        for pattern in &config.commits.ignore.clone().unwrap_or_else(|| vec![]) {
            if matches_commit_pattern(&parsed, pattern) {
                ignored = true;
                break;
            }
        }

        if ignored {
            continue;
        }

        // Check for explicit BREAKING CHANGE footers
        let mut breaking = breaking_flag;
        for (token, _) in &footers {
            if token == "BREAKING CHANGE" || token == "BREAKING-CHANGE" {
                breaking = true;
            }
        }

        linted.push(LintedCommit {
            hash: commit.hash,
            author: commit.author,
            url: commit.url,
            ty: mapped_ty,
            scope,
            breaking,
            brief,
            body,
            footers,
        });
    }

    Ok(linted)
}

/// Checks if a parsed commit matches a given commit pattern.
///
/// All defined sub-patterns in the [`cfg::CommitPattern`] must match for the
/// commit to be considered a match (logical AND).
///
/// # Arguments
///
/// * `commit` - The parsed commit to check.
/// * `pattern` - The pattern to match against.
///
/// # Returns
///
/// `true` if the commit matches the pattern, `false` otherwise.
fn matches_commit_pattern(commit: &ParsedCommit, pattern: &cfg::CommitPattern) -> bool {
    let mut matched = true;
    if let Some(ref pat) = pattern.brief {
        if !matches_pat(pat, &commit.brief) {
            matched = false;
        }
    }
    if let Some(ref pat) = pattern.scope {
        match &commit.scope {
            Some(s) => {
                if !matches_pat(pat, s) {
                    matched = false;
                }
            }
            None => matched = false, // Pattern requires a scope, but commit has none
        }
    }
    if let Some(ref pat) = pattern.body {
        match &commit.body {
            Some(b) => {
                if !matches_pat(pat, b) {
                    matched = false;
                }
            }
            None => matched = false,
        }
    }
    if let Some(ref pat) = pattern.footer {
        // A commit matches the footer pattern if AT LEAST ONE of its footers matches
        let any_match = commit.footers.iter().any(|(t, v)| {
            let line = format!("{}: {}", t, v);
            matches_pat(pat, &line)
        });
        if !any_match {
            matched = false;
        }
    }
    matched
}

/// Evaluates a single [`cfg::Pat`] against a given text string.
///
/// # Arguments
///
/// * `pat` - The pattern to evaluate.
/// * `text` - The text to match against.
///
/// # Returns
///
/// `true` if the text matches the pattern, `false` otherwise.
fn matches_pat(pat: &cfg::Pat, text: &str) -> bool {
    use cfg::Pat;
    match pat {
        Pat::Prefix(p) => text.starts_with(p),
        Pat::Suffix(s) => text.ends_with(s),
        Pat::Not(inner) => !matches_pat(inner, text),
        Pat::Find(f) => matches_find(f, text),
        Pat::Regex(r) => {
            if let Ok(re) = Regex::new(r) {
                re.is_match(text)
            } else {
                false // Fallback on invalid regex
            }
        }
        Pat::Exact(e) => text == e,
    }
}

/// Evaluates a [`cfg::Find`] operation against a given text string.
///
/// # Arguments
///
/// * `find` - The find operation to evaluate.
/// * `text` - The text to search within.
///
/// # Returns
///
/// `true` if the text contains the specified pattern, `false` otherwise.
fn matches_find(find: &cfg::Find, text: &str) -> bool {
    use cfg::Find;
    match find {
        Find::Regex(r) => {
            if let Ok(re) = Regex::new(r) {
                re.is_match(text)
            } else {
                false
            }
        }
        Find::Exact(e) => text.contains(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cfg::{CommitPattern, Commits, Find, LogConfig, Pat};
    use std::collections::HashMap;

    fn mock_config() -> cfg::Config {
        let mut types = HashMap::new();
        types.insert("feat".to_string(), "Feature".to_string());
        types.insert("fix".to_string(), "Fix".to_string());

        cfg::Config {
            commits: Commits {
                types,
                ignore: Some(vec![]),
            },
            log: LogConfig {
                include_commit_url: false,
                include_commit_hash: false,
                separate_scope_lists: false,
                collect_thanks: false,
                thanks_subtitle: None,
            },
        }
    }

    fn mock_commit(hash: &str, message: &str) -> git::Commit {
        git::Commit {
            hash: hash.to_string(),
            author: "Test Author <test@example.com>".to_string(),
            url: None,
            message: message.to_string(),
        }
    }

    #[test]
    fn test_lint_valid_commit() {
        let config = mock_config();
        let commits = vec![mock_commit("abc1234", "feat: add new feature")];
        let result = lint(commits, &config).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].ty, "Feature");
        assert_eq!(result[0].brief, "add new feature");
    }

    #[test]
    fn test_lint_invalid_format() {
        let config = mock_config();
        let commits = vec![mock_commit("abc1234", "invalid commit message")];
        let result = lint(commits, &config);
        assert!(matches!(result, Err(LintError::InvalidFormat { .. })));
    }

    #[test]
    fn test_lint_unknown_type() {
        let config = mock_config();
        let commits = vec![mock_commit("abc1234", "chore: update dependencies")];
        let result = lint(commits, &config);
        assert!(matches!(result, Err(LintError::UnknownType { .. })));
    }

    #[test]
    fn test_lint_with_scope_and_breaking() {
        let config = mock_config();
        let commits = vec![mock_commit(
            "abc1234",
            "feat(core)!: redesign API\n\nBREAKING CHANGE: new API",
        )];
        let result = lint(commits, &config).unwrap();
        assert_eq!(result[0].scope, Some("core".to_string()));
        assert!(result[0].breaking);
    }

    #[test]
    fn test_lint_ignore_pattern() {
        let mut config = mock_config();
        config.commits.ignore.as_mut().unwrap().push(CommitPattern {
            brief: Some(Pat::Regex("^WIP:".to_string())),
            scope: None,
            body: None,
            footer: None,
        });

        let commits = vec![
            mock_commit("1", "feat: valid"),
            mock_commit("2", "feat: WIP: ignore me"),
        ];
        let result = lint(commits, &config).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].hash, "1");
    }

    #[test]
    fn test_matches_pat_prefix() {
        assert!(matches_pat(&Pat::Prefix("feat".to_string()), "feat: add"));
        assert!(!matches_pat(&Pat::Prefix("fix".to_string()), "feat: add"));
    }

    #[test]
    fn test_matches_pat_not() {
        let pat = Pat::Not(Box::new(Pat::Exact("fix".to_string())));
        assert!(matches_pat(&pat, "feat"));
        assert!(!matches_pat(&pat, "fix"));
    }

    #[test]
    fn test_matches_find_regex() {
        let find = Find::Regex(r"\d+".to_string());
        assert!(matches_find(&find, "version 123"));
        assert!(!matches_find(&find, "no numbers"));
    }
}
