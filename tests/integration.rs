//! Integration tests using temporary Git repositories to verify the full pipeline.

use cargo_chlog::{cfg, git, lint, log};
use git2::{Oid, Repository, Signature};
use std::fs;
use std::path::{Path, PathBuf};

const CONFIG_TOML: &str = r#"
[commits.types]
feat = "Feature"
fix = "Fix"
docs = "Documentation"
chore = "Chore"

[log]
include_commit_url = true
include_commit_hash = true
separate_scope_lists = true
collect_thanks = true
thanks_subtitle = "Contributors"

[[commits.ignore]]
brief = { regex = "^WIP:" }
"#;

/// A bare-bones temp directory that cleans itself up on drop.
struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        let unique = format!(
            "cargo-chlog-integ-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let path = std::env::temp_dir().join(unique);
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn init_repo(dir: &Path) -> Repository {
    Repository::init(dir).unwrap()
}

/// Creates a commit on `repo`'s current `HEAD` using an empty tree.
fn commit(repo: &Repository, message: &str) -> Oid {
    let sig = Signature::now("Test Author", "test@example.com").unwrap();
    let tree_id = repo.index().unwrap().write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();

    let parents: Vec<git2::Commit> = repo
        .head()
        .ok()
        .and_then(|h| h.peel_to_commit().ok())
        .into_iter()
        .collect();
    let parent_refs: Vec<&git2::Commit> = parents.iter().collect();

    repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &parent_refs)
        .unwrap()
}

fn write_config(dir: &Path, content: &str) -> PathBuf {
    let path = dir.join("Chlog.toml");
    fs::write(&path, content).unwrap();
    path
}

#[test]
fn test_check_success() {
    let dir = TempDir::new();
    let repo = init_repo(dir.path());

    // Create a root commit to act as the `since` boundary
    commit(&repo, "chore: init");
    commit(&repo, "feat: add new feature");
    commit(&repo, "fix: resolve bug");

    // Fetch commits strictly after the root commit (HEAD~2)
    let commits = git::history_in(&repo, Some("HEAD~2")).unwrap();
    assert_eq!(commits.len(), 2);

    let config_path = write_config(dir.path(), CONFIG_TOML);
    let config = cfg::parse_from(config_path);

    let result = lint::lint(commits, &config);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().len(), 2);
}

#[test]
fn test_check_failure_invalid_format() {
    let dir = TempDir::new();
    let repo = init_repo(dir.path());

    commit(&repo, "feat: valid commit");
    commit(&repo, "invalid commit message"); // Missing colon and type structure

    // `None` defaults to fetching only the latest commit (HEAD)
    let commits = git::history_in(&repo, None).unwrap();
    assert_eq!(commits.len(), 1);

    let config_path = write_config(dir.path(), CONFIG_TOML);
    let config = cfg::parse_from(config_path);

    let result = lint::lint(commits, &config);
    assert!(result.is_err());
    match result.unwrap_err() {
        lint::LintError::InvalidFormat { .. } => {}
        _ => panic!("Expected InvalidFormat error"),
    }
}

#[test]
fn test_check_failure_unknown_type() {
    let dir = TempDir::new();
    let repo = init_repo(dir.path());

    commit(&repo, "unknown: this type is not in config");

    let commits = git::history_in(&repo, None).unwrap();
    assert_eq!(commits.len(), 1);

    let config_path = write_config(dir.path(), CONFIG_TOML);
    let config = cfg::parse_from(config_path);

    let result = lint::lint(commits, &config);
    assert!(result.is_err());
    match result.unwrap_err() {
        lint::LintError::UnknownType { ty, .. } => assert_eq!(ty, "unknown"),
        _ => panic!("Expected UnknownType error"),
    }
}

#[test]
fn test_log_generation_full() {
    let dir = TempDir::new();
    let repo = init_repo(dir.path());

    // Setup remote to test URL generation
    repo.remote("origin", "git@github.com:owner/repo.git")
        .unwrap();

    // Create a root commit to act as the `since` boundary
    commit(&repo, "chore: init");
    commit(
        &repo,
        "feat(api)!: redesign endpoints\n\nBREAKING CHANGE: removed v1",
    );
    commit(
        &repo,
        "fix(ui): button alignment\n\nThe buttons were overlapping.\n\nRefs: #123",
    );
    commit(&repo, "feat: add dashboard");
    commit(&repo, "chore: WIP: temporary stuff"); // Valid format, but ignored by regex pattern
    commit(&repo, "docs: update readme");

    // Fetch all commits after the root commit (HEAD~5)
    let commits = git::history_in(&repo, Some("HEAD~5")).unwrap();
    assert_eq!(commits.len(), 5);

    let config_path = write_config(dir.path(), CONFIG_TOML);
    let config = cfg::parse_from(config_path);

    let linted = lint::lint(commits, &config).unwrap();
    assert_eq!(linted.len(), 4); // "WIP: temporary stuff" is ignored

    let md = log::generate_markdown(&linted, &config.log);

    // 1. Check Breaking Changes section
    assert!(md.contains("## Breaking Changes"));
    assert!(md.contains("redesign endpoints"));

    // 2. Check Features section & Scope Grouping
    assert!(md.contains("## Feature"));
    assert!(md.contains("### `api`"));
    assert!(md.contains("### `General`")); // Fallback for commits without a scope
    assert!(md.contains("add dashboard"));

    // 3. Check Fixes section, Body, and Footers
    assert!(md.contains("## Fix"));
    assert!(md.contains("### `ui`"));
    assert!(md.contains("button alignment"));
    assert!(md.contains("The buttons were overlapping.")); // Body
    assert!(md.contains("*Refs*: #123")); // Footer

    // 4. Check Docs section
    assert!(md.contains("## Documentation"));
    assert!(md.contains("update readme"));

    // 5. Check URL and Hash formatting
    assert!(md.contains("https://github.com/owner/repo/commit/"));

    // 6. Check Thanks/Contributors section
    assert!(md.contains("## Contributors"));
    assert!(md.contains("Test Author <test@example.com>"));

    // 7. Verify ignored commit is completely absent
    assert!(!md.contains("temporary stuff"));
}
