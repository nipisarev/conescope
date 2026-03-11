use std::rc::Rc;

use conescope_core::instance::{InstanceType, TerminalTab};
use gpui::prelude::*;
use gpui::{SharedString, div, px, rgba};

use crate::theme::Theme;

/// Callback type for shell tab events (click/close) taking a shell tab ID.
pub type ShellTabCb = Rc<dyn Fn(usize, &gpui::MouseDownEvent, &mut gpui::Window, &mut gpui::App)>;

const TERMINAL_ICON: &str = "\u{25B8}";

#[allow(clippy::too_many_arguments)]
pub fn render_tab_bar(
    instance_type: InstanceType,
    active_tab: TerminalTab,
    shell_tabs: &[(usize, bool)],
    on_click_primary: impl Fn(&gpui::MouseDownEvent, &mut gpui::Window, &mut gpui::App) + 'static,
    on_click_shell: &ShellTabCb,
    on_close_shell: &ShellTabCb,
    on_click_add: impl Fn(&gpui::MouseDownEvent, &mut gpui::Window, &mut gpui::App) + 'static,
    font_size: f32,
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

    let border = theme.border;

    let mut tabs = bar.child(border_b_spacer(border).w(px(8.)));

    tabs = tabs.child(render_tab(
        primary_label,
        active_tab == TerminalTab::Primary,
        false,
        on_click_primary,
        None::<Box<dyn Fn(&gpui::MouseDownEvent, &mut gpui::Window, &mut gpui::App)>>,
        font_size,
        active_bg,
        inactive_bg,
        border,
    ));

    // Render N shell tabs
    for (i, &(id, _alive)) in shell_tabs.iter().enumerate() {
        let label = if i == 0 {
            "Shell".to_owned()
        } else {
            format!("Shell {}", i + 1)
        };
        let is_active = active_tab == TerminalTab::Shell(id);

        let on_click = on_click_shell.clone();
        let on_close = on_close_shell.clone();

        tabs = tabs.child(render_tab(
            &label,
            is_active,
            true,
            move |ev, window, cx| on_click(id, ev, window, cx),
            Some(
                move |ev: &gpui::MouseDownEvent, window: &mut gpui::Window, cx: &mut gpui::App| {
                    on_close(id, ev, window, cx);
                },
            ),
            font_size,
            active_bg,
            inactive_bg,
            border,
        ));
    }

    // Share the add callback between the spacer area and the "+" button
    let on_click_add = Rc::new(on_click_add);
    let on_click_add_spacer = on_click_add.clone();

    tabs.child(
        div()
            .id("tab-bar-spacer")
            .h_full()
            .flex_1()
            .border_b_1()
            .border_color(border)
            .on_mouse_down(gpui::MouseButton::Left, move |ev, window, cx| {
                on_click_add_spacer(ev, window, cx);
            }),
    )
    .child(
        div()
            .h_full()
            .flex()
            .items_center()
            .border_b_1()
            .border_color(border)
            .child(
                div()
                    .px(px(6.))
                    .py(px(2.))
                    .cursor_pointer()
                    .text_size(px(font_size + 2.0))
                    .text_color(rgba(0x6666_66ff))
                    .hover(|s| s.text_color(rgba(0xffff_ffff)).bg(rgba(0x3c3c_3cff)))
                    .on_mouse_down(gpui::MouseButton::Left, move |ev, window, cx| {
                        on_click_add(ev, window, cx);
                    })
                    .child("+"),
            ),
    )
}

fn border_b_spacer(border: gpui::Rgba) -> gpui::Div {
    div().h_full().border_b_1().border_color(border)
}

#[allow(clippy::too_many_arguments)]
fn render_tab(
    label: &str,
    active: bool,
    closeable: bool,
    on_click: impl Fn(&gpui::MouseDownEvent, &mut gpui::Window, &mut gpui::App) + 'static,
    on_close: Option<impl Fn(&gpui::MouseDownEvent, &mut gpui::Window, &mut gpui::App) + 'static>,
    font_size: f32,
    active_bg: gpui::Rgba,
    inactive_bg: gpui::Rgba,
    border: gpui::Rgba,
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
        .text_size(px(font_size))
        .text_color(fg)
        .bg(bg)
        .cursor_pointer()
        .on_mouse_down(gpui::MouseButton::Left, on_click)
        .child(text);

    // Add close button for closeable tabs
    let base = if closeable {
        if let Some(on_close) = on_close {
            base.child(
                div()
                    .id(SharedString::from(format!("close-tab-{label}")))
                    .ml(px(6.))
                    .text_size(px(font_size - 2.0))
                    .text_color(rgba(0x6666_66ff))
                    .hover(|s| s.text_color(rgba(0xffff_ffff)))
                    .cursor_pointer()
                    .on_mouse_down(gpui::MouseButton::Left, move |ev, window, cx| {
                        cx.stop_propagation();
                        on_close(ev, window, cx);
                    })
                    .child("\u{00D7}"), // × multiplication sign
            )
        } else {
            base
        }
    } else {
        base
    };

    if active {
        base.border_l_1().border_r_1().border_color(border)
    } else {
        base.border_b_1()
            .border_color(border)
            .hover(|s| s.bg(rgba(0x3333_33ff)))
    }
}
