# Untitled Scratch Files Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Clicking empty area of editor tab bar creates a new untitled text file backed by a temp file in `~/.conescope/scratch/`.

**Architecture:** Scratch files are real files on disk, reusing existing file-backed tab logic. `EditorTabs` gains a counter and `NewUntitled` event. `CodeEditor` gains auto-save on change (debounced) and a "Save As" method via `rfd` crate. Scratch files persist across sessions.

**Tech Stack:** Rust, GPUI, rfd (native file dialogs), existing conescope-core/conescope-ui crates.

---

### Task 1: Add `scratch_dir()` helper to conescope-core

**Files:**
- Modify: `crates/conescope-core/src/settings.rs:53-59`
- Test: `crates/conescope-core/src/settings.rs` (inline tests)

**Step 1: Write the failing test**

Add to the `mod tests` block in `crates/conescope-core/src/settings.rs`:

```rust
#[test]
fn scratch_dir_is_inside_settings_dir() {
    let scratch = SettingsJson::scratch_dir();
    let settings = SettingsJson::settings_dir();
    assert!(scratch.starts_with(&settings));
    assert!(scratch.ends_with("scratch"));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p conescope-core scratch_dir`
Expected: FAIL — `scratch_dir` method doesn't exist.

**Step 3: Write minimal implementation**

Add method to `impl SettingsJson` in `crates/conescope-core/src/settings.rs` (after `settings_dir()`):

```rust
/// Returns `$HOME/.conescope/scratch` — directory for untitled scratch files.
#[must_use]
pub fn scratch_dir() -> PathBuf {
    Self::settings_dir().join("scratch")
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p conescope-core scratch_dir`
Expected: PASS

**Step 5: Commit**

```bash
git add crates/conescope-core/src/settings.rs
git commit -m "feat: add scratch_dir() helper for untitled files"
```

---

### Task 2: Add `is_scratch_file()` and `next_untitled_name()` helpers

**Files:**
- Modify: `crates/conescope-core/src/settings.rs`

**Step 1: Write the failing tests**

```rust
#[test]
fn is_scratch_file_detects_scratch_paths() {
    let scratch = SettingsJson::scratch_dir();
    let path = scratch.join("Untitled-1");
    assert!(SettingsJson::is_scratch_file(&path));
    assert!(!SettingsJson::is_scratch_file(Path::new("/tmp/foo.rs")));
}

#[test]
fn next_untitled_name_increments() {
    assert_eq!(SettingsJson::next_untitled_name(0), "Untitled-1");
    assert_eq!(SettingsJson::next_untitled_name(4), "Untitled-5");
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p conescope-core untitled`
Expected: FAIL

**Step 3: Write minimal implementation**

Add to `impl SettingsJson`:

```rust
/// Returns true if the path is inside the scratch directory.
#[must_use]
pub fn is_scratch_file(path: &Path) -> bool {
    path.starts_with(Self::scratch_dir())
}

/// Generate an untitled filename: `Untitled-{n+1}`.
#[must_use]
pub fn next_untitled_name(counter: usize) -> String {
    format!("Untitled-{}", counter + 1)
}
```

**Step 4: Run tests to verify they pass**

Run: `cargo test -p conescope-core untitled && cargo test -p conescope-core scratch`
Expected: PASS

**Step 5: Commit**

```bash
git add crates/conescope-core/src/settings.rs
git commit -m "feat: add is_scratch_file() and next_untitled_name() helpers"
```

---

### Task 3: Add `rfd` dependency to workspace

**Files:**
- Modify: `Cargo.toml` (workspace root)
- Modify: `crates/conescope-ui/Cargo.toml`

**Step 1: Add `rfd` to workspace dependencies**

In root `Cargo.toml`, add to `[workspace.dependencies]`:

```toml
rfd = "0.15"
```

In `crates/conescope-ui/Cargo.toml`, add to `[dependencies]`:

```toml
rfd.workspace = true
```

**Step 2: Verify it compiles**

Run: `cargo check -p conescope-ui`
Expected: compiles without errors.

**Step 3: Commit**

```bash
git add Cargo.toml crates/conescope-ui/Cargo.toml
git commit -m "chore: add rfd crate for native file dialogs"
```

---

### Task 4: Add `NewUntitled` event and click handler to `EditorTabs`

**Files:**
- Modify: `crates/conescope-ui/src/views/editor_tabs.rs`

**Step 1: Add `NewUntitled` event variant and counter field**

Add to `EditorTabsEvent` enum:

```rust
#[derive(Debug, Clone)]
pub enum EditorTabsEvent {
    SelectTab(usize),
    CloseTab(usize),
    NewUntitled, // NEW
}
```

Add `next_untitled_id: usize` field to `EditorTabs` struct:

```rust
pub struct EditorTabs {
    app_state: Entity<AppState>,
    tabs: Vec<EditorTab>,
    active_index: Option<usize>,
    next_untitled_id: usize, // NEW
}
```

Update `new()`:

```rust
pub fn new(app_state: Entity<AppState>) -> Self {
    Self {
        app_state,
        tabs: Vec::new(),
        active_index: None,
        next_untitled_id: 0,
    }
}
```

**Step 2: Add `create_untitled()` method**

```rust
/// Create a new untitled scratch file and open it as a tab.
/// Returns the scratch file path.
pub fn create_untitled(&mut self, cx: &mut gpui::Context<Self>) -> String {
    use conescope_core::settings::SettingsJson;

    let scratch_dir = SettingsJson::scratch_dir();
    std::fs::create_dir_all(&scratch_dir).ok();

    let name = SettingsJson::next_untitled_name(self.next_untitled_id);
    self.next_untitled_id += 1;

    let path = scratch_dir.join(&name);
    // Create empty file
    std::fs::write(&path, "").ok();

    let path_str = path.to_string_lossy().to_string();
    self.open_tab(&path_str, cx);
    cx.emit(EditorTabsEvent::NewUntitled);
    path_str
}
```

**Step 3: Add `init_counter_from_scratch_dir()` method**

Called on startup to resume counter from existing scratch files:

```rust
/// Scan scratch dir and set counter to max existing + 1.
pub fn init_counter_from_scratch_dir(&mut self) {
    use conescope_core::settings::SettingsJson;

    let scratch_dir = SettingsJson::scratch_dir();
    let max_id = std::fs::read_dir(&scratch_dir)
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            name.strip_prefix("Untitled-")
                .and_then(|n| n.parse::<usize>().ok())
        })
        .max()
        .unwrap_or(0);
    self.next_untitled_id = max_id;
}
```

**Step 4: Add click handler on the trailing spacer**

In the `Render` impl, change the trailing spacer from:

```rust
bar.child(border_b_spacer().flex_1())
```

to:

```rust
bar.child(
    border_b_spacer()
        .flex_1()
        .cursor_pointer()
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(|this, _event: &gpui::MouseDownEvent, _window, cx| {
                this.create_untitled(cx);
            }),
        ),
)
```

Also add the same click handler to the empty-tabs case (the early return when `self.tabs.is_empty()`):

```rust
if self.tabs.is_empty() {
    return div()
        .h(px(28.))
        .border_b_1()
        .border_color(rgba(BORDER_COLOR))
        .bg(theme.background)
        .cursor_pointer()
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(|this, _event: &gpui::MouseDownEvent, _window, cx| {
                this.create_untitled(cx);
            }),
        );
}
```

**Step 5: Verify it compiles**

Run: `cargo check -p conescope-ui`
Expected: compiles.

**Step 6: Commit**

```bash
git add crates/conescope-ui/src/views/editor_tabs.rs
git commit -m "feat: click empty editor tab bar area to create untitled file"
```

---

### Task 5: Wire `NewUntitled` event in `FocusView`

**Files:**
- Modify: `crates/conescope-ui/src/views/focus_view.rs`

**Step 1: Handle `NewUntitled` event**

In `FocusView::new()`, the existing `EditorTabsEvent` subscription (lines 66-86) handles `SelectTab` and `CloseTab`. Add a `NewUntitled` arm that opens the file in the code editor and ensures editor panel is visible:

```rust
EditorTabsEvent::NewUntitled => {
    if let Some(path) = et2.read(cx).active_path().map(str::to_owned) {
        cv2.update(cx, |v, cx| v.open_file(&path, cx));
    }
    // Ensure editor panel is visible
    app_state_tabs.update(cx, |s, cx| s.ensure_editor_visible(cx));
}
```

**Step 2: Add `ensure_editor_visible()` to `AppState`**

In `crates/conescope-ui/src/state/app_state.rs`, add:

```rust
/// Make the editor panel visible (if currently hidden).
pub fn ensure_editor_visible(&self, cx: &mut gpui::Context<Self>) {
    if !self.editor_visible(cx) {
        self.toggle_editor(cx);
    }
}
```

**Step 3: Call `init_counter_from_scratch_dir()` on startup**

In `FocusView::new()`, after creating editor_tabs (line 50), add:

```rust
editor_tabs.update(cx, |tabs, _| tabs.init_counter_from_scratch_dir());
```

**Step 4: Restore scratch files on startup**

In `FocusView::new()`, after restoring saved tabs (line 89-99), add logic to scan scratch dir and open any scratch files not already in tabs:

```rust
// Restore scratch files not already in saved tabs
let scratch_dir = conescope_core::settings::SettingsJson::scratch_dir();
if let Ok(entries) = std::fs::read_dir(&scratch_dir) {
    let existing_paths: std::collections::HashSet<String> =
        editor_tabs.read(cx).tab_paths().into_iter().collect();
    let mut scratch_paths: Vec<String> = entries
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_name()
                .to_string_lossy()
                .starts_with("Untitled-")
        })
        .map(|e| e.path().to_string_lossy().to_string())
        .filter(|p| !existing_paths.contains(p))
        .collect();
    scratch_paths.sort();
    for path in scratch_paths {
        editor_tabs.update(cx, |tabs, cx| tabs.open_tab(&path, cx));
    }
}
```

**Step 5: Verify it compiles**

Run: `cargo check -p conescope-ui`
Expected: compiles.

**Step 6: Commit**

```bash
git add crates/conescope-ui/src/views/focus_view.rs crates/conescope-ui/src/state/app_state.rs
git commit -m "feat: wire untitled tab creation in focus view with scratch restore"
```

---

### Task 6: Add auto-save on change to `CodeEditor`

**Files:**
- Modify: `crates/conescope-ui/src/views/code_viewer.rs`

**Step 1: Add debounced auto-save for scratch files**

In `CodeEditor`, modify the `InputEvent::Change` handler in `ensure_editor` (line 61-67) to trigger auto-save:

```rust
cx.subscribe(&state, |this, _state, event, cx| {
    if let InputEvent::Change = event {
        // Auto-save scratch files on change
        if let Some(ref path) = this.file_path {
            if conescope_core::settings::SettingsJson::is_scratch_file(
                std::path::Path::new(path),
            ) {
                // Save immediately (debouncing optional, file is small)
                let _ = this.save_file(cx);
            }
        }
    }
})
.detach();
```

Note: `save_file` takes `&gpui::App` but the subscriber closure receives `&mut gpui::Context<Self>`. Since `Context` derefs to `App`, this should work. If not, extract content and path, write via `std::fs::write`.

Actually, looking at the subscriber signature more carefully — the closure receives `(this: &mut CodeEditor, state: &Entity<InputState>, event: &InputEvent, cx: &mut gpui::Context<CodeEditor>)`. We can't call `this.save_file(cx)` because `save_file` takes `&gpui::App` and the context can deref. But let's be safe and write directly:

```rust
cx.subscribe(&state, |this, _state, event, cx| {
    if let InputEvent::Change = event {
        if let Some(ref path) = this.file_path {
            if conescope_core::settings::SettingsJson::is_scratch_file(
                std::path::Path::new(path),
            ) {
                if let Some(ref editor) = this.editor_state {
                    let content = editor.read(cx).value().to_string();
                    let _ = std::fs::write(path, content);
                }
            }
        }
    }
})
.detach();
```

**Step 2: Verify it compiles**

Run: `cargo check -p conescope-ui`
Expected: compiles.

**Step 3: Commit**

```bash
git add crates/conescope-ui/src/views/code_viewer.rs
git commit -m "feat: auto-save scratch files on editor change"
```

---

### Task 7: Add `SaveFile` action and "Save As" for untitled files

**Files:**
- Modify: `crates/conescope-ui/src/actions.rs`
- Modify: `crates/conescope/src/main.rs`
- Modify: `crates/conescope-ui/src/views/code_viewer.rs`
- Modify: `crates/conescope-ui/src/views/focus_view.rs`
- Modify: `crates/conescope-ui/src/views/app_view.rs`

**Step 1: Add `SaveFile` action**

In `crates/conescope-ui/src/actions.rs`, add `SaveFile` to the actions list:

```rust
actions!(
    conescope,
    [
        Quit,
        NewInstance,
        CloseInstance,
        CloseTab,
        SaveFile, // NEW
        ReturnToOverview,
        // ... rest unchanged
    ]
);
```

**Step 2: Bind Cmd+S in main.rs**

In `crates/conescope/src/main.rs`, import `SaveFile` and add keybinding:

```rust
use conescope_ui::actions::{
    CloseSettings, CloseTab, SaveFile, /* ... existing imports */
};
```

Add to `bind_keys()`:

```rust
KeyBinding::new("cmd-s", SaveFile, None),
```

**Step 3: Add `save_as()` method to `CodeEditor`**

In `crates/conescope-ui/src/views/code_viewer.rs`, add a new event variant and save_as method:

Add event:
```rust
#[derive(Debug, Clone)]
pub enum CodeEditorEvent {
    FileOpened(String),
    FileSavedAs(String), // NEW: path after Save As
}
```

Add method:
```rust
/// Open a native "Save As" dialog. On success, writes content to the chosen path,
/// deletes the scratch file, and emits `FileSavedAs`.
pub fn save_as(&mut self, cx: &mut gpui::Context<Self>) {
    let Some(ref current_path) = self.file_path else {
        return;
    };
    let content = self
        .editor_state
        .as_ref()
        .map(|s| s.read(cx).value().to_string())
        .unwrap_or_default();
    let old_path = current_path.clone();

    cx.spawn(async move |this, mut cx| {
        let handle = rfd::AsyncFileDialog::new()
            .set_file_name("Untitled")
            .save_file()
            .await;

        if let Some(file) = handle {
            let new_path = file.path().to_string_lossy().to_string();
            // Write content to new location
            if std::fs::write(&new_path, &content).is_ok() {
                // Delete old scratch file
                if conescope_core::settings::SettingsJson::is_scratch_file(
                    std::path::Path::new(&old_path),
                ) {
                    let _ = std::fs::remove_file(&old_path);
                }
                cx.update(|cx| {
                    this.update(cx, |editor, cx| {
                        editor.file_path = Some(new_path.clone());
                        cx.emit(CodeEditorEvent::FileSavedAs(new_path));
                        cx.notify();
                    });
                });
            }
        }
    })
    .detach();
}
```

**Step 4: Add `SaveFile` action handler in `FocusView`**

Add a public method to `FocusView`:

```rust
/// Handle Cmd+S: if active tab is a scratch file, trigger Save As dialog.
/// Otherwise, save in place.
pub fn save_active_file(&self, cx: &mut gpui::Context<Self>) {
    let active_path = self.editor_tabs.read(cx).active_path().map(str::to_owned);
    let Some(path) = active_path else { return };

    if conescope_core::settings::SettingsJson::is_scratch_file(std::path::Path::new(&path)) {
        self.code_editor.update(cx, |editor, cx| editor.save_as(cx));
    } else {
        self.code_editor.update(cx, |editor, cx| {
            let _ = editor.save_file(cx);
        });
    }
}
```

**Step 5: Handle `FileSavedAs` event to update tab path**

In `FocusView::new()`, subscribe to `CodeEditor` events. Add after the existing subscriptions:

```rust
let et_for_save = editor_tabs.clone();
let app_state_for_save = app_state.clone();
cx.subscribe(&code_editor, move |_this, _editor, event, cx| {
    if let CodeEditorEvent::FileSavedAs(new_path) = event {
        // Update the tab's path from scratch → real path
        et_for_save.update(cx, |tabs, cx| {
            tabs.update_active_path(new_path, cx);
        });
        // Persist tabs
        let tab_paths = et_for_save.read(cx).tab_paths();
        let active = et_for_save.read(cx).active_path().map(str::to_owned);
        app_state_for_save.update(cx, |s, cx| s.save_editor_tabs(tab_paths, active, cx));
    }
})
.detach();
```

**Step 6: Add `update_active_path()` to `EditorTabs`**

In `crates/conescope-ui/src/views/editor_tabs.rs`:

```rust
/// Update the active tab's path (after Save As).
pub fn update_active_path(&mut self, new_path: &str, cx: &mut gpui::Context<Self>) {
    if let Some(idx) = self.active_index {
        if let Some(tab) = self.tabs.get_mut(idx) {
            tab.path = new_path.to_owned();
            tab.name = std::path::Path::new(new_path)
                .file_name()
                .map_or_else(|| new_path.to_owned(), |n| n.to_string_lossy().to_string());
            tab.modified = false;
            cx.notify();
        }
    }
}
```

**Step 7: Wire `SaveFile` action in `app_view.rs`**

In `with_action_handlers()`, add after the `CloseTab` handler:

```rust
.on_action({
    let app_state = app_state.clone();
    let focus_view = focus_view.clone();
    move |_: &SaveFile, _window, cx| {
        if app_state.read(cx).view_mode(cx) != ViewMode::Focus {
            return;
        }
        focus_view.update(cx, |fv, cx| fv.save_active_file(cx));
    }
})
```

Import `SaveFile` in `app_view.rs`.

**Step 8: Verify it compiles**

Run: `cargo check -p conescope-ui && cargo check -p conescope`
Expected: compiles.

**Step 9: Commit**

```bash
git add crates/conescope-ui/src/actions.rs crates/conescope/src/main.rs \
  crates/conescope-ui/src/views/code_viewer.rs \
  crates/conescope-ui/src/views/focus_view.rs \
  crates/conescope-ui/src/views/app_view.rs \
  crates/conescope-ui/src/views/editor_tabs.rs
git commit -m "feat: Cmd+S Save As for untitled scratch files via rfd"
```

---

### Task 8: Delete scratch file on tab close

**Files:**
- Modify: `crates/conescope-ui/src/views/editor_tabs.rs`

**Step 1: Delete scratch file when closing a tab**

Modify `close_tab()` in `EditorTabs` to delete the file if it's a scratch file:

```rust
pub fn close_tab(&mut self, index: usize, cx: &mut gpui::Context<Self>) {
    if index >= self.tabs.len() {
        return;
    }
    // Delete scratch file from disk
    let path = &self.tabs[index].path;
    if conescope_core::settings::SettingsJson::is_scratch_file(std::path::Path::new(path)) {
        let _ = std::fs::remove_file(path);
    }
    self.tabs.remove(index);
    // ... rest unchanged
}
```

Wait — the design says "keep temp file, restore on next session". So we should NOT delete on close. Let me re-read... The user chose option 2: "Keep temp file, restore on next session — Untitled tabs persist across app restarts until explicitly discarded or saved."

So closing the tab keeps the file, and on next startup it gets restored. The file is only deleted when the user does "Save As" (already handled in Task 7).

**This task is unnecessary — skip it.**

Actually, we should reconsider. If the user closes the tab, the scratch file stays. On next startup, we scan and restore it. That means there's no way to discard a scratch file except by saving it. Let me keep it simple per the design decision and NOT delete on close.

**Skip this task.**

---

### Task 8 (revised): Run full verification

**Step 1: Run `just verify`**

Run: `just verify`
Expected: fmt-check + clippy + test all pass.

**Step 2: Manual test**

1. `just run`
2. Focus an instance (Cmd+1)
3. Toggle editor visible (Cmd+E)
4. Click empty area of editor tab bar
5. Verify: new "Untitled-1" tab appears, empty editor
6. Type some text
7. Verify: `~/.conescope/scratch/Untitled-1` exists with the typed content
8. Press Cmd+S → native Save As dialog appears
9. Save to `/tmp/test-file.txt`
10. Verify: tab name changes to `test-file.txt`, scratch file deleted
11. Click empty area again → "Untitled-2" tab
12. Close it (Cmd+W), restart app → "Untitled-2" tab restored

**Step 3: Commit any fixes**

If `just verify` or manual test reveals issues, fix and commit.

**Step 4: Final commit (if all clean)**

```bash
just verify
git add -A
git commit -m "feat: untitled scratch files — complete implementation"
```

---

### Summary of changes

| File | What |
|------|------|
| `Cargo.toml` | Add `rfd` workspace dep |
| `crates/conescope-ui/Cargo.toml` | Add `rfd` dep |
| `crates/conescope-core/src/settings.rs` | `scratch_dir()`, `is_scratch_file()`, `next_untitled_name()` |
| `crates/conescope-ui/src/actions.rs` | Add `SaveFile` action |
| `crates/conescope-ui/src/views/editor_tabs.rs` | `NewUntitled` event, counter, `create_untitled()`, `init_counter_from_scratch_dir()`, `update_active_path()`, click handlers |
| `crates/conescope-ui/src/views/code_viewer.rs` | Auto-save on change for scratch files, `save_as()` method, `FileSavedAs` event |
| `crates/conescope-ui/src/views/focus_view.rs` | Wire events, `save_active_file()`, scratch file restore on startup |
| `crates/conescope-ui/src/views/app_view.rs` | `SaveFile` action handler |
| `crates/conescope/src/main.rs` | Cmd+S keybinding |
