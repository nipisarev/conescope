use gpui::prelude::*;
use gpui::{Entity, MouseButton, div, px, relative, rgba};

use conescope_core::instance::InstanceType;

use crate::state::app_state::AppState;
use crate::views::colors::{hex_to_rgba, status_color};
use crate::views::text_input::TextInput;

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

/// Compute grid (columns, rows) from instance count.
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
    editing_input: Option<&Entity<TextInput>>,
    terminal_font_size: f32,
    font_family: &str,
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
            editing_input,
        ))
        .child(render_tile_meta(tile))
        .child(render_tile_body(
            tile,
            app_state,
            terminal_font_size,
            font_family,
        ))
        .into_any_element()
}

fn render_tile_header(
    tile: &TileData,
    status_rgba: gpui::Rgba,
    app_state: Entity<AppState>,
    editing_input: Option<&Entity<TextInput>>,
) -> gpui::AnyElement {
    let close_id = tile.id.clone();
    let close_title = tile.title.clone();
    let close_state = app_state.clone();

    let title_element: gpui::AnyElement = if let Some(input) = editing_input {
        div().flex_1().child(input.clone()).into_any_element()
    } else {
        render_static_title(tile, app_state)
    };

    let token_label = format_tokens_compact(tile.tokens_used);

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
        // Token count
        .child(
            div()
                .text_size(px(10.))
                .text_color(rgba(0x5555_55ff))
                .child(token_label),
        )
        .child(
            div()
                .ml(px(2.))
                .cursor_pointer()
                .text_size(px(12.))
                .text_color(rgba(0x6666_66ff))
                .hover(|s| s.text_color(rgba(0xcccc_ccff)))
                .child("\u{00d7}") // × character
                .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                    close_state.update(cx, |s, cx| {
                        s.request_close_instance(&close_id, &close_title, cx);
                    });
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
            app_state.update(cx, |s, cx| {
                s.start_edit_title(&click_id, &current_title, cx);
            });
            // Focus the text input
            let editing_input = app_state.read(cx).editing_input.clone();
            if let Some(input) = editing_input {
                input.read(cx).focus_handle.clone().focus(window, cx);
            }
        })
        .child(tile.title.clone())
        .into_any_element()
}

fn render_tile_meta(tile: &TileData) -> gpui::Div {
    let path_text = tile.project_path.as_deref().map_or_else(
        || match tile.instance_type {
            InstanceType::Project => "Claude Project".to_owned(),
            InstanceType::Terminal => "~".to_owned(),
        },
        shorten_path,
    );
    let stats_text = format_stats(tile.tokens_used, tile.cost_estimate);

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
                .child(stats_text),
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

/// Format token count for tile header (always shows, e.g. "0.0k").
#[allow(clippy::cast_precision_loss)]
fn format_tokens_compact(tokens: i64) -> String {
    let k = tokens as f64 / 1_000.0;
    format!("{k:.1}k")
}

/// Format token count for compact display.
#[allow(clippy::cast_precision_loss)]
fn format_tokens(tokens: i64) -> String {
    if tokens >= 1_000_000 {
        format!("{:.1}M", tokens as f64 / 1_000_000.0)
    } else if tokens >= 1_000 {
        format!("{:.1}K", tokens as f64 / 1_000.0)
    } else {
        format!("{tokens}")
    }
}

/// Format cost for compact display.
fn format_cost(cost: f64) -> String {
    if cost >= 0.01 {
        format!("${cost:.2}")
    } else if cost > 0.0 {
        format!("${cost:.3}")
    } else {
        String::new()
    }
}

/// Combined stats string for tile metadata.
fn format_stats(tokens: i64, cost: f64) -> String {
    let tokens_str = if tokens > 0 {
        format_tokens(tokens)
    } else {
        String::new()
    };
    let cost_str = format_cost(cost);
    match (tokens_str.is_empty(), cost_str.is_empty()) {
        (true, true) => String::new(),
        (false, true) => format!("{tokens_str} tok"),
        (true, false) => cost_str,
        (false, false) => format!("{tokens_str} tok \u{00b7} {cost_str}"),
    }
}

/// Tile body: terminal preview + click-to-focus handler.
fn render_tile_body(
    tile: &TileData,
    app_state: Entity<AppState>,
    terminal_font_size: f32,
    font_family: &str,
) -> gpui::Div {
    let tile_id = tile.id.clone();
    let tile_focus_handle = tile.focus_handle.clone();

    div()
        .flex_1()
        .overflow_hidden()
        .cursor_pointer()
        .font_family(gpui::SharedString::from(font_family.to_owned()))
        .text_size(px(terminal_font_size))
        .line_height(relative(1.0))
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

fn render_empty_state(app_state: Entity<AppState>) -> gpui::AnyElement {
    div()
        .size_full()
        .flex()
        .items_center()
        .justify_center()
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

        // Empty state: show "+ New Window" tile
        if il.is_empty() {
            return build_grid(vec![render_empty_state(self.app_state.clone())], 1);
        }

        let ps = state.project_store.read(cx);

        let editing_tile_id = state.editing_tile_id.clone();
        let editing_input = state.editing_input.clone();

        #[allow(clippy::cast_precision_loss)]
        let terminal_font_size = state
            .settings_store
            .read(cx)
            .settings()
            .get_i64("terminal_font_size")
            .unwrap_or(13) as f32;

        let font_family = state
            .settings_store
            .read(cx)
            .settings()
            .get("font_family")
            .unwrap_or("Menlo")
            .to_owned();

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
                    instance_type: inst.instance_type(),
                    project_path,
                    tokens_used: inst.instance.tokens_used,
                    cost_estimate: inst.instance.cost_estimate,
                    terminal_view: inst.terminal_view.clone(),
                    focus_handle: inst.focus_handle.clone(),
                }
            })
            .collect();

        let (cols, _rows) = grid_dimensions(tiles.len());

        let slots: Vec<gpui::AnyElement> = tiles
            .iter()
            .map(|tile| {
                let input = if editing_tile_id.as_deref() == Some(tile.id.as_str()) {
                    editing_input.as_ref()
                } else {
                    None
                };
                render_tile(
                    tile,
                    self.app_state.clone(),
                    input,
                    terminal_font_size,
                    &font_family,
                )
            })
            .collect();

        build_grid(slots, cols)
    }
}

struct TileData {
    id: String,
    num: i64,
    title: String,
    status: conescope_core::instance::InstanceStatus,
    color: gpui::Rgba,
    instance_type: InstanceType,
    project_path: Option<String>,
    tokens_used: i64,
    cost_estimate: f64,
    terminal_view: Option<gpui::Entity<gpui_ghostty_terminal::view::TerminalView>>,
    focus_handle: Option<gpui::FocusHandle>,
}
