use std::path::Path;

use gpui::prelude::*;
use gpui::{Entity, EventEmitter, MouseButton, Rgba, div, px};

use crate::state::app_state::AppState;
use crate::theme::Theme;

#[derive(Debug, Clone)]
pub enum EditorTabsEvent {
    SelectTab(usize),
    CloseTab(usize),
    NewUntitled,
}

impl EventEmitter<EditorTabsEvent> for EditorTabs {}

#[derive(Debug, Clone)]
pub struct EditorTab {
    pub path: String,
    pub name: String,
    pub modified: bool,
    /// `Some(staged)` for diff tabs, `None` for file tabs.
    pub diff_mode: Option<bool>,
}

pub struct EditorTabs {
    app_state: Entity<AppState>,
    tabs: Vec<EditorTab>,
    active_index: Option<usize>,
    next_untitled_id: usize,
    /// Current `project_id` for scratch file creation (set by `FocusView` on instance switch).
    pub current_project_id: Option<String>,
}

impl std::fmt::Debug for EditorTabs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EditorTabs")
            .field("tabs", &self.tabs)
            .field("active_index", &self.active_index)
            .field("next_untitled_id", &self.next_untitled_id)
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
            next_untitled_id: 0,
            current_project_id: None,
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
            diff_mode: None,
        });
        let idx = self.tabs.len() - 1;
        self.active_index = Some(idx);
        cx.emit(EditorTabsEvent::SelectTab(idx));
        cx.notify();
    }

    /// Open a diff tab. If already open, just focus it.
    pub fn open_diff_tab(&mut self, path: &str, staged: bool, cx: &mut gpui::Context<Self>) {
        // Check if this exact diff is already open
        if let Some(idx) = self
            .tabs
            .iter()
            .position(|t| t.path == path && t.diff_mode == Some(staged))
        {
            self.active_index = Some(idx);
            cx.emit(EditorTabsEvent::SelectTab(idx));
            cx.notify();
            return;
        }

        let file_name = std::path::Path::new(path)
            .file_name()
            .map_or_else(|| path.to_owned(), |n| n.to_string_lossy().to_string());

        self.tabs.push(EditorTab {
            path: path.to_owned(),
            name: format!("{file_name} [diff]"),
            modified: false,
            diff_mode: Some(staged),
        });
        let idx = self.tabs.len() - 1;
        self.active_index = Some(idx);
        cx.emit(EditorTabsEvent::SelectTab(idx));
        cx.notify();
    }

    /// Close a tab by index. Scratch files are archived instead of deleted.
    pub fn close_tab(&mut self, index: usize, cx: &mut gpui::Context<Self>) {
        if index >= self.tabs.len() {
            return;
        }
        // Archive scratch file on close
        let path = &self.tabs[index].path;
        if conescope_core::settings::SettingsJson::is_scratch_file(std::path::Path::new(path)) {
            let path_buf = std::path::PathBuf::from(path);
            // Extract project_id from path: scratch/{project_id}/Untitled-N
            let project_id = path_buf
                .parent()
                .and_then(|p| p.file_name())
                .map(|n| n.to_string_lossy().to_string());
            let archive =
                conescope_core::settings::SettingsJson::archive_dir(project_id.as_deref());
            std::fs::create_dir_all(&archive).ok();
            if let Some(file_name) = path_buf.file_name() {
                let dest = archive.join(file_name);
                let _ = std::fs::rename(path, &dest);
            }
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

    /// Close the currently active tab (if any).
    pub fn close_active_tab(&mut self, cx: &mut gpui::Context<Self>) {
        if let Some(idx) = self.active_index {
            self.close_tab(idx, cx);
        }
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

    /// Currently active tab (if any).
    #[must_use]
    pub fn active_tab(&self) -> Option<&EditorTab> {
        self.active_index.and_then(|idx| self.tabs.get(idx))
    }

    /// Number of open tabs.
    #[must_use]
    pub fn tab_count(&self) -> usize {
        self.tabs.len()
    }

    /// Get all open tab paths (for session persistence). Diff tabs excluded.
    #[must_use]
    pub fn tab_paths(&self) -> Vec<String> {
        self.tabs
            .iter()
            .filter(|t| t.diff_mode.is_none())
            .map(|t| t.path.clone())
            .collect()
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
                diff_mode: None,
            });
        }
        self.active_index = active.and_then(|a| self.tabs.iter().position(|t| t.path == a));
        if self.active_index.is_none() && !self.tabs.is_empty() {
            self.active_index = Some(0);
        }
    }

    /// Create a new untitled scratch file and open it as a tab.
    /// Returns the scratch file path.
    pub fn create_untitled(
        &mut self,
        project_id: Option<&str>,
        cx: &mut gpui::Context<Self>,
    ) -> String {
        use conescope_core::settings::SettingsJson;

        let scratch_dir = SettingsJson::scratch_dir(project_id);
        std::fs::create_dir_all(&scratch_dir).ok();

        let name = SettingsJson::next_untitled_name(self.next_untitled_id);
        self.next_untitled_id += 1;

        let path = scratch_dir.join(&name);
        // Create empty file
        std::fs::write(&path, "").ok();

        let path_str = path.to_string_lossy().to_string();
        self.open_tab(&path_str, cx);
        cx.emit(EditorTabsEvent::NewUntitled);
        path_str
    }

    /// Scan scratch dir and set counter to max existing + 1.
    pub fn init_counter_from_scratch_dir(&mut self, project_id: Option<&str>) {
        use conescope_core::settings::SettingsJson;

        let scratch_dir = SettingsJson::scratch_dir(project_id);
        let max_id = std::fs::read_dir(&scratch_dir)
            .into_iter()
            .flatten()
            .filter_map(std::result::Result::ok)
            .filter_map(|e| {
                let name = e.file_name().to_string_lossy().to_string();
                name.strip_prefix("Untitled-")
                    .and_then(|n| n.parse::<usize>().ok())
            })
            .max()
            .unwrap_or(0);
        self.next_untitled_id = max_id;
    }

    /// Update the active tab's path (after Save As).
    pub fn update_active_path(&mut self, new_path: &str, cx: &mut gpui::Context<Self>) {
        if let Some(idx) = self.active_index {
            if let Some(tab) = self.tabs.get_mut(idx) {
                new_path.clone_into(&mut tab.path);
                tab.name = std::path::Path::new(new_path)
                    .file_name()
                    .map_or_else(|| new_path.to_owned(), |n| n.to_string_lossy().to_string());
                tab.modified = false;
                cx.notify();
            }
        }
    }
}

impl Render for EditorTabs {
    fn render(
        &mut self,
        _window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> impl IntoElement {
        let state = self.app_state.read(cx);
        let theme = state.theme().clone();
        let font_size = state
            .settings_store
            .read(cx)
            .settings()
            .ui_editor_font_size();

        if self.tabs.is_empty() {
            // Empty bar: full bottom border baseline, click to create untitled
            return div()
                .h(px(28.))
                .border_b_1()
                .border_color(theme.border)
                .bg(theme.background)
                .cursor_pointer()
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|this, event: &gpui::MouseDownEvent, _window, cx| {
                        if event.click_count == 2 {
                            let pid = this.current_project_id.clone();
                            this.create_untitled(pid.as_deref(), cx);
                        }
                    }),
                );
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
        bar = bar.child(border_b_spacer(theme.border).w(px(8.)));

        for (i, tab) in self.tabs.iter().enumerate() {
            let active = self.active_index == Some(i);
            bar = bar.child(render_tab(
                tab,
                i,
                active,
                active_bg,
                inactive_bg,
                font_size,
                &theme,
                cx,
            ));
        }

        // Flex spacer with bottom border (continues the baseline), click to create untitled
        bar.child(
            border_b_spacer(theme.border)
                .flex_1()
                .cursor_pointer()
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|this, event: &gpui::MouseDownEvent, _window, cx| {
                        if event.click_count == 2 {
                            let pid = this.current_project_id.clone();
                            this.create_untitled(pid.as_deref(), cx);
                        }
                    }),
                ),
        )
    }
}

/// Small spacer div with only a bottom border (the baseline).
fn border_b_spacer(border: Rgba) -> gpui::Div {
    div().h_full().border_b_1().border_color(border)
}

#[allow(clippy::too_many_arguments)]
fn render_tab(
    tab: &EditorTab,
    index: usize,
    active: bool,
    active_bg: gpui::Rgba,
    inactive_bg: gpui::Rgba,
    font_size: f32,
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
        .text_size(px(font_size))
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
                .text_size(px(font_size - 2.0))
                .text_color(theme.text_faint)
                .cursor_pointer()
                .hover(|s| s.text_color(theme.text))
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |this, _event: &gpui::MouseDownEvent, _window, cx| {
                        this.close_tab(close_index, cx);
                    }),
                )
                .child("\u{00D7}"), // × close button
        );

    if active {
        base.border_l_1().border_r_1().border_color(theme.border)
    } else {
        base.border_b_1()
            .border_color(theme.border)
            .hover(|s| s.bg(theme.element_hover))
    }
}
