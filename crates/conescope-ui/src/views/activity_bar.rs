use gpui::prelude::*;
use gpui::{Entity, Hsla, MouseButton, div, px, svg};

use conescope_core::instance::InstanceType;

use crate::icons;
use crate::state::app_state::AppState;
use crate::state::session_detector::SessionStatus;
use crate::state::settings_store::{SidebarTab, ViewMode};
use crate::theme::Theme;

#[derive(Debug)]
pub struct ActivityBar {
    app_state: Entity<AppState>,
    cached_statuses: Vec<(String, SessionStatus)>,
    cached_pulse_opacity: f32,
}

impl ActivityBar {
    #[must_use]
    pub fn new(app_state: Entity<AppState>, cx: &mut gpui::Context<Self>) -> Self {
        let pulse_timer = app_state.read(cx).pulse_timer.clone();
        cx.observe(&pulse_timer, |this, timer, cx| {
            this.cached_pulse_opacity = timer.read(cx).opacity();
            cx.notify();
        })
        .detach();

        let instance_list = app_state.read(cx).instance_list.clone();
        cx.observe(&instance_list, |this, _list, cx| {
            this.update_cached_statuses(cx);
            cx.notify();
        })
        .detach();

        let mut bar = Self {
            app_state,
            cached_statuses: Vec::new(),
            cached_pulse_opacity: 1.0,
        };
        bar.update_cached_statuses(cx);
        bar.cached_pulse_opacity = pulse_timer.read(cx).opacity();
        bar
    }

    fn update_cached_statuses(&mut self, cx: &gpui::App) {
        let state = self.app_state.read(cx);
        let il = state.instance_list.read(cx);
        self.cached_statuses = il
            .entries()
            .iter()
            .map(|e| {
                let entry = e.read(cx);
                (entry.id().to_owned(), entry.session_status())
            })
            .collect();
    }
}

fn status_color(status: SessionStatus) -> gpui::Rgba {
    match status {
        SessionStatus::Working => gpui::rgba(0x4ade_80ff),
        SessionStatus::Question => gpui::rgba(0xf871_71ff),
        SessionStatus::Waiting | SessionStatus::Finished => gpui::rgba(0xfacc_15ff),
        SessionStatus::Stopped => gpui::rgba(0x9ca3_afff),
    }
}

fn status_label(status: SessionStatus) -> Option<&'static str> {
    match status {
        SessionStatus::Question => Some("Q"),
        SessionStatus::Waiting => Some("W"),
        SessionStatus::Finished => Some("F"),
        _ => None,
    }
}

fn render_status_badge(status: SessionStatus, pulse_opacity: f32, font_size: f32) -> gpui::Div {
    let color = status_color(status);
    let opacity = if status.is_pulsing() {
        pulse_opacity
    } else {
        1.0
    };
    let badge_size = px((font_size * 0.55).max(6.0));

    let badge = div()
        .absolute()
        .bottom(px(-1.))
        .right(px(-1.))
        .size(badge_size)
        .rounded(badge_size)
        .bg(color)
        .opacity(opacity);

    if let Some(label) = status_label(status) {
        badge
            .flex()
            .items_center()
            .justify_center()
            .text_size(px((font_size * 0.35).max(5.0)))
            .text_color(gpui::rgba(0x0000_00ff))
            .child(label)
    } else {
        badge
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

/// Aggregate token/cost stats across all instances.
fn collect_token_stats(app_state: &Entity<AppState>, cx: &gpui::App) -> (i64, f64) {
    let state = app_state.read(cx);
    let il = state.instance_list.read(cx);
    let mut total_tokens: i64 = 0;
    let mut total_cost: f64 = 0.0;

    for entry in il.entries() {
        let inst = entry.read(cx);
        total_tokens += inst.instance.tokens_used;
        total_cost += inst.instance.cost_estimate;
    }
    (total_tokens, total_cost)
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

        let (total_tokens, total_cost) = collect_token_stats(&self.app_state, cx);

        let token_text = format!("{}k tokens", total_tokens / 1000);
        let cost_text = format!("${total_cost:.2}");

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

        let icon_size = px((font_size * 0.85).max(10.0));
        let pulse = self.cached_pulse_opacity;
        let mut status_dots = div().flex().flex_row().items_center().gap(px(4.));
        for (_id, status) in &self.cached_statuses {
            let icon_wrapper = div()
                .relative()
                .size(icon_size)
                .rounded(icon_size)
                .bg(theme.panel)
                .child(render_status_badge(*status, pulse, font_size));
            status_dots = status_dots.child(icon_wrapper);
        }

        div()
            .h(bar_height)
            .w_full()
            .flex()
            .flex_row()
            .items_center()
            .px(px(8.))
            .bg(theme.background)
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
                    .child(status_dots)
                    .child(token_text)
                    .child(cost_text),
            )
    }
}
