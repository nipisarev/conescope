# Markdown Preview Design

## Overview

Tab-based markdown preview in the editor area. Follows the existing DiffViewer pattern — a `[preview]` tab opens alongside the source `.md` tab, showing rendered markdown.

## Trigger

- **Icon**: Eye/book SVG in the editor tab bar trailing area (right side, next to "+" button). Only visible when the active tab is a `.md` file.
- **Keybinding**: `Cmd+Shift+M` toggles the preview tab.
- **Action**: `ToggleMarkdownPreview` — opens preview if none exists, closes it if already open.

## Tab Model

`EditorTab` gets a new field: `preview: bool`.

- `preview = false` — normal file or diff tab (existing behavior)
- `preview = true` — markdown preview tab
- Tab name: `"filename.md [preview]"`
- Session persistence: preview tabs filtered out (like diff tabs)
- Deduplication: only one preview tab per source path

## View Switching

`render_editor_area()` conditional expands:

```
active_tab.preview       → MarkdownViewer
active_tab.diff_mode     → DiffViewer
else                     → CodeEditor
```

## MarkdownViewer Entity

New file: `crates/conescope-ui/src/views/markdown_viewer.rs`

**State:**
- `file_path: String`
- `content: String`
- `scroll_handle: UniformListScrollHandle`

**Methods:**
- `show_preview(path, content, cx)` — sets content, notifies
- `update_content(content, cx)` — updates on source change
- `clear(cx)` — resets state

**Rendering:**
- Parse markdown with `pulldown-cmark` crate (CommonMark compliant)
- Map parser events directly to GPUI elements (no HTML intermediate):
  - `# H1` → `div().text_size(px(24.)).font_weight(BOLD)`
  - `## H2` → `div().text_size(px(20.)).font_weight(BOLD)`
  - `### H3` → `div().text_size(px(16.)).font_weight(SEMIBOLD)`
  - Paragraphs → `div().mb(px(8.))` with text children
  - Code blocks → `div().bg(theme.panel).rounded(px(4.)).p(px(8.)).font_family("monospace")`
  - Inline code → span with `bg(theme.panel)` and monospace font
  - Lists → indented divs with bullet (`•`) or number prefixes
  - Links → `div().text_color(theme.accent).cursor_pointer()`
  - Blockquotes → `div().border_l_2().border_color(theme.border).pl(px(12.))`
  - Horizontal rules → `div().h(px(1.)).bg(theme.border).my(px(8.))`
- Scrollable container with `.id("md-preview").overflow_y_scroll()`
- All colors from `Theme` struct

## Wiring (FocusView)

1. Create `MarkdownViewer` entity in `FocusView::new()` alongside `DiffViewer`
2. Subscribe to `EditorTabsEvent::OpenPreview(path)`:
   - Read file content from disk
   - Call `markdown_viewer.show_preview(path, content, cx)`
3. `render_editor_area()` checks `active_tab.preview` first

## Auto-close

In `EditorTabs::close_tab()`, after removing a tab:
- Check if any remaining preview tab's source path has no matching file tab
- If orphaned, close the preview tab too

Preview tabs survive switching to other tabs — only closed when:
- Source `.md` tab is closed (auto-close)
- User explicitly closes the preview tab
- `Cmd+Shift+M` toggles it off

## Auto-update

When `CodeEditor` buffer changes for a `.md` file:
- Emit `BufferChanged(path, content)` event
- `FocusView` forwards to `MarkdownViewer::update_content()` if preview is open for that path

## Dependencies

- `pulldown-cmark` — CommonMark parser, pure Rust, no allocator dependency
- Add to `conescope-ui/Cargo.toml`

## Files Changed

| File | Change |
|------|--------|
| `conescope-ui/Cargo.toml` | Add `pulldown-cmark` dependency |
| `views/markdown_viewer.rs` | New — MarkdownViewer entity + render |
| `views/mod.rs` | Add `pub mod markdown_viewer;` |
| `views/editor_tabs.rs` | Add `preview` field, `open_preview_tab()`, preview icon in tab bar |
| `views/focus_view.rs` | Create MarkdownViewer, wire events, expand render_editor_area |
| `views/code_viewer.rs` | Emit `BufferChanged` event for `.md` files |
| `actions.rs` | Add `ToggleMarkdownPreview` action |
| `conescope/src/main.rs` | Bind `Cmd+Shift+M` to `ToggleMarkdownPreview` |
| `icons.rs` | Add preview icon SVG path (if not already available) |
