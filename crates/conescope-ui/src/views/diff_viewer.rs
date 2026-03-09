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

    /// Load diff data for a file.
    pub fn show_diff(
        &mut self,
        path: &str,
        staged: bool,
        hunks: Vec<DiffHunk>,
        cx: &mut gpui::Context<Self>,
    ) {
        self.file_path = Some(path.to_owned());
        self.staged = staged;
        self.hunks = hunks;
        cx.notify();
    }

    /// Clear displayed diff.
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
        let state = self.app_state.read(cx);
        let theme = state.theme().clone();
        let settings = state.settings_store.read(cx).settings().clone();
        #[allow(clippy::cast_precision_loss)]
        let font_size = settings.editor_font_size as f32;

        let Some(ref file_path) = self.file_path else {
            return div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .text_size(px(font_size))
                .text_color(theme.text_disabled)
                .child("No diff selected")
                .into_any_element();
        };

        let font_family = settings.font_family;

        let staged_label = if self.staged {
            "(staged)"
        } else {
            "(unstaged)"
        };

        // Header
        let header = div()
            .w_full()
            .flex()
            .items_center()
            .gap(px(8.))
            .px(px(12.))
            .py(px(6.))
            .border_b_1()
            .border_color(theme.border)
            .bg(theme.background)
            .text_size(px(font_size))
            .child(div().text_color(theme.text).child(file_path.clone()))
            .child(div().text_color(theme.text_muted).child(staged_label));

        // Diff content
        let content = if self.hunks.is_empty() {
            div()
                .id("diff-viewer-scroll")
                .flex_1()
                .min_h_0()
                .overflow_y_scroll()
                .bg(theme.editor_bg)
                .flex()
                .items_center()
                .justify_center()
                .text_size(px(font_size))
                .text_color(theme.text_disabled)
                .child("No changes in this file")
        } else {
            let mut content_div = div()
                .id("diff-viewer-scroll")
                .flex_1()
                .min_h_0()
                .overflow_y_scroll()
                .track_scroll(&self.scroll_handle)
                .bg(theme.editor_bg)
                .font_family(SharedString::from(font_family))
                .text_size(px(font_size));

            for hunk in &self.hunks {
                // Hunk header
                content_div = content_div.child(
                    div()
                        .w_full()
                        .px(px(8.))
                        .py(px(2.))
                        .bg(theme.diff_hunk_header_bg)
                        .text_size(px(font_size - 2.0))
                        .text_color(theme.text_faint)
                        .child(hunk.header.clone()),
                );

                // Diff lines
                for line in &hunk.lines {
                    content_div = content_div.child(render_diff_line(line, font_size, &theme));
                }
            }

            content_div
        };

        div()
            .size_full()
            .flex()
            .flex_col()
            .child(header)
            .child(content)
            .into_any_element()
    }
}

/// Render a single diff line with line number gutters and content.
fn render_diff_line(line: &DiffLine, font_size: f32, theme: &Theme) -> impl IntoElement {
    let (bg, prefix, text_color) = match line.origin {
        LineOrigin::Addition => (theme.diff_add_bg, "+", theme.diff_add_text),
        LineOrigin::Deletion => (theme.diff_delete_bg, "-", theme.diff_delete_text),
        LineOrigin::Context => (rgba(0x0000_0000), " ", theme.text),
    };

    let old_ln = line.old_lineno.map_or_else(String::new, |n| n.to_string());
    let new_ln = line.new_lineno.map_or_else(String::new, |n| n.to_string());

    // Trim trailing newline from content
    let content = line.content.trim_end_matches('\n');

    div()
        .w_full()
        .flex()
        .bg(bg)
        .child(
            // Old line number gutter
            div()
                .w(px(36.))
                .flex_shrink_0()
                .text_size(px(font_size - 3.0))
                .text_color(theme.text_faint)
                .text_right()
                .px(px(4.))
                .child(old_ln),
        )
        .child(
            // New line number gutter
            div()
                .w(px(36.))
                .flex_shrink_0()
                .text_size(px(font_size - 3.0))
                .text_color(theme.text_faint)
                .text_right()
                .px(px(4.))
                .child(new_ln),
        )
        .child(
            // Prefix (+/-/space) and content
            div()
                .flex_1()
                .overflow_hidden()
                .text_color(text_color)
                .child(format!("{prefix}{content}")),
        )
}
