# Issues

## 001: Per-instance Focus View settings

### Problem

`SessionState` is stored as a single global JSON blob in the DB (`session_state` key). Focus View layout settings — `folder_panel_visible`, `editor_panel_visible`, `terminal_panel_visible`, `sidebar_width`, `terminal_height`, `sidebar_tab`, `open_editor_tabs`, `active_editor_tab` — are shared across all instances.

When switching between instances or restarting the app, every instance gets the same panel layout. Users expect each instance to remember its own layout (e.g., terminal hidden in one project, sidebar collapsed in another).

Window position/size (`window_x/y/width/height`) and app-level state (`view_mode`, `focused_instance_id`) should remain global.

### Current state

- `SettingsStore` holds one `SessionState`
- `save_session()` serializes the entire struct to a single `session_state` DB key
- `load_session()` deserializes from that key on startup
- Switching instances does not save/restore per-instance layout

### Proposed fix

1. Split `SessionState` into two structs:
   - `GlobalSessionState` — `view_mode`, `focused_instance_id`, `window_x/y/width/height`
   - `InstanceSessionState` — `folder_panel_visible`, `editor_panel_visible`, `terminal_panel_visible`, `sidebar_width`, `terminal_height`, `sidebar_tab`, `open_editor_tabs`, `active_editor_tab`

2. Store per-instance state keyed by instance ID (e.g., DB key `instance_session:{id}` or a new `instance_settings` column in the `instances` table).

3. On instance switch: save current instance's layout, load the target instance's layout (falling back to defaults if none saved).

4. On app startup: restore global state + focused instance's layout.

### Files affected

- `crates/conescope-ui/src/state/settings_store.rs` — split structs, per-instance save/load
- `crates/conescope-ui/src/state/db_worker.rs` — new DB operations for per-instance state
- `crates/conescope-ui/src/views/app_view.rs` — save/restore on instance switch
- `crates/conescope-ui/src/views/focus_view.rs` — read layout from per-instance state
- `crates/conescope-core/src/database.rs` — migration if using a new column/table

---

## 002: Git-aware file tree highlighting

**Status:** Open

### Problem

File tree colors files by extension (`file_ext_color()` in `file_tree.rs`) — each extension gets a unique color (Rust = orange, TS = blue, JSON = green, etc.). This is noisy and doesn't convey useful information. Meanwhile, git-modified files have no visual distinction at all.

Directories also show no indication that they contain changed files, so users can't tell which folders to expand to find modifications.

### Current state

- `file_ext_color()` maps ~12 extensions to hardcoded `Rgba` values, used for both file name text and icon color
- Directories always use `theme.text_muted`
- `FileTree` has no access to `GitStore` or git status data
- `Theme` struct has no git-status colors (no `modified`, `added`, `deleted`, `conflict` tokens)
- Zed's theme JSON files include `version_control.modified`/`created`/`deleted` color tokens in the `style` section, but they're not parsed

### Desired behavior (reference: Zed editor)

1. **Uniform base color**: All file names use a single color from the theme (e.g., `theme.text` or `theme.text_muted`) regardless of extension. Icons can keep per-type colors.

2. **Git-modified files**: Files with uncommitted changes get a distinct color + brightness shift based on status:
   - **Modified** — theme color (e.g., yellow/amber from `version_control.modified`)
   - **Added/Untracked** — theme color (e.g., green from `version_control.created`)
   - **Deleted** — theme color (e.g., red from `version_control.deleted`)
   - **Conflicted** — theme color (e.g., orange/red)

3. **Directory propagation**: If any file inside a directory (at any depth) has git changes, that directory's name is highlighted with the same git color. This lets users spot which folders contain changes without expanding them. If a directory contains files with mixed statuses, use the highest-priority color (e.g., conflict > modified > added).

### Proposed fix

1. **Add git status colors to `Theme`**:
   - Parse `version_control.modified`, `version_control.created`, `version_control.deleted` from Zed theme JSON in `parse_zed_theme()`
   - Add fields: `pub vcs_modified: Rgba`, `pub vcs_added: Rgba`, `pub vcs_deleted: Rgba`, `pub vcs_conflict: Rgba`
   - Provide sensible defaults for themes missing these tokens

2. **Remove per-extension file name coloring**:
   - Delete `file_ext_color()` function
   - All file names use `theme.text` (or `theme.text_muted` for directories)
   - File icons can optionally keep per-type colors (separate concern)

3. **Pass git status into `FileTree`**:
   - Give `FileTree` access to `Entity<GitStore>` (or receive a `HashMap<String, FileStatus>` snapshot)
   - On `GitStore` change notifications, refresh the display

4. **Build a changed-paths set for directory propagation**:
   - When git status is updated, compute a `HashSet<String>` of all ancestor directories of changed files
   - Store the "worst" status for each directory (conflict > modified > added)
   - During `render_entry()`, look up each entry's path in the status map to determine color

5. **Apply colors in `render_entry()`**:
   - If file/dir has git status → use the corresponding `theme.vcs_*` color for the name text
   - Otherwise → use `theme.text` (files) or `theme.text_muted` (directories)

### Files affected

- `crates/conescope-ui/src/theme.rs` — add `vcs_*` color fields, parse from theme JSON
- `crates/conescope-ui/src/views/file_tree.rs` — remove `file_ext_color()`, add git status lookup, directory propagation logic
- `crates/conescope-ui/src/state/git_store.rs` — possibly expose a file status map or subscribe pattern
- `crates/conescope-ui/src/views/focus_view.rs` — pass `GitStore` entity to `FileTree` if not already available

---

## 003: Opening a file from FileTree should auto-show the editor panel

**Status:** Open

### Problem

When the editor panel is hidden (toggled off via ActivityBar), clicking a file in the FileTree does nothing visible. The tab and code editor are updated internally, but the editor panel stays hidden. The user expects the editor to appear automatically when they explicitly open a file.

### Current state

- `FileTreeEvent::OpenFile` handler in `focus_view.rs:88` calls `editor_tabs.open_tab()` and `code_editor.open_file()` but does **not** call `AppState::ensure_editor_visible()`
- `AppState::ensure_editor_visible()` already exists (`app_state.rs:316`) — it checks `editor_panel_visible` and calls `toggle_editor()` if false
- This method is already used for `EditorTabsEvent::NewUntitled` but not for file-tree opens
- The ActivityBar toggle reads `editor_panel_visible` from `SessionState`, so calling `ensure_editor_visible()` will also sync the toggle state

### Fix

In the `FileTreeEvent::OpenFile` subscription in `FocusView::new()` (`focus_view.rs:~88`), add a call to `app_state.update(cx, AppState::ensure_editor_visible)` after opening the tab/file. One line change.

### Files affected

- `crates/conescope-ui/src/views/focus_view.rs` — add `ensure_editor_visible` call in `OpenFile` handler

---

## 004: Editor auto-hide and ActivityBar toggle desync

**Status:** Open

### Problem

When no file is open, the editor panel is automatically hidden in `focus_view.rs:738`:

```rust
let editor_visible =
    !is_terminal && state.editor_visible(cx) && self.editor_tabs.read(cx).tab_count() > 0;
```

But `editor_panel_visible` in `SessionState` stays `true`, so the ActivityBar toggle still appears enabled. This is confusing — the toggle says "on" but the editor is hidden.

Additionally, if the user explicitly turns the editor back on via the toggle, it should stay visible (empty state) and not be auto-hidden again until the next FocusView session (app restart or entering FocusView fresh).

### Current state

- `editor_visible` in render is computed as `editor_panel_visible && tab_count > 0` — auto-hides when no tabs
- `editor_panel_visible` in `SessionState` is never set to `false` by auto-hide, so ActivityBar toggle is out of sync
- No distinction between "user toggled editor off" vs "auto-hidden because no tabs"
- If user turns editor on via toggle after auto-hide, it will immediately auto-hide again because `tab_count` is still 0

### Desired behavior

1. When entering FocusView with no open files: auto-hide the editor **and** set the ActivityBar toggle to off (sync the visible state)
2. When user explicitly enables the editor via toggle: show the editor panel (even if empty/no tabs) and **stop auto-hiding** for the rest of this session
3. When user opens a file (from FileTree, GitPanel, etc.): show editor naturally (issue 003 handles this)
4. On app restart / fresh FocusView entry: allow one-time auto-hide again if no files are open

### Proposed fix

1. **Add `editor_auto_hidden: bool` transient flag** to `FocusView` (not persisted):
   - Set to `true` on FocusView init when `tab_count == 0` — this triggers the initial auto-hide
   - Set to `false` when user explicitly toggles editor on

2. **Sync ActivityBar on auto-hide**: When auto-hiding (no tabs on FocusView entry), also set `editor_panel_visible = false` in `SessionState` so the toggle reflects reality

3. **Change render logic**:
   - Remove the `&& tab_count > 0` condition from `editor_visible`
   - Instead, the auto-hide decision happens once at FocusView init: if no tabs → set `editor_panel_visible = false`
   - After that, `editor_visible` simply follows `editor_panel_visible`
   - When editor is visible but has no tabs, show an empty state placeholder

4. **User toggle behavior**: `toggle_editor()` sets `editor_panel_visible` and clears `editor_auto_hidden`, preventing further auto-hides

### Files affected

- `crates/conescope-ui/src/views/focus_view.rs` — auto-hide logic on init, remove `tab_count` guard from render
- `crates/conescope-ui/src/state/app_state.rs` — possibly adjust `toggle_editor` / `ensure_editor_visible`
- `crates/conescope-ui/src/state/settings_store.rs` — sync `editor_panel_visible` on auto-hide

---

## 005: Context menu stops working after deleting a file

**Status:** Open

### Problem

After deleting (or trashing) a file via the FileTree context menu, subsequent right-clicks on other files no longer open the context menu. The entire FileTree becomes unresponsive to right-click until the view is re-entered or the app is restarted.

### Root cause analysis

The context menu items (Trash, Delete) handle `MouseButton::Left` `on_mouse_down`. When the user clicks a menu item:

1. Mouse-down fires → handler runs: deletes file, sets `context_menu = None`, calls `rebuild_entries()` + `cx.notify()`
2. Re-render removes the context menu overlay and the deleted entry from the DOM
3. The element that received the mouse-down no longer exists in the element tree
4. GPUI may not deliver the corresponding mouse-up event properly, leaving internal hit-testing or mouse state in a stuck/stale state
5. Subsequent right-clicks are either swallowed or fail to trigger `on_mouse_down(MouseButton::Right, ...)` on entries

This is a known class of GPUI issue: removing the element that initiated a mouse-down before mouse-up completes can break subsequent mouse event delivery.

Additionally, the context menu Trash/Delete handlers duplicate the logic from `on_trash()`/`on_delete()` action handlers — they inline the file removal code rather than dispatching the action. This duplication means fixes need to be applied in two places.

### Current state

- Context menu items use `on_mouse_down(MouseButton::Left, cx.listener(...))` with inline delete logic
- Handlers call `rebuild_entries()` synchronously during mouse-down, destroying the clicked element mid-event
- No deferred/async cleanup — the DOM mutation happens immediately within the event handler
- Dismiss overlay (`ft-ctx-dismiss`) handles `MouseButton::Right` to close the menu, but doesn't re-dispatch the right-click to the entry underneath
- `on_trash()`/`on_delete()` action handlers exist separately with the same logic

### Proposed fix

1. **Defer DOM-mutating operations**: Instead of deleting the file and rebuilding entries synchronously inside the mouse-down handler, set a pending action (e.g., `self.pending_delete = Some(path)`) and call `cx.notify()`. Process the actual deletion in the next render cycle or via `cx.defer()` / `cx.after_layout()`, ensuring the mouse event cycle completes before the element tree changes.

2. **Deduplicate**: Have context menu Trash/Delete items dispatch the `FileTrash`/`FileDelete` actions instead of inlining the logic. This centralizes the behavior in the action handlers.

3. **Alternative approach**: Use `cx.on_next_frame()` or `cx.defer()` to schedule the `rebuild_entries()` call after the current event is fully processed:
   ```rust
   // Instead of immediate rebuild:
   this.rebuild_entries();
   cx.notify();
   
   // Defer it:
   cx.defer(|this, cx| {
       this.rebuild_entries();
       cx.notify();
   });
   ```

4. **Dismiss overlay right-click**: When the dismiss overlay receives a right-click, consider closing the menu and then re-dispatching or allowing the event to reach the entry underneath, so users can right-click a different file to open a new context menu in one gesture.

### Files affected

- `crates/conescope-ui/src/views/file_tree.rs` — defer rebuild after delete, deduplicate context menu handlers, fix dismiss overlay right-click passthrough

---

## 006: New file/folder inline rename is broken; new file should open in editor after rename

**Status:** Open

### Problem

1. **Rename doesn't work**: When creating a new file or folder via the context menu ("New File" / "New Folder"), the code sets `rename_path` to enter inline rename mode, but the rename widget never receives keyboard input — typing has no effect.

2. **New file not opened in editor**: After creating a new file, it should be opened in the editor automatically. But the open must happen *after* the user finishes renaming (confirms with Enter), not immediately on creation, so the user has a chance to name it first.

3. **New folder should not open in editor**: Folders should enter rename mode but not be opened in the editor on confirm.

### Root cause

The `render_rename_entry()` function renders a plain `div` with `on_key_down`. But `on_key_down` only fires on elements that have keyboard focus. The rename div has no `FocusHandle` and `.track_focus()` — it never receives focus, so key events never reach it. The rename widget is effectively dead UI.

Current rename widget (`file_tree.rs:993`):
```rust
div()
    .id("ft-rename-input")
    .on_key_down(cx.listener(|this, ev, _, cx| { ... }))
```

Missing: `.track_focus(&focus_handle)` + `window.focus(&focus_handle)` to actually direct keyboard input to it.

Additionally, the rename input is not a real text input — it's a div displaying text with manual `on_key_down` character handling (append char on key, pop on backspace). This doesn't support cursor positioning, text selection, IME, or standard text editing shortcuts (Cmd+A, Cmd+Z, etc.).

### Desired behavior

1. **New File**: Create file → enter inline rename mode with focus → user types name → Enter confirms rename → file opens in editor with final name
2. **New Folder**: Create folder → enter inline rename mode with focus → user types name → Enter confirms rename → folder stays in tree (not opened)
3. **Escape**: Cancel rename, keep the file/folder with its default name ("untitled" / "new-folder"), do not open in editor
4. **Click outside**: Confirm rename (same as Enter)

### Proposed fix

1. **Fix focus on rename widget**: Add a dedicated `FocusHandle` for the rename input. When entering rename mode (`on_new_file`, `on_new_folder`, `on_rename`), focus this handle via `window.focus(&handle)`. Attach it to the rename div with `.track_focus(&handle)`.

2. **Replace manual key handling with a proper text input**: Use a real GPUI text input (or `gpui_component::Input`) for the rename field. This gives proper text editing, cursor, selection, and IME support. Handle Enter/Escape via the input's event callbacks.

3. **Track whether the rename is for a new file vs existing**: Add a flag (e.g., `rename_is_new_file: bool`) to distinguish "rename after create" from "rename existing". On confirm:
   - If `rename_is_new_file == true` and the entry is a file → emit `FileTreeEvent::OpenFile` with the new path
   - If it's a folder or an existing rename → don't open

4. **Click-outside-to-confirm**: Add a mouse-down handler on the root container that confirms the rename when clicking outside the rename input area.

### Files affected

- `crates/conescope-ui/src/views/file_tree.rs` — fix rename widget focus, replace with real input, add new-file-open-on-confirm logic, click-outside handling

---

## 007: Instance title not colored with instance color in overview grid

**Status:** Open

### Problem

In the overview grid, the instance number (`#1`, `#2`, ...) is colored with the instance's assigned color, but the title next to it uses the generic `theme.text` color. Both should use the instance color, matching the Electron implementation.

### Current state (Rust)

- `overview_grid.rs:109` — number: `.text_color(tile.color)` — correct
- `overview_grid.rs:160` — title in `render_static_title()`: `.text_color(theme.text)` — wrong, should be `tile.color`

### Electron reference

`src/components/Overview/InstanceTile.tsx:172-174`:
```tsx
<span className="tile-number" style={{ color: tileColor }}>#{num}</span>
<span className="tile-title" style={{ color: tileColor }}>{instance.title}</span>
```

Both number and title use `tileColor`.

### Fix

Change `render_static_title()` in `overview_grid.rs` to use `tile.color` instead of `theme.text` for the title text color. One-line change.

### Files affected

- `crates/conescope-ui/src/views/overview_grid.rs` — change `.text_color(theme.text)` to `.text_color(tile.color)` in `render_static_title()`

Additionally, all instances currently show the **same color** because:

### Color assignment is broken

- New instances are always created with `color: None` (`new_instance_modal.rs:65`)
- Every fallback resolves `None` to the same hardcoded blue `rgba(0x6464_b5f6)` — in overview grid, activity bar, and top bar
- Result: every instance is the same blue regardless of position or project

### Electron reference

In the Electron version (`src/stores/instanceStore.ts`):
- **Project instances**: `color: null` at creation, tile derives color from `project.color` at render time (`InstanceTile.tsx:146: project?.color || '#888'`). Projects are assigned unique colors from `PROJECT_COLORS` palette on creation.
- **Terminal instances**: assigned a color directly from `PROJECT_COLORS[count % len]` at creation time (line 86)

### Fix (color assignment)

Two approaches — either can work:

1. **Assign color at instance creation** (simpler): In `new_instance_modal.rs`, assign `color: Some(PROJECT_COLORS[count % len].to_string())` based on the current instance count. Both project and terminal instances get their own color.

2. **Derive from project at render time** (matches Electron): For project instances with `color: None`, look up `project.color` in the tile-building code instead of falling back to hardcoded blue. For terminal instances, assign a color from the palette at creation. This requires passing the project store into the color resolution.

### Additional files affected

- `crates/conescope-ui/src/views/new_instance_modal.rs` — assign color from `PROJECT_COLORS` palette
- `crates/conescope-ui/src/views/overview_grid.rs` — fix fallback to use project color instead of hardcoded blue
- `crates/conescope-ui/src/views/activity_bar.rs` — same fallback fix
- `crates/conescope-ui/src/views/top_bar.rs` — same fallback fix
