use gpui::prelude::*;
use gpui::{Entity, div, rgba};

use crate::state::app_state::AppState;

#[derive(Debug)]
pub struct FocusView {
    app_state: Entity<AppState>,
}

impl FocusView {
    #[must_use]
    pub fn new(app_state: Entity<AppState>) -> Self {
        Self { app_state }
    }
}

impl Render for FocusView {
    fn render(
        &mut self,
        _window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> impl IntoElement {
        let state = self.app_state.read(cx);
        let focused_id = state.focused_instance_id(cx);

        let Some(id) = focused_id else {
            return div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .text_color(rgba(0x6666_66ff))
                .child("No instance focused");
        };

        let il = state.instance_list.read(cx);
        let Some(entry) = il.find_by_id(id, cx) else {
            return div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .text_color(rgba(0x6666_66ff))
                .child("Instance not found");
        };

        let inst = entry.read(cx);
        if let Some(ref tv) = inst.terminal_view {
            div()
                .size_full()
                .flex()
                .flex_col()
                .child(div().flex_1().child(tv.clone()))
        } else {
            div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .text_color(rgba(0x6666_66ff))
                .child("Terminal not attached")
        }
    }
}

/// Register a window bounds observer that resizes the focused instance's PTY.
///
/// Must be called once after creating the `AppView`, from within the window context.
pub fn register_focus_resize(
    app_state: &Entity<AppState>,
    window: &mut gpui::Window,
    cx: &mut gpui::App,
) -> gpui::Subscription {
    use crate::state::settings_store::ViewMode;
    use crate::terminal::compute_cell_metrics;

    let app_state = app_state.clone();

    app_state.update(cx, |_, cx| {
        cx.observe_window_bounds(window, move |this, window, cx| {
            // `this` is already `&mut AppState` — observe_window_bounds wraps
            // the callback in weak.update(), so reading the entity again would
            // cause a double-lease panic.
            if this.view_mode(cx) != ViewMode::Focus {
                return;
            }
            let Some(focused_id) = this.focused_instance_id(cx) else {
                return;
            };
            let il = this.instance_list.read(cx);
            let Some(entry) = il.find_by_id(focused_id, cx) else {
                return;
            };

            let size = window.viewport_size();
            // Subtract TopBar (36px) and ActivityBar (28px)
            let content_height = f32::from(size.height) - 36.0 - 28.0;
            let width = f32::from(size.width);

            let Some((cell_width, cell_height)) = compute_cell_metrics(window) else {
                return;
            };

            #[allow(clippy::cast_sign_loss)]
            let cols = (width / cell_width).floor().max(1.0) as u16;
            #[allow(clippy::cast_sign_loss)]
            let rows = (content_height / cell_height).floor().max(1.0) as u16;

            // Extract terminal_view before mutable borrow
            let tv = entry.read(cx).terminal_view.clone();
            entry.read(cx).resize_pty(cols, rows);
            if let Some(tv) = tv {
                tv.update(cx, |view, cx| view.resize_terminal(cols, rows, cx));
            }
        })
    })
}
