use gpui::prelude::*;
use gpui::{AppContext, Entity, div, rgba};

use crate::actions::{
    CloseInstance, FocusInstance1, FocusInstance2, FocusInstance3, FocusInstance4, FocusInstance5,
    FocusInstance6, FocusInstance7, FocusInstance8, FocusInstance9, NewInstance, ReturnToOverview,
};
use crate::state::app_state::AppState;
use crate::state::settings_store::ViewMode;

use super::activity_bar::ActivityBar;
use super::focus_view::FocusView;
use super::new_instance_modal::NewInstanceModal;
use super::overview_grid::OverviewGrid;
use super::top_bar::TopBar;

pub struct AppView {
    pub app_state: Entity<AppState>,
    pub top_bar: Entity<TopBar>,
    pub activity_bar: Entity<ActivityBar>,
    pub overview_grid: Entity<OverviewGrid>,
    pub focus_view: Entity<FocusView>,
    pub new_instance_modal: Entity<NewInstanceModal>,
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
        let focus_view = cx.new(|_| FocusView::new(app_state.clone()));
        let new_instance_modal = cx.new(|_| NewInstanceModal::new(app_state.clone()));
        Self {
            app_state,
            top_bar,
            activity_bar,
            overview_grid,
            focus_view,
            new_instance_modal,
        }
    }
}

fn focus_instance_n(n: i64, app_state: &Entity<AppState>, cx: &mut gpui::App) {
    let id = {
        let state = app_state.read(cx);
        let il = state.instance_list.read(cx);
        il.find_by_number(n, cx)
            .map(|entry| entry.read(cx).id().to_owned())
    };
    if let Some(id) = id {
        app_state.update(cx, |s, cx| s.focus_instance(&id, cx));
    }
}

/// Make the root div stateful and chain all keyboard action handlers onto it.
fn with_action_handlers(
    root: gpui::Div,
    app_state: &Entity<AppState>,
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
            move |_: &CloseInstance, _window, cx| {
                let id = {
                    let state = app_state.read(cx);
                    if state.view_mode(cx) != ViewMode::Focus {
                        return;
                    }
                    let Some(id) = state.focused_instance_id(cx) else {
                        return;
                    };
                    id.to_owned()
                };
                let il = app_state.read(cx).instance_list.clone();
                app_state.update(cx, AppState::return_to_overview);
                il.update(cx, |list, cx| list.remove_instance(&id, cx));
            }
        });

    // FocusInstance1..9
    macro_rules! focus_action {
        ($root:expr, $action:ty, $n:expr, $app_state:expr) => {
            $root.on_action({
                let app_state = $app_state.clone();
                move |_: &$action, _window, cx| {
                    focus_instance_n($n, &app_state, cx);
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
        let modal_open = state.new_instance_modal_open;

        let root = div()
            .size_full()
            .flex()
            .flex_col()
            .bg(rgba(0x1e1e_1eff))
            .text_color(rgba(0xd4d4_d4ff));

        with_action_handlers(root, &self.app_state)
            // Top bar
            .child(self.top_bar.clone())
            // Main content area
            .child(match view_mode {
                ViewMode::Overview => div()
                    .flex_1()
                    .child(self.overview_grid.clone())
                    .into_any_element(),
                ViewMode::Focus => div()
                    .flex_1()
                    .child(self.focus_view.clone())
                    .into_any_element(),
            })
            // Activity bar (bottom)
            .child(self.activity_bar.clone())
            // Modal overlay (conditionally rendered)
            .when(modal_open, |el| {
                el.child(self.new_instance_modal.clone())
            })
    }
}
