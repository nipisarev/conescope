use gpui::prelude::*;
use gpui::{AppContext, Entity, div};

use crate::actions::{
    CloseSettings, CloseTab, FocusInstance1, FocusInstance2, FocusInstance3, FocusInstance4,
    FocusInstance5, FocusInstance6, FocusInstance7, FocusInstance8, FocusInstance9, NewInstance,
    OpenSettings, ReturnToOverview, SaveFile, ToggleEditor, ToggleGitPanel, ToggleSidebar,
    ToggleTerminal,
};
use crate::state::app_state::AppState;
use crate::state::settings_store::ViewMode;

use super::activity_bar::ActivityBar;
use super::confirm_modal::ConfirmModal;
use super::error_modal::ErrorModal;
use super::focus_view::FocusView;
use super::new_instance_modal::NewInstanceModal;
use super::overview_grid::OverviewGrid;
use super::questions_panel::QuestionsPanel;
use super::settings_view::SettingsView;
use super::top_bar::TopBar;

pub struct AppView {
    pub app_state: Entity<AppState>,
    pub top_bar: Entity<TopBar>,
    pub activity_bar: Entity<ActivityBar>,
    pub overview_grid: Entity<OverviewGrid>,
    pub focus_view: Entity<FocusView>,
    pub new_instance_modal: Entity<NewInstanceModal>,
    pub settings_view: Entity<SettingsView>,
    pub confirm_modal: Entity<ConfirmModal>,
    pub questions_panel: Entity<QuestionsPanel>,
    pub error_modal: Entity<ErrorModal>,
}

impl std::fmt::Debug for AppView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppView").finish_non_exhaustive()
    }
}

impl AppView {
    #[must_use]
    pub fn new(app_state: Entity<AppState>, cx: &mut gpui::Context<Self>) -> Self {
        let top_bar = cx.new(|_| TopBar::new(app_state.clone()));
        let activity_bar = cx.new(|_| ActivityBar::new(app_state.clone()));
        let overview_grid = cx.new(|_| OverviewGrid::new(app_state.clone()));
        let focus_view = cx.new(|cx| FocusView::new(app_state.clone(), cx));
        let new_instance_modal = cx.new(|_| NewInstanceModal::new(app_state.clone()));
        let settings_view = cx.new(|cx| SettingsView::new(app_state.clone(), cx));
        let confirm_modal = cx.new(|_| ConfirmModal::new(app_state.clone()));
        let questions_panel = cx.new(|_| QuestionsPanel::new(app_state.clone()));
        let error_modal = cx.new(|_| ErrorModal::new(app_state.clone()));

        // Observe settings store: propagate font/theme changes to all terminal views.
        let settings_store = app_state.read(cx).settings_store.clone();
        let app_state_for_font = app_state.clone();
        cx.observe(&settings_store, move |_this, store, cx| {
            let settings = store.read(cx).settings().clone();
            let font_family = settings.font_family.clone();
            let lhr = settings.terminal_line_height as f32;
            let colors = app_state_for_font.read(cx).theme().terminal_colors();
            let entries: Vec<_> = app_state_for_font
                .read(cx)
                .instance_list
                .read(cx)
                .entries()
                .to_vec();
            for entry in entries {
                entry.update(cx, |e, cx| {
                    e.update_font(&font_family, cx);
                    e.update_line_height(lhr, cx);
                    e.update_colors(&colors, cx);
                });
            }
        })
        .detach();

        Self {
            app_state,
            top_bar,
            activity_bar,
            overview_grid,
            focus_view,
            new_instance_modal,
            settings_view,
            confirm_modal,
            questions_panel,
            error_modal,
        }
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
    fn render(
        &mut self,
        _window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> impl IntoElement {
        let state = self.app_state.read(cx);
        let view_mode = state.view_mode(cx);
        let new_instance_modal_open = state.new_instance_modal_open;
        let confirm_open = state.confirm_action.is_some();
        let questions_open = state.questions_queue_open;
        let error_open = state.error_message.is_some();

        let theme = state.theme();
        let root = div()
            .size_full()
            .flex()
            .flex_col()
            .bg(theme.background)
            .text_color(theme.text);

        with_action_handlers(root, &self.app_state, &self.focus_view, &self.settings_view)
            // Top bar
            .child(self.top_bar.clone())
            // Main content area
            .child(match view_mode {
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
            })
            // Activity bar (bottom)
            .child(self.activity_bar.clone())
            // Modal overlays (conditionally rendered)
            .when(new_instance_modal_open, |el| {
                el.child(self.new_instance_modal.clone())
            })
            .when(confirm_open, |el| {
                el.child(self.confirm_modal.clone())
            })
            .when(questions_open, |el| {
                el.child(self.questions_panel.clone())
            })
            .when(error_open, |el| {
                el.child(self.error_modal.clone())
            })
    }
}
