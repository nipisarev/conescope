use std::path::Path;

use gpui::prelude::*;
use gpui::{Entity, EventEmitter, MouseButton, Pixels, Point, ScrollHandle, div, px, rgba};

use conescope_git::status::{FileStatus, GitFileEntry, StageStatus};

use crate::state::app_state::AppState;
use crate::state::git_store::{GitStore, GitStoreEvent};
use crate::theme::Theme;

#[derive(Debug, Clone)]
pub enum GitPanelEvent {
    OpenFile(String),
}

impl EventEmitter<GitPanelEvent> for GitPanel {}

/// Tracks which file's context menu is open and where to render it.
struct ContextMenuState {
    path: String,
    status: FileStatus,
    stage: StageStatus,
    position: Point<Pixels>,
}

pub struct GitPanel {
    app_state: Entity<AppState>,
    git_store: Entity<GitStore>,
    staged_expanded: bool,
    unstaged_expanded: bool,
    selected: Option<(String, StageStatus)>,
    scroll_handle: ScrollHandle,
    root_handle: ScrollHandle,
    context_menu: Option<ContextMenuState>,
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
            root_handle: ScrollHandle::new(),
            context_menu: None,
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

        let mut root = div()
            .id("git-panel-root")
            .track_scroll(&self.root_handle)
            .relative()
            .size_full()
            .flex()
            .flex_col()
            .min_h_0()
            .child(render_header_bar(&branch, &theme, cx))
            .child(scroll_div);

        // Context menu overlay
        if let Some(ref menu) = self.context_menu {
            // Full-screen transparent dismiss backdrop
            root = root.child(
                div()
                    .id("git-ctx-dismiss")
                    .absolute()
                    .size_full()
                    .top_0()
                    .left_0()
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, _, _, cx| {
                            this.context_menu = None;
                            cx.notify();
                        }),
                    )
                    .on_mouse_down(
                        MouseButton::Right,
                        cx.listener(|this, _, _, cx| {
                            this.context_menu = None;
                            cx.notify();
                        }),
                    ),
            );
            // The actual menu
            root = root.child(render_context_menu(menu, &theme, cx));
        }

        root.into_any_element()
    }
}

/// Render the context menu as an absolute-positioned overlay.
fn render_context_menu(
    menu: &ContextMenuState,
    theme: &Theme,
    cx: &mut gpui::Context<GitPanel>,
) -> impl IntoElement {
    let x = f32::from(menu.position.x);
    let y = f32::from(menu.position.y);
    let items = context_menu_items(menu, theme, cx);

    div()
        .absolute()
        .top(px(y))
        .left(px(x))
        .w(px(160.))
        .bg(theme.surface)
        .border_1()
        .border_color(theme.border)
        .rounded(px(4.))
        .py(px(4.))
        .text_size(px(12.))
        .shadow_md()
        .children(items)
}

/// Build the list of context menu item elements for the given menu state.
fn context_menu_items(
    menu: &ContextMenuState,
    theme: &Theme,
    cx: &mut gpui::Context<GitPanel>,
) -> Vec<gpui::AnyElement> {
    let path = menu.path.clone();
    let stage = menu.stage;
    let status = menu.status;
    let staged = stage == StageStatus::Staged;

    let mut items: Vec<gpui::AnyElement> = Vec::new();

    // Stage / Unstage
    if staged {
        let p = path.clone();
        items.push(
            context_menu_item(
                "Unstage file",
                theme,
                cx.listener(move |this, _, _, cx| {
                    let p = p.clone();
                    this.git_store
                        .update(cx, |store, cx| store.unstage_file(&p, cx));
                    this.context_menu = None;
                    cx.notify();
                }),
            )
            .into_any_element(),
        );
    } else {
        let p = path.clone();
        items.push(
            context_menu_item(
                "Stage file",
                theme,
                cx.listener(move |this, _, _, cx| {
                    let p = p.clone();
                    this.git_store
                        .update(cx, |store, cx| store.stage_file(&p, cx));
                    this.context_menu = None;
                    cx.notify();
                }),
            )
            .into_any_element(),
        );
    }

    // Discard changes (unstaged modified/deleted/untracked files)
    if !staged
        && matches!(
            status,
            FileStatus::Modified | FileStatus::Deleted | FileStatus::Untracked
        )
    {
        let p = path.clone();
        items.push(
            context_menu_item(
                "Discard changes",
                theme,
                cx.listener(move |this, _, _, cx| {
                    let p = p.clone();
                    this.git_store
                        .update(cx, |store, cx| store.discard_file(&p, cx));
                    this.context_menu = None;
                    cx.notify();
                }),
            )
            .into_any_element(),
        );
    }

    // Open file
    {
        let p = path.clone();
        items.push(
            context_menu_item(
                "Open file",
                theme,
                cx.listener(move |this, _, _, cx| {
                    cx.emit(GitPanelEvent::OpenFile(p.clone()));
                    this.context_menu = None;
                    cx.notify();
                }),
            )
            .into_any_element(),
        );
    }

    // Show diff
    {
        let p = path.clone();
        items.push(
            context_menu_item(
                "Show diff",
                theme,
                cx.listener(move |this, _, _, cx| {
                    this.git_store.update(cx, |_store, cx| {
                        cx.emit(GitStoreEvent::OpenDiff {
                            path: p.clone(),
                            staged,
                        });
                    });
                    this.context_menu = None;
                    cx.notify();
                }),
            )
            .into_any_element(),
        );
    }

    items
}

/// A single clickable context menu item row.
fn context_menu_item(
    label: &str,
    theme: &Theme,
    on_click: impl Fn(&gpui::MouseDownEvent, &mut gpui::Window, &mut gpui::App) + 'static,
) -> gpui::Div {
    let hover_bg = theme.element_hover;
    div()
        .px(px(10.))
        .py(px(4.))
        .text_color(theme.text)
        .cursor_pointer()
        .hover(move |s| s.bg(hover_bg))
        .on_mouse_down(MouseButton::Left, on_click)
        .child(label.to_owned())
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
    let status = entry.status;
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
    let path_for_rclick = path.clone();

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
        .on_mouse_down(
            MouseButton::Right,
            cx.listener(move |this, event: &gpui::MouseDownEvent, _, cx| {
                // Convert window coords to panel-relative coords
                let root_origin = this.root_handle.bounds().origin;
                this.context_menu = Some(ContextMenuState {
                    path: path_for_rclick.clone(),
                    status,
                    stage,
                    position: Point {
                        x: event.position.x - root_origin.x,
                        y: event.position.y - root_origin.y,
                    },
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
