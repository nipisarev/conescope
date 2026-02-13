use gpui::prelude::*;
use gpui::{Entity, Hsla, MouseButton, div, px, svg};

use crate::actions::OpenSettings;
use crate::icons;
use crate::state::app_state::AppState;
use crate::state::settings_store::ViewMode;
use crate::theme::Theme;
use crate::views::colors::{default_instance_color, hex_to_rgba};

#[derive(Debug)]
pub struct TopBar {
    app_state: Entity<AppState>,
}

impl TopBar {
    #[must_use]
    pub fn new(app_state: Entity<AppState>) -> Self {
        Self { app_state }
    }
}

/// Resolve the focused instance title, positional number, and project color.
fn focused_info(state: &AppState, cx: &gpui::App) -> Option<(usize, String, gpui::Rgba)> {
    let id = state.focused_instance_id(cx)?;
    let il = state.instance_list.read(cx);
    let pos = il.entries().iter().position(|e| e.read(cx).id() == id)?;
    let entry = &il.entries()[pos];
    let inst = entry.read(cx);
    let title = inst
        .instance
        .title
        .as_deref()
        .unwrap_or("Untitled")
        .to_owned();
    let color = inst
        .instance
        .color
        .as_deref()
        .map_or_else(default_instance_color, hex_to_rgba);
    Some((pos + 1, title, color))
}

impl Render for TopBar {
    fn render(
        &mut self,
        _window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> impl IntoElement {
        let state = self.app_state.read(cx);
        let theme = state.theme().clone();
        let view_mode = state.view_mode(cx);
        let font_size = state.settings_store.read(cx).settings().ui_font_size();
        let sidebar_open = state.sidebar_open(cx);

        // In Focus mode, title goes in left section; center is empty.
        let (left_title, center_text, title_color) = match view_mode {
            ViewMode::Overview => (None, String::new(), theme.text_muted),
            ViewMode::Focus => {
                if let Some((num, title, color)) = focused_info(state, cx) {
                    (Some(format!("\u{2318}{num} {title}")), String::new(), color)
                } else {
                    (None, String::new(), theme.text_muted)
                }
            }
            ViewMode::Settings => (None, "SETTINGS".to_string(), theme.text_muted),
        };

        let app_state_for_close = self.app_state.clone();
        let app_state_for_back = self.app_state.clone();
        let app_state_for_sidebar = self.app_state.clone();

        let right_section = match view_mode {
            ViewMode::Focus => {
                render_focus_buttons(app_state_for_back, app_state_for_close, font_size, &theme)
            }
            ViewMode::Overview => render_overview_buttons(&self.app_state, font_size, &theme),
            ViewMode::Settings => render_settings_buttons(&self.app_state, font_size, &theme),
        };

        let bar_height = px(font_size * 2.0 + 10.0);
        let icon_size = px(font_size + 1.0);
        let sidebar_icon_color: Hsla = if sidebar_open {
            theme.accent.into()
        } else {
            theme.text_muted.into()
        };
        let sidebar_hover: Hsla = theme.text.into();

        // Left section: traffic light spacer + sidebar toggle + optional focused title
        let mut left_section = div()
            .flex()
            .flex_row()
            .items_center()
            .child(div().w(px(76.)))
            .child(
                div()
                    .px(px(6.))
                    .py(px(4.))
                    .rounded(px(4.))
                    .cursor_pointer()
                    .hover(move |s| s.text_color(sidebar_hover))
                    .child(
                        svg()
                            .path(icons::ICON_SIDEBAR)
                            .size(icon_size)
                            .text_color(sidebar_icon_color)
                            .flex_shrink_0(),
                    )
                    .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                        app_state_for_sidebar.update(cx, AppState::toggle_sidebar_open);
                    }),
            );

        if let Some(title_text) = left_title {
            left_section =
                left_section.child(div().ml(px(8.)).text_color(title_color).child(title_text));
        }

        div()
            .id("top-bar")
            .h(bar_height)
            .w_full()
            .flex()
            .flex_row()
            .items_center()
            .text_size(px(font_size))
            .bg(theme.panel)
            .border_b_1()
            .border_color(theme.border)
            .on_click(|event, window, _cx| {
                if event.click_count() == 2 {
                    window.titlebar_double_click();
                }
            })
            // Left section: traffic lights + sidebar toggle + optional title
            .child(left_section)
            // Center title
            .child(
                div()
                    .flex_1()
                    .flex()
                    .justify_center()
                    .text_color(title_color)
                    .child(center_text),
            )
            .child(right_section)
    }
}

fn render_overview_buttons(
    app_state: &Entity<AppState>,
    font_size: f32,
    theme: &Theme,
) -> gpui::Div {
    let app_state_q = app_state.clone();
    let text_muted: Hsla = theme.text_muted.into();
    let text: Hsla = theme.text.into();
    let icon_size = px(font_size + 1.0);

    div()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(8.))
        .pr(px(12.))
        // Questions button
        .child(
            div()
                .px(px(6.))
                .py(px(4.))
                .rounded(px(4.))
                .cursor_pointer()
                .hover(move |s| s.text_color(text))
                .child(
                    svg()
                        .path(icons::ICON_QUESTION)
                        .size(icon_size)
                        .text_color(text_muted)
                        .flex_shrink_0(),
                )
                .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                    app_state_q.update(cx, AppState::toggle_questions_queue);
                }),
        )
        // Settings button
        .child(
            div()
                .px(px(6.))
                .py(px(4.))
                .rounded(px(4.))
                .cursor_pointer()
                .hover(move |s| s.text_color(text))
                .child(
                    svg()
                        .path(icons::ICON_SETTINGS)
                        .size(icon_size)
                        .text_color(text_muted)
                        .flex_shrink_0(),
                )
                .on_mouse_down(MouseButton::Left, |_, window, cx| {
                    window.dispatch_action(Box::new(OpenSettings), cx);
                }),
        )
}

fn render_settings_buttons(
    app_state: &Entity<AppState>,
    font_size: f32,
    theme: &Theme,
) -> gpui::Div {
    let text_muted: Hsla = theme.text_muted.into();
    let text: Hsla = theme.text.into();
    let border = theme.border;
    let border_variant = theme.border_variant;
    let app_state = app_state.clone();

    div()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(8.))
        .pr(px(12.))
        .child(
            div()
                .id("settings-save-btn")
                .flex()
                .flex_row()
                .items_center()
                .px(px(10.))
                .py(px(3.))
                .rounded(px(16.))
                .border_1()
                .border_color(theme.text_disabled)
                .cursor_pointer()
                .text_size(px(font_size - 1.0))
                .text_color(text_muted)
                .hover(move |s| s.bg(border).border_color(border_variant).text_color(text))
                .on_click(move |_, _, cx| {
                    app_state.update(cx, AppState::close_settings);
                })
                .child("Save"),
        )
}

fn render_focus_buttons(
    app_state_for_back: Entity<AppState>,
    app_state_for_close: Entity<AppState>,
    font_size: f32,
    theme: &Theme,
) -> gpui::Div {
    let text_muted: Hsla = theme.text_muted.into();
    let text: Hsla = theme.text.into();
    let destructive_hover: Hsla = theme.destructive_hover.into();
    let icon_size = px(font_size + 1.0);

    div()
        .flex()
        .flex_row()
        .gap(px(8.))
        .pr(px(12.))
        // Minimize button (return to overview)
        .child(
            div()
                .px(px(6.))
                .py(px(4.))
                .rounded(px(4.))
                .cursor_pointer()
                .hover(move |s| s.text_color(text))
                .child(
                    svg()
                        .path(icons::ICON_BACK)
                        .size(icon_size)
                        .text_color(text_muted)
                        .flex_shrink_0(),
                )
                .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                    app_state_for_back.update(cx, AppState::return_to_overview);
                }),
        )
        // Close button (red on hover)
        .child(
            div()
                .px(px(6.))
                .py(px(4.))
                .rounded(px(4.))
                .cursor_pointer()
                .hover(move |s| s.text_color(destructive_hover))
                .child(
                    svg()
                        .path(icons::ICON_CLOSE_CIRCLE)
                        .size(icon_size)
                        .text_color(text_muted)
                        .flex_shrink_0(),
                )
                .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                    let (id, title) = {
                        let s = app_state_for_close.read(cx);
                        let fid = s
                            .focused_instance_id(cx)
                            .unwrap_or_default()
                            .to_owned();
                        let il = s.instance_list.read(cx);
                        let t = il
                            .find_by_id(&fid, cx)
                            .map(|e| {
                                e.read(cx)
                                    .instance
                                    .title
                                    .clone()
                                    .unwrap_or_else(|| "Untitled".into())
                            })
                            .unwrap_or_default();
                        (fid, t)
                    };
                    app_state_for_close.update(cx, |s, cx| {
                        s.request_close_instance(&id, &title, cx);
                    });
                }),
        )
}
