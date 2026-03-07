use gpui::prelude::*;
use gpui::{Entity, div, px};

use crate::state::app_state::AppState;
use crate::views::sidebar::{SIDEBAR_WIDTH, Sidebar};

#[derive(Debug)]
pub struct OverlaySidebarView {
    app_state: Entity<AppState>,
    sidebar: Entity<Sidebar>,
}

impl OverlaySidebarView {
    #[must_use]
    pub fn new(app_state: Entity<AppState>, cx: &mut gpui::Context<Self>) -> Self {
        let sidebar = cx.new(|_| Sidebar::new(app_state.clone()));
        Self { app_state, sidebar }
    }
}

impl Render for OverlaySidebarView {
    fn render(
        &mut self,
        _window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> impl IntoElement {
        let theme = self.app_state.read(cx).theme().clone();

        let sidebar_content = self.sidebar.update(cx, |s, cx| {
            s.render_with_width(SIDEBAR_WIDTH, true, cx)
                .into_any_element()
        });

        div()
            .size_full()
            .overflow_hidden()
            .rounded(px(10.))
            .text_color(theme.text)
            .child(div().h_full().w(px(SIDEBAR_WIDTH)).child(sidebar_content))
    }
}
