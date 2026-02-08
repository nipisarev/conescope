use gpui::prelude::*;
use gpui::{Entity, MouseButton, div, px};

use crate::state::app_state::AppState;

#[derive(Debug)]
pub struct ConfirmModal {
    app_state: Entity<AppState>,
}

impl ConfirmModal {
    #[must_use]
    pub fn new(app_state: Entity<AppState>) -> Self {
        Self { app_state }
    }
}

impl Render for ConfirmModal {
    fn render(
        &mut self,
        _window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> impl IntoElement {
        let state = self.app_state.read(cx);
        let Some(action) = state.confirm_action.clone() else {
            return div().into_any_element();
        };
        let theme = state.theme().clone();

        let app_cancel = self.app_state.clone();
        let app_confirm = self.app_state.clone();
        let app_backdrop = self.app_state.clone();

        let border = theme.border;
        let border_variant = theme.border_variant;
        let text = theme.text;
        let destructive_hover = theme.destructive_hover;

        // Backdrop
        div()
            .id("confirm-backdrop")
            .absolute()
            .size_full()
            .top_0()
            .left_0()
            .bg(theme.backdrop)
            .flex()
            .items_center()
            .justify_center()
            .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                app_backdrop.update(cx, AppState::cancel_confirm);
            })
            .child(
                div()
                    .w(px(340.))
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
                            .text_color(theme.text)
                            .child(action.title),
                    )
                    // Message
                    .child(
                        div()
                            .px(px(16.))
                            .py(px(12.))
                            .text_color(theme.text_muted)
                            .text_size(px(13.))
                            .child(action.message),
                    )
                    // Buttons
                    .child(
                        div()
                            .px(px(16.))
                            .py(px(12.))
                            .border_t_1()
                            .border_color(theme.border)
                            .flex()
                            .flex_row()
                            .justify_end()
                            .gap(px(8.))
                            .child(
                                div()
                                    .id("confirm-cancel")
                                    .px(px(12.))
                                    .py(px(6.))
                                    .rounded(px(4.))
                                    .cursor_pointer()
                                    .bg(border)
                                    .hover(move |s| s.bg(border_variant))
                                    .text_color(text)
                                    .text_size(px(13.))
                                    .child("Cancel")
                                    .on_click(move |_, _, cx| {
                                        app_cancel.update(cx, AppState::cancel_confirm);
                                    }),
                            )
                            .child(
                                div()
                                    .id("confirm-ok")
                                    .px(px(12.))
                                    .py(px(6.))
                                    .rounded(px(4.))
                                    .cursor_pointer()
                                    .bg(theme.destructive)
                                    .hover(move |s| s.bg(destructive_hover))
                                    .text_color(gpui::rgba(0xffff_ffff))
                                    .text_size(px(13.))
                                    .child("Close")
                                    .on_click(move |_, _, cx| {
                                        app_confirm.update(cx, AppState::confirm_close);
                                    }),
                            ),
                    ),
            )
            .into_any_element()
    }
}
