use gpui::prelude::*;
use gpui::{AppContext, Entity, MouseButton, SharedString, div, px, relative, rgba};

use conescope_core::instance::{InstanceType, TerminalTab};

use crate::state::app_state::AppState;
use crate::terminal::spawn_terminal_pane;
use crate::views::code_viewer::CodeViewer;
use crate::views::editor_tabs::{EditorTabs, EditorTabsEvent};
use crate::views::file_tree::{FileTree, FileTreeEvent};
use crate::views::resizable_divider::{Axis, DragState, DragTarget, clamp_size, render_divider};
use crate::views::terminal_tabs::render_tab_bar;

const SIDEBAR_MIN: f32 = 120.0;
const SIDEBAR_MAX: f32 = 600.0;
const TERMINAL_MIN: f32 = 80.0;
const TERMINAL_MAX: f32 = 800.0;

/// Heights of fixed UI elements for PTY resize calculations.
const TOP_BAR_HEIGHT: f32 = 36.0;
const ACTIVITY_BAR_HEIGHT: f32 = 28.0;
const TERMINAL_TABS_HEIGHT: f32 = 28.0;
const DIVIDER_SIZE: f32 = 4.0;

pub struct FocusView {
    app_state: Entity<AppState>,
    drag: Option<DragState>,
    file_tree: Entity<FileTree>,
    code_viewer: Entity<CodeViewer>,
    editor_tabs: Entity<EditorTabs>,
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
    pub fn new(app_state: Entity<AppState>, cx: &mut gpui::Context<Self>) -> Self {
        let file_tree = cx.new(|_| FileTree::new(app_state.clone()));
        let code_viewer = cx.new(|_| CodeViewer::new());
        let editor_tabs = cx.new(|_| EditorTabs::new());

        // FileTree → open file in editor
        let cv = code_viewer.clone();
        let et = editor_tabs.clone();
        cx.subscribe(&file_tree, move |_this, _ft, event, cx| {
            let FileTreeEvent::OpenFile(path) = event;
            et.update(cx, |tabs, cx| tabs.open_tab(path, cx));
            cv.update(cx, |viewer, cx| viewer.open_file(path, cx));
        })
        .detach();

        // EditorTabs → select/close + persist tabs
        let cv2 = code_viewer.clone();
        let et2 = editor_tabs.clone();
        let app_state_tabs = app_state.clone();
        cx.subscribe(&editor_tabs, move |_this, _tabs, event, cx| {
            match event {
                EditorTabsEvent::SelectTab(_idx) => {
                    if let Some(path) = et2.read(cx).active_path().map(str::to_owned) {
                        cv2.update(cx, |v, cx| v.open_file(&path, cx));
                    }
                }
                EditorTabsEvent::CloseTab(_idx) => {
                    if let Some(path) = et2.read(cx).active_path().map(str::to_owned) {
                        cv2.update(cx, |v, cx| v.open_file(&path, cx));
                    } else {
                        cv2.update(cx, CodeViewer::close_file);
                    }
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
                code_viewer.update(cx, |v, cx| v.open_file(&path, cx));
            }
        }

        Self {
            app_state,
            drag: None,
            file_tree,
            code_viewer,
            editor_tabs,
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

    fn build_tab_bar(
        &self,
        entry: &gpui::Entity<crate::state::instance_entry::InstanceEntry>,
        cx: &mut gpui::Context<Self>,
    ) -> gpui::Div {
        let inst = entry.read(cx);
        let instance_type = inst.instance_type();
        let active_tab = inst.active_tab;
        let has_shell = inst.shell_terminal_view.is_some();

        let entry_for_primary = entry.clone();
        let entry_for_shell = entry.clone();
        let entry_for_add = entry.clone();
        let app_state_for_shell = self.app_state.clone();
        let app_state_for_add = self.app_state.clone();

        render_tab_bar(
            instance_type,
            active_tab,
            has_shell,
            cx.listener(move |_this, _event: &gpui::MouseDownEvent, window, cx| {
                entry_for_primary.update(cx, |e, cx| {
                    e.set_active_tab(TerminalTab::Primary, cx);
                });
                let fh = entry_for_primary.read(cx).focus_handle.clone();
                if let Some(fh) = fh {
                    fh.focus(window, cx);
                }
            }),
            cx.listener(move |_this, _event: &gpui::MouseDownEvent, window, cx| {
                let needs_spawn = entry_for_shell.read(cx).shell_terminal_view.is_none();
                if needs_spawn {
                    let cwd = {
                        let state = app_state_for_shell.read(cx);
                        let inst = entry_for_shell.read(cx);
                        inst.instance.project_id.as_ref().and_then(|pid| {
                            state
                                .project_store
                                .read(cx)
                                .get(pid)
                                .map(|p| p.path.clone())
                        })
                    };
                    let ff = app_state_for_shell
                        .read(cx)
                        .settings_store
                        .read(cx)
                        .settings()
                        .get("font_family")
                        .map(str::to_owned);
                    let pane = spawn_terminal_pane(cwd.as_deref(), ff.as_deref(), window, cx);
                    entry_for_shell.update(cx, |e, cx| {
                        e.attach_shell_terminal(pane);
                        e.start_shell_output_polling(cx);
                        e.set_active_tab(TerminalTab::Shell, cx);
                    });
                } else {
                    entry_for_shell.update(cx, |e, cx| {
                        e.set_active_tab(TerminalTab::Shell, cx);
                    });
                }
                let fh = entry_for_shell.read(cx).shell_focus_handle.clone();
                if let Some(fh) = fh {
                    fh.focus(window, cx);
                }
            }),
            // "+" button: spawn a new shell tab and switch to it
            cx.listener(move |_this, _event: &gpui::MouseDownEvent, window, cx| {
                let needs_spawn = entry_for_add.read(cx).shell_terminal_view.is_none();
                if needs_spawn {
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
                    let ff = app_state_for_add
                        .read(cx)
                        .settings_store
                        .read(cx)
                        .settings()
                        .get("font_family")
                        .map(str::to_owned);
                    let pane = spawn_terminal_pane(cwd.as_deref(), ff.as_deref(), window, cx);
                    entry_for_add.update(cx, |e, cx| {
                        e.attach_shell_terminal(pane);
                        e.start_shell_output_polling(cx);
                        e.set_active_tab(TerminalTab::Shell, cx);
                    });
                } else {
                    entry_for_add.update(cx, |e, cx| {
                        e.set_active_tab(TerminalTab::Shell, cx);
                    });
                }
                let fh = entry_for_add.read(cx).shell_focus_handle.clone();
                if let Some(fh) = fh {
                    fh.focus(window, cx);
                }
            }),
        )
    }
}

#[allow(clippy::too_many_arguments)]
fn render_terminal_pane(
    terminal_view: Option<&gpui::Entity<gpui_ghostty_terminal::view::TerminalView>>,
    focus_handle: Option<&gpui::FocusHandle>,
    tab_bar: gpui::Div,
    height: f32,
    fill_height: bool,
    font_size: f32,
    font_family: &str,
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
                    .font_family(SharedString::from(font_family.to_owned()))
                    .text_size(px(font_size))
                    .line_height(relative(1.0))
                    .child(tv.clone()),
            );
        if fill_height {
            base.flex_1()
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
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_color(rgba(0x5555_55ff))
                    .child("Terminal not attached"),
            );
        if fill_height {
            base.flex_1()
        } else {
            base.h(px(height))
        }
    }
}

fn render_editor_area(
    editor_tabs: &Entity<EditorTabs>,
    code_viewer: &Entity<CodeViewer>,
) -> gpui::Div {
    div()
        .flex_1()
        .min_h_0()
        .flex()
        .flex_col()
        .child(editor_tabs.clone())
        .child(
            div()
                .flex_1()
                .min_h_0()
                .overflow_hidden()
                .child(code_viewer.clone()),
        )
}

#[allow(clippy::too_many_arguments)]
fn render_main_area(
    editor_visible: bool,
    terminal_visible: bool,
    editor_tabs: &Entity<EditorTabs>,
    code_viewer: &Entity<CodeViewer>,
    terminal_view: Option<&gpui::Entity<gpui_ghostty_terminal::view::TerminalView>>,
    focus_handle: Option<&gpui::FocusHandle>,
    tab_bar: gpui::Div,
    terminal_height: f32,
    terminal_font_size: f32,
    font_family: &str,
    dragging_terminal: bool,
    drag_listener: impl Fn(&gpui::MouseDownEvent, &mut gpui::Window, &mut gpui::App) + 'static,
) -> gpui::Div {
    let mut col = div().flex_1().min_h_0().flex().flex_col().overflow_hidden();

    if editor_visible {
        col = col.child(render_editor_area(editor_tabs, code_viewer));
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
        ));
    }

    if !editor_visible && !terminal_visible {
        col = col.child(
            div()
                .flex_1()
                .flex()
                .items_center()
                .justify_center()
                .text_color(rgba(0x5555_55ff))
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
        let state = self.app_state.read(cx);
        let focused_id = state.focused_instance_id(cx);

        let Some(id) = focused_id else {
            return div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .text_color(rgba(0x6666_66ff))
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
                .text_color(rgba(0x6666_66ff))
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
        let terminal_font_size = state
            .settings_store
            .read(cx)
            .settings()
            .get_i64("terminal_font_size")
            .unwrap_or(13) as f32;

        let font_family = state
            .settings_store
            .read(cx)
            .settings()
            .get("font_family")
            .unwrap_or("Menlo")
            .to_owned();

        let dragging_sidebar = self
            .drag
            .as_ref()
            .is_some_and(|d| d.target == DragTarget::Sidebar);
        let dragging_terminal = self
            .drag
            .as_ref()
            .is_some_and(|d| d.target == DragTarget::Terminal);

        let tab_bar = self.build_tab_bar(&entry, cx);

        let main_area = render_main_area(
            editor_visible,
            terminal_visible,
            &self.editor_tabs,
            &self.code_viewer,
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
                        .border_r_1()
                        .border_color(rgba(0x3c3c_3cff))
                        .bg(rgba(0x2525_26ff))
                        .child(self.file_tree.clone()),
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

/// Register a window bounds observer that resizes the focused instance's PTY.
///
/// Must be called once after creating the `AppView`, from within the window context.
pub fn register_focus_resize(
    app_state: &Entity<AppState>,
    window: &mut gpui::Window,
    cx: &mut gpui::App,
) -> gpui::Subscription {
    use crate::state::settings_store::ViewMode;
    use crate::terminal::compute_cell_metrics;

    let app_state = app_state.clone();

    app_state.update(cx, |_, cx| {
        cx.observe_window_bounds(window, move |this, window, cx| {
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

            let size = window.viewport_size();
            // Start with full viewport, subtract top bar and activity bar
            let mut content_height = f32::from(size.height) - TOP_BAR_HEIGHT - ACTIVITY_BAR_HEIGHT;
            let mut content_width = f32::from(size.width);

            // Subtract sidebar width + divider if visible
            if this.sidebar_visible(cx) {
                content_width -= this.sidebar_width(cx) + DIVIDER_SIZE;
            }

            // Account for terminal tabs height
            content_height -= TERMINAL_TABS_HEIGHT;

            // If both editor and terminal visible, terminal gets fixed height
            if this.editor_visible(cx) && this.terminal_visible(cx) {
                content_height = this.terminal_height(cx) - TERMINAL_TABS_HEIGHT - DIVIDER_SIZE;
            }

            // If only editor visible (no terminal), no PTY to resize
            if this.editor_visible(cx) && !this.terminal_visible(cx) {
                return;
            }

            #[allow(clippy::cast_precision_loss)]
            let term_font_size = this
                .settings_store
                .read(cx)
                .settings()
                .get_i64("terminal_font_size")
                .unwrap_or(13) as f32;
            let font_family = this
                .settings_store
                .read(cx)
                .settings()
                .get("font_family")
                .map(str::to_owned);
            let Some((cell_width, cell_height)) =
                compute_cell_metrics(window, Some(term_font_size), font_family.as_deref())
            else {
                return;
            };

            #[allow(clippy::cast_sign_loss)]
            let cols = (content_width / cell_width).floor().max(1.0) as u16;
            #[allow(clippy::cast_sign_loss)]
            let rows = (content_height / cell_height).floor().max(1.0) as u16;

            let inst = entry.read(cx);
            let tv = inst.terminal_view.clone();
            let shell_tv = inst.shell_terminal_view.clone();
            inst.resize_pty(cols, rows);
            inst.resize_shell_pty(cols, rows);
            if let Some(tv) = tv {
                tv.update(cx, |view, cx| view.resize_terminal(cols, rows, cx));
            }
            if let Some(tv) = shell_tv {
                tv.update(cx, |view, cx| view.resize_terminal(cols, rows, cx));
            }
        })
    })
}
