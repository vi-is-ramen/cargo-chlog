//! `cargo-chlog` is a tool for generating changelogs from Git commits
//! following the Conventional Commits specification.
//!
//! It provides utilities to parse configuration files, fetch Git history,
//! lint commit messages, and generate Markdown-formatted changelogs.

pub extern crate annotate_snippets as ans;

pub mod cfg;
pub mod cli;
pub mod git;
pub mod lint;
pub mod log;

/// Global renderer for `annotate-snippets` used for styled error output.
pub static ANS_RENDERER: ans::Renderer =
    ans::Renderer::styled().decor_style(ans::renderer::DecorStyle::Unicode);

/// Prints a formatted error message to standard error using `annotate-snippets`.
///
/// # Arguments
///
/// * `message` - The error message to display.
pub fn error(message: &str) {
    eprintln!(
        "{}",
        ANS_RENDERER.render(&[ans::Group::with_title(
            ans::Level::ERROR.primary_title(message)
        )])
    );
}
