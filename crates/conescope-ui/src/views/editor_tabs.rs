use std::path::Path;

use gpui::prelude::*;
use gpui::{Entity, EventEmitter, MouseButton, div, px, rgba};

use crate::state::app_state::AppState;
use crate::theme::Theme;

/// Border color shared across tab bar elements (matches terminal tabs).
const BORDER_COLOR: u32 = 0x3c3c_3cff;

#[derive(Debug, Clone)]
pub enum EditorTabsEvent {
    SelectTab(usize),
    CloseTab(usize),
}

impl EventEmitter<EditorTabsEvent> for EditorTabs {}

#[derive(Debug, Clone)]
pub struct EditorTab {
    pub path: String,
    pub name: String,
    pub modified: bool,
}

pub struct EditorTabs {
    app_state: Entity<AppState>,
    tabs: Vec<EditorTab>,
    active_index: Option<usize>,
}

impl std::fmt::Debug for EditorTabs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EditorTabs")
            .field("tabs", &self.tabs)
            .field("active_index", &self.active_index)
            .finish_non_exhaustive()
    }
}

impl EditorTabs {
    #[must_use]
    pub fn new(app_state: Entity<AppState>) -> Self {
        Self {
            app_state,
            tabs: Vec::new(),
            active_index: None,
        }
    }

    /// Open a file tab. If already open, just focus it.
    pub fn open_tab(&mut self, path: &str, cx: &mut gpui::Context<Self>) {
        // Check if already open
        if let Some(idx) = self.tabs.iter().position(|t| t.path == path) {
            self.active_index = Some(idx);
            cx.emit(EditorTabsEvent::SelectTab(idx));
            cx.notify();
            return;
        }

        let name = Path::new(path)
            .file_name()
            .map_or_else(|| path.to_owned(), |n| n.to_string_lossy().to_string());

        self.tabs.push(EditorTab {
            path: path.to_owned(),
            name,
            modified: false,
        });
        let idx = self.tabs.len() - 1;
        self.active_index = Some(idx);
        cx.emit(EditorTabsEvent::SelectTab(idx));
        cx.notify();
    }

    /// Close a tab by index.
    pub fn close_tab(&mut self, index: usize, cx: &mut gpui::Context<Self>) {
        if index >= self.tabs.len() {
            return;
        }
        self.tabs.remove(index);
        // Adjust active index
        if self.tabs.is_empty() {
            self.active_index = None;
        } else if let Some(active) = self.active_index {
            if active >= self.tabs.len() {
                self.active_index = Some(self.tabs.len() - 1);
            } else if active > index {
                self.active_index = Some(active - 1);
            }
        }
        cx.emit(EditorTabsEvent::CloseTab(index));
        cx.notify();
    }

    /// Toggle the modified indicator for a file.
    pub fn set_modified(&mut self, path: &str, modified: bool, cx: &mut gpui::Context<Self>) {
        if let Some(tab) = self.tabs.iter_mut().find(|t| t.path == path) {
            tab.modified = modified;
            cx.notify();
        }
    }

    /// Currently active file path.
    #[must_use]
    pub fn active_path(&self) -> Option<&str> {
        self.active_index
            .and_then(|idx| self.tabs.get(idx))
            .map(|t| t.path.as_str())
    }

    /// Number of open tabs.
    #[must_use]
    pub fn tab_count(&self) -> usize {
        self.tabs.len()
    }

    /// Get all open tab paths (for session persistence).
    #[must_use]
    pub fn tab_paths(&self) -> Vec<String> {
        self.tabs.iter().map(|t| t.path.clone()).collect()
    }

    /// Restore tabs from saved paths. Does not emit events.
    pub fn restore_tabs(&mut self, paths: &[String], active: Option<&str>) {
        self.tabs.clear();
        for path in paths {
            let name = Path::new(path)
                .file_name()
                .map_or_else(|| path.clone(), |n| n.to_string_lossy().to_string());
            self.tabs.push(EditorTab {
                path: path.clone(),
                name,
                modified: false,
            });
        }
        self.active_index = active.and_then(|a| self.tabs.iter().position(|t| t.path == a));
        if self.active_index.is_none() && !self.tabs.is_empty() {
            self.active_index = Some(0);
        }
    }
}

impl Render for EditorTabs {
    fn render(
        &mut self,
        _window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> impl IntoElement {
        let theme = self.app_state.read(cx).theme().clone();

        if self.tabs.is_empty() {
            // Empty bar: full bottom border baseline
            return div()
                .h(px(28.))
                .border_b_1()
                .border_color(rgba(BORDER_COLOR))
                .bg(theme.background);
        }

        let active_bg = theme.editor_bg;
        let inactive_bg = theme.background;

        let mut bar = div()
            .h(px(28.))
            .flex()
            .flex_row()
            .items_end()
            .bg(inactive_bg);

        // Left padding spacer with bottom border
        bar = bar.child(border_b_spacer().w(px(8.)));

        for (i, tab) in self.tabs.iter().enumerate() {
            let active = self.active_index == Some(i);
            bar = bar.child(render_tab(
                tab,
                i,
                active,
                active_bg,
                inactive_bg,
                &theme,
                cx,
            ));
        }

        // Flex spacer with bottom border (continues the baseline)
        bar.child(border_b_spacer().flex_1())
    }
}

/// Small spacer div with only a bottom border (the baseline).
fn border_b_spacer() -> gpui::Div {
    div().h_full().border_b_1().border_color(rgba(BORDER_COLOR))
}

#[allow(clippy::too_many_arguments)]
fn render_tab(
    tab: &EditorTab,
    index: usize,
    active: bool,
    active_bg: gpui::Rgba,
    inactive_bg: gpui::Rgba,
    theme: &Theme,
    cx: &mut gpui::Context<EditorTabs>,
) -> gpui::Div {
    let fg = if active { theme.text } else { theme.text_muted };

    let label = if tab.modified {
        format!("{} \u{25CF}", tab.name) // ● modified indicator
    } else {
        tab.name.clone()
    };

    let bg = if active { active_bg } else { inactive_bg };

    let close_index = index;

    let base = div()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(4.))
        .h_full()
        .px(px(12.))
        .text_size(px(12.))
        .text_color(fg)
        .bg(bg)
        .cursor_pointer()
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |this, _event: &gpui::MouseDownEvent, _window, cx| {
                this.active_index = Some(index);
                cx.emit(EditorTabsEvent::SelectTab(index));
                cx.notify();
            }),
        )
        .child(label)
        .child(
            div()
                .text_size(px(10.))
                .text_color(theme.text_faint)
                .cursor_pointer()
                .hover(|s| s.text_color(rgba(0xcccc_ccff)))
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |this, _event: &gpui::MouseDownEvent, _window, cx| {
                        this.close_tab(close_index, cx);
                    }),
                )
                .child("\u{00D7}"), // × close button
        );

    if active {
        // Active tab: left+right borders, no bottom border (connects to editor content)
        base.border_l_1()
            .border_r_1()
            .border_color(rgba(BORDER_COLOR))
    } else {
        // Inactive tab: bottom border (the baseline), hover effect
        base.border_b_1()
            .border_color(rgba(BORDER_COLOR))
            .hover(|s| s.bg(rgba(0x3333_33ff)))
    }
}
