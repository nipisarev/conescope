# Untitled Scratch Files — Design

## Summary
Clicking empty area of editor tab bar creates a new untitled text file backed by a temp file on disk at `~/.conescope/scratch/`. Content auto-saves. Cmd+S opens native "Save As" dialog. Scratch files persist across sessions until explicitly saved or tab closed.

## Decisions
- **Storage**: Immediate temp file on disk (crash-safe, reuses file-backed tab logic)
- **Close behavior**: Keep temp file, restore on next session
- **Naming**: `Untitled-1`, `Untitled-2`, etc. Sequential counter, resumes from highest on restart
- **Cmd+S**: Native macOS "Save As" dialog via `rfd` crate. On save, tab path updates, scratch file deleted.

## Data Flow
1. Click empty area in editor tab bar → create `~/.conescope/scratch/Untitled-N` → open as tab
2. Edits auto-save to scratch file (debounced 500ms)
3. Cmd+S on untitled tab → native "Save As" → on success: update tab path, delete scratch file
4. Close tab → scratch file stays on disk
5. Session restore → scan scratch dir, restore as editor tabs, deduplicate with persisted tabs
6. Counter resumes from highest existing N + 1

## Untitled Tab Detection
Path check — tab is "untitled" if path starts with scratch dir. No new enum variant needed:
```rust
fn is_scratch_file(path: &Path) -> bool {
    path.starts_with(scratch_dir())
}
```

## Auto-Save
On `CodeEditorEvent::Change`, debounce 500ms, write to scratch path via existing `save_file()`.

## Save As Flow (Cmd+S on untitled tab)
1. Open `rfd::AsyncFileDialog::new().save_file()`
2. On success: write content to chosen path, delete scratch file, update tab path
3. On cancel: no-op

## Session Restore
In `FocusView::new()`, after restoring persisted editor tabs, scan `~/.conescope/scratch/` for `Untitled-*` files. Add as tabs, deduplicate by path.

## Files to Modify
| File | Change |
|------|--------|
| `conescope-ui/src/views/editor_tabs.rs` | Click handler on empty area, untitled counter, `is_scratch_file()` helper |
| `conescope-ui/src/views/code_viewer.rs` | Auto-save on change (debounced), "Save As" method |
| `conescope-ui/src/views/focus_view.rs` | Wire Cmd+S to Save As for untitled tabs, restore scratch tabs on startup |
| `conescope-core/src/settings.rs` | `scratch_dir()` path helper |

## New Dependency
`rfd` crate — Rust File Dialog. Async native macOS save dialog. Works with GPUI async model.
