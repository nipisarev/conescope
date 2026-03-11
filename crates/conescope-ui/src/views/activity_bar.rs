use gpui::prelude::*;
use gpui::{Entity, Hsla, MouseButton, div, px, svg};

use conescope_core::instance::InstanceType;

use crate::icons;
use crate::state::app_state::AppState;
use crate::state::settings_store::{SidebarTab, ViewMode};
use crate::theme::Theme;

#[derive(Debug)]
pub struct ActivityBar {
    app_state: Entity<AppState>,
}

impl ActivityBar {
    #[must_use]
    pub fn new(app_state: Entity<AppState>, _cx: &mut gpui::Context<Self>) -> Self {
        Self { app_state }
    }
}

/// Render a panel toggle icon for the activity bar.
#[allow(clippy::too_many_arguments)]
fn render_panel_toggle(
    icon_path: &'static str,
    active: bool,
    app_state: Entity<AppState>,
    toggle_fn: fn(&mut AppState, &mut gpui::Context<AppState>),
    font_size: f32,
    theme: &Theme,
) -> gpui::Div {
    let fg: Hsla = if active {
        theme.accent.into()
    } else {
        theme.text_muted.into()
    };
    let hover_fg: Hsla = theme.text.into();

    div()
        .px(px(4.))
        .py(px(2.))
        .rounded(px(3.))
        .cursor_pointer()
        .hover(move |s| s.text_color(hover_fg))
        .on_mouse_down(MouseButton::Left, move |_, _, cx| {
            app_state.update(cx, toggle_fn);
        })
        .child(
            svg()
                .path(icon_path)
                .size(px(font_size + 1.0))
                .text_color(fg)
                .flex_shrink_0(),
        )
}

/// Aggregate token stats across all instances.
fn collect_total_tokens(app_state: &Entity<AppState>, cx: &gpui::App) -> i64 {
    let state = app_state.read(cx);
    let il = state.instance_list.read(cx);
    il.entries()
        .iter()
        .map(|e| e.read(cx).instance.tokens_used)
        .sum()
}

struct PanelState {
    sidebar: bool,
    editor: bool,
    terminal: bool,
    sidebar_tab: SidebarTab,
}

/// Build the left section of the activity bar.
#[allow(clippy::too_many_arguments)]
fn build_left_section(
    view_mode: ViewMode,
    focused_type: Option<InstanceType>,
    panels: &PanelState,
    app_state: &Entity<AppState>,
    font_size: f32,
    theme: &Theme,
) -> gpui::Div {
    let app_state_grid = app_state.clone();
    let accent = theme.accent;
    let text_faint = theme.text_faint;
    let text = theme.text;

    let is_overview = view_mode == ViewMode::Overview;
    let is_settings = view_mode == ViewMode::Settings;
    let show_solid = is_overview || is_settings;
    let (icon_path, icon_color): (&str, Hsla) = if show_solid {
        (icons::ICON_CONESCOPE_SOLID, accent.into())
    } else {
        (icons::ICON_CONESCOPE_OUTLINE, text_faint.into())
    };
    let hover_color: Hsla = text.into();
    let logo_size = px(font_size + 3.0);
    let separator_h = px(font_size + 3.0);

    let mut left = div()
        .flex_1()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(4.))
        .child(
            div()
                .px(px(2.))
                .py(px(1.))
                .rounded(px(3.))
                .cursor_pointer()
                .text_color(icon_color)
                .when(!show_solid, move |el| {
                    el.hover(move |s| s.text_color(hover_color))
                })
                .child(
                    svg()
                        .path(icon_path)
                        .size(logo_size)
                        .text_color(icon_color)
                        .flex_shrink_0(),
                )
                .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                    if view_mode == ViewMode::Focus {
                        app_state_grid.update(cx, AppState::return_to_overview);
                    } else if view_mode == ViewMode::Settings {
                        app_state_grid.update(cx, AppState::close_settings);
                    }
                }),
        );

    match view_mode {
        ViewMode::Focus => {
            if focused_type != Some(InstanceType::Terminal) {
                left = left.child(div().w(px(1.)).h(separator_h).mx(px(4.)).bg(theme.border));
                left = left
                    .child(render_panel_toggle(
                        icons::ICON_SIDEBAR,
                        panels.sidebar && panels.sidebar_tab == SidebarTab::Files,
                        app_state.clone(),
                        AppState::toggle_sidebar,
                        font_size,
                        theme,
                    ))
                    .child(render_panel_toggle(
                        icons::ICON_GIT,
                        panels.sidebar && panels.sidebar_tab == SidebarTab::Git,
                        app_state.clone(),
                        AppState::toggle_git_panel,
                        font_size,
                        theme,
                    ))
                    .child(render_panel_toggle(
                        icons::ICON_EDITOR,
                        panels.editor,
                        app_state.clone(),
                        AppState::toggle_editor,
                        font_size,
                        theme,
                    ))
                    .child(render_panel_toggle(
                        icons::ICON_TERMINAL,
                        panels.terminal,
                        app_state.clone(),
                        AppState::toggle_terminal,
                        font_size,
                        theme,
                    ));
            }
        }
        ViewMode::Overview | ViewMode::Settings => {
            // No instance buttons or panel toggles outside focus mode
        }
    }

    left
}

impl Render for ActivityBar {
    fn render(
        &mut self,
        _window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> impl IntoElement {
        let state = self.app_state.read(cx);
        let view_mode = state.view_mode(cx);
        let sidebar_visible = state.sidebar_visible(cx);
        let sidebar_tab = state.sidebar_tab(cx);
        let editor_visible = state.editor_visible(cx);
        let terminal_visible = state.terminal_visible(cx);
        let font_size = state.settings_store.read(cx).settings().ui_font_size();

        // Determine focused instance type (for hiding panel toggles on Terminal)
        let focused_type = state.focused_instance_id(cx).and_then(|id| {
            state
                .instance_list
                .read(cx)
                .find_by_id(id, cx)
                .map(|e| e.read(cx).instance_type())
        });

        let theme = state.theme().clone();

        let total_tokens = collect_total_tokens(&self.app_state, cx);
        let token_text = format!("{}k tokens", total_tokens / 1000);

        let panels = PanelState {
            sidebar: sidebar_visible,
            editor: editor_visible,
            terminal: terminal_visible,
            sidebar_tab,
        };
        let left = build_left_section(
            view_mode,
            focused_type,
            &panels,
            &self.app_state,
            font_size,
            &theme,
        );

        let bar_height = px(font_size * 2.0 + 4.0);

        div()
            .h(bar_height)
            .w_full()
            .flex()
            .flex_row()
            .items_center()
            .px(px(8.))
            .bg(theme.background)
            .rounded_bl(px(10.))
            .rounded_br(px(10.))
            .border_t_1()
            .border_color(theme.border)
            .child(left)
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(12.))
                    .text_color(theme.text_faint)
                    .text_size(px(font_size - 1.0))
                    .child(token_text),
            )
    }
}
