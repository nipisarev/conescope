# Git Support — Design Document

**Date:** 2026-02-09
**Status:** Approved

## Scope

Add a Git sidebar panel showing staged/unstaged files with diff viewing, stage/unstage/discard actions, and a context menu. The Git panel replaces the File Tree in the sidebar via tab switching.

**Out of scope (future):** File tree highlighting for changed files/directories, editor gutter change markers, git worktree instance support. Architecture accounts for these.

## Architecture Overview

```
conescope-git (new crate, no UI deps)
├── repository.rs   — GitRepo: hybrid git2 + CLI wrapper
├── cli.rs          — async shell git command execution
├── status.rs       — FileStatus, GitFileEntry types
└── diff.rs         — DiffHunk, DiffLine types

conescope-ui
├── state/git_store.rs    — GitStore entity (GPUI state bridge)
├── views/git_panel.rs    — Git file list view (sidebar)
├── views/diff_viewer.rs  — Read-only unified diff view (editor area)
└── (modified) activity_bar, focus_view, app_state, editor_tabs, actions
```

---

## 1. New Crate: `conescope-git`

Pure Rust, no GPUI dependency. Hybrid backend: git2 for reads, shell git for writes.

### 1.1 `repository.rs` — GitRepo

```rust
pub struct GitRepo {
    repo: git2::Repository,
    git_binary: PathBuf,
    work_dir: PathBuf,
}
```

**git2 for reads** (fast, in-process):
- `open(path: &Path) -> Result<Self>` — discover repo via `git2::Repository::discover()`
- `status() -> Result<Vec<GitFileEntry>>` — file statuses via `git2::Repository::statuses()`
- `diff_file(path: &str, staged: bool) -> Result<Vec<DiffHunk>>` — unified diff via `git2::Diff`
- `head_branch() -> Option<String>` — current branch name
- `load_blob(path: &str) -> Result<Vec<u8>>` — file content from HEAD tree

**CLI for writes** (robust, respects hooks/gitconfig):
- `stage(paths: &[&str]) -> Result<()>` — `git add -- <paths>`
- `unstage(paths: &[&str]) -> Result<()>` — `git reset HEAD -- <paths>`
- `discard(paths: &[&str]) -> Result<()>` — `git checkout -- <paths>` (tracked) or delete (untracked)

### 1.2 `cli.rs` — Git CLI Wrapper

```rust
pub struct GitCli {
    binary: PathBuf,
    work_dir: PathBuf,
}

impl GitCli {
    pub fn find_binary() -> Result<PathBuf>;  // via which::which("git")
    pub async fn run(&self, args: &[&str]) -> Result<String>;  // stdout or error with stderr
}
```

- Runs `Command::new(&self.binary)` with `-C <work_dir>`
- Captures stdout/stderr, returns `Result<String>`
- Errors include stderr content for diagnostics

### 1.3 `status.rs` — Types

```rust
pub struct GitFileEntry {
    pub path: String,            // relative to repo root
    pub status: FileStatus,
    pub stage: StageStatus,
}

pub enum FileStatus {
    Modified,
    Added,
    Deleted,
    Renamed,
    Untracked,
}

pub enum StageStatus {
    Staged,
    Unstaged,
}
```

Status mapping from `git2::Status` bitflags:
- `INDEX_*` flags → `StageStatus::Staged`
- `WT_*` flags → `StageStatus::Unstaged`
- `WT_NEW` → `FileStatus::Untracked` + `StageStatus::Unstaged`
- Files with both `INDEX_*` and `WT_*` flags → two entries (one staged, one unstaged)

### 1.4 `diff.rs` — Diff Types

```rust
pub struct DiffHunk {
    pub header: String,          // @@ -12,6 +12,8 @@ fn main()
    pub lines: Vec<DiffLine>,
}

pub struct DiffLine {
    pub origin: LineOrigin,
    pub old_lineno: Option<u32>,
    pub new_lineno: Option<u32>,
    pub content: String,
}

pub enum LineOrigin {
    Context,
    Addition,
    Deletion,
}
```

Built from `git2::Diff::print(DiffFormat::Patch, ...)` callback, grouped by hunk.

---

## 2. GPUI State Layer: `GitStore` Entity

`conescope-ui/src/state/git_store.rs` — bridges `conescope-git` with the UI.

```rust
pub struct GitStore {
    repo: Option<Arc<Mutex<GitRepo>>>,
    entries: Vec<GitFileEntry>,
    current_path: Option<String>,
    branch: Option<String>,
}
```

### Lifecycle
- `AppState` owns `Entity<GitStore>`, created at startup
- Instance focus change → `GitStore::set_project(path, cx)` opens/switches repo
- No `.git` found → `repo = None`, entries cleared, Git panel shows "Not a git repository"

### Refresh Strategy
- `refresh(cx)` — spawns background task via `cx.spawn()`, queries git2 status, updates `entries` + `branch`, calls `cx.notify()`
- Triggered on: instance focus change, after stage/unstage/discard, manual refresh action
- No filesystem watcher — explicit refresh keeps complexity low

### Write Operations
All async, delegate to CLI, refresh on success:
```rust
pub fn stage_file(&mut self, path: &str, cx: &mut Context<Self>);
pub fn unstage_file(&mut self, path: &str, cx: &mut Context<Self>);
pub fn discard_file(&mut self, path: &str, cx: &mut Context<Self>);
```

Pattern: spawn CLI command → await result → call `self.refresh(cx)` → emit event.

### Events
```rust
pub enum GitStoreEvent {
    StatusChanged,                    // entries updated
    OpenDiff { path: String, staged: bool },  // show diff in editor
}
```

### Threading Model
- `GitRepo` wrapped in `Arc<Mutex<GitRepo>>` (git2::Repository is `!Sync`)
- All git2 reads: clone Arc, spawn background thread, lock mutex, read, return via channel
- CLI writes: async Command, no mutex needed
- Single mutex, no nested locks → no deadlock risk
- No concurrent writes (all dispatched sequentially from UI events)

---

## 3. Git Panel View

`conescope-ui/src/views/git_panel.rs` — renders file list in sidebar.

### Layout
```
┌─ Git ───────────────────┐
│ ↻ Refresh    main       │  header: refresh button + branch name
│─────────────────────────│
│ ▾ Staged (3)            │  collapsible section
│   M  src/main.rs        │
│   A  src/new_file.rs    │
│   D  old_module.rs      │
│─────────────────────────│
│ ▾ Unstaged (2)          │  collapsible section
│   M  Cargo.toml         │
│   ?? test.txt           │  untracked files
└─────────────────────────┘
```

### Structure
```rust
pub struct GitPanel {
    app_state: Entity<AppState>,
    git_store: Entity<GitStore>,
    staged_expanded: bool,
    unstaged_expanded: bool,
    selected: Option<String>,
    scroll_handle: ScrollHandle,
}
```

### Behavior
- **Click file** → emits `GitStoreEvent::OpenDiff(path, staged)` → FocusView opens diff in editor
- **Right-click file** → context menu:
  - Stage file / Unstage file (toggles based on current stage status)
  - Discard changes (with confirmation for tracked files)
  - Open file (opens in CodeEditor, not diff)
  - Check diff (same as click — opens DiffViewer)
- **Status icons:** `M` modified (yellow), `A` added (green), `D` deleted (red), `??` untracked (grey)
- **Sorting:** alphabetical within each section
- **Empty state:** "No changes" or "Not a git repository"

### Context Menu Actions
```rust
actions!(git, [
    StageFile, UnstageFile, DiscardFile,
    OpenFile, ShowDiff, RefreshStatus,
    ToggleGitPanel,
])
```

---

## 4. Diff Viewer

`conescope-ui/src/views/diff_viewer.rs` — read-only unified diff in editor area.

### Layout
```
┌─ Editor Tabs ───────────────────────┐
│  main.rs  │  Cargo.toml [diff] │    │  diff tabs marked with [diff]
├─────────────────────────────────────┤
│ src/main.rs (unstaged)              │  file path + staged/unstaged label
├─────────────────────────────────────┤
│ @@ -12,6 +12,8 @@ fn main() {       │  hunk header (dimmed)
│  12 │  12 │     let app = App::new();│  context (default)
│  13 │     │-    app.run();           │  deletion (red bg)
│     │  13 │+    app.start();         │  addition (green bg)
│     │  14 │+    app.init_git();      │  addition (green bg)
│  14 │  15 │     Ok(())              │  context
└─────────────────────────────────────┘
```

### Structure
```rust
pub struct DiffViewer {
    file_path: String,
    staged: bool,
    hunks: Vec<DiffHunk>,
    scroll_handle: ScrollHandle,
}
```

### Rendering
Custom `Render` impl — no Input widget, purely read-only styled divs:
- Monospace font, inherits terminal font from settings
- Two gutter columns: old line number, new line number
- Origin indicator: `-` (red), `+` (green), ` ` (default)
- Line background: faint red for deletions, faint green for additions
- Hunk headers: dimmed text, full width
- Scrollable with `.overflow_y_scroll()`

### EditorTabs Integration
Currently `EditorTabs` tracks `Vec<String>` (file paths). Extended to:
```rust
pub enum EditorTab {
    File(String),                         // regular file in CodeEditor
    Diff { path: String, staged: bool },  // diff in DiffViewer
}
```

Tab title: filename for `File`, `filename [diff]` for `Diff`.

FocusView renders `CodeEditor` or `DiffViewer` based on active tab variant.

---

## 5. Activity Bar & Sidebar Integration

### Activity Bar Changes
Focus-mode left section gets a fourth toggle:
```
[Sidebar] [Git] [Editor] [Terminal]
  📁       🔀      📝       >_
```

Rendered via existing `render_panel_toggle()` pattern with a git icon.

### Sidebar Tab State
```rust
pub enum SidebarTab {
    Files,
    Git,
}

// Added to SessionState (persisted):
pub sidebar_tab: SidebarTab,
```

### Toggle Logic
```rust
// AppState methods:
fn toggle_sidebar(&mut self, cx) {
    if self.sidebar_visible() && self.sidebar_tab() == SidebarTab::Files {
        self.set_sidebar_visible(false, cx);
    } else {
        self.set_sidebar_visible(true, cx);
        self.set_sidebar_tab(SidebarTab::Files, cx);
    }
}

fn toggle_git_panel(&mut self, cx) {
    if self.sidebar_visible() && self.sidebar_tab() == SidebarTab::Git {
        self.set_sidebar_visible(false, cx);
    } else {
        self.set_sidebar_visible(true, cx);
        self.set_sidebar_tab(SidebarTab::Git, cx);
    }
}
```

### FocusView Sidebar Rendering
```rust
// In FocusView::render(), sidebar area:
match sidebar_tab {
    SidebarTab::Files => self.file_tree.render(),
    SidebarTab::Git => self.git_panel.render(),
}
```

Small tab strip at sidebar top (`[Files] [Git]`) for switching without activity bar.

### Keyboard Shortcuts
- `Cmd+Shift+G` → `ToggleGitPanel` (matches VS Code)
- `Cmd+B` → `ToggleSidebar` (existing, unchanged)

---

## 6. Dependency Changes

### Workspace Cargo.toml
```toml
[workspace]
members = [
    # ... existing
    "crates/conescope-git",
]
```

### crates/conescope-git/Cargo.toml
```toml
[package]
name = "conescope-git"
version = "0.1.0"
edition = "2024"

[dependencies]
git2 = "0.20"
which = "7"
anyhow = "1"

[dev-dependencies]
tempfile = "3"
```

### crates/conescope-ui/Cargo.toml
```toml
[dependencies]
conescope-git = { path = "../conescope-git" }
```

---

## 7. Quality & Testing

### Checks Relevant to This Feature
| Concern | Mitigation |
|---------|-----------|
| Lifetime/borrow | `git2::Repository` is `!Sync` — `Arc<Mutex<>>` enforced by compiler |
| Deadlocks | Single mutex per GitRepo, no nested locks |
| Races | CLI writes followed by git2 reads — always refresh after writes |
| Performance | Background threads for all git ops, UI never blocks |
| No unsafe | Pure safe Rust throughout |

### Tests

**Unit tests** (`conescope-git`):
- Create temp repo with `git2::Repository::init()` + `tempfile::tempdir()`
- Add/modify files, verify `status()` returns correct entries
- Stage files, verify staged vs unstaged separation
- Generate diffs, verify hunk/line structure
- Test CLI wrapper with real git binary

**Integration test** (`conescope-git`):
- Full cycle: init repo → create file → status (untracked) → stage → status (staged) → modify → status (staged + unstaged) → discard unstaged → verify clean working tree

### No Additional Tooling Needed
- No unsafe code → no Miri/ASan/TSan
- No nightly features required
- Existing clippy strict mode + `deny(warnings)` covers the new crate
- `just verify` gate unchanged

---

## 8. Future Requirements (Architecture Hooks)

These are **not implemented** in this phase but the architecture supports them:

### File Tree Highlighting
`FileTree` subscribes to `GitStoreEvent::StatusChanged`, queries `git_store.read(cx).entries` to color files and their ancestor directories. `GitFileEntry.path` is relative to repo root — walk path components to mark parent dirs.

### Editor Gutter Change Markers
`CodeEditor` subscribes to `GitStoreEvent::StatusChanged`. For the currently open file, call `GitRepo::load_blob(path)` to get HEAD content, diff against buffer content, mark changed lines in the gutter. Requires adding a gutter rendering layer to CodeEditor.

### Git Worktree Instances
`GitRepo` gets `worktree_add(path, branch)` / `worktree_list()` / `worktree_remove(path)` methods via CLI. `NewInstanceModal` gets a "New Worktree" option that creates a worktree and opens a new instance pointed at it. Each instance's `GitStore` operates independently on its worktree's repo.
