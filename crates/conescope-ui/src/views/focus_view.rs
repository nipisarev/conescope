use std::rc::Rc;

use gpui::prelude::*;
use gpui::{AppContext, Entity, MouseButton, SharedString, div, px, relative};

use crate::theme::Theme;

use conescope_core::instance::{InstanceType, TerminalTab};

use crate::state::app_state::AppState;
use crate::state::git_store::GitStoreEvent;
use crate::state::settings_store::SidebarTab;
use crate::terminal::spawn_terminal_pane;
use crate::views::code_viewer::{CodeEditor, CodeEditorEvent};
use crate::views::diff_viewer::DiffViewer;
use crate::views::editor_tabs::{EditorTabs, EditorTabsEvent};
use crate::views::file_tree::{FileTree, FileTreeEvent};
use crate::views::git_panel::{GitPanel, GitPanelEvent};
use crate::views::resizable_divider::{Axis, DragState, DragTarget, clamp_size, render_divider};
use crate::views::terminal_tabs::{ShellTabCb, render_tab_bar};

const SIDEBAR_MIN: f32 = 120.0;
const SIDEBAR_MAX: f32 = 600.0;
const TERMINAL_MIN: f32 = 80.0;
const TERMINAL_MAX: f32 = 800.0;

/// Heights of fixed UI elements for PTY resize calculations.
const TOP_BAR_HEIGHT: f32 = 36.0;
const ACTIVITY_BAR_HEIGHT: f32 = 28.0;
const TERMINAL_TABS_HEIGHT: f32 = 28.0;
const DIVIDER_SIZE: f32 = 1.0;

pub struct FocusView {
    app_state: Entity<AppState>,
    drag: Option<DragState>,
    file_tree: Entity<FileTree>,
    code_editor: Entity<CodeEditor>,
    editor_tabs: Entity<EditorTabs>,
    git_panel: Entity<GitPanel>,
    diff_viewer: Entity<DiffViewer>,
    /// Last focused instance ID, used to detect switches.
    last_focused_id: Option<String>,
}

impl std::fmt::Debug for FocusView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FocusView")
            .field("drag", &self.drag)
            .finish_non_exhaustive()
    }
}

impl FocusView {
    #[must_use]
    #[allow(clippy::too_many_lines)]
    pub fn new(app_state: Entity<AppState>, cx: &mut gpui::Context<Self>) -> Self {
        let git_store = app_state.read(cx).git_store.clone();
        let file_tree = cx.new(|cx| FileTree::new(app_state.clone(), Some(git_store.clone()), cx));
        let code_editor = cx.new(|_| CodeEditor::new());
        let editor_tabs = cx.new(|_| EditorTabs::new(app_state.clone()));

        let git_panel = cx.new(|_| GitPanel::new(app_state.clone(), git_store.clone()));
        let diff_viewer = cx.new(|_| DiffViewer::new(app_state.clone()));

        // Get the initial project_id for the focused instance
        let initial_project_id = {
            let state = app_state.read(cx);
            state.focused_instance_id(cx).and_then(|fid| {
                state
                    .instance_list
                    .read(cx)
                    .find_by_id(fid, cx)
                    .and_then(|e| e.read(cx).instance.project_id.clone())
            })
        };
        editor_tabs.update(cx, |tabs, _| {
            tabs.current_project_id.clone_from(&initial_project_id);
            tabs.init_counter_from_scratch_dir(initial_project_id.as_deref());
        });

        // FileTree → open file in editor
        let cv = code_editor.clone();
        let et = editor_tabs.clone();
        cx.subscribe(&file_tree, move |this, _ft, event, cx| {
            let FileTreeEvent::OpenFile(path) = event;
            et.update(cx, |tabs, cx| tabs.open_tab(path, cx));
            cv.update(cx, |viewer, cx| viewer.open_file(path, cx));
            this.app_state.update(cx, AppState::ensure_editor_visible);
        })
        .detach();

        // EditorTabs → select/close + persist tabs
        let cv2 = code_editor.clone();
        let et2 = editor_tabs.clone();
        let app_state_tabs = app_state.clone();
        cx.subscribe(&editor_tabs, move |_this, _tabs, event, cx| {
            match event {
                EditorTabsEvent::SelectTab(_idx) => {
                    let active = et2.read(cx).active_tab().cloned();
                    if let Some(tab) = active {
                        if tab.diff_mode.is_none() {
                            cv2.update(cx, |v, cx| v.open_file(&tab.path, cx));
                        }
                        // Diff tabs: DiffViewer already has content; render will switch view
                    }
                }
                EditorTabsEvent::CloseTab(_idx) => {
                    let active = et2.read(cx).active_tab().cloned();
                    if let Some(tab) = active {
                        if tab.diff_mode.is_none() {
                            cv2.update(cx, |v, cx| v.open_file(&tab.path, cx));
                        }
                        // Diff tabs: render will switch view automatically
                    } else {
                        cv2.update(cx, CodeEditor::close_file);
                    }
                }
                EditorTabsEvent::NewUntitled => {
                    if let Some(path) = et2.read(cx).active_path().map(str::to_owned) {
                        cv2.update(cx, |v, cx| v.open_file(&path, cx));
                    }
                    // Ensure editor panel is visible
                    app_state_tabs.update(cx, AppState::ensure_editor_visible);
                }
            }
            // Persist tab state after any change
            let tabs = et2.read(cx).tab_paths();
            let active = et2.read(cx).active_path().map(str::to_owned);
            app_state_tabs.update(cx, |s, cx| s.save_editor_tabs(tabs, active, cx));
        })
        .detach();

        // Restore editor tabs from session state
        let (saved_tabs, saved_active) = app_state.read(cx).saved_editor_tabs(cx);
        if !saved_tabs.is_empty() {
            editor_tabs.update(cx, |tabs, _| {
                tabs.restore_tabs(&saved_tabs, saved_active.as_deref());
            });
            // Open the active file in the viewer
            let active_path = editor_tabs.read(cx).active_path().map(str::to_owned);
            if let Some(path) = active_path {
                code_editor.update(cx, |v, cx| v.open_file(&path, cx));
            }
        }

        // Restore scratch files not already in saved tabs
        {
            let scratch_dir =
                conescope_core::settings::SettingsJson::scratch_dir(initial_project_id.as_deref());
            if let Ok(entries) = std::fs::read_dir(&scratch_dir) {
                let existing_paths: std::collections::HashSet<String> =
                    editor_tabs.read(cx).tab_paths().into_iter().collect();
                let mut scratch_paths: Vec<String> = entries
                    .filter_map(std::result::Result::ok)
                    .filter(|e| e.file_name().to_string_lossy().starts_with("Untitled-"))
                    .map(|e| e.path().to_string_lossy().to_string())
                    .filter(|p| !existing_paths.contains(p))
                    .collect();
                scratch_paths.sort();
                for path in scratch_paths {
                    editor_tabs.update(cx, |tabs, cx| tabs.open_tab(&path, cx));
                }
            }
        }

        // Auto-hide editor if no tabs open (sync ActivityBar toggle on fresh entry)
        if editor_tabs.read(cx).tab_count() == 0 && app_state.read(cx).editor_visible(cx) {
            app_state.update(cx, AppState::toggle_editor);
        }

        // Subscribe to FileSavedAs: update tab path + persist
        let et_for_save = editor_tabs.clone();
        let app_state_for_save = app_state.clone();
        cx.subscribe(&code_editor, move |_this, _editor, event, cx| {
            if let CodeEditorEvent::FileSavedAs(new_path) = event {
                // Update the tab's path from scratch → real path
                et_for_save.update(cx, |tabs, cx| {
                    tabs.update_active_path(new_path, cx);
                });
                // Persist tabs
                let tab_paths = et_for_save.read(cx).tab_paths();
                let active = et_for_save.read(cx).active_path().map(str::to_owned);
                app_state_for_save.update(cx, |s, cx| s.save_editor_tabs(tab_paths, active, cx));
            }
        })
        .detach();

        // GitStore::OpenDiff → diff the file, open diff tab, show in DiffViewer
        let dv_for_git = diff_viewer.clone();
        let gs_for_diff = git_store.clone();
        let et_for_diff = editor_tabs.clone();
        cx.subscribe(&git_store, move |this, _store, event, cx| {
            if let GitStoreEvent::OpenDiff { path, staged } = event {
                let path = path.clone();
                let staged = *staged;
                let dv = dv_for_git.clone();
                let path_for_cb = path.clone();
                // Open a diff tab in editor tabs
                et_for_diff.update(cx, |tabs, cx| tabs.open_diff_tab(&path, staged, cx));
                gs_for_diff.update(cx, |store, cx| {
                    store.diff_file(&path, staged, cx, move |hunks, cx| {
                        dv.update(cx, |viewer, cx| {
                            viewer.show_diff(&path_for_cb, staged, hunks, cx);
                        });
                    });
                });
                this.app_state.update(cx, AppState::ensure_editor_visible);
            }
        })
        .detach();

        // GitPanel::OpenFile → open file in editor tabs + code editor
        let cv_for_git = code_editor.clone();
        let et_for_git = editor_tabs.clone();
        let gs_for_open = git_store.clone();
        cx.subscribe(&git_panel, move |this, _panel, event, cx| {
            let GitPanelEvent::OpenFile(rel_path) = event;
            let abs_path = gs_for_open.read(cx).resolve_path(rel_path);
            et_for_git.update(cx, |tabs, cx| tabs.open_tab(&abs_path, cx));
            cv_for_git.update(cx, |viewer, cx| viewer.open_file(&abs_path, cx));
            this.app_state.update(cx, AppState::ensure_editor_visible);
        })
        .detach();

        // Set initial project path on git store
        if let Some(ref pid) = initial_project_id {
            let project_path = app_state
                .read(cx)
                .project_store
                .read(cx)
                .get(pid)
                .map(|p| p.path.clone());
            if let Some(path) = project_path {
                git_store.update(cx, |store, cx| {
                    store.set_project(Some(&path), cx);
                });
            }
        }

        let last_focused_id = app_state
            .read(cx)
            .focused_instance_id(cx)
            .map(str::to_owned);

        Self {
            app_state,
            drag: None,
            file_tree,
            code_editor,
            editor_tabs,
            git_panel,
            diff_viewer,
            last_focused_id,
        }
    }

    fn on_drag_move(
        &mut self,
        event: &gpui::MouseMoveEvent,
        _window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) {
        let Some(ref mut drag) = self.drag else {
            return;
        };
        let current = drag.position_component(event.position);
        let delta = drag.delta(current);
        if delta.abs() < 0.5 {
            return;
        }
        let app_state = self.app_state.clone();
        match drag.target {
            DragTarget::Sidebar => {
                let width = app_state.read(cx).sidebar_width(cx);
                let new_width = clamp_size(width + delta, SIDEBAR_MIN, SIDEBAR_MAX);
                app_state.update(cx, |s, cx| s.set_sidebar_width(new_width, cx));
            }
            DragTarget::Terminal => {
                let height = app_state.read(cx).terminal_height(cx);
                let new_height = clamp_size(height - delta, TERMINAL_MIN, TERMINAL_MAX);
                app_state.update(cx, |s, cx| s.set_terminal_height(new_height, cx));
            }
        }
    }

    fn on_drag_end(
        &mut self,
        _event: &gpui::MouseUpEvent,
        _window: &mut gpui::Window,
        _cx: &mut gpui::Context<Self>,
    ) {
        self.drag = None;
    }

    /// Handle Cmd+S: if active tab is a scratch file, trigger Save As dialog.
    /// Otherwise, save in place.
    pub fn save_active_file(&self, cx: &mut gpui::Context<Self>) {
        let active_path = self.editor_tabs.read(cx).active_path().map(str::to_owned);
        let Some(path) = active_path else { return };

        if conescope_core::settings::SettingsJson::is_scratch_file(std::path::Path::new(&path)) {
            self.code_editor.update(cx, CodeEditor::save_as);
        } else {
            self.code_editor.update(cx, |editor, cx| {
                let _ = editor.save_file(cx);
            });
        }
    }

    /// Close the active tab based on what's focused.
    ///
    /// - Terminal focused + shell tab → close that shell tab
    /// - Terminal focused + primary tab → request close instance
    /// - Editor focused → close active editor tab
    pub fn close_active_tab(&mut self, window: &mut gpui::Window, cx: &mut gpui::Context<Self>) {
        let state = self.app_state.read(cx);
        let Some(id) = state.focused_instance_id(cx) else {
            return;
        };
        let il = state.instance_list.read(cx);
        let Some(entry) = il.find_by_id(id, cx).cloned() else {
            return;
        };

        let (terminal_focused, active_tab, id_owned, title) = {
            let inst = entry.read(cx);
            let term_focused = inst
                .active_focus_handle()
                .is_some_and(|fh| fh.is_focused(window));
            let title = inst
                .instance
                .title
                .clone()
                .unwrap_or_else(|| format!("Instance {id}"));
            (term_focused, inst.active_tab, id.to_owned(), title)
        };

        if terminal_focused {
            if let TerminalTab::Shell(shell_id) = active_tab {
                entry.update(cx, |e, cx| e.close_shell_tab(shell_id, cx));
                // Focus whatever tab is now active
                let fh = entry.read(cx).active_focus_handle().cloned();
                if let Some(fh) = fh {
                    fh.focus(window, cx);
                }
            } else {
                // Primary tab → close the instance
                self.app_state.update(cx, |s, cx| {
                    s.request_close_instance(&id_owned, &title, cx);
                });
            }
        } else {
            // Editor focused → close active editor tab
            self.editor_tabs.update(cx, EditorTabs::close_active_tab);
        }
    }

    #[allow(clippy::too_many_lines)]
    fn build_tab_bar(
        &self,
        entry: &gpui::Entity<crate::state::instance_entry::InstanceEntry>,
        theme: &Theme,
        cx: &mut gpui::Context<Self>,
    ) -> gpui::Div {
        let inst = entry.read(cx);
        let instance_type = inst.instance_type();
        let active_tab = inst.active_tab;
        let shell_tabs = inst.shell_tab_info();

        let entry_for_primary = entry.clone();
        let entry_for_add = entry.clone();
        let app_state_for_add = self.app_state.clone();

        // Click shell tab → switch to it
        let entry_click = entry.clone();
        let on_click_shell: ShellTabCb = Rc::new(move |shell_id, _ev, window, cx| {
            entry_click.update(cx, |e, cx| {
                e.set_active_tab(TerminalTab::Shell(shell_id), cx);
            });
            let fh = entry_click.read(cx).shell_focus_handle(shell_id).cloned();
            if let Some(fh) = fh {
                fh.focus(window, cx);
            }
        });

        // Close shell tab
        let entry_close = entry.clone();
        let on_close_shell: ShellTabCb = Rc::new(move |shell_id, _ev, window, cx| {
            entry_close.update(cx, |e, cx| e.close_shell_tab(shell_id, cx));
            let fh = entry_close.read(cx).active_focus_handle().cloned();
            if let Some(fh) = fh {
                fh.focus(window, cx);
            }
        });

        render_tab_bar(
            instance_type,
            active_tab,
            &shell_tabs,
            cx.listener(move |_this, _event: &gpui::MouseDownEvent, window, cx| {
                entry_for_primary.update(cx, |e, cx| {
                    e.set_active_tab(TerminalTab::Primary, cx);
                });
                let fh = entry_for_primary.read(cx).focus_handle.clone();
                if let Some(fh) = fh {
                    fh.focus(window, cx);
                }
            }),
            &on_click_shell,
            &on_close_shell,
            // "+" button: always spawn a new shell tab
            cx.listener(move |_this, _event: &gpui::MouseDownEvent, window, cx| {
                let cwd = {
                    let state = app_state_for_add.read(cx);
                    let inst = entry_for_add.read(cx);
                    inst.instance.project_id.as_ref().and_then(|pid| {
                        state
                            .project_store
                            .read(cx)
                            .get(pid)
                            .map(|p| p.path.clone())
                    })
                };
                let ff = Some(
                    app_state_for_add
                        .read(cx)
                        .settings_store
                        .read(cx)
                        .settings()
                        .font_family
                        .clone(),
                );
                let tc = app_state_for_add.read(cx).theme().terminal_colors();
                let lhr = app_state_for_add
                    .read(cx)
                    .settings_store
                    .read(cx)
                    .settings()
                    .terminal_line_height as f32;
                let pane = spawn_terminal_pane(cwd.as_deref(), ff.as_deref(), lhr, &tc, window, cx);
                entry_for_add.update(cx, |e, cx| {
                    let (id, rx) = e.add_shell_tab(pane, cx);
                    e.start_shell_output_polling(id, rx, cx);
                    e.set_active_tab(TerminalTab::Shell(id), cx);
                });
                let fh = entry_for_add.read(cx).active_focus_handle().cloned();
                if let Some(fh) = fh {
                    fh.focus(window, cx);
                }
            }),
            theme,
        )
    }
}

#[allow(clippy::too_many_arguments)]
fn render_terminal_pane(
    terminal_view: Option<&gpui::Entity<crate::terminal::terminal_view::TerminalView>>,
    focus_handle: Option<&gpui::FocusHandle>,
    tab_bar: gpui::Div,
    height: f32,
    fill_height: bool,
    font_size: f32,
    font_family: &str,
    theme: &Theme,
) -> gpui::Div {
    let fh = focus_handle.cloned();
    let click_handler =
        move |_: &gpui::MouseDownEvent, window: &mut gpui::Window, cx: &mut gpui::App| {
            if let Some(ref fh) = fh {
                fh.focus(window, cx);
            }
        };

    if let Some(tv) = terminal_view {
        let base = div()
            .flex()
            .flex_col()
            .overflow_hidden()
            .on_mouse_down(MouseButton::Left, click_handler)
            .child(tab_bar)
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .px(px(2.))
                    .font_family(SharedString::from(font_family.to_owned()))
                    .text_size(px(font_size))
                    .line_height(relative(1.0))
                    .child(tv.clone()),
            );
        if fill_height {
            base.flex_1().min_h_0()
        } else {
            base.h(px(height))
        }
    } else {
        let base = div()
            .flex()
            .flex_col()
            .overflow_hidden()
            .child(tab_bar)
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_color(theme.text_disabled)
                    .child("Terminal not attached"),
            );
        if fill_height {
            base.flex_1().min_h_0()
        } else {
            base.h(px(height))
        }
    }
}

fn render_editor_area(
    editor_tabs: &Entity<EditorTabs>,
    code_editor: &Entity<CodeEditor>,
    diff_viewer: &Entity<DiffViewer>,
    active_diff_mode: Option<bool>,
    theme: &Theme,
) -> gpui::Div {
    div()
        .flex_1()
        .min_h_0()
        .flex()
        .flex_col()
        .bg(theme.editor_bg)
        .child(editor_tabs.clone())
        .child(
            div()
                .flex_1()
                .min_h_0()
                .overflow_hidden()
                .child(if active_diff_mode.is_some() {
                    diff_viewer.clone().into_any_element()
                } else {
                    code_editor.clone().into_any_element()
                }),
        )
}

#[allow(clippy::too_many_arguments)]
fn render_main_area(
    editor_visible: bool,
    terminal_visible: bool,
    editor_tabs: &Entity<EditorTabs>,
    code_editor: &Entity<CodeEditor>,
    diff_viewer: &Entity<DiffViewer>,
    active_diff_mode: Option<bool>,
    terminal_view: Option<&gpui::Entity<crate::terminal::terminal_view::TerminalView>>,
    focus_handle: Option<&gpui::FocusHandle>,
    tab_bar: gpui::Div,
    terminal_height: f32,
    terminal_font_size: f32,
    font_family: &str,
    dragging_terminal: bool,
    drag_listener: impl Fn(&gpui::MouseDownEvent, &mut gpui::Window, &mut gpui::App) + 'static,
    theme: &Theme,
) -> gpui::Div {
    let mut col = div().flex_1().min_h_0().flex().flex_col().overflow_hidden();

    if editor_visible {
        col = col.child(render_editor_area(
            editor_tabs,
            code_editor,
            diff_viewer,
            active_diff_mode,
            theme,
        ));
    }

    if editor_visible && terminal_visible {
        col = col.child(render_divider(
            Axis::Vertical,
            dragging_terminal,
            drag_listener,
        ));
    }

    if terminal_visible {
        let fill = !editor_visible;
        col = col.child(render_terminal_pane(
            terminal_view,
            focus_handle,
            tab_bar,
            terminal_height,
            fill,
            terminal_font_size,
            font_family,
            theme,
        ));
    }

    if !editor_visible && !terminal_visible {
        col = col.child(
            div()
                .flex_1()
                .flex()
                .items_center()
                .justify_center()
                .text_color(theme.text_disabled)
                .text_size(px(12.))
                .child("All panels hidden (Cmd+B/E/T to toggle)"),
        );
    }

    col
}

impl Render for FocusView {
    #[allow(clippy::too_many_lines)]
    fn render(
        &mut self,
        _window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> impl IntoElement {
        // Read focused_id before borrowing state for the rest of render
        let focused_id = self
            .app_state
            .read(cx)
            .focused_instance_id(cx)
            .map(str::to_owned);

        // Detect instance switch: layout already switched by focus_instance()
        if focused_id != self.last_focused_id {
            // Restore editor tabs from the new instance's layout
            let (paths, active) = self.app_state.read(cx).saved_editor_tabs(cx);
            self.editor_tabs.update(cx, |tabs, _| {
                tabs.restore_tabs(&paths, active.as_deref());
            });
            if let Some(path) = self.editor_tabs.read(cx).active_path().map(str::to_owned) {
                self.code_editor.update(cx, |v, cx| v.open_file(&path, cx));
            } else {
                self.code_editor.update(cx, CodeEditor::close_file);
            }

            // Update project_id on editor_tabs and restore scratch files for new instance
            let project_id = focused_id.as_ref().and_then(|fid| {
                let state = self.app_state.read(cx);
                state
                    .instance_list
                    .read(cx)
                    .find_by_id(fid, cx)
                    .and_then(|e| e.read(cx).instance.project_id.clone())
            });
            self.editor_tabs.update(cx, |tabs, _| {
                tabs.current_project_id.clone_from(&project_id);
                tabs.init_counter_from_scratch_dir(project_id.as_deref());
            });

            // Update git store with project path for the new instance
            {
                let git_store = self.app_state.read(cx).git_store.clone();
                let project_path = project_id.as_ref().and_then(|pid| {
                    self.app_state
                        .read(cx)
                        .project_store
                        .read(cx)
                        .get(pid)
                        .map(|p| p.path.clone())
                });
                git_store.update(cx, |store, cx| {
                    store.set_project(project_path.as_deref(), cx);
                });
            }

            // Clear diff viewer on instance switch
            self.diff_viewer.update(cx, DiffViewer::clear);

            // Restore scratch files for the new instance's project
            if focused_id.is_some() {
                let scratch_dir =
                    conescope_core::settings::SettingsJson::scratch_dir(project_id.as_deref());
                if let Ok(entries) = std::fs::read_dir(&scratch_dir) {
                    let existing: std::collections::HashSet<String> =
                        self.editor_tabs.read(cx).tab_paths().into_iter().collect();
                    let mut paths: Vec<String> = entries
                        .filter_map(std::result::Result::ok)
                        .filter(|e| e.file_name().to_string_lossy().starts_with("Untitled-"))
                        .map(|e| e.path().to_string_lossy().to_string())
                        .filter(|p| !existing.contains(p))
                        .collect();
                    paths.sort();
                    for p in paths {
                        self.editor_tabs
                            .update(cx, |tabs, cx| tabs.open_tab(&p, cx));
                    }
                }
            }

            // Auto-hide editor if no tabs after restoration (sync ActivityBar toggle)
            if self.editor_tabs.read(cx).tab_count() == 0
                && self.app_state.read(cx).editor_visible(cx)
            {
                self.app_state.update(cx, AppState::toggle_editor);
            }

            self.last_focused_id.clone_from(&focused_id);
        }

        let state = self.app_state.read(cx);
        let theme = state.theme().clone();
        let sidebar_tab = state.sidebar_tab(cx);

        let Some(id) = focused_id.as_deref() else {
            return div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .text_color(theme.text_faint)
                .child("No instance focused");
        };

        let il = state.instance_list.read(cx);
        let entry = il.find_by_id(id, cx).cloned();
        let sidebar_width = state.sidebar_width(cx);
        let terminal_height = state.terminal_height(cx);

        let Some(entry) = entry else {
            return div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .text_color(theme.text_faint)
                .child("Instance not found");
        };

        let inst = entry.read(cx);
        let is_terminal = inst.instance_type() == InstanceType::Terminal;

        // Terminal instances: force-hide sidebar and editor (no project root / code)
        let sidebar_visible = !is_terminal && state.sidebar_visible(cx);
        let editor_visible = !is_terminal && state.editor_visible(cx);
        let terminal_visible = is_terminal || state.terminal_visible(cx);
        let terminal_view = inst.active_terminal_view().cloned();
        let click_focus_handle = inst.active_focus_handle().cloned();

        #[allow(clippy::cast_precision_loss)]
        let terminal_font_size = state.settings_store.read(cx).settings().terminal_font_size as f32;

        let font_family = state.settings_store.read(cx).settings().font_family.clone();

        let dragging_sidebar = self
            .drag
            .as_ref()
            .is_some_and(|d| d.target == DragTarget::Sidebar);
        let dragging_terminal = self
            .drag
            .as_ref()
            .is_some_and(|d| d.target == DragTarget::Terminal);

        let active_diff_mode = self
            .editor_tabs
            .read(cx)
            .active_tab()
            .and_then(|t| t.diff_mode);

        let tab_bar = self.build_tab_bar(&entry, &theme, cx);

        let main_area = render_main_area(
            editor_visible,
            terminal_visible,
            &self.editor_tabs,
            &self.code_editor,
            &self.diff_viewer,
            active_diff_mode,
            terminal_view.as_ref(),
            click_focus_handle.as_ref(),
            tab_bar,
            terminal_height,
            terminal_font_size,
            &font_family,
            dragging_terminal,
            cx.listener(|this, event: &gpui::MouseDownEvent, _window, _cx| {
                this.drag = Some(DragState {
                    target: DragTarget::Terminal,
                    axis: Axis::Vertical,
                    last_pos: f32::from(event.position.y),
                });
            }),
            &theme,
        );

        let mut root = div()
            .size_full()
            .flex()
            .flex_row()
            .overflow_hidden()
            .font_family(SharedString::from(font_family))
            .on_mouse_move(cx.listener(Self::on_drag_move))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::on_drag_end))
            .on_mouse_up_out(MouseButton::Left, cx.listener(Self::on_drag_end));

        if sidebar_visible {
            root = root
                .child(
                    div()
                        .w(px(sidebar_width))
                        .h_full()
                        .flex()
                        .flex_col()
                        .overflow_hidden()
                        .bg(theme.panel)
                        .child(match sidebar_tab {
                            SidebarTab::Files => self.file_tree.clone().into_any_element(),
                            SidebarTab::Git => self.git_panel.clone().into_any_element(),
                        }),
                )
                .child(render_divider(
                    Axis::Horizontal,
                    dragging_sidebar,
                    cx.listener(|this, event: &gpui::MouseDownEvent, _window, _cx| {
                        this.drag = Some(DragState {
                            target: DragTarget::Sidebar,
                            axis: Axis::Horizontal,
                            last_pos: f32::from(event.position.x),
                        });
                    }),
                ));
        }

        root.child(main_area)
    }
}

/// Resize the focused terminal PTY and view to match the available pixel area.
///
/// Reads layout parameters from `AppState`, computes cell metrics using the window's
/// text system, and applies the resulting cols/rows to both the PTY and the terminal view.
fn resize_focused_terminal(this: &AppState, window: &mut gpui::Window, cx: &mut gpui::App) {
    use crate::state::settings_store::ViewMode;
    use crate::terminal::compute_cell_metrics;

    if this.view_mode(cx) != ViewMode::Focus {
        return;
    }
    let Some(focused_id) = this.focused_instance_id(cx) else {
        return;
    };
    let il = this.instance_list.read(cx);
    let Some(entry) = il.find_by_id(focused_id, cx) else {
        return;
    };

    let is_terminal =
        entry.read(cx).instance_type() == conescope_core::instance::InstanceType::Terminal;
    let sidebar_visible = !is_terminal && this.sidebar_visible(cx);
    let editor_visible = !is_terminal && this.editor_visible(cx);
    let terminal_visible = is_terminal || this.terminal_visible(cx);

    // If only editor visible (no terminal), no PTY to resize
    if editor_visible && !terminal_visible {
        return;
    }

    let size = window.viewport_size();
    let mut content_height = f32::from(size.height) - TOP_BAR_HEIGHT - ACTIVITY_BAR_HEIGHT;
    let mut content_width = f32::from(size.width);

    if sidebar_visible {
        content_width -= this.sidebar_width(cx) + DIVIDER_SIZE;
    }

    content_height -= TERMINAL_TABS_HEIGHT;

    if editor_visible && terminal_visible {
        content_height = this.terminal_height(cx) - TERMINAL_TABS_HEIGHT - DIVIDER_SIZE;
    }

    let settings = this.settings_store.read(cx).settings().clone();
    #[allow(clippy::cast_precision_loss)]
    let term_font_size = settings.terminal_font_size as f32;
    let font_family = Some(settings.font_family.clone());
    let lhr = settings.terminal_line_height as f32;
    let Some((cell_width, cell_height)) =
        compute_cell_metrics(window, Some(term_font_size), font_family.as_deref(), lhr)
    else {
        return;
    };

    #[allow(clippy::cast_sign_loss)]
    let cols = (content_width / cell_width).floor().max(1.0) as u16;
    #[allow(clippy::cast_sign_loss)]
    let rows = (content_height / cell_height).floor().max(1.0) as u16;

    let inst = entry.read(cx);
    let tv = inst.terminal_view.clone();
    let shell_tvs: Vec<_> = inst
        .shell_tabs
        .iter()
        .map(|t| t.terminal_view.clone())
        .collect();
    inst.resize_pty(cols, rows);
    inst.resize_all_shell_ptys(cols, rows);
    if let Some(tv) = tv {
        tv.update(cx, |view, cx| view.resize_terminal(cols, rows, cx));
    }
    for stv in shell_tvs {
        stv.update(cx, |view, cx| view.resize_terminal(cols, rows, cx));
    }
}

/// Register observers that resize the focused instance's PTY.
///
/// Responds to:
/// 1. Window bounds changes (resize/move)
/// 2. Settings store changes (view mode switch, focused instance change, panel toggles)
///
/// Must be called once after creating the `AppView`, from within the window context.
pub fn register_focus_resize(
    app_state: &Entity<AppState>,
    window: &mut gpui::Window,
    cx: &mut gpui::App,
) -> gpui::Subscription {
    let app_state_observer = app_state.clone();
    let window_handle = window.window_handle();

    // Resize on app_state changes (view mode switch, focused instance, panel toggles).
    // Uses App::observe (no entity context) to avoid re-entrant borrows.
    cx.observe(app_state, move |entity, cx| {
        let _ = cx.update_window(window_handle, |_, window, cx| {
            entity.update(cx, |state, cx| {
                resize_focused_terminal(state, window, cx);
            });
        });
    })
    .detach();

    app_state_observer.update(cx, |_, cx| {
        cx.observe_window_bounds(window, |this, window, cx| {
            resize_focused_terminal(this, window, cx);
        })
    })
}
