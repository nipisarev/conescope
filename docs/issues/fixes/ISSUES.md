# Issues

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

---

## 008: Git-aware file tree highlighting [SKIP IT]

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
