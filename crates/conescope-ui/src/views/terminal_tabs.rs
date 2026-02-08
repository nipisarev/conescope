use conescope_core::instance::{InstanceType, TerminalTab};
use gpui::prelude::*;
use gpui::{div, px, rgba};

/// Terminal icon prefix for tab labels.
const TERMINAL_ICON: &str = "\u{25B8}"; // ▸ small right-pointing triangle

/// Render a terminal tab bar.
///
/// Project instances: "Claude" + "Shell" tabs.
/// Terminal instances: "Terminal" tab + "Shell" tab (when shell exists).
/// Both types get a "+" button to spawn additional shell terminals.
#[allow(clippy::too_many_arguments)]
pub fn render_tab_bar(
    instance_type: InstanceType,
    active_tab: TerminalTab,
    has_shell: bool,
    on_click_primary: impl Fn(&gpui::MouseDownEvent, &mut gpui::Window, &mut gpui::App) + 'static,
    on_click_shell: impl Fn(&gpui::MouseDownEvent, &mut gpui::Window, &mut gpui::App) + 'static,
    on_click_add: impl Fn(&gpui::MouseDownEvent, &mut gpui::Window, &mut gpui::App) + 'static,
) -> gpui::Div {
    // Terminal tabs don't have direct access to theme; use simple hardcoded values
    // that work for both dark/light (these are fairly neutral).
    let bar = div()
        .h(px(28.))
        .flex()
        .flex_row()
        .items_center()
        .gap(px(1.))
        .px(px(8.))
        .border_b_1()
        .border_color(rgba(0x3c3c_3cff))
        .bg(rgba(0x1e1e_1eff));

    let primary_label = match instance_type {
        InstanceType::Project => "Claude",
        InstanceType::Terminal => "Terminal",
    };

    let mut tabs = bar.child(render_tab(
        primary_label,
        active_tab == TerminalTab::Primary,
        on_click_primary,
    ));

    // Show Shell tab for Project instances always, for Terminal instances only when spawned
    let show_shell = instance_type == InstanceType::Project || has_shell;
    if show_shell {
        tabs = tabs.child(render_tab(
            "Shell",
            active_tab == TerminalTab::Shell,
            on_click_shell,
        ));
    }

    // "+" button on the right
    tabs.child(div().flex_1()) // spacer
        .child(
            div()
                .px(px(6.))
                .py(px(2.))
                .rounded(px(4.))
                .cursor_pointer()
                .text_size(px(14.))
                .text_color(rgba(0x6666_66ff))
                .hover(|s| s.text_color(rgba(0xffff_ffff)).bg(rgba(0x3c3c_3cff)))
                .on_mouse_down(gpui::MouseButton::Left, on_click_add)
                .child("+"),
        )
}

fn render_tab(
    label: &str,
    active: bool,
    on_click: impl Fn(&gpui::MouseDownEvent, &mut gpui::Window, &mut gpui::App) + 'static,
) -> gpui::Div {
    let bg = if active {
        rgba(0x2d2d_2dff)
    } else {
        rgba(0x1e1e_1eff)
    };
    let fg = if active {
        rgba(0xd4d4_d4ff)
    } else {
        rgba(0x7777_77ff)
    };

    let text = format!("{TERMINAL_ICON} {label}");

    div()
        .px(px(12.))
        .py(px(4.))
        .text_size(px(12.))
        .text_color(fg)
        .bg(bg)
        .rounded(px(4.))
        .cursor_pointer()
        .hover(|s| s.bg(rgba(0x3333_33ff)))
        .on_mouse_down(gpui::MouseButton::Left, on_click)
        .child(text)
}
