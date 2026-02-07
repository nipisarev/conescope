use gpui::prelude::*;
use gpui::{Entity, MouseButton, SharedString, div, px, rgba};

use conescope_core::instance::InstanceType;

use crate::state::app_state::AppState;
use crate::state::settings_store::ViewMode;
use crate::views::colors::hex_to_rgba;

struct InstanceTooltip {
    title: SharedString,
    status: SharedString,
}

impl Render for InstanceTooltip {
    fn render(
        &mut self,
        _window: &mut gpui::Window,
        _cx: &mut gpui::Context<Self>,
    ) -> impl IntoElement {
        div()
            .bg(rgba(0x3333_33ff))
            .border_1()
            .border_color(rgba(0x5555_55ff))
            .rounded(px(4.))
            .px(px(8.))
            .py(px(4.))
            .flex()
            .flex_col()
            .gap(px(2.))
            .child(
                div()
                    .text_color(rgba(0xdddd_ddff))
                    .text_size(px(12.))
                    .child(self.title.clone()),
            )
            .child(
                div()
                    .text_color(rgba(0x8888_88ff))
                    .text_size(px(11.))
                    .child(self.status.clone()),
            )
    }
}

#[derive(Debug)]
pub struct ActivityBar {
    app_state: Entity<AppState>,
}

impl ActivityBar {
    #[must_use]
    pub fn new(app_state: Entity<AppState>) -> Self {
        Self { app_state }
    }
}

struct InstanceButton {
    id: String,
    num: i64,
    color: gpui::Rgba,
    is_focused: bool,
    title: String,
    status: String,
}

fn render_instance_button(
    btn: &InstanceButton,
    app_state: &Entity<AppState>,
) -> gpui::Stateful<gpui::Div> {
    let app_state_btn = app_state.clone();
    let tooltip_title: SharedString = btn.title.clone().into();
    let tooltip_status: SharedString = btn.status.clone().into();
    let element_id: SharedString = format!("inst-btn-{}", btn.num).into();

    div()
        .id(element_id)
        .px(px(6.))
        .py(px(1.))
        .rounded(px(3.))
        .cursor_pointer()
        .text_size(px(11.))
        .when(btn.is_focused, |el| el.bg(rgba(0x0e63_9cff)))
        .when(!btn.is_focused, |el| el.hover(|s| s.bg(rgba(0x3333_33ff))))
        .text_color(btn.color)
        .child(format!("{}", btn.num))
        .on_click({
            let id = btn.id.clone();
            move |_, _, cx| {
                app_state_btn.update(cx, |s, cx| s.focus_instance(&id, cx));
            }
        })
        .tooltip(move |_window, cx| {
            cx.new(|_| InstanceTooltip {
                title: tooltip_title.clone(),
                status: tooltip_status.clone(),
            })
            .into()
        })
}

/// Render a panel toggle icon for the activity bar.
fn render_panel_toggle(
    icon: &str,
    active: bool,
    app_state: Entity<AppState>,
    toggle_fn: fn(&mut AppState, &mut gpui::Context<AppState>),
) -> gpui::Div {
    let fg = if active {
        rgba(0x0e63_9cff) // blue when active
    } else {
        rgba(0x5555_55ff) // gray when inactive
    };

    div()
        .px(px(4.))
        .py(px(2.))
        .rounded(px(3.))
        .cursor_pointer()
        .text_size(px(12.))
        .text_color(fg)
        .hover(|s| s.text_color(rgba(0xcccc_ccff)))
        .on_mouse_down(MouseButton::Left, move |_, _, cx| {
            app_state.update(cx, toggle_fn);
        })
        .child(icon.to_owned())
}

/// Collect instance buttons and aggregate token/cost stats.
fn collect_instance_data(
    app_state: &Entity<AppState>,
    cx: &gpui::App,
) -> (Vec<InstanceButton>, i64, f64) {
    let state = app_state.read(cx);
    let il = state.instance_list.read(cx);
    let focused_id = state.focused_instance_id(cx).map(str::to_owned);
    let mut buttons = Vec::new();
    let mut total_tokens: i64 = 0;
    let mut total_cost: f64 = 0.0;

    for entry in il.entries() {
        let inst = entry.read(cx);
        let num = inst.instance.instance_number.unwrap_or(0);
        let color = inst
            .instance
            .color
            .as_deref()
            .map_or_else(|| rgba(0x6464_b5f6), hex_to_rgba);
        let is_focused = focused_id.as_deref() == Some(inst.id());
        let title = inst
            .instance
            .title
            .clone()
            .unwrap_or_else(|| format!("Instance {num}"));
        let status = inst.status().as_str().to_owned();
        buttons.push(InstanceButton {
            id: inst.id().to_owned(),
            num,
            color,
            is_focused,
            title,
            status,
        });
        total_tokens += inst.instance.tokens_used;
        total_cost += inst.instance.cost_estimate;
    }
    buttons.sort_by_key(|b| b.num);
    (buttons, total_tokens, total_cost)
}

struct PanelState {
    sidebar: bool,
    editor: bool,
    terminal: bool,
}

/// Build the left section of the activity bar.
fn build_left_section(
    view_mode: ViewMode,
    focused_type: Option<InstanceType>,
    buttons: &[InstanceButton],
    panels: &PanelState,
    app_state: &Entity<AppState>,
) -> gpui::Div {
    let app_state_grid = app_state.clone();

    let mut left = div()
        .flex_1()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(4.))
        .child(
            div()
                .px(px(4.))
                .py(px(2.))
                .rounded(px(3.))
                .cursor_pointer()
                .text_size(px(12.))
                .when(view_mode == ViewMode::Overview, |el| {
                    el.bg(rgba(0x0e63_9cff)).text_color(rgba(0xffff_ffff))
                })
                .when(view_mode != ViewMode::Overview, |el| {
                    el.text_color(rgba(0x6666_66ff))
                        .hover(|s| s.bg(rgba(0x3c3c_3cff)).text_color(rgba(0xcccc_ccff)))
                })
                .child("\u{25a6}")
                .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                    if view_mode == ViewMode::Focus {
                        app_state_grid.update(cx, AppState::return_to_overview);
                    }
                }),
        );

    match view_mode {
        ViewMode::Focus => {
            // Only show panel toggles for Project instances (not Terminal)
            if focused_type != Some(InstanceType::Terminal) {
                left = left.child(div().w(px(1.)).h(px(16.)).mx(px(4.)).bg(rgba(0x3c3c_3cff)));
                left = left
                    .child(render_panel_toggle(
                        "\u{1F4C1}",
                        panels.sidebar,
                        app_state.clone(),
                        AppState::toggle_sidebar,
                    ))
                    .child(render_panel_toggle(
                        "\u{2338}",
                        panels.editor,
                        app_state.clone(),
                        AppState::toggle_editor,
                    ))
                    .child(render_panel_toggle(
                        "\u{25B8}",
                        panels.terminal,
                        app_state.clone(),
                        AppState::toggle_terminal,
                    ));
            }
        }
        ViewMode::Overview => {
            for btn in buttons {
                left = left.child(render_instance_button(btn, app_state));
            }
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
        let editor_visible = state.editor_visible(cx);
        let terminal_visible = state.terminal_visible(cx);

        // Determine focused instance type (for hiding panel toggles on Terminal)
        let focused_type = state.focused_instance_id(cx).and_then(|id| {
            state
                .instance_list
                .read(cx)
                .find_by_id(id, cx)
                .map(|e| e.read(cx).instance_type())
        });

        let (buttons, total_tokens, total_cost) = collect_instance_data(&self.app_state, cx);

        let token_text = format!("{}k tokens", total_tokens / 1000);
        let cost_text = format!("${total_cost:.2}");

        let panels = PanelState {
            sidebar: sidebar_visible,
            editor: editor_visible,
            terminal: terminal_visible,
        };
        let left = build_left_section(view_mode, focused_type, &buttons, &panels, &self.app_state);

        div()
            .h(px(28.))
            .w_full()
            .flex()
            .flex_row()
            .items_center()
            .px(px(8.))
            .bg(rgba(0x1818_18ff))
            .border_t_1()
            .border_color(rgba(0x3c3c_3cff))
            .child(left)
            .child(
                div()
                    .flex()
                    .flex_row()
                    .gap(px(12.))
                    .text_color(rgba(0x6666_66ff))
                    .text_size(px(11.))
                    .child(token_text)
                    .child(cost_text),
            )
    }
}
