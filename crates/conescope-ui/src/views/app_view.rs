use gpui::prelude::*;
use gpui::{AppContext, Entity, div, rgba};

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

impl Render for AppView {
    fn render(
        &mut self,
        _window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> impl IntoElement {
        let state = self.app_state.read(cx);
        let view_mode = state.view_mode(cx);
        let modal_open = state.new_instance_modal_open;

        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(rgba(0x1e1e_1eff))
            .text_color(rgba(0xd4d4_d4ff))
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
