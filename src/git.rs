//! Git history fetching and commit information extraction.

use git2::{Repository, Sort};
use std::fmt;

/// Info about a single commit, as needed for changelog generation.
#[derive(Debug, Clone)]
pub struct Commit {
    /// Author's display name, e.g. `"Jane Doe <jane@example.com>"`.
    ///
    /// A signature's name/email are always present in practice; either half
    /// falls back to `"unknown"` only in the unusual case where it isn't
    /// valid UTF-8.
    pub author: String,

    /// Full (40-character) commit hash.
    pub hash: String,

    /// URL to this commit on the remote hosting service, derived from the
    /// `origin` remote's URL. `None` if there is no `origin` remote, or its
    /// URL isn't in a recognized form (see [`remote_commit_url_base`]).
    pub url: Option<String>,

    /// Whole commit message (summary + body), exactly as stored by git.
    pub message: String,
}

/// Errors that can occur while reading commit history.
#[derive(Debug)]
pub enum Error {
    /// Wraps a native `git2::Error`.
    Git(git2::Error),
}

impl fmt::Display for Error {
    /// Formats the git error into a user-friendly string.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Git(err) => write!(f, "git error: {err}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<git2::Error> for Error {
    /// Converts a `git2::Error` into our custom `Error` type.
    fn from(err: git2::Error) -> Self {
        Error::Git(err)
    }
}

/// Returns commit history since `since`, discovering the repository starting
/// from the current directory (walking up, like `git` itself does).
///
/// `since` is any revision spec `git2` understands via [`Repository::revparse_single`]
/// — a tag name, a commit hash, `~N`/`HEAD~N`, etc. (this matches the
/// notations documented on the `--since` CLI flag). Commits strictly after
/// `since` up to and including `HEAD` are returned, newest first.
///
/// If `since` is `None`, it defaults to `HEAD~1`, so only the current tip
/// commit is returned. As a special case, if the repository has no commit
/// before `HEAD` (so `HEAD~1` doesn't resolve) *and* `since` was `None`, this
/// is treated as "everything", not an error.
pub fn history(since: Option<&str>) -> Result<Vec<Commit>, Error> {
    let repo = Repository::discover(".")?;
    history_in(&repo, since)
}

/// Same as [`history`], but operates on an already-open [`Repository`].
///
/// Split out from [`history`] mainly so tests (and callers that already have
/// a `Repository`, e.g. because they opened it with non-default options) can
/// avoid repository discovery.
pub fn history_in(repo: &Repository, since: Option<&str>) -> Result<Vec<Commit>, Error> {
    let since_spec = since.unwrap_or("HEAD~1");

    let since_oid = match repo.revparse_single(since_spec) {
        Ok(obj) => Some(obj.peel_to_commit()?.id()),
        // Only swallow the error for our own default spec: if the caller
        // explicitly passed a bad `since`, they should hear about it.
        Err(_) if since.is_none() => None,
        Err(err) => return Err(err.into()),
    };

    let base_url = remote_commit_url_base(repo);

    let mut revwalk = repo.revwalk()?;
    revwalk.set_sorting(Sort::TOPOLOGICAL | Sort::TIME)?;
    revwalk.push_head()?;
    if let Some(oid) = since_oid {
        revwalk.hide(oid)?;
    }

    let mut commits = Vec::new();
    for oid in revwalk {
        let commit = repo.find_commit(oid?)?;
        commits.push(commit_info(&commit, base_url.as_deref()));
    }
    Ok(commits)
}

/// Builds a [`Commit`] from a `git2::Commit`.
fn commit_info(commit: &git2::Commit, base_url: Option<&str>) -> Commit {
    let hash = commit.id().to_string();
    let author = format_author(&commit.author());

    // `message()` errors (rather than returning `None`, pre-0.21) for
    // non-UTF-8 messages; fall back to a lossy conversion of the raw bytes
    // rather than losing the commit.
    let message = commit
        .message()
        .map(str::to_owned)
        .unwrap_or_else(|_| String::from_utf8_lossy(commit.message_bytes()).into_owned());

    let url = base_url.map(|base| format!("{base}/commit/{hash}"));

    Commit {
        author,
        hash,
        url,
        message,
    }
}

/// Formats a commit signature as `"Name <email>"`, degrading gracefully if
/// either part isn't valid UTF-8 (git2 surfaces that as an `Err`, not an
/// absent value — a signature's name/email are otherwise always present).
fn format_author(sig: &git2::Signature) -> String {
    let name = sig.name().unwrap_or("unknown");
    let email = sig.email().unwrap_or("unknown");
    format!("{name} <{email}>")
}

/// Derives a web base URL (e.g. `"https://github.com/owner/repo"`) from the
/// repository's `origin` remote, so a full commit URL can be built as
/// `"{base}/commit/{hash}"`.
///
/// Returns `None` if there's no `origin` remote, its URL can't be read, or
/// it's not in one of the forms handled by [`normalize_remote_url`].
fn remote_commit_url_base(repo: &Repository) -> Option<String> {
    let remote = repo.find_remote("origin").ok()?;
    // `url()` returns `Result<&str, Error>` as of git2 0.21 (previously
    // `Option<&str>`); `.ok()` folds a non-UTF-8 URL into `None` same as a
    // missing one, which is fine here since we can't build a link either way.
    normalize_remote_url(remote.url().ok()?)
}

/// Normalizes a git remote URL into an `https://host/owner/repo` web base
/// URL. Handles the three common forms:
///
/// * `https://host/owner/repo(.git)`      (already what we want)
/// * `git@host:owner/repo(.git)`          (SCP-like SSH syntax)
/// * `ssh://git@host/owner/repo(.git)`    (explicit SSH syntax)
///
/// This assumes a GitHub-style `/commit/{hash}` URL layout; adjust
/// `commit_info`'s `format!` if you're hosting on something that uses a
/// different path (e.g. Bitbucket's `/commits/{hash}`).
fn normalize_remote_url(url: &str) -> Option<String> {
    let url = url.strip_suffix(".git").unwrap_or(url);

    if let Some(rest) = url.strip_prefix("git@") {
        let (host, path) = rest.split_once(':')?;
        return Some(format!("https://{host}/{path}"));
    }

    if let Some(rest) = url.strip_prefix("ssh://git@") {
        return Some(format!("https://{rest}"));
    }

    if url.starts_with("https://") || url.starts_with("http://") {
        return Some(url.to_owned());
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use git2::Oid;
    use std::path::{Path, PathBuf};

    /// A bare-bones temp directory that cleans itself up on drop, so the
    /// tests below don't need a `tempfile` dev-dependency.
    struct TempDir(PathBuf);

    impl TempDir {
        fn new() -> Self {
            let unique = format!(
                "cargo-chlog-test-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            );
            let path = std::env::temp_dir().join(unique);
            std::fs::create_dir_all(&path).unwrap();
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// Creates a commit on `repo`'s current `HEAD`, using its (empty) index
    /// as the tree, so tests don't need to touch the filesystem at all.
    fn commit(repo: &Repository, message: &str) -> Oid {
        let sig = git2::Signature::now("Test Author", "test@example.com").unwrap();
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

    fn init_repo(dir: &Path) -> Repository {
        Repository::init(dir).unwrap()
    }

    #[test]
    fn test_format_author() {
        let sig = git2::Signature::now("John Doe", "john@example.com").unwrap();
        assert_eq!(format_author(&sig), "John Doe <john@example.com>");
    }

    #[test]
    fn test_normalize_remote_url() {
        assert_eq!(
            normalize_remote_url("git@github.com:owner/repo.git"),
            Some("https://github.com/owner/repo".to_string())
        );
        assert_eq!(
            normalize_remote_url("ssh://git@github.com/owner/repo.git"),
            Some("https://github.com/owner/repo".to_string())
        );
        assert_eq!(
            normalize_remote_url("https://github.com/owner/repo"),
            Some("https://github.com/owner/repo".to_string())
        );
        assert_eq!(normalize_remote_url("invalid-url"), None);
    }

    #[test]
    fn defaults_to_only_the_tip_commit() {
        let dir = TempDir::new();
        let repo = init_repo(dir.path());
        commit(&repo, "first");
        let second = commit(&repo, "second\n\nwith a body");

        let commits = history_in(&repo, None).unwrap();

        assert_eq!(commits.len(), 1);
        assert_eq!(commits[0].hash, second.to_string());
        assert_eq!(commits[0].message, "second\n\nwith a body");
        assert_eq!(commits[0].author, "Test Author <test@example.com>");
        assert_eq!(commits[0].url, None); // no `origin` remote configured
    }

    #[test]
    fn single_commit_repo_defaults_to_everything() {
        let dir = TempDir::new();
        let repo = init_repo(dir.path());
        commit(&repo, "only commit");

        // HEAD~1 doesn't exist here; should not error, should return the
        // one commit that does exist.
        let commits = history_in(&repo, None).unwrap();
        assert_eq!(commits.len(), 1);
    }

    #[test]
    fn since_a_specific_commit_returns_everything_after_it() {
        let dir = TempDir::new();
        let repo = init_repo(dir.path());
        let first = commit(&repo, "first");
        commit(&repo, "second");
        commit(&repo, "third");

        let commits = history_in(&repo, Some(&first.to_string())).unwrap();

        assert_eq!(commits.len(), 2);
        assert_eq!(commits[0].message, "third");
        assert_eq!(commits[1].message, "second");
    }

    #[test]
    fn since_head_returns_nothing() {
        let dir = TempDir::new();
        let repo = init_repo(dir.path());
        commit(&repo, "only commit");

        let commits = history_in(&repo, Some("HEAD")).unwrap();
        assert!(commits.is_empty());
    }

    #[test]
    fn bad_explicit_since_is_an_error() {
        let dir = TempDir::new();
        let repo = init_repo(dir.path());
        commit(&repo, "only commit");

        assert!(history_in(&repo, Some("not-a-real-rev")).is_err());
    }

    #[test]
    fn derives_github_url_from_origin_remote() {
        let dir = TempDir::new();
        let repo = init_repo(dir.path());
        repo.remote("origin", "git@github.com:vi-is-ramen/cargo-chlog.git")
            .unwrap();
        let head = commit(&repo, "only commit");

        let commits = history_in(&repo, None).unwrap();

        assert_eq!(
            commits[0].url.as_deref(),
            Some(format!("https://github.com/vi-is-ramen/cargo-chlog/commit/{}", head).as_str())
        );
    }
}
