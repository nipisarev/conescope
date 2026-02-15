use gpui::prelude::*;
use gpui::{Entity, MouseButton, div, px, relative, svg};

use conescope_core::instance::InstanceType;

use crate::icons;
use crate::state::app_state::AppState;
use crate::state::session_detector::{SessionEvent, SessionStatus};
use crate::theme::Theme;
use crate::views::colors::{default_instance_color, hex_to_rgba};
use crate::views::question_overlay::render_question_overlay;
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

#[allow(clippy::too_many_arguments)]
fn render_tile(
    tile: &TileData,
    app_state: Entity<AppState>,
    editing_input: Option<&Entity<TextInput>>,
    terminal_font_size: f32,
    font_family: &str,
    theme: &Theme,
) -> gpui::AnyElement {
    let status_color = match tile.session_status {
        SessionStatus::Working => gpui::rgba(0x4ade_80ff), // green
        SessionStatus::Question => gpui::rgba(0xf871_71ff), // red
        SessionStatus::Waiting | SessionStatus::Finished => gpui::rgba(0xfacc_15ff), // yellow
        SessionStatus::Stopped => gpui::rgba(0x9ca3_afff), // gray
    };
    let dot_opacity = if tile.session_status.is_pulsing() {
        tile.pulse_opacity
    } else {
        1.0
    };

    div()
        .flex_1()
        .min_w(px(200.))
        .flex()
        .flex_col()
        .bg(theme.panel)
        .border_r_1()
        .border_b_1()
        .border_color(theme.border)
        .child(render_tile_header(
            tile,
            status_color,
            dot_opacity,
            app_state.clone(),
            editing_input,
            terminal_font_size,
            theme,
        ))
        .child(render_tile_body(
            tile,
            app_state,
            terminal_font_size,
            font_family,
            theme,
        ))
        .into_any_element()
}

#[allow(clippy::too_many_arguments)]
fn render_tile_header(
    tile: &TileData,
    status_color: gpui::Rgba,
    dot_opacity: f32,
    app_state: Entity<AppState>,
    editing_input: Option<&Entity<TextInput>>,
    font_size: f32,
    theme: &Theme,
) -> gpui::AnyElement {
    let close_id = tile.id.clone();
    let close_title = tile.title.clone();
    let close_state = app_state.clone();

    let title_element: gpui::AnyElement = if let Some(input) = editing_input {
        div().child(input.clone()).into_any_element()
    } else {
        render_static_title(tile, app_state, font_size)
    };

    let path_text = tile.project_path.as_deref().map_or_else(
        || match tile.instance_type {
            InstanceType::Project => "Claude Project".to_owned(),
            InstanceType::Terminal => "~".to_owned(),
        },
        shorten_path,
    );

    let icon_size = px(font_size);
    let secondary_size = px(font_size - 1.0);

    // Group 1: [cmd icon] [number] [title]
    let group1 = div()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(3.))
        .flex_shrink_0()
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(1.))
                .text_size(px(font_size))
                .font_weight(gpui::FontWeight::BOLD)
                .text_color(tile.color)
                .child(
                    svg()
                        .path(icons::ICON_COMMAND_REG)
                        .size(icon_size)
                        .text_color(tile.color)
                        .flex_shrink_0(),
                )
                .child(format!("{}", tile.num)),
        )
        .child(title_element);

    // Group 2: [folder icon] [path]
    let group2 = div()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(2.))
        .min_w(px(20.))
        .text_size(secondary_size)
        .text_color(theme.text)
        .overflow_x_hidden()
        .child(
            svg()
                .path(icons::ICON_FOLDER_REG)
                .size(icon_size)
                .text_color(theme.text)
                .flex_shrink_0(),
        )
        .child(path_text);

    // Group 3: [git-branch icon] [branch] [+N -N]  (only if git)
    let group3 = render_git_badge(tile, theme, icon_size, secondary_size);

    // Left side: three groups with wider spacing between them
    let mut left = div()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(10.))
        .overflow_x_hidden()
        .child(group1)
        .child(group2);

    if let Some(badge) = group3 {
        left = left.child(badge);
    }

    let right = render_tile_controls(
        status_color,
        dot_opacity,
        icon_size,
        close_state,
        close_id,
        close_title,
        theme,
    );

    // Full header: left ... spacer ... right
    div()
        .px(px(4.))
        .pt(px(2.))
        .pb(px(2.))
        .flex()
        .flex_row()
        .items_center()
        .gap(px(4.))
        .child(left)
        .child(div().flex_1()) // spacer
        .child(right)
        .into_any_element()
}

/// Right side controls: status dot + close button.
#[allow(clippy::too_many_arguments)]
fn render_tile_controls(
    status_color: gpui::Rgba,
    dot_opacity: f32,
    icon_size: gpui::Pixels,
    close_state: Entity<AppState>,
    close_id: String,
    close_title: String,
    theme: &Theme,
) -> gpui::Div {
    let text_color = theme.text;
    div()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(4.))
        .flex_shrink_0()
        .child(
            div()
                .w(px(8.))
                .h(px(8.))
                .rounded(px(4.))
                .bg(status_color)
                .opacity(dot_opacity)
                .flex_shrink_0(),
        )
        .child(
            div()
                .cursor_pointer()
                .text_color(text_color)
                .child(
                    svg()
                        .path(icons::ICON_TRASH)
                        .size(icon_size)
                        .text_color(text_color),
                )
                .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                    close_state.update(cx, |s, cx| {
                        s.request_close_instance(&close_id, &close_title, cx);
                    });
                }),
        )
}

fn render_static_title(
    tile: &TileData,
    app_state: Entity<AppState>,
    font_size: f32,
) -> gpui::AnyElement {
    let click_id = tile.id.clone();
    let current_title = tile.title.clone();

    div()
        .text_size(px(font_size))
        .font_weight(gpui::FontWeight::MEDIUM)
        .text_color(tile.color)
        .flex_shrink_0()
        .overflow_x_hidden()
        .cursor_pointer()
        .on_mouse_down(MouseButton::Left, move |_, window, cx| {
            app_state.update(cx, |s, cx| {
                s.start_edit_title(&click_id, &current_title, cx);
            });
            let editing_input = app_state.read(cx).editing_input.clone();
            if let Some(input) = editing_input {
                input.read(cx).focus_handle.clone().focus(window, cx);
            }
        })
        .child(tile.title.clone())
        .into_any_element()
}

/// Git branch icon, branch name, and +/- diff stats as standalone div.
fn render_git_badge(
    tile: &TileData,
    theme: &Theme,
    icon_size: gpui::Pixels,
    secondary_size: gpui::Pixels,
) -> Option<gpui::Div> {
    let branch = tile.git_branch.as_ref()?;

    let green = gpui::rgba(0x4ec9_4eff);
    let red = gpui::rgba(0xf851_49ff);

    let mut badge = div()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(3.))
        .text_size(secondary_size)
        .text_color(theme.text_muted)
        .flex_shrink_0()
        .child(
            svg()
                .path(icons::ICON_GIT_REG)
                .size(icon_size)
                .text_color(theme.text_muted)
                .flex_shrink_0(),
        )
        .child(branch.clone());

    if tile.git_insertions > 0 {
        badge = badge.child(
            div()
                .text_color(green)
                .child(format!("+{}", tile.git_insertions)),
        );
    }
    if tile.git_deletions > 0 {
        badge = badge.child(
            div()
                .text_color(red)
                .child(format!("-{}", tile.git_deletions)),
        );
    }

    Some(badge)
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

/// Tile body: terminal preview + click-to-focus handler.
///
/// A transparent overlay sits on top of the terminal to capture clicks,
/// preventing the `TerminalView`'s `stop_propagation()` from blocking focus switch.
fn render_tile_body(
    tile: &TileData,
    app_state: Entity<AppState>,
    terminal_font_size: f32,
    font_family: &str,
    theme: &Theme,
) -> gpui::Div {
    let tile_id = tile.id.clone();
    let tile_focus_handle = tile.focus_handle.clone();
    let panel = theme.panel;

    let session_event = tile.session_event.clone();
    let overlay_id = tile.id.clone();
    let overlay_state = app_state.clone();

    let terminal_child = tile.terminal_view.as_ref().map(|tv| {
        let mut container = div()
            .size_full()
            .relative()
            .px(px(4.))
            .child(tv.clone())
            .child(
                div()
                    .absolute()
                    .top_0()
                    .left_0()
                    .size_full()
                    .cursor_pointer()
                    .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                        app_state.update(cx, |s, cx| s.focus_instance(&tile_id, cx));
                        if let Some(ref fh) = tile_focus_handle {
                            fh.focus(window, cx);
                        }
                    }),
            );

        container = container.children(
            session_event
                .as_ref()
                .map(|event| render_question_overlay(event, &overlay_id, &overlay_state, theme)),
        );

        container.into_any_element()
    });

    div()
        .flex_1()
        .overflow_hidden()
        .font_family(gpui::SharedString::from(font_family.to_owned()))
        .text_size(px(terminal_font_size))
        .line_height(relative(1.0))
        .hover(move |s| s.bg(panel))
        .children(terminal_child)
}

fn render_empty_state(app_state: Entity<AppState>, theme: &Theme) -> gpui::AnyElement {
    let bg = theme.background;
    let panel = theme.panel;
    div()
        .size_full()
        .flex()
        .items_center()
        .justify_center()
        .cursor_pointer()
        .bg(bg)
        .hover(move |s| s.bg(panel))
        .on_mouse_down(MouseButton::Left, move |_, _, cx| {
            app_state.update(cx, AppState::toggle_new_instance_modal);
        })
        .child(
            div()
                .flex()
                .flex_col()
                .items_center()
                .gap(px(4.))
                .text_color(theme.text_disabled)
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

        let theme = state.theme();

        // Empty state: show "+ New Window" tile
        if il.is_empty() {
            return build_grid(vec![render_empty_state(self.app_state.clone(), theme)], 1);
        }

        let ps = state.project_store.read(cx);

        let editing_tile_id = state.editing_tile_id.clone();
        let editing_input = state.editing_input.clone();

        let terminal_font_size = state.settings_store.read(cx).settings().ui_font_size();

        let font_family = state.settings_store.read(cx).settings().font_family.clone();

        let pulse_opacity = state.pulse_timer.read(cx).opacity();

        let tiles: Vec<TileData> = il
            .entries()
            .iter()
            .enumerate()
            .map(|(i, entry)| {
                let inst = entry.read(cx);
                // Prefer live CWD, fall back to project path
                let project_path = inst.current_cwd.clone().or_else(|| {
                    inst.instance
                        .project_id
                        .as_ref()
                        .and_then(|pid| ps.get(pid))
                        .map(|p| p.path.clone())
                });
                let git = &inst.git_summary;
                let title = inst
                    .instance
                    .title
                    .as_deref()
                    .filter(|t| !t.is_empty())
                    .unwrap_or_else(|| {
                        // Fallback: last folder name from project path
                        project_path
                            .as_deref()
                            .and_then(|p| p.rsplit('/').find(|s| !s.is_empty()))
                            .unwrap_or("Untitled")
                    })
                    .to_owned();
                TileData {
                    id: inst.id().to_owned(),
                    num: i + 1,
                    title,
                    color: inst
                        .instance
                        .color
                        .as_deref()
                        .map_or_else(default_instance_color, hex_to_rgba),
                    instance_type: inst.instance_type(),
                    project_path,
                    terminal_view: inst.terminal_view.clone(),
                    focus_handle: inst.focus_handle.clone(),
                    git_branch: git.branch.clone(),
                    git_insertions: git.insertions,
                    git_deletions: git.deletions,
                    session_status: inst.session_status(),
                    session_event: inst.session_event().cloned(),
                    pulse_opacity,
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
                    theme,
                )
            })
            .collect();

        build_grid(slots, cols)
    }
}

#[allow(dead_code)]
struct TileData {
    id: String,
    num: usize,
    title: String,
    color: gpui::Rgba,
    instance_type: InstanceType,
    project_path: Option<String>,
    terminal_view: Option<gpui::Entity<crate::terminal::terminal_view::TerminalView>>,
    focus_handle: Option<gpui::FocusHandle>,
    git_branch: Option<String>,
    git_insertions: usize,
    git_deletions: usize,
    session_status: SessionStatus,
    session_event: Option<SessionEvent>,
    pulse_opacity: f32,
}
