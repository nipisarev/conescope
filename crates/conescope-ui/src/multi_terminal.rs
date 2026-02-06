use gpui::prelude::*;
use gpui::{CursorStyle, Entity, div, px, rgba};
use gpui_ghostty_terminal::view::TerminalView;

/// A 2×2 grid of terminal views for gate testing.
pub struct MultiTerminalGrid {
    pub views: [Entity<TerminalView>; 4],
}

impl std::fmt::Debug for MultiTerminalGrid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MultiTerminalGrid")
            .field("views", &"[Entity<TerminalView>; 4]")
            .finish()
    }
}

impl Render for MultiTerminalGrid {
    fn render(&mut self, _: &mut gpui::Window, _: &mut gpui::Context<Self>) -> impl IntoElement {
        let divider = || div().bg(rgba(0x4040_40ff));

        div()
            .size_full()
            .flex()
            .flex_col()
            .cursor(CursorStyle::IBeam)
            // Top row
            .child(
                div()
                    .flex_1()
                    .w_full()
                    .flex()
                    .flex_row()
                    .child(div().flex_1().h_full().child(self.views[0].clone()))
                    .child(divider().w(px(1.)).h_full())
                    .child(div().flex_1().h_full().child(self.views[1].clone())),
            )
            // Horizontal divider
            .child(divider().h(px(1.)).w_full())
            // Bottom row
            .child(
                div()
                    .flex_1()
                    .w_full()
                    .flex()
                    .flex_row()
                    .child(div().flex_1().h_full().child(self.views[2].clone()))
                    .child(divider().w(px(1.)).h_full())
                    .child(div().flex_1().h_full().child(self.views[3].clone())),
            )
    }
}
