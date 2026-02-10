# Issues

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
