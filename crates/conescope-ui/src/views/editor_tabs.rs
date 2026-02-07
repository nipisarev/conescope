use std::path::Path;

use gpui::prelude::*;
use gpui::{EventEmitter, MouseButton, div, px, rgba};

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

#[derive(Debug, Default)]
pub struct EditorTabs {
    tabs: Vec<EditorTab>,
    active_index: Option<usize>,
}

impl EditorTabs {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
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
        if self.tabs.is_empty() {
            return div()
                .h(px(28.))
                .border_b_1()
                .border_color(rgba(0x3c3c_3cff));
        }

        let mut bar = div()
            .h(px(28.))
            .flex()
            .flex_row()
            .items_center()
            .overflow_hidden()
            .border_b_1()
            .border_color(rgba(0x3c3c_3cff));

        for (i, tab) in self.tabs.iter().enumerate() {
            let active = self.active_index == Some(i);
            bar = bar.child(render_tab(tab, i, active, cx));
        }

        bar
    }
}

fn render_tab(
    tab: &EditorTab,
    index: usize,
    active: bool,
    cx: &mut gpui::Context<EditorTabs>,
) -> gpui::Div {
    let bg = if active {
        rgba(0x1e1e_1eff)
    } else {
        rgba(0x2d2d_2dff)
    };
    let fg = if active {
        rgba(0xd4d4_d4ff)
    } else {
        rgba(0x8888_88ff)
    };

    let label = if tab.modified {
        format!("{} \u{25CF}", tab.name) // ● modified indicator
    } else {
        tab.name.clone()
    };

    let close_index = index;

    div()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(4.))
        .px(px(12.))
        .h_full()
        .bg(bg)
        .text_color(fg)
        .text_size(px(12.))
        .border_r_1()
        .border_color(rgba(0x3c3c_3cff))
        .cursor_pointer()
        .hover(|s| s.bg(rgba(0x2525_26ff)))
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
                .text_color(rgba(0x6666_66ff))
                .cursor_pointer()
                .hover(|s| s.text_color(rgba(0xcccc_ccff)))
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |this, _event: &gpui::MouseDownEvent, _window, cx| {
                        this.close_tab(close_index, cx);
                    }),
                )
                .child("\u{00D7}"), // × close button
        )
}
