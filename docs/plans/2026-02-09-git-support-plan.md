# Git Support Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add a Git sidebar panel showing staged/unstaged files with diff viewing, stage/unstage/discard actions, and context menus.

**Architecture:** New `conescope-git` crate (hybrid git2+CLI backend). New `GitStore` GPUI entity bridges git ops to UI. New `GitPanel` view in tabbed sidebar, new `DiffViewer` for unified diffs in editor area. Activity bar gets a Git toggle button.

**Tech Stack:** `git2` 0.20 (reads), `which` 7 (find git binary), `std::process::Command` (CLI writes), GPUI entities/views.

**Design doc:** `docs/plans/2026-02-09-git-support-design.md`

---

## Task 1: Create `conescope-git` crate with `GitRepo` and CLI wrapper

**Files:**
- Create: `crates/conescope-git/Cargo.toml`
- Create: `crates/conescope-git/src/lib.rs`
- Create: `crates/conescope-git/src/cli.rs`
- Create: `crates/conescope-git/src/repository.rs`
- Create: `crates/conescope-git/src/status.rs`
- Create: `crates/conescope-git/src/diff.rs`
- Modify: `Cargo.toml:3` (workspace members — already uses glob `crates/*`, no change needed)
- Modify: `Cargo.toml:11-29` (add workspace dependencies)

**Step 1: Create crate scaffold**

`crates/conescope-git/Cargo.toml`:
```toml
[package]
name = "conescope-git"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true

[dependencies]
git2.workspace = true
which.workspace = true
anyhow.workspace = true
tracing.workspace = true

[dev-dependencies]
tempfile = "3"

[lints]
workspace = true
```

Add to workspace `Cargo.toml` `[workspace.dependencies]`:
```toml
git2 = "0.20"
which = "7"
```

**Step 2: Implement `status.rs` — types**

```rust
// crates/conescope-git/src/status.rs
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileStatus {
    Modified,
    Added,
    Deleted,
    Renamed,
    Untracked,
}

impl fmt::Display for FileStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Modified => write!(f, "M"),
            Self::Added => write!(f, "A"),
            Self::Deleted => write!(f, "D"),
            Self::Renamed => write!(f, "R"),
            Self::Untracked => write!(f, "??"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StageStatus {
    Staged,
    Unstaged,
}

#[derive(Debug, Clone)]
pub struct GitFileEntry {
    pub path: String,
    pub status: FileStatus,
    pub stage: StageStatus,
}
```

**Step 3: Implement `diff.rs` — types**

```rust
// crates/conescope-git/src/diff.rs

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineOrigin {
    Context,
    Addition,
    Deletion,
}

#[derive(Debug, Clone)]
pub struct DiffLine {
    pub origin: LineOrigin,
    pub old_lineno: Option<u32>,
    pub new_lineno: Option<u32>,
    pub content: String,
}

#[derive(Debug, Clone)]
pub struct DiffHunk {
    pub header: String,
    pub lines: Vec<DiffLine>,
}
```

**Step 4: Implement `cli.rs` — Git CLI wrapper**

```rust
// crates/conescope-git/src/cli.rs
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};

#[derive(Debug)]
pub struct GitCli {
    binary: PathBuf,
    work_dir: PathBuf,
}

impl GitCli {
    pub fn new(work_dir: &Path) -> Result<Self> {
        let binary = which::which("git").context("git binary not found in PATH")?;
        Ok(Self {
            binary,
            work_dir: work_dir.to_owned(),
        })
    }

    pub fn run(&self, args: &[&str]) -> Result<String> {
        let output = Command::new(&self.binary)
            .args(args)
            .current_dir(&self.work_dir)
            .output()
            .with_context(|| format!("failed to run git {}", args.join(" ")))?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("git {} failed: {}", args.join(" "), stderr.trim());
        }
    }

    pub fn stage(&self, paths: &[&str]) -> Result<()> {
        let mut args = vec!["add", "--"];
        args.extend(paths);
        self.run(&args)?;
        Ok(())
    }

    pub fn unstage(&self, paths: &[&str]) -> Result<()> {
        let mut args = vec!["reset", "HEAD", "--"];
        args.extend(paths);
        self.run(&args)?;
        Ok(())
    }

    /// Discard working tree changes for tracked files.
    pub fn discard(&self, paths: &[&str]) -> Result<()> {
        let mut args = vec!["checkout", "--"];
        args.extend(paths);
        self.run(&args)?;
        Ok(())
    }
}
```

**Step 5: Implement `repository.rs` — `GitRepo` hybrid wrapper**

```rust
// crates/conescope-git/src/repository.rs
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use git2::{DiffFormat, DiffOptions, Repository, StatusOptions};

use crate::cli::GitCli;
use crate::diff::{DiffHunk, DiffLine, LineOrigin};
use crate::status::{FileStatus, GitFileEntry, StageStatus};

pub struct GitRepo {
    repo: Repository,
    cli: GitCli,
    work_dir: PathBuf,
}

impl std::fmt::Debug for GitRepo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GitRepo")
            .field("work_dir", &self.work_dir)
            .finish_non_exhaustive()
    }
}

impl GitRepo {
    /// Open a git repository by discovering it from the given path.
    pub fn open(path: &Path) -> Result<Self> {
        let repo = Repository::discover(path)
            .with_context(|| format!("no git repo found at {}", path.display()))?;
        let work_dir = repo
            .workdir()
            .context("bare repositories not supported")?
            .to_owned();
        let cli = GitCli::new(&work_dir)?;
        Ok(Self {
            repo,
            cli,
            work_dir,
        })
    }

    pub fn work_dir(&self) -> &Path {
        &self.work_dir
    }

    /// Get the current branch name (e.g. "main").
    pub fn head_branch(&self) -> Option<String> {
        self.repo
            .head()
            .ok()
            .and_then(|r| r.shorthand().map(str::to_owned))
    }

    /// Get status of all files (staged + unstaged + untracked).
    pub fn status(&self) -> Result<Vec<GitFileEntry>> {
        let mut opts = StatusOptions::new();
        opts.include_untracked(true)
            .recurse_untracked_dirs(true)
            .include_ignored(false);

        let statuses = self.repo.statuses(Some(&mut opts))?;
        let mut entries = Vec::new();

        for entry in statuses.iter() {
            let Some(path) = entry.path() else { continue };
            let s = entry.status();

            // Staged (index) changes
            if s.intersects(
                git2::Status::INDEX_NEW
                    | git2::Status::INDEX_MODIFIED
                    | git2::Status::INDEX_DELETED
                    | git2::Status::INDEX_RENAMED,
            ) {
                let status = if s.contains(git2::Status::INDEX_NEW) {
                    FileStatus::Added
                } else if s.contains(git2::Status::INDEX_DELETED) {
                    FileStatus::Deleted
                } else if s.contains(git2::Status::INDEX_RENAMED) {
                    FileStatus::Renamed
                } else {
                    FileStatus::Modified
                };
                entries.push(GitFileEntry {
                    path: path.to_owned(),
                    status,
                    stage: StageStatus::Staged,
                });
            }

            // Unstaged (working tree) changes
            if s.intersects(
                git2::Status::WT_MODIFIED
                    | git2::Status::WT_DELETED
                    | git2::Status::WT_NEW,
            ) {
                let status = if s.contains(git2::Status::WT_NEW) {
                    FileStatus::Untracked
                } else if s.contains(git2::Status::WT_DELETED) {
                    FileStatus::Deleted
                } else {
                    FileStatus::Modified
                };
                entries.push(GitFileEntry {
                    path: path.to_owned(),
                    status,
                    stage: StageStatus::Unstaged,
                });
            }
        }

        entries.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(entries)
    }

    /// Get unified diff hunks for a single file.
    /// `staged=true` diffs index vs HEAD, `staged=false` diffs workdir vs index.
    pub fn diff_file(&self, path: &str, staged: bool) -> Result<Vec<DiffHunk>> {
        let mut opts = DiffOptions::new();
        opts.pathspec(path);

        let diff = if staged {
            let head_tree = self.repo.head()?.peel_to_tree()?;
            self.repo
                .diff_tree_to_index(Some(&head_tree), None, Some(&mut opts))?
        } else {
            self.repo
                .diff_index_to_workdir(None, Some(&mut opts))?
        };

        let mut hunks: Vec<DiffHunk> = Vec::new();

        diff.print(DiffFormat::Patch, |_delta, hunk, line| {
            // New hunk
            if let Some(hunk_header) = hunk {
                let header = String::from_utf8_lossy(hunk_header.header()).trim().to_owned();
                hunks.push(DiffHunk {
                    header,
                    lines: Vec::new(),
                });
            }

            let origin = match line.origin() {
                '+' => LineOrigin::Addition,
                '-' => LineOrigin::Deletion,
                ' ' => LineOrigin::Context,
                _ => return true, // skip file headers, etc.
            };

            let diff_line = DiffLine {
                origin,
                old_lineno: line.old_lineno(),
                new_lineno: line.new_lineno(),
                content: String::from_utf8_lossy(line.content()).to_string(),
            };

            if let Some(last_hunk) = hunks.last_mut() {
                last_hunk.lines.push(diff_line);
            }

            true
        })?;

        Ok(hunks)
    }

    // --- Write operations (delegate to CLI) ---

    pub fn stage(&self, paths: &[&str]) -> Result<()> {
        self.cli.stage(paths)
    }

    pub fn unstage(&self, paths: &[&str]) -> Result<()> {
        self.cli.unstage(paths)
    }

    /// Discard changes. Tracked files restored via `git checkout --`.
    /// Untracked files deleted from disk.
    pub fn discard(&self, paths: &[&str]) -> Result<()> {
        // Separate tracked vs untracked
        let statuses = self.status()?;
        let untracked: std::collections::HashSet<&str> = statuses
            .iter()
            .filter(|e| e.status == FileStatus::Untracked)
            .map(|e| e.path.as_str())
            .collect();

        let tracked: Vec<&str> = paths.iter().copied().filter(|p| !untracked.contains(p)).collect();
        let to_delete: Vec<&str> = paths.iter().copied().filter(|p| untracked.contains(p)).collect();

        if !tracked.is_empty() {
            self.cli.discard(&tracked)?;
        }
        for path in to_delete {
            let full = self.work_dir.join(path);
            let _ = std::fs::remove_file(full);
        }
        Ok(())
    }
}
```

**Step 6: Wire up `lib.rs`**

```rust
// crates/conescope-git/src/lib.rs
pub mod cli;
pub mod diff;
pub mod repository;
pub mod status;
```

**Step 7: Write tests for `conescope-git`**

Create `crates/conescope-git/tests/integration.rs`:
```rust
use std::fs;
use std::process::Command;

use conescope_git::repository::GitRepo;
use conescope_git::status::{FileStatus, StageStatus};
use tempfile::TempDir;

fn init_repo() -> (TempDir, GitRepo) {
    let dir = TempDir::new().unwrap();
    Command::new("git")
        .args(["init"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    // Initial commit so HEAD exists
    fs::write(dir.path().join("README.md"), "# test").unwrap();
    Command::new("git")
        .args(["add", "."])
        .current_dir(dir.path())
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "init"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    let repo = GitRepo::open(dir.path()).unwrap();
    (dir, repo)
}

#[test]
fn status_untracked_file() {
    let (dir, repo) = init_repo();
    fs::write(dir.path().join("new.txt"), "hello").unwrap();
    let entries = repo.status().unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].path, "new.txt");
    assert_eq!(entries[0].status, FileStatus::Untracked);
    assert_eq!(entries[0].stage, StageStatus::Unstaged);
}

#[test]
fn stage_and_unstage() {
    let (dir, repo) = init_repo();
    fs::write(dir.path().join("new.txt"), "hello").unwrap();

    repo.stage(&["new.txt"]).unwrap();
    let entries = repo.status().unwrap();
    assert!(entries.iter().any(|e| e.path == "new.txt" && e.stage == StageStatus::Staged));

    repo.unstage(&["new.txt"]).unwrap();
    let entries = repo.status().unwrap();
    assert!(entries.iter().any(|e| e.path == "new.txt" && e.stage == StageStatus::Unstaged));
}

#[test]
fn diff_modified_file() {
    let (dir, repo) = init_repo();
    fs::write(dir.path().join("README.md"), "# changed\nnew line").unwrap();
    let hunks = repo.diff_file("README.md", false).unwrap();
    assert!(!hunks.is_empty());
    assert!(hunks[0].lines.iter().any(|l| l.origin == conescope_git::diff::LineOrigin::Addition));
}

#[test]
fn discard_tracked_file() {
    let (dir, repo) = init_repo();
    fs::write(dir.path().join("README.md"), "modified").unwrap();
    assert_ne!(fs::read_to_string(dir.path().join("README.md")).unwrap(), "# test");

    repo.discard(&["README.md"]).unwrap();
    assert_eq!(fs::read_to_string(dir.path().join("README.md")).unwrap(), "# test");
}

#[test]
fn discard_untracked_file() {
    let (dir, repo) = init_repo();
    fs::write(dir.path().join("junk.txt"), "delete me").unwrap();
    assert!(dir.path().join("junk.txt").exists());

    repo.discard(&["junk.txt"]).unwrap();
    assert!(!dir.path().join("junk.txt").exists());
}

#[test]
fn head_branch() {
    let (_dir, repo) = init_repo();
    let branch = repo.head_branch();
    assert!(branch.is_some());
    // Default branch in modern git is usually "main" or "master"
}
```

**Step 8: Build and run tests**

Run: `just build && cargo test -p conescope-git`
Expected: All tests pass.

**Step 9: Commit**

```bash
git add crates/conescope-git/ Cargo.toml Cargo.lock
git commit -m "feat: add conescope-git crate with hybrid git2+CLI backend"
```

---

## Task 2: Add `GitStore` GPUI entity

**Files:**
- Create: `crates/conescope-ui/src/state/git_store.rs`
- Modify: `crates/conescope-ui/src/state/mod.rs` (add `pub mod git_store;`)
- Modify: `crates/conescope-ui/src/state/app_state.rs:19-34` (add `git_store` field)
- Modify: `crates/conescope-ui/Cargo.toml:9` (add `conescope-git` dep)

**Step 1: Add dependency**

In `crates/conescope-ui/Cargo.toml`, add:
```toml
conescope-git = { path = "../conescope-git" }
```

**Step 2: Create `git_store.rs`**

```rust
// crates/conescope-ui/src/state/git_store.rs
use std::sync::{Arc, Mutex};

use gpui::{EventEmitter, prelude::*};

use conescope_git::diff::DiffHunk;
use conescope_git::repository::GitRepo;
use conescope_git::status::GitFileEntry;

#[derive(Debug, Clone)]
pub enum GitStoreEvent {
    StatusChanged,
    OpenDiff { path: String, staged: bool },
}

impl EventEmitter<GitStoreEvent> for GitStore {}

#[derive(Debug)]
pub struct GitStore {
    repo: Option<Arc<Mutex<GitRepo>>>,
    entries: Vec<GitFileEntry>,
    branch: Option<String>,
    current_path: Option<String>,
}

impl GitStore {
    #[must_use]
    pub fn new() -> Self {
        Self {
            repo: None,
            entries: Vec::new(),
            branch: None,
            current_path: None,
        }
    }

    /// Switch to a new project path. Opens the git repo (if any).
    pub fn set_project(&mut self, path: Option<&str>, cx: &mut gpui::Context<Self>) {
        if self.current_path.as_deref() == path {
            return;
        }
        self.current_path = path.map(str::to_owned);
        self.repo = path.and_then(|p| {
            GitRepo::open(std::path::Path::new(p))
                .ok()
                .map(|r| Arc::new(Mutex::new(r)))
        });
        if self.repo.is_some() {
            self.refresh(cx);
        } else {
            self.entries.clear();
            self.branch = None;
            cx.emit(GitStoreEvent::StatusChanged);
            cx.notify();
        }
    }

    /// Refresh status from git. Runs git2 on a background thread.
    pub fn refresh(&mut self, cx: &mut gpui::Context<Self>) {
        let Some(repo) = self.repo.clone() else {
            return;
        };
        cx.spawn(async move |this, cx| {
            let result = std::thread::spawn(move || {
                let repo = repo.lock().expect("git repo lock poisoned");
                let entries = repo.status().unwrap_or_default();
                let branch = repo.head_branch();
                (entries, branch)
            })
            .join()
            .expect("git status thread panicked");

            cx.update(|cx| {
                if let Some(store) = this.upgrade() {
                    store.update(cx, |store, cx| {
                        store.entries = result.0;
                        store.branch = result.1;
                        cx.emit(GitStoreEvent::StatusChanged);
                        cx.notify();
                    });
                }
            });
        })
        .detach();
    }

    /// Get diff hunks for a file. Runs on background thread.
    pub fn diff_file(
        &self,
        path: &str,
        staged: bool,
        cx: &mut gpui::Context<Self>,
        callback: impl FnOnce(Vec<DiffHunk>) + Send + 'static,
    ) {
        let Some(repo) = self.repo.clone() else {
            callback(Vec::new());
            return;
        };
        let path = path.to_owned();
        cx.spawn(async move |_this, _cx| {
            let result = std::thread::spawn(move || {
                let repo = repo.lock().expect("git repo lock poisoned");
                repo.diff_file(&path, staged).unwrap_or_default()
            })
            .join()
            .expect("git diff thread panicked");
            callback(result);
        })
        .detach();
    }

    /// Stage a file (CLI). Refreshes after.
    pub fn stage_file(&mut self, path: &str, cx: &mut gpui::Context<Self>) {
        let Some(repo) = self.repo.clone() else {
            return;
        };
        let path = path.to_owned();
        cx.spawn(async move |this, cx| {
            let result = std::thread::spawn(move || {
                let repo = repo.lock().expect("git repo lock poisoned");
                repo.stage(&[path.as_str()])
            })
            .join()
            .expect("git stage thread panicked");

            if let Err(e) = result {
                tracing::error!("git stage failed: {e}");
            }
            cx.update(|cx| {
                if let Some(store) = this.upgrade() {
                    store.update(cx, |store, cx| store.refresh(cx));
                }
            });
        })
        .detach();
    }

    /// Unstage a file (CLI). Refreshes after.
    pub fn unstage_file(&mut self, path: &str, cx: &mut gpui::Context<Self>) {
        let Some(repo) = self.repo.clone() else {
            return;
        };
        let path = path.to_owned();
        cx.spawn(async move |this, cx| {
            let result = std::thread::spawn(move || {
                let repo = repo.lock().expect("git repo lock poisoned");
                repo.unstage(&[path.as_str()])
            })
            .join()
            .expect("git unstage thread panicked");

            if let Err(e) = result {
                tracing::error!("git unstage failed: {e}");
            }
            cx.update(|cx| {
                if let Some(store) = this.upgrade() {
                    store.update(cx, |store, cx| store.refresh(cx));
                }
            });
        })
        .detach();
    }

    /// Discard changes for a file (CLI). Refreshes after.
    pub fn discard_file(&mut self, path: &str, cx: &mut gpui::Context<Self>) {
        let Some(repo) = self.repo.clone() else {
            return;
        };
        let path = path.to_owned();
        cx.spawn(async move |this, cx| {
            let result = std::thread::spawn(move || {
                let repo = repo.lock().expect("git repo lock poisoned");
                repo.discard(&[path.as_str()])
            })
            .join()
            .expect("git discard thread panicked");

            if let Err(e) = result {
                tracing::error!("git discard failed: {e}");
            }
            cx.update(|cx| {
                if let Some(store) = this.upgrade() {
                    store.update(cx, |store, cx| store.refresh(cx));
                }
            });
        })
        .detach();
    }

    #[must_use]
    pub fn entries(&self) -> &[GitFileEntry] {
        &self.entries
    }

    #[must_use]
    pub fn branch(&self) -> Option<&str> {
        self.branch.as_deref()
    }

    #[must_use]
    pub fn has_repo(&self) -> bool {
        self.repo.is_some()
    }
}
```

**Step 3: Register module**

In `crates/conescope-ui/src/state/mod.rs`, add:
```rust
pub mod git_store;
```

**Step 4: Add `git_store` to `AppState`**

Modify `crates/conescope-ui/src/state/app_state.rs`:

Add to struct fields (after `settings_store` at line 22):
```rust
pub git_store: Entity<GitStore>,
```

Add to import (line 7):
```rust
use super::git_store::GitStore;
```

In `AppState::new()` (lines 46-69), create the entity:
```rust
let git_store = cx.new(|_| GitStore::new());
```
And add to the `Self { ... }` block:
```rust
git_store,
```

Add `Debug` field to `fmt` impl (line 39):
```rust
.field("git_store", &"...")
```

**Step 5: Build and verify**

Run: `just build`
Expected: Compiles without errors.

**Step 6: Commit**

```bash
git add crates/conescope-ui/src/state/git_store.rs crates/conescope-ui/src/state/mod.rs crates/conescope-ui/src/state/app_state.rs crates/conescope-ui/Cargo.toml Cargo.lock
git commit -m "feat: add GitStore GPUI entity bridging conescope-git to UI"
```

---

## Task 3: Add `SidebarTab` state and toggle logic

**Files:**
- Modify: `crates/conescope-ui/src/state/settings_store.rs:14-35` (add `sidebar_tab` to `SessionState`)
- Modify: `crates/conescope-ui/src/state/app_state.rs:250-310` (add `toggle_git_panel`, `sidebar_tab` accessor)
- Modify: `crates/conescope-ui/src/actions.rs` (add `ToggleGitPanel`)

**Step 1: Add `SidebarTab` enum and field to `SessionState`**

In `crates/conescope-ui/src/state/settings_store.rs`, add before `SessionState` struct:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SidebarTab {
    #[serde(other)]
    Files,
    Git,
}
```

Add to `SessionState` struct (after `folder_panel_visible` at line 20):
```rust
#[serde(default)]
pub sidebar_tab: SidebarTab,
```

Add default impl — `SidebarTab` defaults to `Files` via `#[serde(other)]`, add to `SessionState::default()`:
```rust
sidebar_tab: SidebarTab::Files,
```

**Step 2: Add `ToggleGitPanel` action**

In `crates/conescope-ui/src/actions.rs`, add to the `actions!` list:
```rust
ToggleGitPanel,
```

**Step 3: Add toggle methods to `AppState`**

In `crates/conescope-ui/src/state/app_state.rs`, add import:
```rust
use super::settings_store::SidebarTab;
```

Add methods after `toggle_sidebar` (line 286):
```rust
#[must_use]
pub fn sidebar_tab(&self, cx: &gpui::App) -> SidebarTab {
    self.settings_store.read(cx).session().sidebar_tab
}

pub fn toggle_git_panel(&mut self, cx: &mut gpui::Context<Self>) {
    let mut session = self.settings_store.read(cx).session().clone();
    if session.folder_panel_visible && session.sidebar_tab == SidebarTab::Git {
        session.folder_panel_visible = false;
    } else {
        session.folder_panel_visible = true;
        session.sidebar_tab = SidebarTab::Git;
    }
    self.settings_store
        .update(cx, |store, _| store.save_session(session));
    cx.notify();
}
```

Also modify existing `toggle_sidebar` to set tab to Files:
```rust
pub fn toggle_sidebar(&mut self, cx: &mut gpui::Context<Self>) {
    let mut session = self.settings_store.read(cx).session().clone();
    if session.folder_panel_visible && session.sidebar_tab == SidebarTab::Files {
        session.folder_panel_visible = false;
    } else {
        session.folder_panel_visible = true;
        session.sidebar_tab = SidebarTab::Files;
    }
    self.settings_store
        .update(cx, |store, _| store.save_session(session));
    cx.notify();
}
```

**Step 4: Wire up keybinding and action handler**

In `crates/conescope/src/main.rs`, add `ToggleGitPanel` to imports and bind:
```rust
KeyBinding::new("cmd-shift-g", ToggleGitPanel, None),
```

In `crates/conescope-ui/src/views/app_view.rs`, add `ToggleGitPanel` import and handler (after the `ToggleTerminal` handler at line 178):
```rust
.on_action({
    let app_state = app_state.clone();
    move |_: &ToggleGitPanel, _window, cx| {
        app_state.update(cx, AppState::toggle_git_panel);
    }
})
```

**Step 5: Build and run tests**

Run: `just verify`
Expected: Compiles, tests pass, fmt+clippy clean.

**Step 6: Commit**

```bash
git add crates/conescope-ui/src/state/settings_store.rs crates/conescope-ui/src/state/app_state.rs crates/conescope-ui/src/actions.rs crates/conescope-ui/src/views/app_view.rs crates/conescope/src/main.rs
git commit -m "feat: add SidebarTab enum, ToggleGitPanel action with Cmd+Shift+G"
```

---

## Task 4: Add Git toggle button to activity bar

**Files:**
- Modify: `crates/conescope-ui/src/icons.rs:47` (add git icon constant)
- Modify: `crates/conescope-ui/src/views/activity_bar.rs:194-296` (add git toggle, pass `sidebar_tab`)
- Create: `icons/git-branch-thin.svg` (git branch icon)

**Step 1: Add git icon SVG**

Download or create a simple git-branch SVG icon. Add to `icons/` directory. Reference Phosphor Icons thin set (same as existing icons).

In `crates/conescope-ui/src/icons.rs`, add:
```rust
pub const ICON_GIT: &str = "icons/git-branch-thin.svg";
```

**Step 2: Add `sidebar_tab` to `PanelState` and rendering**

In `activity_bar.rs`, modify `PanelState` (line 194):
```rust
struct PanelState {
    sidebar: bool,
    editor: bool,
    terminal: bool,
    sidebar_tab: SidebarTab,
}
```

Add import at top:
```rust
use crate::state::settings_store::SidebarTab;
```

In `Render for ActivityBar` (line 326), read sidebar_tab:
```rust
let sidebar_tab = state.sidebar_tab(cx);
```

Update `PanelState` creation (line 326):
```rust
let panels = PanelState {
    sidebar: sidebar_visible,
    editor: editor_visible,
    terminal: terminal_visible,
    sidebar_tab,
};
```

**Step 3: Add Git toggle button in `build_left_section`**

In `build_left_section`, inside the `ViewMode::Focus` arm (lines 258-283), add the git toggle after the sidebar toggle:

```rust
.child(render_panel_toggle(
    icons::ICON_GIT,
    panels.sidebar && panels.sidebar_tab == SidebarTab::Git,
    app_state.clone(),
    AppState::toggle_git_panel,
    theme,
))
```

The sidebar toggle's `active` check should also account for tab:
```rust
.child(render_panel_toggle(
    icons::ICON_SIDEBAR,
    panels.sidebar && panels.sidebar_tab == SidebarTab::Files,
    app_state.clone(),
    AppState::toggle_sidebar,
    theme,
))
```

**Step 4: Build and test visually**

Run: `just run`
Expected: Activity bar shows 4 toggle buttons in focus mode: Sidebar, Git, Editor, Terminal. Clicking Git highlights it and toggles sidebar visibility with Git tab.

**Step 5: Commit**

```bash
git add icons/ crates/conescope-ui/src/icons.rs crates/conescope-ui/src/views/activity_bar.rs
git commit -m "feat: add Git toggle button in activity bar"
```

---

## Task 5: Create `GitPanel` view

**Files:**
- Create: `crates/conescope-ui/src/views/git_panel.rs`
- Modify: `crates/conescope-ui/src/views/mod.rs` (add `pub mod git_panel;`)

**Step 1: Create `git_panel.rs`**

```rust
// crates/conescope-ui/src/views/git_panel.rs
use gpui::prelude::*;
use gpui::{Entity, EventEmitter, MouseButton, ScrollHandle, div, px, rgba, svg};

use conescope_git::status::{FileStatus, GitFileEntry, StageStatus};

use crate::icons;
use crate::state::app_state::AppState;
use crate::state::git_store::{GitStore, GitStoreEvent};
use crate::theme::Theme;

#[derive(Debug, Clone)]
pub enum GitPanelEvent {
    OpenFile(String),
}

impl EventEmitter<GitPanelEvent> for GitPanel {}

pub struct GitPanel {
    app_state: Entity<AppState>,
    git_store: Entity<GitStore>,
    staged_expanded: bool,
    unstaged_expanded: bool,
    selected: Option<(String, StageStatus)>,
    scroll_handle: ScrollHandle,
}

impl std::fmt::Debug for GitPanel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GitPanel").finish_non_exhaustive()
    }
}

impl GitPanel {
    #[must_use]
    pub fn new(app_state: Entity<AppState>, git_store: Entity<GitStore>) -> Self {
        Self {
            app_state,
            git_store,
            staged_expanded: true,
            unstaged_expanded: true,
            selected: None,
            scroll_handle: ScrollHandle::new(),
        }
    }

    pub fn refresh(&self, cx: &mut gpui::Context<Self>) {
        self.git_store.update(cx, |store, cx| store.refresh(cx));
    }
}

impl Render for GitPanel {
    fn render(
        &mut self,
        _window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> impl IntoElement {
        let theme = self.app_state.read(cx).theme().clone();
        let store = self.git_store.read(cx);

        if !store.has_repo() {
            return div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .text_color(theme.text_disabled)
                .text_size(px(12.))
                .child("Not a git repository")
                .into_any_element();
        }

        let entries = store.entries().to_vec();
        let branch = store.branch().unwrap_or("HEAD").to_owned();

        let staged: Vec<&GitFileEntry> = entries.iter().filter(|e| e.stage == StageStatus::Staged).collect();
        let unstaged: Vec<&GitFileEntry> = entries.iter().filter(|e| e.stage == StageStatus::Unstaged).collect();

        let git_store = self.git_store.clone();

        // Header
        let header = div()
            .h(px(28.))
            .px(px(8.))
            .flex()
            .flex_row()
            .items_center()
            .justify_between()
            .border_b_1()
            .border_color(theme.border)
            .child(
                div()
                    .text_size(px(11.))
                    .text_color(theme.text_muted)
                    .child(format!("Git: {branch}")),
            )
            .child(
                div()
                    .cursor_pointer()
                    .text_size(px(11.))
                    .text_color(theme.text_faint)
                    .hover(|s| s.text_color(rgba(0xcccc_ccff)))
                    .on_mouse_down(MouseButton::Left, {
                        let git_store = git_store.clone();
                        move |_, _, cx| {
                            git_store.update(cx, |store, cx| store.refresh(cx));
                        }
                    })
                    .child("↻"),
            );

        // File list
        let mut list = div()
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .id("git-panel-scroll")
            .track_scroll(&self.scroll_handle);

        // Staged section
        if !staged.is_empty() {
            list = list.child(render_section_header(
                "Staged",
                staged.len(),
                self.staged_expanded,
                cx.listener(|this, _, _, _| {
                    this.staged_expanded = !this.staged_expanded;
                }),
                &theme,
            ));
            if self.staged_expanded {
                for entry in &staged {
                    list = list.child(render_file_entry(
                        entry,
                        &self.selected,
                        &git_store,
                        &theme,
                        cx,
                    ));
                }
            }
        }

        // Unstaged section
        if !unstaged.is_empty() {
            list = list.child(render_section_header(
                "Unstaged",
                unstaged.len(),
                self.unstaged_expanded,
                cx.listener(|this, _, _, _| {
                    this.unstaged_expanded = !this.unstaged_expanded;
                }),
                &theme,
            ));
            if self.unstaged_expanded {
                for entry in &unstaged {
                    list = list.child(render_file_entry(
                        entry,
                        &self.selected,
                        &git_store,
                        &theme,
                        cx,
                    ));
                }
            }
        }

        if staged.is_empty() && unstaged.is_empty() {
            list = list.child(
                div()
                    .px(px(12.))
                    .py(px(24.))
                    .text_size(px(12.))
                    .text_color(theme.text_disabled)
                    .child("No changes"),
            );
        }

        div()
            .size_full()
            .flex()
            .flex_col()
            .child(header)
            .child(list)
            .into_any_element()
    }
}

fn render_section_header(
    title: &str,
    count: usize,
    expanded: bool,
    on_click: impl Fn(&gpui::MouseDownEvent, &mut gpui::Window, &mut gpui::App) + 'static,
    theme: &Theme,
) -> gpui::Div {
    let arrow = if expanded { "▾" } else { "▸" };
    div()
        .h(px(24.))
        .px(px(8.))
        .flex()
        .flex_row()
        .items_center()
        .gap(px(4.))
        .cursor_pointer()
        .bg(theme.background)
        .text_size(px(11.))
        .text_color(theme.text_muted)
        .on_mouse_down(MouseButton::Left, on_click)
        .child(arrow)
        .child(format!("{title} ({count})"))
}

fn render_file_entry(
    entry: &GitFileEntry,
    selected: &Option<(String, StageStatus)>,
    git_store: &Entity<GitStore>,
    theme: &Theme,
    cx: &mut gpui::Context<GitPanel>,
) -> gpui::Div {
    let is_selected = selected
        .as_ref()
        .is_some_and(|(p, s)| p == &entry.path && s == &entry.stage);

    let (status_label, status_color) = match entry.status {
        FileStatus::Modified => ("M", theme.accent),
        FileStatus::Added => ("A", rgba(0x73c9_91ff)),
        FileStatus::Deleted => ("D", rgba(0xe06c_75ff)),
        FileStatus::Renamed => ("R", rgba(0xe5c0_7bff)),
        FileStatus::Untracked => ("??", theme.text_faint),
    };

    let file_name = std::path::Path::new(&entry.path)
        .file_name()
        .map_or_else(|| entry.path.clone(), |n| n.to_string_lossy().to_string());

    let path = entry.path.clone();
    let staged = entry.stage == StageStatus::Staged;
    let stage_status = entry.stage;
    let git_store_click = git_store.clone();

    let bg = if is_selected {
        theme.element_hover
    } else {
        theme.panel
    };

    div()
        .h(px(24.))
        .px(px(12.))
        .flex()
        .flex_row()
        .items_center()
        .gap(px(6.))
        .bg(bg)
        .cursor_pointer()
        .hover(|s| s.bg(theme.element_hover))
        .text_size(px(12.))
        .on_mouse_down(MouseButton::Left, cx.listener(move |this, _, _, cx| {
            this.selected = Some((path.clone(), stage_status));
            this.git_store.update(cx, |store, cx| {
                cx.emit(GitStoreEvent::OpenDiff {
                    path: path.clone(),
                    staged,
                });
            });
            cx.notify();
        }))
        .child(
            div()
                .text_color(status_color)
                .text_size(px(10.))
                .w(px(18.))
                .child(status_label),
        )
        .child(
            div()
                .text_color(theme.text)
                .flex_1()
                .overflow_hidden()
                .child(file_name),
        )
        .child(
            div()
                .text_color(theme.text_faint)
                .text_size(px(10.))
                .overflow_hidden()
                .child(entry.path.clone()),
        )
}
```

**Step 2: Register module**

In `crates/conescope-ui/src/views/mod.rs`:
```rust
pub mod git_panel;
```

**Step 3: Build and verify**

Run: `just build`
Expected: Compiles.

**Step 4: Commit**

```bash
git add crates/conescope-ui/src/views/git_panel.rs crates/conescope-ui/src/views/mod.rs
git commit -m "feat: add GitPanel view with staged/unstaged file list"
```

---

## Task 6: Create `DiffViewer` view

**Files:**
- Create: `crates/conescope-ui/src/views/diff_viewer.rs`
- Modify: `crates/conescope-ui/src/views/mod.rs` (add `pub mod diff_viewer;`)

**Step 1: Create `diff_viewer.rs`**

```rust
// crates/conescope-ui/src/views/diff_viewer.rs
use gpui::prelude::*;
use gpui::{Entity, ScrollHandle, SharedString, div, px, rgba};

use conescope_git::diff::{DiffHunk, DiffLine, LineOrigin};

use crate::state::app_state::AppState;
use crate::theme::Theme;

pub struct DiffViewer {
    app_state: Entity<AppState>,
    file_path: Option<String>,
    staged: bool,
    hunks: Vec<DiffHunk>,
    scroll_handle: ScrollHandle,
}

impl std::fmt::Debug for DiffViewer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DiffViewer")
            .field("file_path", &self.file_path)
            .finish_non_exhaustive()
    }
}

impl DiffViewer {
    #[must_use]
    pub fn new(app_state: Entity<AppState>) -> Self {
        Self {
            app_state,
            file_path: None,
            staged: false,
            hunks: Vec::new(),
            scroll_handle: ScrollHandle::new(),
        }
    }

    /// Load diff data for display.
    pub fn show_diff(&mut self, path: &str, staged: bool, hunks: Vec<DiffHunk>, cx: &mut gpui::Context<Self>) {
        self.file_path = Some(path.to_owned());
        self.staged = staged;
        self.hunks = hunks;
        cx.notify();
    }

    pub fn clear(&mut self, cx: &mut gpui::Context<Self>) {
        self.file_path = None;
        self.hunks.clear();
        cx.notify();
    }

    #[must_use]
    pub fn file_path(&self) -> Option<&str> {
        self.file_path.as_deref()
    }

    #[must_use]
    pub fn is_staged(&self) -> bool {
        self.staged
    }
}

impl Render for DiffViewer {
    fn render(
        &mut self,
        _window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> impl IntoElement {
        let theme = self.app_state.read(cx).theme().clone();
        let font_family = self.app_state.read(cx)
            .settings_store.read(cx)
            .settings().font_family.clone();

        let Some(ref path) = self.file_path else {
            return div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .text_size(px(12.))
                .text_color(theme.text_disabled)
                .child("No diff selected")
                .into_any_element();
        };

        let stage_label = if self.staged { "staged" } else { "unstaged" };

        // Header
        let header = div()
            .h(px(28.))
            .px(px(12.))
            .flex()
            .flex_row()
            .items_center()
            .gap(px(8.))
            .border_b_1()
            .border_color(theme.border)
            .bg(theme.background)
            .text_size(px(12.))
            .child(
                div().text_color(theme.text).child(path.clone()),
            )
            .child(
                div()
                    .text_color(theme.text_faint)
                    .text_size(px(10.))
                    .child(format!("({stage_label})")),
            );

        // Diff content
        let mut content = div()
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .id("diff-viewer-scroll")
            .track_scroll(&self.scroll_handle)
            .font_family(SharedString::from(font_family))
            .text_size(px(13.))
            .bg(theme.editor_bg);

        if self.hunks.is_empty() {
            content = content.child(
                div()
                    .px(px(12.))
                    .py(px(24.))
                    .text_color(theme.text_disabled)
                    .child("No changes in this file"),
            );
        } else {
            for hunk in &self.hunks {
                // Hunk header
                content = content.child(
                    div()
                        .px(px(12.))
                        .py(px(4.))
                        .text_color(theme.text_faint)
                        .text_size(px(11.))
                        .bg(rgba(0x2222_44ff))
                        .child(hunk.header.clone()),
                );

                // Lines
                for line in &hunk.lines {
                    content = content.child(render_diff_line(line, &theme));
                }
            }
        }

        div()
            .size_full()
            .flex()
            .flex_col()
            .child(header)
            .child(content)
            .into_any_element()
    }
}

fn render_diff_line(line: &DiffLine, theme: &Theme) -> gpui::Div {
    let (bg, prefix, text_color) = match line.origin {
        LineOrigin::Addition => (rgba(0x2a3a_2aff), "+", rgba(0x98c3_79ff)),
        LineOrigin::Deletion => (rgba(0x3a2a_2aff), "-", rgba(0xe06c_75ff)),
        LineOrigin::Context => (rgba(0x0000_0000), " ", theme.text.into()),
    };

    let old_ln = line
        .old_lineno
        .map_or_else(|| "   ".to_owned(), |n| format!("{n:3}"));
    let new_ln = line
        .new_lineno
        .map_or_else(|| "   ".to_owned(), |n| format!("{n:3}"));

    // Trim trailing newline from content for display
    let content = line.content.trim_end_matches('\n');

    div()
        .h(px(18.))
        .flex()
        .flex_row()
        .items_center()
        .bg(bg)
        .text_size(px(13.))
        // Line number gutters
        .child(
            div()
                .w(px(36.))
                .text_color(theme.text_faint)
                .text_size(px(10.))
                .px(px(2.))
                .child(old_ln),
        )
        .child(
            div()
                .w(px(36.))
                .text_color(theme.text_faint)
                .text_size(px(10.))
                .px(px(2.))
                .child(new_ln),
        )
        // Origin prefix
        .child(
            div()
                .w(px(14.))
                .text_color(text_color)
                .child(prefix),
        )
        // Content
        .child(
            div()
                .flex_1()
                .overflow_hidden()
                .text_color(text_color)
                .child(content.to_owned()),
        )
}
```

**Step 2: Register module**

In `crates/conescope-ui/src/views/mod.rs`:
```rust
pub mod diff_viewer;
```

**Step 3: Build and verify**

Run: `just build`
Expected: Compiles.

**Step 4: Commit**

```bash
git add crates/conescope-ui/src/views/diff_viewer.rs crates/conescope-ui/src/views/mod.rs
git commit -m "feat: add DiffViewer for unified diff display in editor area"
```

---

## Task 7: Integrate `GitPanel` and `DiffViewer` into `FocusView`

**Files:**
- Modify: `crates/conescope-ui/src/views/focus_view.rs:30-40,520-714` (add git_panel, diff_viewer, sidebar tab switching)

**Step 1: Add fields to `FocusView`**

Add imports:
```rust
use crate::state::git_store::{GitStore, GitStoreEvent};
use crate::state::settings_store::SidebarTab;
use crate::views::diff_viewer::DiffViewer;
use crate::views::git_panel::{GitPanel, GitPanelEvent};
```

Add to `FocusView` struct:
```rust
git_panel: Entity<GitPanel>,
diff_viewer: Entity<DiffViewer>,
```

**Step 2: Create entities in `FocusView::new`**

In `FocusView::new()`, after `editor_tabs` creation:
```rust
let git_store = app_state.read(cx).git_store.clone();
let git_panel = cx.new(|_| GitPanel::new(app_state.clone(), git_store.clone()));
let diff_viewer = cx.new(|_| DiffViewer::new(app_state.clone()));
```

**Step 3: Subscribe to `GitStoreEvent::OpenDiff`**

Wire up diff viewing: when git panel emits OpenDiff, fetch hunks and show in diff_viewer:
```rust
let dv = diff_viewer.clone();
let gs = git_store.clone();
cx.subscribe(&git_store, move |_this, _store, event, cx| {
    if let GitStoreEvent::OpenDiff { path, staged } = event {
        let path = path.clone();
        let staged = *staged;
        let dv = dv.clone();
        gs.update(cx, |store, cx| {
            store.diff_file(&path, staged, cx, move |hunks| {
                // callback runs after background thread
                // We need to update DiffViewer - but callback is Send, no cx
                // Instead, store hunks and notify
            });
        });
    }
});
```

NOTE: The callback-based approach from `GitStore::diff_file` is tricky because we need a `cx` in the callback. **Simpler approach**: change `GitStore::diff_file` to update an internal `pending_diff` field, then DiffViewer reads it on next render. OR better: make `diff_file` emit a `DiffReady` event. Let the implementer choose the cleanest pattern — the key contract is:

1. GitPanel click → emits `GitStoreEvent::OpenDiff`
2. FocusView subscribes, calls `git_store.diff_file()`
3. When diff is ready, FocusView calls `diff_viewer.update(cx, |dv, cx| dv.show_diff(...))`
4. FocusView also ensures editor panel is visible and switches to diff tab

**Step 4: Subscribe to `GitPanelEvent::OpenFile`**

Wire up "open file" from context menu:
```rust
cx.subscribe(&git_panel, move |_this, _panel, event, cx| {
    let GitPanelEvent::OpenFile(path) = event;
    et.update(cx, |tabs, cx| tabs.open_tab(path, cx));
    cv.update(cx, |viewer, cx| viewer.open_file(path, cx));
});
```

**Step 5: Update sidebar rendering**

In `FocusView::render()`, replace the sidebar panel content based on tab:
```rust
let sidebar_tab = state.sidebar_tab(cx);
```

Change line 697 from:
```rust
.child(self.file_tree.clone()),
```
To:
```rust
.child(match sidebar_tab {
    SidebarTab::Files => self.file_tree.clone().into_any_element(),
    SidebarTab::Git => self.git_panel.clone().into_any_element(),
}),
```

**Step 6: Trigger `GitStore::set_project` on instance switch**

In the instance-switch block (around line 564), after updating `editor_tabs.current_project_id`, add:
```rust
let project_path = project_id.as_ref().and_then(|pid| {
    self.app_state
        .read(cx)
        .project_store
        .read(cx)
        .get(pid)
        .map(|p| p.path.clone())
});
self.app_state.read(cx).git_store.update(cx, |store, cx| {
    store.set_project(project_path.as_deref(), cx);
});
```

**Step 7: Add entities to `Self { ... }` return**

```rust
git_panel,
diff_viewer,
```

**Step 8: Build and test**

Run: `just run`
Expected: Sidebar switches between FileTree and GitPanel. GitPanel shows file statuses. Clicking a file shows diff in DiffViewer.

**Step 9: Commit**

```bash
git add crates/conescope-ui/src/views/focus_view.rs
git commit -m "feat: integrate GitPanel and DiffViewer into FocusView sidebar"
```

---

## Task 8: Add context menu to `GitPanel`

**Files:**
- Modify: `crates/conescope-ui/src/views/git_panel.rs` (add right-click context menu)

**Step 1: Add context menu rendering**

GPUI doesn't have a built-in context menu widget. Implement a simple popup overlay approach:

Add to `GitPanel` struct:
```rust
context_menu: Option<ContextMenuState>,
```

```rust
struct ContextMenuState {
    path: String,
    stage: StageStatus,
    position: gpui::Point<gpui::Pixels>,
}
```

**Step 2: Add right-click handler to file entries**

On `render_file_entry`, add:
```rust
.on_mouse_down(MouseButton::Right, cx.listener(move |this, event, _window, _cx| {
    this.context_menu = Some(ContextMenuState {
        path: path.clone(),
        stage: stage_status,
        position: event.position,
    });
}))
```

**Step 3: Render context menu overlay**

In `GitPanel::render()`, after the main content, conditionally render a positioned overlay:
```rust
.when(self.context_menu.is_some(), |el| {
    el.child(render_context_menu(/* ... */))
})
```

The context menu shows 3-4 items:
- Stage/Unstage (toggles based on current status)
- Discard changes
- Open file
- Show diff

Each item calls the corresponding `git_store` method or emits events.

**Step 4: Dismiss on click outside**

Add a full-screen transparent overlay behind the menu that dismisses on click.

**Step 5: Build and test**

Run: `just run`
Expected: Right-click on git panel file shows context menu with working actions.

**Step 6: Commit**

```bash
git add crates/conescope-ui/src/views/git_panel.rs
git commit -m "feat: add right-click context menu to GitPanel"
```

---

## Task 9: Wire `DiffViewer` into `EditorTabs`

**Files:**
- Modify: `crates/conescope-ui/src/views/editor_tabs.rs:21-26` (extend `EditorTab` enum)
- Modify: `crates/conescope-ui/src/views/focus_view.rs` (render DiffViewer for diff tabs)

**Step 1: Extend `EditorTab` to support diff tabs**

In `editor_tabs.rs`, change `EditorTab` struct to add an optional diff marker:
```rust
#[derive(Debug, Clone)]
pub struct EditorTab {
    pub path: String,
    pub name: String,
    pub modified: bool,
    pub diff_mode: Option<bool>, // Some(staged) for diff tabs, None for file tabs
}
```

**Step 2: Add `open_diff_tab` method**

```rust
pub fn open_diff_tab(&mut self, path: &str, staged: bool, cx: &mut gpui::Context<Self>) {
    // Check if this diff is already open
    if let Some(idx) = self.tabs.iter().position(|t| t.path == path && t.diff_mode == Some(staged)) {
        self.active_index = Some(idx);
        cx.emit(EditorTabsEvent::SelectTab(idx));
        cx.notify();
        return;
    }

    let file_name = std::path::Path::new(path)
        .file_name()
        .map_or_else(|| path.to_owned(), |n| n.to_string_lossy().to_string());

    self.tabs.push(EditorTab {
        path: path.to_owned(),
        name: format!("{file_name} [diff]"),
        modified: false,
        diff_mode: Some(staged),
    });
    let idx = self.tabs.len() - 1;
    self.active_index = Some(idx);
    cx.emit(EditorTabsEvent::SelectTab(idx));
    cx.notify();
}
```

**Step 3: Update `FocusView` to render `DiffViewer` for diff tabs**

In `render_editor_area`, check the active tab's `diff_mode`:
- If `None` → render `CodeEditor` (existing behavior)
- If `Some(staged)` → render `DiffViewer`

**Step 4: Build and test**

Run: `just run`
Expected: Clicking a file in GitPanel opens a "[diff]" tab. Tab shows DiffViewer. Regular file tabs still show CodeEditor.

**Step 5: Commit**

```bash
git add crates/conescope-ui/src/views/editor_tabs.rs crates/conescope-ui/src/views/focus_view.rs
git commit -m "feat: wire DiffViewer into EditorTabs with diff tab support"
```

---

## Task 10: Final integration, polish, and verification

**Files:**
- Modify: `crates/conescope-ui/src/views/focus_view.rs` (sidebar tab strip header)
- All files for verification

**Step 1: Add sidebar tab strip**

At the top of the sidebar content area, render a small tab strip:
```rust
div()
    .h(px(24.))
    .flex()
    .flex_row()
    .items_center()
    .border_b_1()
    .border_color(theme.border)
    .child(render_sidebar_tab("Files", sidebar_tab == SidebarTab::Files, /* click */))
    .child(render_sidebar_tab("Git", sidebar_tab == SidebarTab::Git, /* click */))
```

Clicking a tab calls `AppState::toggle_sidebar` or `AppState::toggle_git_panel` accordingly.

**Step 2: Run full verify**

Run: `just verify`
Expected: `fmt-check` + `clippy` + all tests pass.

**Step 3: Visual testing**

Run: `just run`

Test the following scenarios:
1. Open a project instance that has a git repo
2. Click Git toggle in activity bar → sidebar shows GitPanel with staged/unstaged files
3. Click Sidebar toggle → sidebar switches to FileTree
4. Click a file in GitPanel → diff tab opens in editor
5. Right-click file → context menu with Stage/Unstage/Discard/Open
6. Stage a file → it moves from Unstaged to Staged
7. Discard changes → file disappears from list
8. Switch instances → GitPanel updates to new project's repo
9. Non-git project → shows "Not a git repository"
10. `Cmd+Shift+G` keyboard shortcut toggles git panel

**Step 4: Commit**

```bash
git add -A
git commit -m "feat: polish git panel integration with sidebar tab strip"
```

---

## Task 11: Update CLAUDE.md

**Files:**
- Modify: `CLAUDE.md`

**Step 1: Add git-related crate to workspace docs**

In the workspace crates section, add:
```
├── conescope-git/      # Git operations: hybrid git2 + CLI backend
│   └── src/
│       ├── cli.rs              # Shell git command wrapper
│       ├── repository.rs       # GitRepo: status, diff, stage, unstage, discard
│       ├── status.rs           # FileStatus, StageStatus, GitFileEntry types
│       └── diff.rs             # DiffHunk, DiffLine types
```

Update the views section to mention new views.

**Step 2: Commit**

```bash
git add CLAUDE.md
git commit -m "docs: update CLAUDE.md with git support crate and views"
```
