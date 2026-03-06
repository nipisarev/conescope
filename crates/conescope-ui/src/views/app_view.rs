use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

use gpui::prelude::*;
use gpui::{
    Animation, AnimationExt, AnyWindowHandle, AppContext, ElementId, Entity, MouseButton,
    WindowKind, WindowOptions, div, ease_in_out, px, size,
};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

use crate::actions::{
    CloseSettings, CloseTab, FocusInstance1, FocusInstance2, FocusInstance3, FocusInstance4,
    FocusInstance5, FocusInstance6, FocusInstance7, FocusInstance8, FocusInstance9, NewInstance,
    NewTerminalTab, OpenSettings, ReturnToOverview, SaveFile, ToggleEditor, ToggleGitPanel,
    ToggleOverviewSidebar, ToggleSidebar, ToggleTerminal,
};
use crate::state::app_state::AppState;
use crate::state::settings_store::ViewMode;
use crate::views::overlay_sidebar::OverlaySidebarView;
use crate::views::resizable_divider::{Axis, DragState, DragTarget, clamp_size};
use crate::views::sidebar::SIDEBAR_WIDTH;

use super::activity_bar::ActivityBar;
use super::confirm_modal::ConfirmModal;
use super::error_modal::ErrorModal;
use super::focus_view::FocusView;
use super::new_instance_modal::NewInstanceModal;
use super::overview_grid::OverviewGrid;
use super::settings_view::SettingsView;
use super::sidebar::Sidebar;
use super::top_bar::TopBar;

const SIDEBAR_MAX: f32 = 600.0;
const OVERLAY_SIDEBAR_PADDING: f32 = 0.0;

pub struct AppView {
    pub app_state: Entity<AppState>,
    pub top_bar: Entity<TopBar>,
    pub activity_bar: Entity<ActivityBar>,
    pub sidebar: Entity<Sidebar>,
    pub overview_grid: Entity<OverviewGrid>,
    pub focus_view: Entity<FocusView>,
    pub new_instance_modal: Entity<NewInstanceModal>,
    pub settings_view: Entity<SettingsView>,
    pub confirm_modal: Entity<ConfirmModal>,
    pub error_modal: Entity<ErrorModal>,
    /// Drag state for resizable pinned sidebar.
    sidebar_drag: Option<DragState>,
    /// Generation counter for sidebar slide animation (bumped on each toggle).
    sidebar_anim_gen: usize,
    /// Whether sidebar was open on previous render (to detect transitions).
    sidebar_was_open: bool,
    /// Whether overlay sidebar was visible on previous render (to detect show transition).
    sidebar_overlay_was_visible: bool,
    /// Child window handle for the overlay sidebar (frosted glass popup).
    overlay_window: Option<AnyWindowHandle>,
    overlay_needs_attach: bool,
    overlay_parent_handle: Option<RawWindowHandle>,
    /// Main window handle (for re-activation after overlay close).
    main_window_handle: Option<AnyWindowHandle>,
    /// Cached parent window origin (screen coords) for overlay initial placement.
    parent_window_origin: Option<gpui::Point<gpui::Pixels>>,
    parent_viewport_size: Option<gpui::Size<gpui::Pixels>>,
    /// Generation counter for overlay animations — cancels stale open/close tasks.
    overlay_anim_gen: Arc<AtomicU32>,
}

impl std::fmt::Debug for AppView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppView").finish_non_exhaustive()
    }
}

impl AppView {
    #[must_use]
    pub fn new(app_state: Entity<AppState>, cx: &mut gpui::Context<Self>) -> Self {
        let top_bar = cx.new(|cx| TopBar::new(app_state.clone(), cx));
        let activity_bar = cx.new(|cx| ActivityBar::new(app_state.clone(), cx));
        let sidebar = cx.new(|_| Sidebar::new(app_state.clone()));
        let overview_grid = cx.new(|cx| OverviewGrid::new(app_state.clone(), cx));
        let focus_view = cx.new(|cx| FocusView::new(app_state.clone(), cx));
        let new_instance_modal = cx.new(|_| NewInstanceModal::new(app_state.clone()));
        let settings_view = cx.new(|cx| SettingsView::new(app_state.clone(), cx));
        let confirm_modal = cx.new(|_| ConfirmModal::new(app_state.clone()));
        let error_modal = cx.new(|_| ErrorModal::new(app_state.clone()));

        // Observe settings store: propagate font/theme changes to all terminal views.
        let settings_store = app_state.read(cx).settings_store.clone();
        let app_state_for_font = app_state.clone();
        cx.observe(&settings_store, move |_this, store, cx| {
            let settings = store.read(cx).settings().clone();
            let font_family = settings.font_family.clone();
            #[allow(clippy::cast_precision_loss)]
            let font_size = settings.terminal_font_size as f32;
            let lhr = settings.terminal_line_height as f32;
            let theme = app_state_for_font.read(cx).theme();
            let colors = theme.terminal_colors();
            let menu_colors = crate::terminal::MenuColors {
                surface: theme.surface,
                border: theme.border,
                hover: theme.element_hover,
                text: theme.text,
                text_faint: theme.text_muted,
            };
            let entries: Vec<_> = app_state_for_font
                .read(cx)
                .instance_list
                .read(cx)
                .entries()
                .to_vec();
            for entry in entries {
                entry.update(cx, |e, cx| {
                    e.update_font(&font_family, cx);
                    e.update_font_size(font_size, cx);
                    e.update_line_height(lhr, cx);
                    e.update_colors(&colors, menu_colors, cx);
                });
            }
        })
        .detach();

        cx.observe(&app_state, |this: &mut Self, _app_state, cx| {
            let sidebar_overlay_visible = this.app_state.read(cx).sidebar_overlay_visible;

            if sidebar_overlay_visible && !this.sidebar_overlay_was_visible {
                this.open_overlay_window_deferred(cx);
            } else if !sidebar_overlay_visible && this.sidebar_overlay_was_visible {
                this.close_overlay_window_deferred(cx);
            }
            this.sidebar_overlay_was_visible = sidebar_overlay_visible;
        })
        .detach();

        Self {
            app_state,
            top_bar,
            activity_bar,
            sidebar,
            overview_grid,
            focus_view,
            new_instance_modal,
            settings_view,
            confirm_modal,
            error_modal,
            sidebar_drag: None,
            sidebar_anim_gen: 0,
            sidebar_was_open: false,
            sidebar_overlay_was_visible: false,
            overlay_window: None,
            overlay_needs_attach: false,
            overlay_parent_handle: None,
            main_window_handle: None,
            parent_window_origin: None,
            parent_viewport_size: None,
            overlay_anim_gen: Arc::new(AtomicU32::new(0)),
        }
    }

    fn open_overlay_window_deferred(&mut self, cx: &mut gpui::Context<Self>) {
        if self.overlay_window.is_some() {
            return;
        }

        let app_state = self.app_state.clone();
        let overlay_width = SIDEBAR_WIDTH + OVERLAY_SIDEBAR_PADDING;
        let window_bounds = self.parent_window_origin.map(|origin| {
            let height = self.parent_viewport_size.map_or(px(800.0), |s| s.height);
            gpui::WindowBounds::Windowed(gpui::Bounds {
                origin,
                size: size(px(overlay_width), height),
            })
        });
        let overlay_handle = cx.open_window(
            WindowOptions {
                window_bounds,
                titlebar: None,
                focus: true,
                show: true,
                kind: WindowKind::PopUp,
                is_movable: false,
                is_resizable: false,
                is_minimizable: false,
                window_background: gpui::WindowBackgroundAppearance::Blurred,
                ..Default::default()
            },
            |window, cx| {
                let view = cx.new(|cx| OverlaySidebarView::new(app_state, cx));
                cx.new(|cx| gpui_component::Root::new(view, window, cx))
            },
        );

        if let Ok(handle) = overlay_handle {
            let handle: AnyWindowHandle = handle.into();
            self.overlay_window = Some(handle);
            self.overlay_needs_attach = true;
            // Activate overlay so macOS delivers mouse events to it
            let _ = cx.update_window(handle, |_, child_window, _| {
                child_window.activate_window();
            });
        }
    }

    fn close_overlay_window_deferred(&mut self, cx: &mut gpui::Context<Self>) {
        let Some(child_handle) = self.overlay_window.take() else {
            return;
        };
        self.overlay_anim_gen.fetch_add(1, Ordering::Relaxed);
        self.overlay_needs_attach = false;
        let parent_raw = self.overlay_parent_handle.take();
        let main_window = self.main_window_handle;

        // Detach from parent and remove
        if let Some(parent_raw) = parent_raw {
            let _ = cx.update_window(child_handle, |_, child_window, _| {
                if let Ok(child_wh) = HasWindowHandle::window_handle(child_window) {
                    conescope_platform::remove_child_window(parent_raw, child_wh.as_raw());
                }
            });
        }
        let _ = cx.update_window(child_handle, |_, child_window, _| {
            child_window.remove_window();
        });
        // Re-activate main window
        if let Some(main_win) = main_window {
            let _ = cx.update_window(main_win, |_, window, _| {
                window.activate_window();
            });
        }
    }

    fn sync_overlay_window_bounds(&mut self, window: &gpui::Window, cx: &mut gpui::Context<Self>) {
        let Some(child_handle) = self.overlay_window else {
            return;
        };
        let main_size = window.viewport_size();
        let overlay_width = SIDEBAR_WIDTH + OVERLAY_SIDEBAR_PADDING;

        if self.overlay_needs_attach {
            self.overlay_needs_attach = false;

            let _ = cx.update_window(child_handle, |_, child_window, _| {
                child_window.resize(size(px(overlay_width), main_size.height));
            });

            if let Ok(parent_wh) = HasWindowHandle::window_handle(window) {
                let parent_raw = parent_wh.as_raw();
                let _ = cx.update_window(child_handle, |_, child_window, _| {
                    if let Ok(child_wh) = HasWindowHandle::window_handle(child_window) {
                        conescope_platform::add_child_window(parent_raw, child_wh.as_raw());
                        conescope_platform::configure_overlay_panel(child_wh.as_raw());
                    }
                });
                self.overlay_parent_handle = Some(parent_raw);
            }
        } else {
            let _ = cx.update_window(child_handle, |_, child_window, _| {
                child_window.resize(size(px(overlay_width), main_size.height));
            });
        }
    }

    /// Start a delayed hover-open of the overlay sidebar (250ms).
    fn start_hover_open(&self, cx: &mut gpui::Context<Self>) {
        let app_state = self.app_state.clone();
        let hover_gen = app_state.update(cx, |s, _| s.bump_sidebar_hover_gen());
        cx.spawn(async move |_this, cx| {
            cx.background_executor()
                .timer(Duration::from_millis(250))
                .await;
            cx.update(|cx| {
                app_state.update(cx, |s, cx| {
                    if s.sidebar_hover_gen == hover_gen && !s.sidebar_overlay_visible {
                        s.show_sidebar_overlay(cx);
                    }
                });
            });
        })
        .detach();
    }

    /// Start a delayed auto-hide of the overlay sidebar (300ms).
    fn start_auto_hide(&self, cx: &mut gpui::Context<Self>) {
        let app_state = self.app_state.clone();
        let hover_gen = app_state.update(cx, |s, _| s.bump_sidebar_hover_gen());
        cx.spawn(async move |_this, cx| {
            cx.background_executor()
                .timer(Duration::from_millis(300))
                .await;
            cx.update(|cx| {
                app_state.update(cx, |s, cx| {
                    if s.sidebar_hover_gen == hover_gen && s.sidebar_overlay_visible {
                        s.hide_sidebar_overlay(cx);
                    }
                });
            });
        })
        .detach();
    }

    /// Cancel any pending hover/hide timer (mouse re-entered sidebar).
    #[allow(dead_code)]
    fn cancel_sidebar_timer(&self, cx: &mut gpui::Context<Self>) {
        self.app_state.update(cx, |s, _| {
            s.bump_sidebar_hover_gen();
        });
    }

    fn on_sidebar_drag_move(
        &mut self,
        event: &gpui::MouseMoveEvent,
        _window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) {
        let Some(ref mut drag) = self.sidebar_drag else {
            return;
        };
        let current = drag.position_component(event.position);
        let delta = drag.delta(current);
        if delta.abs() < 0.5 {
            return;
        }
        let app_state = self.app_state.clone();
        let width = app_state.read(cx).sidebar_width(cx);
        let new_width = clamp_size(width + delta, SIDEBAR_WIDTH, SIDEBAR_MAX);
        app_state.update(cx, |s, cx| s.set_sidebar_width(new_width, cx));
    }

    fn on_sidebar_drag_end(
        &mut self,
        _event: &gpui::MouseUpEvent,
        _window: &mut gpui::Window,
        _cx: &mut gpui::Context<Self>,
    ) {
        self.sidebar_drag = None;
    }
}

fn focus_instance_n(
    n: usize,
    app_state: &Entity<AppState>,
    window: &mut gpui::Window,
    cx: &mut gpui::App,
) {
    let id = {
        let state = app_state.read(cx);
        let il = state.instance_list.read(cx);
        il.find_by_index(n.wrapping_sub(1))
            .map(|entry| entry.read(cx).id().to_owned())
    };
    if let Some(id) = id {
        app_state.update(cx, |s, cx| s.focus_instance(&id, cx));
        // Focus the terminal so keyboard input goes to the PTY
        let fh = {
            let state = app_state.read(cx);
            let il = state.instance_list.read(cx);
            il.find_by_id(&id, cx)
                .and_then(|entry| entry.read(cx).focus_handle.clone())
        };
        if let Some(fh) = fh {
            fh.focus(window, cx);
        }
    }
}

/// Make the root div stateful and chain all keyboard action handlers onto it.
#[allow(clippy::too_many_lines)]
fn with_action_handlers(
    root: gpui::Div,
    app_state: &Entity<AppState>,
    focus_view: &Entity<FocusView>,
    settings_view: &Entity<SettingsView>,
) -> gpui::Stateful<gpui::Div> {
    let root = root
        .id("app-root")
        .key_context("AppView")
        .on_action({
            let app_state = app_state.clone();
            move |_: &NewInstance, _window, cx| {
                app_state.update(cx, AppState::toggle_new_instance_modal);
            }
        })
        .on_action({
            let app_state = app_state.clone();
            move |_: &ReturnToOverview, _window, cx| {
                app_state.update(cx, AppState::return_to_overview);
            }
        })
        .on_action({
            let app_state = app_state.clone();
            let focus_view = focus_view.clone();
            move |_: &CloseTab, window, cx| {
                if app_state.read(cx).view_mode(cx) != ViewMode::Focus {
                    return;
                }
                focus_view.update(cx, |fv, cx| fv.close_active_tab(window, cx));
            }
        })
        .on_action({
            let focus_view = focus_view.clone();
            move |_: &NewTerminalTab, window, cx| {
                focus_view.update(cx, |fv, cx| fv.add_new_terminal_tab(window, cx));
            }
        })
        .on_action({
            let app_state = app_state.clone();
            let focus_view = focus_view.clone();
            move |_: &SaveFile, _window, cx| {
                if app_state.read(cx).view_mode(cx) != ViewMode::Focus {
                    return;
                }
                focus_view.update(cx, |fv, cx| fv.save_active_file(cx));
            }
        });

    let root = root
        .on_action({
            let app_state = app_state.clone();
            move |_: &ToggleSidebar, _window, cx| {
                app_state.update(cx, AppState::toggle_sidebar);
            }
        })
        .on_action({
            let app_state = app_state.clone();
            move |_: &ToggleEditor, _window, cx| {
                app_state.update(cx, AppState::toggle_editor);
            }
        })
        .on_action({
            let app_state = app_state.clone();
            move |_: &ToggleTerminal, _window, cx| {
                app_state.update(cx, AppState::toggle_terminal);
            }
        })
        .on_action({
            let app_state = app_state.clone();
            move |_: &ToggleGitPanel, _window, cx| {
                app_state.update(cx, AppState::toggle_git_panel);
            }
        })
        .on_action({
            let app_state = app_state.clone();
            move |_: &ToggleOverviewSidebar, _window, cx| {
                app_state.update(cx, AppState::toggle_sidebar_visibility);
            }
        })
        .on_action({
            let app_state = app_state.clone();
            let settings_view = settings_view.clone();
            move |_: &OpenSettings, _window, cx| {
                let view_mode = app_state.read(cx).view_mode(cx);
                if view_mode == ViewMode::Settings {
                    app_state.update(cx, AppState::close_settings);
                } else {
                    app_state.update(cx, AppState::open_settings);
                    settings_view.update(cx, SettingsView::reload_settings);
                }
            }
        })
        .on_action({
            let app_state = app_state.clone();
            move |_: &CloseSettings, _window, cx| {
                if app_state.read(cx).view_mode(cx) != ViewMode::Settings {
                    return;
                }
                app_state.update(cx, AppState::close_settings);
            }
        });

    // FocusInstance1..9
    macro_rules! focus_action {
        ($root:expr, $action:ty, $n:expr, $app_state:expr) => {
            $root.on_action({
                let app_state = $app_state.clone();
                move |_: &$action, window, cx| {
                    focus_instance_n($n, &app_state, window, cx);
                }
            })
        };
    }

    let root = focus_action!(root, FocusInstance1, 1, app_state);
    let root = focus_action!(root, FocusInstance2, 2, app_state);
    let root = focus_action!(root, FocusInstance3, 3, app_state);
    let root = focus_action!(root, FocusInstance4, 4, app_state);
    let root = focus_action!(root, FocusInstance5, 5, app_state);
    let root = focus_action!(root, FocusInstance6, 6, app_state);
    let root = focus_action!(root, FocusInstance7, 7, app_state);
    let root = focus_action!(root, FocusInstance8, 8, app_state);
    focus_action!(root, FocusInstance9, 9, app_state)
}

impl Render for AppView {
    #[allow(clippy::too_many_lines)]
    fn render(
        &mut self,
        window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> impl IntoElement {
        // Extract all state data before any mutable borrows
        let (
            view_mode,
            new_instance_modal_open,
            confirm_open,
            error_open,
            sidebar_open,
            sidebar_overlay_visible,
            pinned_sidebar_width,
            theme,
        ) = {
            let state = self.app_state.read(cx);
            (
                state.view_mode(cx),
                state.new_instance_modal_open,
                state.confirm_action.is_some(),
                state.error_message.is_some(),
                state.sidebar_open(cx),
                state.sidebar_overlay_visible,
                state.sidebar_width(cx).max(SIDEBAR_WIDTH),
                state.theme().clone(),
            )
        };
        self.parent_window_origin = Some(window.bounds().origin);
        self.parent_viewport_size = Some(window.viewport_size());
        self.main_window_handle = Some(gpui::Window::window_handle(window));

        if sidebar_overlay_visible {
            self.sync_overlay_window_bounds(window, cx);
        }

        // Root: relative container for absolute overlay positioning
        let root = div()
            .size_full()
            .relative()
            .bg(theme.sidebar_glass_bg)
            .text_color(theme.text);

        let root =
            with_action_handlers(root, &self.app_state, &self.focus_view, &self.settings_view);

        // Content card: TopBar + view + ActivityBar
        let content_view = match view_mode {
            ViewMode::Overview => div()
                .key_context("Overview")
                .flex_1()
                .min_h_0()
                .child(self.overview_grid.clone())
                .into_any_element(),
            ViewMode::Focus => div()
                .flex_1()
                .min_h_0()
                .child(self.focus_view.clone())
                .into_any_element(),
            ViewMode::Settings => div()
                .flex_1()
                .min_h_0()
                .child(self.settings_view.clone())
                .into_any_element(),
        };

        // Content card: rounded, opaque, floating on base layer with margin on all sides
        let mut content_card = div()
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .m(px(12.))
            .rounded(px(10.))
            .bg(theme.background)
            .overflow_hidden()
            .shadow_lg()
            .child(self.top_bar.clone())
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .flex_row()
                    .child(content_view),
            )
            .child(self.activity_bar.clone());

        // Drag handlers for resizable pinned sidebar
        if sidebar_open {
            content_card = content_card
                .on_mouse_move(cx.listener(Self::on_sidebar_drag_move))
                .on_mouse_up(MouseButton::Left, cx.listener(Self::on_sidebar_drag_end))
                .on_mouse_up_out(MouseButton::Left, cx.listener(Self::on_sidebar_drag_end));
        }

        // Detect sidebar open/close transitions and bump animation gen
        if sidebar_open != self.sidebar_was_open {
            self.sidebar_anim_gen = self.sidebar_anim_gen.wrapping_add(1);
            self.sidebar_was_open = sidebar_open;
        }

        // Sidebar: always rendered, animated wrapper controls visible width
        let sidebar_content = self.sidebar.update(cx, |s, cx| {
            s.render_with_width(pinned_sidebar_width, false, cx)
                .into_any_element()
        });

        let target_w = if sidebar_open {
            pinned_sidebar_width
        } else {
            0.0
        };
        let start_w = if sidebar_open {
            0.0
        } else {
            pinned_sidebar_width
        };
        let anim_gen = self.sidebar_anim_gen;

        let sidebar_wrapper = div()
            .h_full()
            .overflow_hidden()
            .flex_shrink_0()
            .child(sidebar_content)
            .with_animation(
                ElementId::Name(format!("sidebar-slide-{anim_gen}").into()),
                Animation::new(Duration::from_millis(400)).with_easing(ease_in_out),
                move |el, delta| {
                    let w = start_w + (target_w - start_w) * delta;
                    el.w(px(w))
                },
            );

        // Drag handle between sidebar and content card (only when sidebar is open)
        let drag_handle = div()
            .w(px(6.))
            .h_full()
            .flex_shrink_0()
            .when(sidebar_open, |el| {
                el.cursor(gpui::CursorStyle::ResizeLeftRight).on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|this, event: &gpui::MouseDownEvent, _window, _cx| {
                        this.sidebar_drag = Some(DragState {
                            target: DragTarget::Sidebar,
                            axis: Axis::Horizontal,
                            last_pos: f32::from(event.position.x),
                        });
                    }),
                )
            });

        // Outer: animated sidebar + drag handle + content card
        let outer = div()
            .size_full()
            .flex()
            .flex_row()
            .child(sidebar_wrapper)
            .when(sidebar_open, |el| el.child(drag_handle))
            .child(content_card);

        root
            // Base + content layers
            .child(outer)
            // Auto-hide overlay when cursor returns to main window (delayed)
            .when(sidebar_overlay_visible, |el| {
                el.on_mouse_move(cx.listener(|this, event: &gpui::MouseMoveEvent, _window, cx| {
                    if f32::from(event.position.x) > 20.0 {
                        this.start_auto_hide(cx);
                    }
                }))
            })
            // Hover trigger strip (left edge, 12px wide)
            .when(!sidebar_open && !sidebar_overlay_visible, |el| {
                el.child(
                    div()
                        .absolute()
                        .top_0()
                        .left_0()
                        .w(px(12.))
                        .h_full()
                        .on_mouse_move(cx.listener(|this, _, _, cx| {
                            this.start_hover_open(cx);
                        })),
                )
            })
            // Modal overlays (highest z_index)
            .when(new_instance_modal_open, |el| {
                el.child(self.new_instance_modal.clone())
            })
            .when(confirm_open, |el| {
                el.child(self.confirm_modal.clone())
            })
            .when(error_open, |el| {
                el.child(self.error_modal.clone())
            })
    }
}
