# cargo-chlog

**Generate beautiful changelogs from your Git commits – the Conventional Commits
way.**

`cargo-chlog` is a Rust‑based CLI tool that parses your Git history, lints
commit messages against the **Conventional Commits** specification, and produces
a Markdown changelog ready for your `CHANGELOG.md`. It integrates seamlessly
with your existing workflow via a flexible `Chlog.toml` configuration file.

---

## Features

* **Lint commits** – ensure every commit follows the Conventional Commits format;
* **Generate changelogs** – produce clean Markdown output grouped by type, scope,
  and breaking changes;
* **Flexible ignoring** – skip commits matching patterns (e.g., `WIP:`, version
  bumps);
* **Rich metadata** – include commit hashes, URLs (GitHub/GitLab/Bitbucket), and
  author thanks;
* **Breaking changes** – automatically pulled to a dedicated section for
  visibility;
* **Customizable types** – define your own type-to-label mappings (e.g., `feat`
  -> `Feature`);
* **Scope grouping** – optionally separate changes by scope within each type;
* **Contributor acknowledgment** – list all contributing authors (opt‑in).

---

## Installation

Install the binary via `cargo`:

```bash
cargo install cargo-chlog
```

To install from source:

```bash
git clone https://github.com/vi-is-ramen/cargo-chlog
cd cargo-chlog
cargo install --path .
```

---

## Usage

`cargo-chlog` provides two subcommands: **`check`** and **`log`**.

```bash
# Check your commit history for compliance
cargo chlog check

# Generate a changelog (default: includes all commits after HEAD~1)
cargo chlog log

# Specify a starting revision (tag, hash, ~N, etc.)
cargo chlog log --since v1.0.0

# Use a custom config file
cargo chlog log --config ./config/Chlog.toml
```

### CLI Options

| Option | Description |
|--------|-------------|
| `-c, --config <PATH>` | Path to `Chlog.toml` (default: `./Chlog.toml`) |
| `-s, --since <SPEC>` | Starting revision (tag, commit hash, `HEAD~N`). Default: `HEAD~1` (latest commit only) |
| `check` | Validate all commits (after `--since`) against the spec |
| `log` | Print the Markdown changelog to stdout |

---

## Configuration

Create a `Chlog.toml` file in your repository root. All sections are **required** (except `ignore` which is optional).

### `[commits.types]` – Define allowed types

Map commit type strings to display labels.

```toml
[commits.types]
feat        = "Feature"
fix         = "Fix"
docs        = "Documentation"
style       = "Style"
refactor    = "Refactoring"
perf        = "Performance"
test        = "Test"
chore       = "Chore"
build       = "Build"
ci          = "CI"
```

### `[[commits.ignore]]` – Skip commits

Define one or more patterns to ignore commits during linting and changelog generation. All sub‑patterns must match (logical **AND**).

```toml
[[commits.ignore]]
brief.prefix = "WIP:"          # skip commits whose brief starts with "WIP:"
scope = "internal"             # skip commits with scope "internal"
body.find = "typo"             # skip if body contains "typo"
footer.regex = "Refs: #\\d+"   # skip if any footer matches the pattern
```

Available pattern types:

- exact string match
- `prefix` – string prefix
- `suffix` – string suffix
- `regex` – regular expression match (the text must match entirely)
- `find` – search for a substring (regex or exact) anywhere
- `not` – invert the match

### `[log]` – Changelog generation settings

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `include_commit_url` | bool | `false` | Include a clickable link to the commit (requires a configured `origin` remote) |
| `include_commit_hash` | bool | `false` | Show the shortened commit hash (e.g., `abc1234`) |
| `separate_scope_lists` | bool | `false` | Group changes within each type by scope (using `###` subheadings) |
| `collect_thanks` | bool | `false` | Include a "Thanks to" section listing all authors |
| `thanks_subtitle` | string | `"Thanks to"` | Custom subtitle for the contributors section |

---

## Example

### Input: `Chlog.toml`

```toml
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
```

### Running `cargo chlog log`

Given the following commits (from a real repository):

```
feat(api)!: redesign endpoints
fix(ui): button alignment
feat: add dashboard
chore: WIP: temporary stuff
docs: update readme
```

The generated Markdown will look like:

```markdown
## Breaking Changes

- redesign endpoints ([`abc1234`](https://github.com/owner/repo/commit/abc1234))

## Feature

### `api`

- redesign endpoints ([`abc1234`](https://github.com/owner/repo/commit/abc1234))

### `General`

- add dashboard ([`def5678`](https://github.com/owner/repo/commit/def5678))

## Fix

### `ui`

- button alignment ([`ghi9012`](https://github.com/owner/repo/commit/ghi9012))

## Documentation

- update readme ([`jkl3456`](https://github.com/owner/repo/commit/jkl3456))

## Contributors

- Test Author <test@example.com>
```

---

## Development & Contributing

This project is under active development. Contributions are welcome!

* [**Repository**](https://github.com/vi-is-ramen/cargo-chlog)
* [**Issues**](https://github.com/vi-is-ramen/cargo-chlog/issues)

### Build & Test

```sh
cargo build
cargo test
```

---

## License

Licensed under either of

- MIT License
- Apache License, Version 2.0

at your option.

---
Made with ❤️ for the Rust community
