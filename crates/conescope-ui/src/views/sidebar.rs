use gpui::prelude::*;
use gpui::{Entity, MouseButton, ScrollHandle, SharedString, div, px, rgba, svg};

use crate::icons;
use crate::state::app_state::AppState;
use crate::state::settings_store::SidebarMode;
use crate::theme::Theme;
use crate::views::colors::{default_instance_color, hex_to_rgba};
use crate::views::text_input::TextInput;

pub const SIDEBAR_WIDTH: f32 = 300.0;

#[derive(Debug)]
pub struct Sidebar {
    app_state: Entity<AppState>,
    scroll_handle: ScrollHandle,
}

impl Sidebar {
    #[must_use]
    pub fn new(app_state: Entity<AppState>) -> Self {
        Self {
            app_state,
            scroll_handle: ScrollHandle::new(),
        }
    }
}

struct SidebarEntry {
    id: String,
    num: usize,
    title: String,
    path: String,
    color: gpui::Rgba,
    is_focused: bool,
    git_branch: Option<String>,
    git_insertions: usize,
    git_deletions: usize,
}

fn collect_sidebar_entries(app_state: &Entity<AppState>, cx: &gpui::App) -> Vec<SidebarEntry> {
    let state = app_state.read(cx);
    let il = state.instance_list.read(cx);
    let focused_id = state.focused_instance_id(cx).map(str::to_owned);

    il.entries()
        .iter()
        .enumerate()
        .map(|(i, entry)| {
            let inst = entry.read(cx);
            let num = i + 1;
            let color = inst
                .instance
                .color
                .as_deref()
                .map_or_else(default_instance_color, hex_to_rgba);
            let title = inst
                .instance
                .title
                .clone()
                .unwrap_or_else(|| format!("Instance {num}"));
            let path = shorten_path(
                inst.current_cwd
                    .as_deref()
                    .or_else(|| {
                        inst.instance.project_id.as_ref().and_then(|pid| {
                            state
                                .project_store
                                .read(cx)
                                .get(pid)
                                .map(|p| p.path.as_str())
                        })
                    })
                    .unwrap_or("~"),
            );
            let git = &inst.git_summary;
            SidebarEntry {
                id: inst.id().to_owned(),
                num,
                title,
                path,
                color,
                is_focused: focused_id.as_deref() == Some(inst.id()),
                git_branch: git.branch.clone(),
                git_insertions: git.insertions,
                git_deletions: git.deletions,
            }
        })
        .collect()
}

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

fn render_instance_row(
    entry: &SidebarEntry,
    app_state: &Entity<AppState>,
    editing_input: Option<&Entity<TextInput>>,
    font_size: f32,
    theme: &Theme,
) -> gpui::Stateful<gpui::Div> {
    let app_state_click = app_state.clone();
    let id = entry.id.clone();
    let element_id: SharedString = format!("sidebar-row-{}", entry.num).into();
    let element_selected = theme.element_selected;
    let element_hover = theme.element_hover;
    let text_muted = theme.text_muted;
    let destructive_hover: gpui::Hsla = theme.destructive_hover.into();
    let icon_size = px(font_size - 2.0);

    div()
        .id(element_id)
        .px(px(8.))
        .py(px(4.))
        .rounded(px(4.))
        .cursor_pointer()
        .flex()
        .flex_col()
        .gap(px(1.))
        .when(entry.is_focused, move |el| el.bg(element_selected))
        .when(!entry.is_focused, move |el| {
            el.hover(move |s| s.bg(element_hover))
        })
        .on_click(move |_, _, cx| {
            app_state_click.update(cx, |s, cx| s.focus_instance(&id, cx));
        })
        // Line 1: command-num + title + close button
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(4.))
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(px(1.))
                        .text_color(entry.color)
                        .text_size(px(font_size - 1.0))
                        .font_weight(gpui::FontWeight::BOLD)
                        .child(
                            svg()
                                .path(icons::ICON_COMMAND)
                                .size(icon_size)
                                .text_color(entry.color)
                                .flex_shrink_0(),
                        )
                        .child(format!("{}", entry.num)),
                )
                .child(if let Some(input) = editing_input {
                    div()
                        .flex_1()
                        .overflow_hidden()
                        .child(input.clone())
                        .into_any_element()
                } else {
                    let app_state_edit = app_state.clone();
                    let edit_id = entry.id.clone();
                    let edit_title = entry.title.clone();
                    div()
                        .text_color(entry.color)
                        .text_size(px(font_size - 1.0))
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .overflow_hidden()
                        .whitespace_nowrap()
                        .cursor_pointer()
                        .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                            cx.stop_propagation();
                            app_state_edit.update(cx, |s, cx| {
                                s.start_edit_title(&edit_id, &edit_title, cx);
                            });
                            let editing_input =
                                app_state_edit.read(cx).editing_input.clone();
                            if let Some(input) = editing_input {
                                input.read(cx).focus_handle.clone().focus(window, cx);
                            }
                        })
                        .child(entry.title.clone())
                        .into_any_element()
                })
                // Spacer to push close button right
                .child(div().flex_1())
                // Close button
                .child(render_close_button(
                    app_state,
                    entry,
                    font_size,
                    text_muted,
                    destructive_hover,
                )),
        )
        // Line 2: shortened path
        .child(
            div()
                .pl(px(4.))
                .text_color(text_muted)
                .text_size(px(font_size - 2.0))
                .overflow_hidden()
                .whitespace_nowrap()
                .child(entry.path.clone()),
        )
        // Line 3: git badge (optional)
        .when(entry.git_branch.is_some(), |el| {
            el.child(render_git_badge(entry, font_size, text_muted))
        })
}

fn render_git_badge(entry: &SidebarEntry, font_size: f32, text_muted: gpui::Rgba) -> gpui::Div {
    let green: gpui::Rgba = rgba(0x4ec9_4eff);
    let red: gpui::Rgba = rgba(0xf851_49ff);
    let git_icon_size = px(font_size - 3.0);

    let mut badge = div()
        .pl(px(4.))
        .flex()
        .flex_row()
        .items_center()
        .gap(px(3.))
        .text_size(px(font_size - 2.0))
        .text_color(text_muted)
        .child(
            svg()
                .path(icons::ICON_GIT)
                .size(git_icon_size)
                .text_color(text_muted)
                .flex_shrink_0(),
        )
        .child(entry.git_branch.as_deref().unwrap_or("").to_owned());

    if entry.git_insertions > 0 {
        badge = badge.child(
            div()
                .text_color(green)
                .child(format!("+{}", entry.git_insertions)),
        );
    }
    if entry.git_deletions > 0 {
        badge = badge.child(
            div()
                .text_color(red)
                .child(format!("-{}", entry.git_deletions)),
        );
    }
    badge
}

fn render_close_button(
    app_state: &Entity<AppState>,
    entry: &SidebarEntry,
    font_size: f32,
    text_muted: gpui::Rgba,
    destructive_hover: gpui::Hsla,
) -> gpui::Div {
    let app_state_close = app_state.clone();
    let close_id = entry.id.clone();
    let close_title = entry.title.clone();

    div()
        .px(px(2.))
        .rounded(px(2.))
        .cursor_pointer()
        .hover(move |s| s.text_color(destructive_hover))
        .on_mouse_down(MouseButton::Left, move |_, _, cx| {
            cx.stop_propagation();
            app_state_close.update(cx, |s, cx| {
                s.request_close_instance(&close_id, &close_title, cx);
            });
        })
        .child(
            svg()
                .path(icons::ICON_TRASH)
                .size(px(font_size - 3.0))
                .text_color(text_muted)
                .flex_shrink_0(),
        )
}

fn render_bottom_section(
    app_state: &Entity<AppState>,
    font_size: f32,
    glass: bool,
    theme: &Theme,
) -> gpui::Div {
    let app_state_click = app_state.clone();
    let element_hover = theme.element_hover;
    let text_muted = theme.text_muted;
    let text_faint = theme.text_faint;
    let icon_size = px(font_size);
    let cmd_icon_size = px(font_size - 3.0);
    let (divider_ml, button_horiz, button_bottom) = if glass {
        (14.0, 10.0, 6.0)
    } else {
        (20.0, 16.0, 12.0)
    };

    div()
        .flex_shrink_0()
        // Divider
        .child(div().h(px(1.)).ml(px(divider_ml)).w_full().bg(theme.border))
        // Bottom bar: "Create new window" button + settings gear
        .child(
            div()
                .my(px(4.))
                .mx(px(button_horiz))
                .mb(px(button_bottom))
                .flex()
                .flex_row()
                .items_center()
                .gap(px(4.))
                // "Create a new window" button
                .child(
                    div()
                        .flex_1()
                        .pl(px(8.))
                        .py(px(6.))
                        .rounded(px(4.))
                        .cursor_pointer()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(px(4.))
                        .hover(move |s| s.bg(element_hover))
                        .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                            app_state_click.update(cx, AppState::toggle_new_instance_modal);
                        })
                        // Folder-plus icon
                        .child(
                            svg()
                                .path(icons::ICON_FOLDER_PLUS)
                                .size(icon_size)
                                .text_color(text_muted)
                                .flex_shrink_0(),
                        )
                        // Label
                        .child(
                            div()
                                .flex_1()
                                .text_color(text_muted)
                                .text_size(px(font_size - 1.0))
                                .child("Create a new window"),
                        )
                        // Shortcut hint
                        .child(
                            div()
                                .flex()
                                .flex_row()
                                .items_center()
                                .gap(px(1.))
                                .text_color(text_faint)
                                .text_size(px(font_size - 2.0))
                                .child(
                                    svg()
                                        .path(icons::ICON_COMMAND)
                                        .size(cmd_icon_size)
                                        .text_color(text_faint)
                                        .flex_shrink_0(),
                                )
                                .child("N"),
                        ),
                )
                // Settings gear icon (separate button)
                .child(
                    div()
                        .px(px(4.))
                        .py(px(6.))
                        .rounded(px(4.))
                        .cursor_pointer()
                        .hover(move |s| s.bg(element_hover))
                        .child(
                            svg()
                                .path(icons::ICON_SETTINGS)
                                .size(icon_size)
                                .text_color(text_muted)
                                .flex_shrink_0(),
                        )
                        .on_mouse_down(MouseButton::Left, {
                            let app_state = app_state.clone();
                            move |_, _, cx| {
                                cx.stop_propagation();
                                app_state.update(cx, AppState::open_settings);
                            }
                        }),
                ),
        )
}

fn render_window_controls() -> gpui::Div {
    let red = rgba(0xff5f_57ff);
    let yellow = rgba(0xfebc_2eff);
    let green = rgba(0x28c8_40ff);

    div()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(8.))
        // Close
        .child(
            div()
                .size(px(12.))
                .rounded_full()
                .bg(red)
                .cursor_pointer()
                .on_mouse_down(MouseButton::Left, |_, window, cx| {
                    cx.stop_propagation();
                    window.dispatch_action(
                        Box::new(crate::actions::Quit),
                        cx,
                    );
                }),
        )
        // Minimize
        .child(
            div()
                .size(px(12.))
                .rounded_full()
                .bg(yellow)
                .cursor_pointer()
                .on_mouse_down(MouseButton::Left, |_, window, cx| {
                    cx.stop_propagation();
                    window.minimize_window();
                }),
        )
        // Zoom
        .child(
            div()
                .size(px(12.))
                .rounded_full()
                .bg(green)
                .cursor_pointer()
                .on_mouse_down(MouseButton::Left, |_, window, cx| {
                    cx.stop_propagation();
                    window.zoom_window();
                }),
        )
}

fn render_sidebar_header(
    app_state: &Entity<AppState>,
    sidebar_mode: SidebarMode,
    glass: bool,
    prefix: &str,
    theme: &Theme,
) -> gpui::Div {
    let sidebar_icon_color = if sidebar_mode == SidebarMode::Pinned {
        theme.text
    } else {
        theme.text_muted
    };
    let element_hover = theme.element_hover;
    let app_state_toggle = app_state.clone();

    let (top_pad, left_margin) = if glass { (24.0, 18.0) } else { (30.0, 24.0) };

    div()
        .flex()
        .flex_row()
        .items_center()
        .h(px(36.))
        .pt(px(top_pad))
        .pb(px(20.))
        .ml(px(left_margin))
        .gap(px(8.))
        .flex_shrink_0()
        // Window controls (left) — hidden in overlay/glass mode
        .child(render_window_controls())
        // Sidebar toggle — next to window controls on the left
        .child(
            div()
                .id(SharedString::from(format!("{prefix}-sidebar-pin-toggle")))
                .p(px(4.))
                .rounded(px(4.))
                .cursor_pointer()
                .hover(move |s| s.bg(element_hover))
                .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                    cx.stop_propagation();
                    app_state_toggle.update(cx, AppState::toggle_sidebar_open);
                })
                .child(
                    svg()
                        .path(icons::ICON_SIDEBAR)
                        .size(px(16.))
                        .text_color(sidebar_icon_color),
                ),
        )
}

impl Sidebar {
    /// Render with an explicit width (for resizable pinned sidebar).
    pub fn render_with_width(
        &mut self,
        width: f32,
        glass: bool,
        cx: &mut gpui::Context<Self>,
    ) -> impl IntoElement {
        let prefix = if glass { "overlay" } else { "pinned" };
        self.render_inner(width, glass, prefix, cx)
    }

    fn render_inner(
        &mut self,
        width: f32,
        glass: bool,
        prefix: &str,
        cx: &mut gpui::Context<Self>,
    ) -> impl IntoElement {
        let state = self.app_state.read(cx);
        let font_size = state.settings_store.read(cx).settings().ui_font_size();
        let theme = state.theme().clone();
        let sidebar_mode = state.sidebar_mode(cx);
        let editing_tile_id = state.editing_tile_id.clone();
        let editing_input = state.editing_input.clone();

        let entries = collect_sidebar_entries(&self.app_state, cx);

        let list_px = if glass { 10.0 } else { 16.0 };
        let mut list = div().flex().flex_col().gap(px(6.)).px(px(list_px));
        for entry in &entries {
            let input = if editing_tile_id.as_deref() == Some(entry.id.as_str()) {
                editing_input.as_ref()
            } else {
                None
            };
            list = list.child(render_instance_row(
                entry,
                &self.app_state,
                input,
                font_size,
                &theme,
            ));
        }

        let effective_width = width.max(SIDEBAR_WIDTH);

        let mut base = div().h_full().w(px(effective_width)).flex().flex_col();

        if glass {
            // Overlay window provides glass blur; no extra bg/border needed
        } else if sidebar_mode == SidebarMode::Pinned {
            // Pinned in base layer: transparent (inherits root glass bg), no border
        } else {
            // Fallback: opaque panel bg with right border
            base = base.bg(theme.panel).border_color(theme.border).border_r_1();
        }

        base
            // Header with window controls + pin toggle
            .child(render_sidebar_header(&self.app_state, sidebar_mode, glass, prefix, &theme))
            // Scrollable instance list
            .child(
                div()
                    .id(SharedString::from(format!("{prefix}-sidebar-scroll")))
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .track_scroll(&self.scroll_handle)
                    .child(list),
            )
            // Pinned bottom section
            .child(render_bottom_section(&self.app_state, font_size, glass, &theme))
    }
}

impl Render for Sidebar {
    fn render(
        &mut self,
        _window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> impl IntoElement {
        self.render_inner(SIDEBAR_WIDTH, false, "pinned", cx)
    }
}
