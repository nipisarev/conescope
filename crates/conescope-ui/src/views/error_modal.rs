use gpui::prelude::*;
use gpui::{Entity, MouseButton, div, px, rgba};

use crate::state::app_state::AppState;

#[derive(Debug)]
pub struct ErrorModal {
    app_state: Entity<AppState>,
}

impl ErrorModal {
    #[must_use]
    pub fn new(app_state: Entity<AppState>) -> Self {
        Self { app_state }
    }
}

impl Render for ErrorModal {
    fn render(
        &mut self,
        _window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> impl IntoElement {
        let state = self.app_state.read(cx);
        let Some(message) = state.error_message.clone() else {
            return div().into_any_element();
        };

        let app_dismiss = self.app_state.clone();
        let app_backdrop = self.app_state.clone();

        div()
            .id("error-backdrop")
            .absolute()
            .size_full()
            .top_0()
            .left_0()
            .bg(rgba(0x0000_0080))
            .flex()
            .items_center()
            .justify_center()
            .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                app_backdrop.update(cx, AppState::dismiss_error);
            })
            .child(
                div()
                    .w(px(400.))
                    .max_h(px(400.))
                    .bg(rgba(0x2d2d_2dff))
                    .rounded(px(8.))
                    .border_1()
                    .border_color(rgba(0x4c4c_4cff))
                    .flex()
                    .flex_col()
                    .overflow_hidden()
                    .on_mouse_down(MouseButton::Left, |_, _, cx| {
                        cx.stop_propagation();
                    })
                    // Header
                    .child(
                        div()
                            .px(px(16.))
                            .py(px(12.))
                            .border_b_1()
                            .border_color(rgba(0x3c3c_3cff))
                            .flex()
                            .flex_row()
                            .items_center()
                            .child(
                                div()
                                    .text_color(rgba(0xcc44_44ff))
                                    .text_size(px(14.))
                                    .child("\u{26a0} Error"),
                            ),
                    )
                    // Message
                    .child(
                        div()
                            .px(px(16.))
                            .py(px(12.))
                            .text_color(rgba(0xcccc_ccff))
                            .text_size(px(13.))
                            .child(message),
                    )
                    // Dismiss button
                    .child(
                        div()
                            .px(px(16.))
                            .py(px(12.))
                            .border_t_1()
                            .border_color(rgba(0x3c3c_3cff))
                            .flex()
                            .justify_end()
                            .child(
                                div()
                                    .id("error-dismiss")
                                    .px(px(12.))
                                    .py(px(6.))
                                    .rounded(px(4.))
                                    .cursor_pointer()
                                    .bg(rgba(0x3c3c_3cff))
                                    .hover(|s| s.bg(rgba(0x4c4c_4cff)))
                                    .text_color(rgba(0xcccc_ccff))
                                    .text_size(px(13.))
                                    .child("Dismiss")
                                    .on_click(move |_, _, cx| {
                                        app_dismiss
                                            .update(cx, AppState::dismiss_error);
                                    }),
                            ),
                    ),
            )
            .into_any_element()
    }
}
