use gpui::prelude::*;
use gpui::{Entity, MouseButton, div, px, rgba};

use crate::state::app_state::AppState;
use crate::state::settings_store::ViewMode;

#[derive(Debug)]
pub struct TopBar {
    app_state: Entity<AppState>,
}

impl TopBar {
    #[must_use]
    pub fn new(app_state: Entity<AppState>) -> Self {
        Self { app_state }
    }
}

impl Render for TopBar {
    fn render(
        &mut self,
        _window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> impl IntoElement {
        let state = self.app_state.read(cx);
        let view_mode = state.view_mode(cx);

        let center_text = match view_mode {
            ViewMode::Overview => "CONESCOPE".to_string(),
            ViewMode::Focus => {
                if let Some(id) = state.focused_instance_id(cx) {
                    let il = state.instance_list.read(cx);
                    if let Some(e) = il.find_by_id(id, cx) {
                        let inst = e.read(cx);
                        let num = inst.instance.instance_number.unwrap_or(0);
                        let title = inst.instance.title.as_deref().unwrap_or("Untitled");
                        format!("#{num} {title}")
                    } else {
                        "CONESCOPE".into()
                    }
                } else {
                    "CONESCOPE".into()
                }
            }
        };

        let app_state_for_add = self.app_state.clone();
        let app_state_for_back = self.app_state.clone();

        div()
            .h(px(36.))
            .w_full()
            .flex()
            .flex_row()
            .items_center()
            .bg(rgba(0x2525_26ff))
            .border_b_1()
            .border_color(rgba(0x3c3c_3cff))
            // Left padding for macOS traffic lights
            .child(div().w(px(76.)))
            // Center title
            .child(
                div()
                    .flex_1()
                    .flex()
                    .justify_center()
                    .text_color(rgba(0x8888_88ff))
                    .child(center_text),
            )
            // Right section
            .child(
                div()
                    .flex()
                    .flex_row()
                    .gap(px(8.))
                    .pr(px(12.))
                    // Back button (Focus mode only)
                    .when(view_mode == ViewMode::Focus, |el| {
                        el.child(
                            div()
                                .px(px(8.))
                                .py(px(4.))
                                .rounded(px(4.))
                                .cursor_pointer()
                                .text_color(rgba(0xcccc_ccff))
                                .hover(|s| s.bg(rgba(0x4444_44ff)))
                                .child("\u{2190}")
                                .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                                    app_state_for_back
                                        .update(cx, AppState::return_to_overview);
                                }),
                        )
                    })
                    // Add button
                    .child(
                        div()
                            .px(px(8.))
                            .py(px(4.))
                            .rounded(px(4.))
                            .cursor_pointer()
                            .text_color(rgba(0xcccc_ccff))
                            .hover(|s| s.bg(rgba(0x4444_44ff)))
                            .child("+")
                            .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                                app_state_for_add
                                    .update(cx, AppState::toggle_new_instance_modal);
                            }),
                    ),
            )
    }
}
