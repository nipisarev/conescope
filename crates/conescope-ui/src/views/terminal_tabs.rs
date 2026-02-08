use conescope_core::instance::{InstanceType, TerminalTab};
use gpui::prelude::*;
use gpui::{div, px, rgba};

use crate::theme::Theme;

/// Terminal icon prefix for tab labels.
const TERMINAL_ICON: &str = "\u{25B8}"; // ▸ small right-pointing triangle

/// Border color shared across tab bar elements.
const BORDER_COLOR: u32 = 0x3c3c_3cff;

/// Render a terminal tab bar with raised-tab pattern: `_|‾|__`
///
/// Active tab has top+left+right borders, no bottom border (connects to content).
/// Inactive tabs and spacers carry a bottom border (the baseline).
#[allow(clippy::too_many_arguments)]
pub fn render_tab_bar(
    instance_type: InstanceType,
    active_tab: TerminalTab,
    has_shell: bool,
    on_click_primary: impl Fn(&gpui::MouseDownEvent, &mut gpui::Window, &mut gpui::App) + 'static,
    on_click_shell: impl Fn(&gpui::MouseDownEvent, &mut gpui::Window, &mut gpui::App) + 'static,
    on_click_add: impl Fn(&gpui::MouseDownEvent, &mut gpui::Window, &mut gpui::App) + 'static,
    theme: &Theme,
) -> gpui::Div {
    let active_bg = theme.terminal_bg;
    let inactive_bg = theme.background;

    let bar = div()
        .h(px(28.))
        .flex()
        .flex_row()
        .items_end()
        .bg(inactive_bg);

    let primary_label = match instance_type {
        InstanceType::Project => "Claude",
        InstanceType::Terminal => "Terminal",
    };

    // Left padding spacer with bottom border
    let mut tabs = bar.child(border_b_spacer().w(px(8.)));

    tabs = tabs.child(render_tab(
        primary_label,
        active_tab == TerminalTab::Primary,
        on_click_primary,
        active_bg,
        inactive_bg,
    ));

    // Show Shell tab for Project instances always, for Terminal instances only when spawned
    let show_shell = instance_type == InstanceType::Project || has_shell;
    if show_shell {
        tabs = tabs.child(render_tab(
            "Shell",
            active_tab == TerminalTab::Shell,
            on_click_shell,
            active_bg,
            inactive_bg,
        ));
    }

    // Flex spacer with bottom border (continues the baseline)
    tabs.child(border_b_spacer().flex_1())
        // "+" button wrapped with bottom border
        .child(
            div()
                .h_full()
                .flex()
                .items_center()
                .border_b_1()
                .border_color(rgba(BORDER_COLOR))
                .child(
                    div()
                        .px(px(6.))
                        .py(px(2.))
                        .cursor_pointer()
                        .text_size(px(14.))
                        .text_color(rgba(0x6666_66ff))
                        .hover(|s| s.text_color(rgba(0xffff_ffff)).bg(rgba(0x3c3c_3cff)))
                        .on_mouse_down(gpui::MouseButton::Left, on_click_add)
                        .child("+"),
                ),
        )
}

/// Small spacer div with only a bottom border (the baseline).
fn border_b_spacer() -> gpui::Div {
    div().h_full().border_b_1().border_color(rgba(BORDER_COLOR))
}

fn render_tab(
    label: &str,
    active: bool,
    on_click: impl Fn(&gpui::MouseDownEvent, &mut gpui::Window, &mut gpui::App) + 'static,
    active_bg: gpui::Rgba,
    inactive_bg: gpui::Rgba,
) -> gpui::Div {
    let fg = if active {
        rgba(0xd4d4_d4ff)
    } else {
        rgba(0x7777_77ff)
    };

    let text = format!("{TERMINAL_ICON} {label}");

    let bg = if active { active_bg } else { inactive_bg };

    let base = div()
        .h_full()
        .flex()
        .items_center()
        .px(px(12.))
        .text_size(px(12.))
        .text_color(fg)
        .bg(bg)
        .cursor_pointer()
        .on_mouse_down(gpui::MouseButton::Left, on_click)
        .child(text);

    if active {
        base.border_l_1()
            .border_r_1()
            .border_color(rgba(BORDER_COLOR))
    } else {
        base.border_b_1()
            .border_color(rgba(BORDER_COLOR))
            .hover(|s| s.bg(rgba(0x3333_33ff)))
    }
}
