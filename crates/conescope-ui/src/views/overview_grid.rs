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

/// Compute grid (columns, rows) from total slot count (instances + empty slot).
fn grid_dimensions(total: usize) -> (usize, usize) {
    match total {
        0 | 1 => (1, 1),
        2 => (2, 1),
        3..=4 => (2, 2),
        5..=6 => (3, 2),
        _ => {
            let cols = 3;
            let rows = total.div_ceil(cols);
            (cols, rows)
        }
    }
}

fn render_tile(
    tile: &TileData,
    app_state: Entity<AppState>,
    edit: Option<&EditCtx>,
) -> gpui::AnyElement {
    let status_rgba = status_color(tile.status);

    div()
        .flex_1()
        .min_w(px(200.))
        .flex()
        .flex_col()
        .border_r_1()
        .border_b_1()
        .border_color(rgba(0x3c3c_3cff))
        .child(render_tile_header(
            tile,
            status_rgba,
            app_state.clone(),
            edit,
        ))
        .child(render_tile_meta(tile))
        .child(render_tile_body(tile, app_state))
        .into_any_element()
}

fn render_tile_header(
    tile: &TileData,
    status_rgba: gpui::Rgba,
    app_state: Entity<AppState>,
    edit: Option<&EditCtx>,
) -> gpui::AnyElement {
    let close_id = tile.id.clone();
    let close_state = app_state.clone();

    let title_element: gpui::AnyElement = if let Some(edit) = edit {
        render_editing_title(tile, &app_state, edit)
    } else {
        render_static_title(tile, app_state)
    };

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
        .child(title_element)
        .child(div().w(px(6.)).h(px(6.)).rounded(px(3.)).bg(status_rgba))
        .child(
            div()
                .ml(px(4.))
                .cursor_pointer()
                .text_size(px(12.))
                .text_color(rgba(0x6666_66ff))
                .hover(|s| s.text_color(rgba(0xcccc_ccff)))
                .child("\u{00d7}") // × character
                .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                    let il = close_state.read(cx).instance_list.clone();
                    il.update(cx, |list, cx| list.remove_instance(&close_id, cx));
                }),
        )
        .into_any_element()
}

fn render_static_title(tile: &TileData, app_state: Entity<AppState>) -> gpui::AnyElement {
    let click_id = tile.id.clone();
    let current_title = tile.title.clone();

    div()
        .flex_1()
        .text_size(px(11.))
        .text_color(rgba(0xaaaa_aaff))
        .overflow_x_hidden()
        .cursor_pointer()
        .on_mouse_down(MouseButton::Left, move |_, window, cx| {
            let focus = app_state.read(cx).edit_focus.clone();
            app_state.update(cx, |s, _| {
                s.start_edit_title(&click_id, &current_title);
            });
            focus.focus(window, cx);
        })
        .child(tile.title.clone())
        .into_any_element()
}

fn render_editing_title(
    _tile: &TileData,
    app_state: &Entity<AppState>,
    edit: &EditCtx,
) -> gpui::AnyElement {
    let key_state = app_state.clone();

    div()
        .id("edit-title")
        .flex_1()
        .text_size(px(11.))
        .text_color(rgba(0xdddd_ddff))
        .bg(rgba(0x1a1a_1aff))
        .px(px(4.))
        .rounded(px(2.))
        .overflow_x_hidden()
        .track_focus(&edit.focus)
        .on_key_down(move |ev, _window, cx| {
            let key = ev.keystroke.key.as_str();
            match key {
                "enter" => {
                    key_state.update(cx, AppState::save_edit_title);
                }
                "escape" => {
                    key_state.update(cx, AppState::cancel_edit_title);
                }
                "backspace" => {
                    key_state.update(cx, |s, cx| {
                        s.editing_buffer.pop();
                        cx.notify();
                    });
                }
                _ => {
                    if let Some(ch) = ev.keystroke.key_char.as_deref() {
                        if !ev.keystroke.modifiers.platform && !ev.keystroke.modifiers.control {
                            let ch = ch.to_owned();
                            key_state.update(cx, |s, cx| {
                                s.editing_buffer.push_str(&ch);
                                cx.notify();
                            });
                        }
                    }
                }
            }
        })
        .child(format!("{}|", edit.buffer))
        .into_any_element()
}

fn render_tile_meta(tile: &TileData) -> gpui::Div {
    let path_text = tile
        .project_path
        .as_deref()
        .map(shorten_path)
        .unwrap_or_default();
    let tokens_text = if tile.tokens_used > 0 {
        format_tokens(tile.tokens_used)
    } else {
        String::new()
    };

    div()
        .h(px(18.))
        .px(px(8.))
        .flex()
        .flex_row()
        .items_center()
        .gap(px(6.))
        .bg(rgba(0x1e1e_1eff))
        .border_b_1()
        .border_color(rgba(0x3c3c_3cff))
        .child(
            div()
                .flex_1()
                .text_size(px(10.))
                .text_color(rgba(0x5555_55ff))
                .overflow_x_hidden()
                .child(path_text),
        )
        .child(
            div()
                .text_size(px(10.))
                .text_color(rgba(0x5555_55ff))
                .child(tokens_text),
        )
}

/// Shorten a path for display (replace $HOME with ~).
fn shorten_path(path: &str) -> String {
    if path.is_empty() {
        return String::new();
    }
    let home = std::env::var("HOME").unwrap_or_default();
    if !home.is_empty() && path.starts_with(&home) {
        return format!("~{}", &path[home.len()..]);
    }
    path.to_owned()
}

/// Format token count for compact display.
#[allow(clippy::cast_precision_loss)]
fn format_tokens(tokens: i64) -> String {
    if tokens >= 1_000_000 {
        format!("{:.1}M tokens", tokens as f64 / 1_000_000.0)
    } else if tokens >= 1_000 {
        format!("{:.1}K tokens", tokens as f64 / 1_000.0)
    } else {
        format!("{tokens} tokens")
    }
}

/// Tile body: terminal preview + click-to-focus handler.
fn render_tile_body(tile: &TileData, app_state: Entity<AppState>) -> gpui::Div {
    let tile_id = tile.id.clone();
    let tile_focus_handle = tile.focus_handle.clone();

    div()
        .flex_1()
        .overflow_hidden()
        .cursor_pointer()
        .hover(|s| s.bg(rgba(0x2525_26ff)))
        .on_mouse_down(MouseButton::Left, move |_, window, cx| {
            app_state.update(cx, |s, cx| s.focus_instance(&tile_id, cx));
            if let Some(ref fh) = tile_focus_handle {
                fh.focus(window, cx);
            }
        })
        .children(
            tile.terminal_view
                .as_ref()
                .map(|tv| div().size_full().child(tv.clone()).into_any_element()),
        )
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
        // Fill incomplete rows with spacers that maintain grid borders
        for _ in 0..(cols - chunk.len()) {
            row = row.child(
                div()
                    .flex_1()
                    .min_w(px(200.))
                    .border_r_1()
                    .border_b_1()
                    .border_color(rgba(0x3c3c_3cff)),
            );
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
        let ps = state.project_store.read(cx);

        let editing_tile_id = state.editing_tile_id.clone();
        let editing_buffer = state.editing_buffer.clone();
        let edit_focus = state.edit_focus.clone();

        let tiles: Vec<TileData> = il
            .entries()
            .iter()
            .map(|entry| {
                let inst = entry.read(cx);
                let project_path = inst
                    .instance
                    .project_id
                    .as_ref()
                    .and_then(|pid| ps.get(pid))
                    .map(|p| p.path.clone());
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
                    project_path,
                    tokens_used: inst.instance.tokens_used,
                    terminal_view: inst.terminal_view.clone(),
                    focus_handle: inst.focus_handle.clone(),
                }
            })
            .collect();

        let (cols, _rows) = grid_dimensions(tiles.len() + 1);

        let mut slots: Vec<gpui::AnyElement> = tiles
            .iter()
            .map(|tile| {
                let edit = if editing_tile_id.as_deref() == Some(tile.id.as_str()) {
                    Some(EditCtx {
                        buffer: editing_buffer.clone(),
                        focus: edit_focus.clone(),
                    })
                } else {
                    None
                };
                render_tile(tile, self.app_state.clone(), edit.as_ref())
            })
            .collect();

        slots.push(render_empty_slot(self.app_state.clone()));

        build_grid(slots, cols)
    }
}

/// Context for an actively-editing tile title.
struct EditCtx {
    buffer: String,
    focus: gpui::FocusHandle,
}

struct TileData {
    id: String,
    num: i64,
    title: String,
    status: conescope_core::instance::InstanceStatus,
    color: gpui::Rgba,
    project_path: Option<String>,
    tokens_used: i64,
    terminal_view: Option<gpui::Entity<gpui_ghostty_terminal::view::TerminalView>>,
    focus_handle: Option<gpui::FocusHandle>,
}
