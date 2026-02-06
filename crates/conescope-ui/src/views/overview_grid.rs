use gpui::prelude::*;
use gpui::{Entity, MouseButton, div, px, rgba};

use crate::state::app_state::AppState;
use crate::views::colors::{hex_to_rgba, status_color};

#[derive(Debug)]
pub struct OverviewGrid {
    app_state: Entity<AppState>,
}

impl OverviewGrid {
    #[must_use]
    pub fn new(app_state: Entity<AppState>) -> Self {
        Self { app_state }
    }
}

/// Compute grid columns from total slot count (instances + 1 for empty slot).
fn grid_cols(total: usize) -> usize {
    match total {
        0 | 1 => 1,
        2..=4 => 2,
        _ => 3,
    }
}

fn render_tile(tile: TileData, app_state: Entity<AppState>) -> gpui::AnyElement {
    let tile_id = tile.id.clone();
    let status_rgba = status_color(tile.status);

    div()
        .flex_1()
        .min_w(px(200.))
        .flex()
        .flex_col()
        .border_r_1()
        .border_b_1()
        .border_color(rgba(0x3c3c_3cff))
        .cursor_pointer()
        .hover(|s| s.bg(rgba(0x2525_26ff)))
        .on_mouse_down(MouseButton::Left, move |_, _, cx| {
            app_state.update(cx, |s, cx| s.focus_instance(&tile_id, cx));
        })
        .child(render_tile_header(&tile, status_rgba))
        .child(render_tile_body(tile.terminal_view))
        .into_any_element()
}

fn render_tile_header(tile: &TileData, status_rgba: gpui::Rgba) -> gpui::Div {
    div()
        .h(px(24.))
        .px(px(8.))
        .flex()
        .flex_row()
        .items_center()
        .gap(px(6.))
        .bg(rgba(0x2525_26ff))
        .border_b_1()
        .border_color(rgba(0x3c3c_3cff))
        .child(
            div()
                .text_size(px(11.))
                .text_color(tile.color)
                .child(format!("#{}", tile.num)),
        )
        .child(
            div()
                .flex_1()
                .text_size(px(11.))
                .text_color(rgba(0xaaaa_aaff))
                .overflow_x_hidden()
                .child(tile.title.clone()),
        )
        .child(div().w(px(6.)).h(px(6.)).rounded(px(3.)).bg(status_rgba))
}

fn render_tile_body(
    terminal_view: Option<gpui::Entity<gpui_ghostty_terminal::view::TerminalView>>,
) -> gpui::Div {
    div()
        .flex_1()
        .overflow_hidden()
        .children(terminal_view.map(|tv| div().size_full().child(tv).into_any_element()))
}

fn render_empty_slot(app_state: Entity<AppState>) -> gpui::AnyElement {
    div()
        .flex_1()
        .min_w(px(200.))
        .flex()
        .items_center()
        .justify_center()
        .border_r_1()
        .border_b_1()
        .border_color(rgba(0x3c3c_3cff))
        .cursor_pointer()
        .bg(rgba(0x1e1e_1eff))
        .hover(|s| s.bg(rgba(0x2525_26ff)))
        .on_mouse_down(MouseButton::Left, move |_, _, cx| {
            app_state.update(cx, AppState::toggle_new_instance_modal);
        })
        .child(
            div()
                .flex()
                .flex_col()
                .items_center()
                .gap(px(4.))
                .text_color(rgba(0x5555_55ff))
                .child(div().text_size(px(24.)).child("+"))
                .child(div().text_size(px(11.)).child("New Window")),
        )
        .into_any_element()
}

fn build_grid(slots: Vec<gpui::AnyElement>, cols: usize) -> gpui::Div {
    let mut rows = Vec::new();
    let mut slots = slots;
    for chunk in slots.chunks_mut(cols) {
        let mut row = div().flex_1().w_full().flex().flex_row();
        for slot in chunk.iter_mut() {
            let el = std::mem::replace(slot, div().into_any_element());
            row = row.child(el);
        }
        for _ in 0..(cols - chunk.len()) {
            row = row.child(div().flex_1().min_w(px(200.)));
        }
        rows.push(row);
    }
    let mut grid = div().size_full().flex().flex_col();
    for row in rows {
        grid = grid.child(row);
    }
    grid
}

impl Render for OverviewGrid {
    fn render(
        &mut self,
        _window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> impl IntoElement {
        let state = self.app_state.read(cx);
        let il = state.instance_list.read(cx);

        let tiles: Vec<TileData> = il
            .entries()
            .iter()
            .map(|entry| {
                let inst = entry.read(cx);
                TileData {
                    id: inst.id().to_owned(),
                    num: inst.instance.instance_number.unwrap_or(0),
                    title: inst
                        .instance
                        .title
                        .as_deref()
                        .unwrap_or("Untitled")
                        .to_owned(),
                    status: inst.status(),
                    color: inst
                        .instance
                        .color
                        .as_deref()
                        .map_or_else(|| rgba(0x6464_b5f6), hex_to_rgba),
                    terminal_view: inst.terminal_view.clone(),
                }
            })
            .collect();

        let cols = grid_cols(tiles.len() + 1);

        let mut slots: Vec<gpui::AnyElement> = tiles
            .into_iter()
            .map(|tile| render_tile(tile, self.app_state.clone()))
            .collect();

        slots.push(render_empty_slot(self.app_state.clone()));

        build_grid(slots, cols)
    }
}

struct TileData {
    id: String,
    num: i64,
    title: String,
    status: conescope_core::instance::InstanceStatus,
    color: gpui::Rgba,
    terminal_view: Option<gpui::Entity<gpui_ghostty_terminal::view::TerminalView>>,
}
