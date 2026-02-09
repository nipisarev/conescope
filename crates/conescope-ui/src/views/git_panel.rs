use std::path::Path;

use gpui::prelude::*;
use gpui::{Entity, EventEmitter, MouseButton, ScrollHandle, div, px, rgba};

use conescope_git::status::{FileStatus, GitFileEntry, StageStatus};

use crate::state::app_state::AppState;
use crate::state::git_store::{GitStore, GitStoreEvent};
use crate::theme::Theme;

#[derive(Debug, Clone)]
pub enum GitPanelEvent {
    OpenFile(String),
}

impl EventEmitter<GitPanelEvent> for GitPanel {}

pub struct GitPanel {
    app_state: Entity<AppState>,
    git_store: Entity<GitStore>,
    staged_expanded: bool,
    unstaged_expanded: bool,
    selected: Option<(String, StageStatus)>,
    scroll_handle: ScrollHandle,
}

impl std::fmt::Debug for GitPanel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GitPanel").finish_non_exhaustive()
    }
}

impl GitPanel {
    #[must_use]
    pub fn new(app_state: Entity<AppState>, git_store: Entity<GitStore>) -> Self {
        Self {
            app_state,
            git_store,
            staged_expanded: true,
            unstaged_expanded: true,
            selected: None,
            scroll_handle: ScrollHandle::new(),
        }
    }

    pub fn refresh(&self, cx: &mut gpui::Context<Self>) {
        self.git_store.update(cx, GitStore::refresh);
    }

    fn render_sections(
        &self,
        entries: &[GitFileEntry],
        theme: &Theme,
        cx: &mut gpui::Context<Self>,
    ) -> gpui::Stateful<gpui::Div> {
        let staged: Vec<&GitFileEntry> = entries
            .iter()
            .filter(|e| e.stage == StageStatus::Staged)
            .collect();
        let unstaged: Vec<&GitFileEntry> = entries
            .iter()
            .filter(|e| e.stage == StageStatus::Unstaged)
            .collect();

        let mut scroll_div = div()
            .id("git-panel-scroll")
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .track_scroll(&self.scroll_handle);

        if !staged.is_empty() {
            scroll_div = scroll_div.child(render_section_header(
                "Staged",
                staged.len(),
                self.staged_expanded,
                cx.listener(|this, _, _, _| {
                    this.staged_expanded = !this.staged_expanded;
                }),
                theme,
            ));
            if self.staged_expanded {
                for entry in &staged {
                    scroll_div = scroll_div.child(render_file_entry(
                        entry,
                        self.selected.as_ref(),
                        theme,
                        cx,
                    ));
                }
            }
        }

        if !unstaged.is_empty() {
            scroll_div = scroll_div.child(render_section_header(
                "Unstaged",
                unstaged.len(),
                self.unstaged_expanded,
                cx.listener(|this, _, _, _| {
                    this.unstaged_expanded = !this.unstaged_expanded;
                }),
                theme,
            ));
            if self.unstaged_expanded {
                for entry in &unstaged {
                    scroll_div = scroll_div.child(render_file_entry(
                        entry,
                        self.selected.as_ref(),
                        theme,
                        cx,
                    ));
                }
            }
        }

        scroll_div
    }
}

impl Render for GitPanel {
    fn render(
        &mut self,
        _window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> impl IntoElement {
        let store = self.git_store.read(cx);
        let theme = self.app_state.read(cx).theme().clone();

        // No repo
        if !store.has_repo() {
            return div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .text_size(px(11.))
                .text_color(theme.text_disabled)
                .child("Not a git repository")
                .into_any_element();
        }

        let entries = store.entries().to_vec();
        let branch = store.branch().unwrap_or("HEAD").to_owned();

        // No changes
        if entries.is_empty() {
            return div()
                .size_full()
                .flex()
                .flex_col()
                .child(render_header_bar(&branch, &theme, cx))
                .child(
                    div()
                        .flex_1()
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_size(px(11.))
                        .text_color(theme.text_disabled)
                        .child("No changes"),
                )
                .into_any_element();
        }

        let scroll_div = self.render_sections(&entries, &theme, cx);

        div()
            .size_full()
            .flex()
            .flex_col()
            .min_h_0()
            .child(render_header_bar(&branch, &theme, cx))
            .child(scroll_div)
            .into_any_element()
    }
}

/// Top bar: "Git: {branch}" + refresh button.
fn render_header_bar(branch: &str, theme: &Theme, cx: &mut gpui::Context<GitPanel>) -> gpui::Div {
    div()
        .flex()
        .flex_row()
        .items_center()
        .justify_between()
        .px(px(8.))
        .py(px(6.))
        .border_b_1()
        .border_color(theme.border)
        .text_size(px(12.))
        .child(
            div()
                .text_color(theme.text_muted)
                .child(format!("Git: {branch}")),
        )
        .child(
            div()
                .cursor_pointer()
                .text_color(theme.text_muted)
                .hover(|s| s.text_color(theme.text))
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|this, _, _, cx| {
                        this.refresh(cx);
                    }),
                )
                .child("\u{21bb}"), // ↻
        )
}

/// Collapsible section header (e.g. "Staged (3)").
fn render_section_header(
    title: &str,
    count: usize,
    expanded: bool,
    on_click: impl Fn(&gpui::MouseDownEvent, &mut gpui::Window, &mut gpui::App) + 'static,
    theme: &Theme,
) -> gpui::Div {
    let arrow = if expanded { "\u{25be}" } else { "\u{25b8}" }; // ▾ / ▸
    div()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(4.))
        .px(px(8.))
        .py(px(4.))
        .bg(theme.background)
        .text_size(px(11.))
        .text_color(theme.text_muted)
        .cursor_pointer()
        .on_mouse_down(MouseButton::Left, on_click)
        .child(arrow)
        .child(format!("{title} ({count})"))
}

/// Single file entry row.
fn render_file_entry(
    entry: &GitFileEntry,
    selected: Option<&(String, StageStatus)>,
    theme: &Theme,
    cx: &mut gpui::Context<GitPanel>,
) -> gpui::Div {
    let path = entry.path.clone();
    let stage = entry.stage;
    let staged = stage == StageStatus::Staged;

    let file_name = Path::new(&entry.path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(&entry.path)
        .to_owned();

    let status_label = entry.status.to_string();
    let status_color = match entry.status {
        FileStatus::Modified => theme.accent,
        FileStatus::Added => rgba(0x73c9_91ff),
        FileStatus::Deleted => rgba(0xe06c_75ff),
        FileStatus::Renamed => rgba(0xe5c0_7bff),
        FileStatus::Untracked => theme.text_faint,
    };

    let is_selected = selected.is_some_and(|(p, s)| p == &path && *s == stage);
    let bg = if is_selected {
        theme.element_hover
    } else {
        theme.panel
    };
    let surface = theme.surface;

    let path_for_click = path.clone();

    div()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(6.))
        .px(px(12.))
        .py(px(3.))
        .text_size(px(12.))
        .bg(bg)
        .cursor_pointer()
        .hover(move |s| s.bg(surface))
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |this, _, _, cx| {
                this.selected = Some((path_for_click.clone(), stage));
                this.git_store.update(cx, |_store, cx| {
                    cx.emit(GitStoreEvent::OpenDiff {
                        path: path_for_click.clone(),
                        staged,
                    });
                });
                cx.notify();
            }),
        )
        .child(
            div()
                .text_color(status_color)
                .text_size(px(10.))
                .w(px(18.))
                .flex_shrink_0()
                .child(status_label),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .overflow_x_hidden()
                .child(
                    div()
                        .text_color(theme.text)
                        .text_size(px(12.))
                        .whitespace_nowrap()
                        .child(file_name),
                )
                .child(
                    div()
                        .text_color(theme.text_faint)
                        .text_size(px(10.))
                        .whitespace_nowrap()
                        .child(path),
                ),
        )
}
