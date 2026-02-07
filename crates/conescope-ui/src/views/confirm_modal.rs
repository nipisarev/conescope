use gpui::prelude::*;
use gpui::{Entity, MouseButton, div, px, rgba};

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

        let app_cancel = self.app_state.clone();
        let app_confirm = self.app_state.clone();
        let app_backdrop = self.app_state.clone();

        // Backdrop
        div()
            .id("confirm-backdrop")
            .absolute()
            .size_full()
            .top_0()
            .left_0()
            .bg(rgba(0x0000_0080))
            .flex()
            .items_center()
            .justify_center()
            .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                app_backdrop.update(cx, AppState::cancel_confirm);
            })
            .child(
                div()
                    .w(px(340.))
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
                            .text_color(rgba(0xdddd_ddff))
                            .child(action.title),
                    )
                    // Message
                    .child(
                        div()
                            .px(px(16.))
                            .py(px(12.))
                            .text_color(rgba(0xaaaa_aaff))
                            .text_size(px(13.))
                            .child(action.message),
                    )
                    // Buttons
                    .child(
                        div()
                            .px(px(16.))
                            .py(px(12.))
                            .border_t_1()
                            .border_color(rgba(0x3c3c_3cff))
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
                                    .bg(rgba(0x3c3c_3cff))
                                    .hover(|s| s.bg(rgba(0x4c4c_4cff)))
                                    .text_color(rgba(0xcccc_ccff))
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
                                    .bg(rgba(0xcc44_44ff))
                                    .hover(|s| s.bg(rgba(0xdd55_55ff)))
                                    .text_color(rgba(0xffff_ffff))
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
