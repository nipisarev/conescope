# CLAUDE.md

Conescope - native macOS app for managing multiple Claude Code CLI instances.
Rust rewrite using GPUI framework (from Zed editor).

## Behavior

- Be concise, sacrifice grammar for brevity
- **CRITICAL**: Update this doc on notable architecture changes
- Always use context7 MCP for library documentation
- Prefer editing existing files over creating new ones

## Commands

Uses [`just`](https://github.com/casey/just) task runner. Run `just` to see all commands.

```bash
just run             # Run the app (debug)
just run-release     # Run the app (release)
just build           # Build workspace (debug)
just build-release   # Build workspace (release)
just test            # Run all tests
just test-verbose    # Run tests with stdout
just check           # Clippy lint check (deny warnings)
just fmt             # Auto-format code
just fmt-check       # Check formatting without modifying
just verify          # Pre-commit gate: fmt-check + clippy + test
just fix             # Auto-format first, then clippy + test
just clean           # Remove target/
just watch           # Clippy on file changes (needs cargo-watch)
just update          # cargo update
```

**Always run `just verify` before committing.** It is the CI gate.

## Workspace Crates

```
crates/
├── conescope/          # Binary crate (conescope-rs). App entry, window setup, keybindings.
├── conescope-ui/       # GPUI views, state management, actions
│   ├── src/actions.rs          # GPUI actions (keyboard shortcuts)
│   ├── src/state/
│   │   ├── app_state.rs        # Root state: view mode, focused instance, modals
│   │   ├── instance_list.rs    # Entity list, add/remove/restore instances
│   │   ├── instance_entry.rs   # Single instance: PTY handle, terminal view, metadata
│   │   ├── project_store.rs    # Saved project paths
│   │   ├── settings_store.rs   # Session state (DB) + user settings (JSON file)
│   │   ├── git_store.rs        # GitStore entity: bridges conescope-git to UI
│   │   └── db_worker.rs        # Async SQLite via flume channels
│   └── src/views/
│       ├── app_view.rs         # Root view, action handlers, mode switching
│       ├── overview_grid.rs    # Grid of instance tiles
│       ├── focus_view.rs       # Single instance full-screen
│       ├── activity_bar.rs     # Left sidebar with instance icons
│       ├── git_panel.rs        # Git sidebar: staged/unstaged files, context menu
│       ├── diff_viewer.rs      # Unified diff viewer for editor area
│       ├── new_instance_modal.rs # Create instance dialog
│       ├── settings_view.rs    # Full-screen JSON settings editor
│       └── top_bar.rs          # Window title bar
├── conescope-git/      # Git operations: hybrid git2 + CLI backend
│   └── src/
│       ├── cli.rs              # Shell git command wrapper
│       ├── repository.rs       # GitRepo: status, diff, stage, unstage, discard
│       ├── status.rs           # FileStatus, StageStatus, GitFileEntry types
│       └── diff.rs             # DiffHunk, DiffLine types
├── conescope-core/     # Data types + SQLite (rusqlite). No UI deps.
│   └── src/
│       ├── database.rs         # DB init, migrations, WAL mode
│       ├── instance.rs         # Instance model + queries
│       ├── project.rs          # Project model + queries
│       ├── question.rs         # Question model + queries
│       ├── settings.rs         # SettingsJson typed struct + file I/O (~/.conescope/settings.json)
│       └── shell.rs            # Shell/environment helpers
├── conescope-pty/      # PTY spawning via portable-pty
└── conescope-platform/ # macOS-specific (window styling)
```

## Key Patterns

### GPUI Framework
- Views are `Entity<T>` (reference-counted, observable). Created via `cx.new(|cx| T::new(cx))`.
- `Render` trait on views returns `impl IntoElement`. Fluent div builder API.
- Actions defined with `actions!(namespace, [Name])` macro, bound with `cx.bind_keys()`.
- Action handlers on elements: `.on_action(|&Action, &mut Window, &mut App| { ... })`.
- State changes: `entity.update(cx, |state, cx| { ... cx.notify(); })`.
- Async: `cx.spawn(async move |mut cx| { ... })`, then `cx.update(|cx| ...)` to touch state.

### State Architecture
- `AppState` is the root entity, holds `Entity<InstanceList>`, `Entity<ProjectStore>`, etc.
- `DbWorker` runs SQLite on a background thread, communicates via `flume` channels.
- All DB writes are fire-and-forget sends. Reads return a `flume::Receiver` awaited in async tasks.
- User settings stored in `~/.conescope/settings.json` as typed `SettingsJson` struct. Edited via full-screen `SettingsView` (ViewMode::Settings).
- Session state (window bounds, panel visibility, view mode) stored in DB `settings` table (`session_state` key).
- Instance terminals use custom `terminal` module wrapping `alacritty_terminal` (Zed's fork, pure Rust VT parser).

### Git Integration
- Hybrid backend: `git2` crate for reads (status, diff, branch), shell `git` CLI for writes (stage, unstage, discard).
- `GitRepo` wraps `git2::Repository` (not `Sync`), stored as `Arc<Mutex<GitRepo>>` in `GitStore`.
- `GitStore` GPUI entity runs blocking git ops on `cx.background_executor()`, updates state via `cx.update()`.
- `GitPanel` sidebar view: staged/unstaged file sections, right-click context menu (stage/unstage/discard/open/diff).
- `DiffViewer` shows unified diffs in editor area, switched via `EditorTab.diff_mode` field.
- Sidebar tabs (Files/Git) with tab strip header; `Cmd+Shift+G` toggles git panel.

### Terminal / PTY
- `portable-pty` spawns shell processes. Master handle stored in `InstanceEntry`.
- PTY output polled on a background thread, fed to terminal via `TerminalView`.
- Dropping `MasterPty` handle sends SIGHUP to child process.
- Terminal focus managed via GPUI `FocusHandle`.

### Database
- SQLite in WAL mode at `data/conescope.db`.
- Migrations in `conescope-core/src/database.rs` via `rusqlite_migration`.
- Tables: `instances`, `projects`, `questions`, `settings`.

## Build Requirements

- Rust 1.85+ (edition 2024)
- zig 0.14 (linked globally: `brew link zig@0.14 --force`). Required by GPUI's C compilation.
- macOS 11+ (GPUI is macOS/Linux only)

## Gotchas

- zig must be 0.14, not 0.15+ (GPUI C compilation breaks on newer zig)
- `just verify` is the pre-commit gate, always run before committing
- SQLite ALTER TABLE can't change NOT NULL constraints — need table rebuild migration
- GPUI `cx.notify()` must be called after state mutations for re-render
- Entity reads (`entity.read(cx)`) borrow immutably — drop before `entity.update(cx, ...)`
- Async tasks can't hold `&mut App` across awaits — use `cx.update()` closures

### Scrollable containers
When content can exceed its container (lists, modals, panels), always:
1. Add `.id("descriptive-name")` — required for `InteractiveElement` scroll trait
2. Use `.overflow_y_scroll()` (vertical) or `.overflow_scroll()` (both axes)
3. **NEVER use `.flex().flex_col()` on the scrollable div itself** — flex layout shrinks children
   to fit (flex-shrink:1), making content_size == bounds.size and scroll_max = 0.
   Instead, put `.flex().flex_col()` on an inner child div, or use block layout on the scroller.
4. Parent must constrain the scrollable div's size (`.size_full()`, `.flex_1()`, `.max_h()`)
5. For large lists (100+ items), prefer `uniform_list()` for virtual rendering

### Terminal module (`conescope-ui/src/terminal/`)

Custom terminal rendering built on `alacritty_terminal` (Zed's fork, pure Rust VT parser):
- `terminal.rs` — `Terminal` entity wrapping `alacritty_terminal::Term`, processes VT bytes, extracts cell grid
- `terminal_view.rs` — `TerminalView` GPUI view with keyboard/mouse input, focus, copy/paste
- `terminal_element.rs` — `TerminalElement` GPUI Element, cell-based rendering (backgrounds, text runs, cursor)
- `mod.rs` — `spawn_terminal_pane()`, font utilities, cell metrics

Actions (`Copy`, `Paste`, `SelectAll`) defined in `terminal_view.rs`, bound in `main.rs`.

## Legacy (Electron)

The original Electron/React version lives in `electron/`, `src/`, `package.json`. Not actively developed.

```bash
npm run dev             # Start dev server + Electron
npm run build           # Build Vite frontend
npm run build:electron  # Package with electron-builder
```
