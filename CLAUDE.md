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
│   │   ├── settings_store.rs   # Key-value settings
│   │   └── db_worker.rs        # Async SQLite via flume channels
│   └── src/views/
│       ├── app_view.rs         # Root view, action handlers, mode switching
│       ├── overview_grid.rs    # Grid of instance tiles
│       ├── focus_view.rs       # Single instance full-screen
│       ├── activity_bar.rs     # Left sidebar with instance icons
│       ├── new_instance_modal.rs # Create instance dialog
│       └── top_bar.rs          # Window title bar
├── conescope-core/     # Data types + SQLite (rusqlite). No UI deps.
│   └── src/
│       ├── database.rs         # DB init, migrations, WAL mode
│       ├── instance.rs         # Instance model + queries
│       ├── project.rs          # Project model + queries
│       ├── question.rs         # Question model + queries
│       ├── settings.rs         # Settings model + queries
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
- Instance terminals use `gpui_ghostty_terminal` (wraps Ghostty/alacritty_terminal).

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

### gpui-ghostty patch (terminal font family)

`gpui_ghostty_terminal::TerminalView` has a private `font` field with no public setter.
We patched the cargo git checkout to add `pub fn set_font(&mut self, font: gpui::Font)`.

**Patch location:** `~/.cargo/git/checkouts/gpui-ghostty-*/e302598/crates/gpui_ghostty_terminal/src/view/mod.rs`

**Re-apply after `cargo clean`:**
```rust
// Add after `new_with_input()` method (~line 312):
/// Override the terminal font (family + fallbacks).
pub fn set_font(&mut self, font: gpui::Font) {
    self.font = font;
}
```

**Limitation:** Changing font family requires app restart for existing terminals.
The `TerminalView.prepaint()` caches line layouts keyed on `(font_size, line_height)` —
changing family doesn't invalidate the cache. New terminals get the correct font immediately.

**TODO:** Fork `github.com/Xuanwo/gpui-ghostty` and update `Cargo.toml` git URL.
The fork should add `set_font` + invalidate `line_layout_key` on font change for live updates.

## Legacy (Electron)

The original Electron/React version lives in `electron/`, `src/`, `package.json`. Not actively developed.

```bash
npm run dev             # Start dev server + Electron
npm run build           # Build Vite frontend
npm run build:electron  # Package with electron-builder
```
