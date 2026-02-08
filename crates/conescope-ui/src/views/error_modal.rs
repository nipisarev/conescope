use gpui::prelude::*;
use gpui::{Entity, MouseButton, div, px};

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
        let theme = state.theme().clone();

        let app_dismiss = self.app_state.clone();
        let app_backdrop = self.app_state.clone();

        let border_variant = theme.border_variant;
        let border = theme.border;

        div()
            .id("error-backdrop")
            .absolute()
            .size_full()
            .top_0()
            .left_0()
            .bg(theme.backdrop)
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
                    .bg(theme.surface)
                    .rounded(px(8.))
                    .border_1()
                    .border_color(theme.border_variant)
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
                            .border_color(theme.border)
                            .flex()
                            .flex_row()
                            .items_center()
                            .child(
                                div()
                                    .text_color(theme.destructive)
                                    .text_size(px(14.))
                                    .child("\u{26a0} Error"),
                            ),
                    )
                    // Message
                    .child(
                        div()
                            .px(px(16.))
                            .py(px(12.))
                            .text_color(theme.text)
                            .text_size(px(13.))
                            .child(message),
                    )
                    // Dismiss button
                    .child(
                        div()
                            .px(px(16.))
                            .py(px(12.))
                            .border_t_1()
                            .border_color(theme.border)
                            .flex()
                            .justify_end()
                            .child(
                                div()
                                    .id("error-dismiss")
                                    .px(px(12.))
                                    .py(px(6.))
                                    .rounded(px(4.))
                                    .cursor_pointer()
                                    .bg(border)
                                    .hover(move |s| s.bg(border_variant))
                                    .text_color(theme.text)
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
