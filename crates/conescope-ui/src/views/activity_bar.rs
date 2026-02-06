use gpui::prelude::*;
use gpui::{Entity, MouseButton, div, px, rgba};

use crate::state::app_state::AppState;
use crate::state::settings_store::ViewMode;
use crate::views::colors::hex_to_rgba;

#[derive(Debug)]
pub struct ActivityBar {
    app_state: Entity<AppState>,
}

impl ActivityBar {
    #[must_use]
    pub fn new(app_state: Entity<AppState>) -> Self {
        Self { app_state }
    }
}

impl Render for ActivityBar {
    fn render(
        &mut self,
        _window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> impl IntoElement {
        let state = self.app_state.read(cx);
        let il = state.instance_list.read(cx);
        let focused_id = state.focused_instance_id(cx).map(str::to_owned);
        let view_mode = state.view_mode(cx);

        // Gather instance info for rendering buttons and totals
        let mut buttons: Vec<(String, i64, gpui::Rgba, bool)> = Vec::new();
        let mut total_tokens: i64 = 0;
        let mut total_cost: f64 = 0.0;

        for entry in il.entries() {
            let inst = entry.read(cx);
            let num = inst.instance.instance_number.unwrap_or(0);
            let color = inst
                .instance
                .color
                .as_deref()
                .map_or_else(|| rgba(0x6464_b5f6), hex_to_rgba);
            let is_focused = focused_id.as_deref() == Some(inst.id());
            buttons.push((inst.id().to_owned(), num, color, is_focused));
            total_tokens += inst.instance.tokens_used;
            total_cost += inst.instance.cost_estimate;
        }

        let token_text = format!("{}k tokens", total_tokens / 1000);
        let cost_text = format!("${total_cost:.2}");

        let app_state_grid = self.app_state.clone();

        div()
            .h(px(28.))
            .w_full()
            .flex()
            .flex_row()
            .items_center()
            .px(px(8.))
            .bg(rgba(0x1818_18ff))
            .border_t_1()
            .border_color(rgba(0x3c3c_3cff))
            // Left: grid icon + instance buttons
            .child(
                div()
                    .flex_1()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(4.))
                    // Grid icon (return to overview)
                    .child(
                        div()
                            .px(px(4.))
                            .cursor_pointer()
                            .text_color(rgba(0x8888_88ff))
                            .hover(|s| s.text_color(rgba(0xcccc_ccff)))
                            .child("\u{25a6}")
                            .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                                if view_mode == ViewMode::Focus {
                                    app_state_grid
                                        .update(cx, AppState::return_to_overview);
                                }
                            }),
                    )
                    // Instance number buttons
                    .children(buttons.into_iter().map(|(id, num, color, is_focused)| {
                        let app_state_btn = self.app_state.clone();
                        div()
                            .px(px(6.))
                            .py(px(1.))
                            .rounded(px(3.))
                            .cursor_pointer()
                            .text_size(px(11.))
                            .when(is_focused, |el| el.bg(rgba(0x0e63_9cff)))
                            .when(!is_focused, |el| el.hover(|s| s.bg(rgba(0x3333_33ff))))
                            .text_color(color)
                            .child(format!("{num}"))
                            .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                                app_state_btn.update(cx, |s, cx| s.focus_instance(&id, cx));
                            })
                    })),
            )
            // Right: totals
            .child(
                div()
                    .flex()
                    .flex_row()
                    .gap(px(12.))
                    .text_color(rgba(0x6666_66ff))
                    .text_size(px(11.))
                    .child(token_text)
                    .child(cost_text),
            )
    }
}
