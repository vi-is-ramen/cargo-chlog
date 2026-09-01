//! Module for generating Markdown-formatted changelogs.

use crate::cfg::LogConfig;
use crate::lint::LintedCommit;
use std::collections::{BTreeMap, HashSet};

/// Generates a Markdown-formatted changelog from a list of linted commits.
///
/// The output respects the provided [`LogConfig`] for grouping by scope,
/// including commit URLs/hashes, and generating a contributors section.
///
/// # Arguments
///
/// * `commits` - A slice of [`LintedCommit`] instances.
/// * `config` - The changelog generation configuration.
///
/// # Returns
///
/// A `String` containing the formatted Markdown changelog.
pub fn generate_markdown(commits: &[LintedCommit], config: &LogConfig) -> String {
    let mut md = String::new();

    let mut types_map: BTreeMap<String, Vec<&LintedCommit>> = BTreeMap::new();
    let mut breaking_changes: Vec<&LintedCommit> = Vec::new();
    let mut authors: HashSet<String> = HashSet::new();

    for commit in commits {
        types_map.entry(commit.ty.clone()).or_default().push(commit);
        if commit.breaking {
            breaking_changes.push(commit);
        }
        if config.collect_thanks {
            authors.insert(commit.author.clone());
        }
    }

    // 1. Breaking Changes section (pulled to the top for visibility)
    if !breaking_changes.is_empty() {
        md.push_str("## Breaking Changes\n\n");
        for commit in &breaking_changes {
            md.push_str(&format!(
                "- {}{}\n",
                format_commit_line(commit, config),
                format_commit_details(commit)
            ));
        }
        md.push('\n');
    }

    // 2. Type sections (e.g. Features, Fixes)
    for (ty, type_commits) in types_map {
        md.push_str(&format!("## {}\n\n", ty));

        if config.separate_scope_lists {
            // Group by scope (nested H3 headings)
            let mut scopes_map: BTreeMap<Option<String>, Vec<&LintedCommit>> = BTreeMap::new();
            for commit in type_commits {
                scopes_map
                    .entry(commit.scope.clone())
                    .or_default()
                    .push(commit);
            }

            for (scope, scope_commits) in scopes_map {
                let scope_title = scope.as_deref().unwrap_or("General");
                md.push_str(&format!("### `{}`\n\n", scope_title));
                for commit in scope_commits {
                    md.push_str(&format!(
                        "- {}{}\n",
                        format_commit_line(commit, config),
                        format_commit_details(commit)
                    ));
                }
                md.push('\n');
            }
        } else {
            // Flat list with bold scope prefixes
            for commit in type_commits {
                let line = if let Some(scope) = &commit.scope {
                    format!("**{}**: {}", scope, format_commit_line(commit, config))
                } else {
                    format_commit_line(commit, config)
                };
                md.push_str(&format!("- {}{}\n", line, format_commit_details(commit)));
            }
            md.push('\n');
        }
    }

    // 3. Thanks section
    if config.collect_thanks && !authors.is_empty() {
        let subtitle = config.thanks_subtitle.as_deref().unwrap_or("Thanks to");
        md.push_str(&format!("## {}\n\n", subtitle));

        let mut sorted_authors: Vec<_> = authors.into_iter().collect();
        sorted_authors.sort();

        for author in sorted_authors {
            md.push_str(&format!("- {}\n", author));
        }
        md.push('\n');
    }

    md.trim_end().to_string() + "\n"
}

/// Formats the primary list item line for a commit (Brief + hash/url metadata).
///
/// # Arguments
///
/// * `commit` - The linted commit to format.
/// * `config` - The changelog configuration.
///
/// # Returns
///
/// A formatted `String` representing the commit's list item header.
fn format_commit_line(commit: &LintedCommit, config: &LogConfig) -> String {
    let mut line = commit.brief.clone();
    let mut meta = Vec::new();

    let short_hash = if commit.hash.len() >= 7 {
        &commit.hash[..7]
    } else {
        &commit.hash
    };

    if config.include_commit_url {
        if let Some(url) = &commit.url {
            if config.include_commit_hash {
                meta.push(format!("[{}]({})", short_hash, url));
            } else {
                meta.push(format!("[link]({})", url));
            }
        } else if config.include_commit_hash {
            meta.push(format!("`{}`", short_hash));
        }
    } else if config.include_commit_hash {
        meta.push(format!("`{}`", short_hash));
    }

    if !meta.is_empty() {
        line.push_str(&format!(" ({})", meta.join(", ")));
    }

    line
}

/// Formats the indented body paragraphs and footers for a commit.
///
/// Ensures that multiline content renders correctly as continuations of a
/// Markdown list item.
///
/// # Arguments
///
/// * `commit` - The linted commit to format.
///
/// # Returns
///
/// A formatted `String` containing the commit's body and footers, properly indented.
fn format_commit_details(commit: &LintedCommit) -> String {
    let mut details = String::new();

    if let Some(body) = &commit.body {
        details.push_str("\n\n  ");
        details.push_str(&body.replace("\n", "\n  "));
    }

    if !commit.footers.is_empty() {
        for (token, value) in &commit.footers {
            // Skip explicit BREAKING CHANGE footers since they are handled in the top section
            if token == "BREAKING CHANGE" || token == "BREAKING-CHANGE" {
                continue;
            }
            details.push_str(&format!("\n\n  *{}*: {}", token, value));
        }
    }

    details
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cfg::LogConfig;
    use crate::lint::LintedCommit;

    fn mock_linted_commit(
        ty: &str,
        scope: Option<&str>,
        breaking: bool,
        brief: &str,
    ) -> LintedCommit {
        LintedCommit {
            hash: "1234567890abcdef".to_string(),
            author: "Test User <test@example.com>".to_string(),
            url: Some("https://github.com/owner/repo/commit/1234567890abcdef".to_string()),
            ty: ty.to_string(),
            scope: scope.map(|s| s.to_string()),
            breaking,
            brief: brief.to_string(),
            body: None,
            footers: vec![],
        }
    }

    fn mock_config() -> LogConfig {
        LogConfig {
            include_commit_url: true,
            include_commit_hash: true,
            separate_scope_lists: true,
            collect_thanks: true,
            thanks_subtitle: Some("Contributors".to_string()),
        }
    }

    #[test]
    fn test_generate_markdown_basic() {
        let commits = vec![mock_linted_commit(
            "Feature",
            Some("core"),
            false,
            "add new feature",
        )];
        let config = mock_config();
        let md = generate_markdown(&commits, &config);

        assert!(md.contains("## Feature"));
        assert!(md.contains("### `core`"));
        assert!(md.contains("add new feature"));
        assert!(md.contains("[1234567]"));
        assert!(md.contains("Contributors"));
        assert!(md.contains("Test User <test@example.com>"));
    }

    #[test]
    fn test_generate_markdown_breaking_changes() {
        let commits = vec![mock_linted_commit("Feature", None, true, "redesign API")];
        let config = mock_config();
        let md = generate_markdown(&commits, &config);

        assert!(md.contains("## Breaking Changes"));
        assert!(md.contains("redesign API"));
    }

    #[test]
    fn test_format_commit_line_no_url() {
        let mut commit = mock_linted_commit("Fix", None, false, "fix bug");
        commit.url = None;
        let config = LogConfig {
            include_commit_url: true,
            include_commit_hash: true,
            separate_scope_lists: false,
            collect_thanks: false,
            thanks_subtitle: None,
        };

        let line = format_commit_line(&commit, &config);
        assert_eq!(line, "fix bug (`1234567`)");
    }
}
