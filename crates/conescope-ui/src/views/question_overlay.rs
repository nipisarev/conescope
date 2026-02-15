use gpui::prelude::*;
use gpui::{AnyElement, Entity, MouseButton, SharedString, div, px};

use crate::state::app_state::AppState;
use crate::state::session_detector::SessionEvent;
use crate::theme::Theme;

#[must_use]
pub fn render_question_overlay(
    event: &SessionEvent,
    instance_id: &str,
    app_state: &Entity<AppState>,
    theme: &Theme,
) -> AnyElement {
    match event {
        SessionEvent::Question {
            text, choices, ..
        } => render_question_card(text, choices, instance_id, app_state.clone(), theme),
        SessionEvent::WaitingForInput => render_badge("Waiting...", theme),
        SessionEvent::Finished => render_badge("Finished", theme),
    }
}

#[allow(clippy::needless_pass_by_value)]
fn render_question_card(
    text: &str,
    choices: &[String],
    instance_id: &str,
    app_state: Entity<AppState>,
    theme: &Theme,
) -> AnyElement {
    let mut card = div()
        .flex()
        .flex_col()
        .gap(px(6.))
        .bg(theme.surface)
        .border_1()
        .border_color(theme.border)
        .rounded(px(6.))
        .px(px(10.))
        .py(px(8.))
        .max_w(px(320.))
        .child(
            div()
                .text_size(px(12.))
                .text_color(theme.text)
                .child(text.to_owned()),
        );

    for (i, choice) in choices.iter().enumerate() {
        let label = format!("{}. {}", i + 1, choice);
        let iid = instance_id.to_owned();
        let state = app_state.clone();
        let btn_id = SharedString::from(format!("choice-{iid}-{i}"));

        card = card.child(
            div()
                .id(btn_id)
                .cursor_pointer()
                .px(px(6.))
                .py(px(3.))
                .rounded(px(4.))
                .text_size(px(11.))
                .text_color(theme.text)
                .bg(theme.element_hover)
                .hover(|s| s.bg(theme.accent))
                .on_click(move |_, _, cx| {
                    state.update(cx, |s, cx| {
                        s.answer_instance_question(&iid, i, cx);
                    });
                })
                .child(label),
        );
    }

    // "More context" button to focus the instance
    let focus_id = instance_id.to_owned();
    let focus_state = app_state.clone();
    card = card.child(
        div()
            .id(SharedString::from(format!(
                "more-ctx-{focus_id}"
            )))
            .cursor_pointer()
            .pt(px(2.))
            .text_size(px(10.))
            .text_color(theme.accent)
            .on_click(move |_, _, cx| {
                focus_state.update(cx, |s, cx| {
                    s.focus_instance(&focus_id, cx);
                });
            })
            .child("More context \u{2192}"),
    );

    // Backdrop + centered card
    div()
        .absolute()
        .top_0()
        .left_0()
        .size_full()
        .bg(theme.backdrop)
        .flex()
        .items_center()
        .justify_center()
        .on_mouse_down(MouseButton::Left, |_, _, cx| {
            cx.stop_propagation();
        })
        .child(card)
        .into_any_element()
}

fn render_badge(label: &str, theme: &Theme) -> AnyElement {
    let badge_bg = gpui::Rgba {
        r: theme.backdrop.r,
        g: theme.backdrop.g,
        b: theme.backdrop.b,
        a: theme.backdrop.a * 0.5,
    };

    div()
        .absolute()
        .top_0()
        .left_0()
        .size_full()
        .bg(badge_bg)
        .flex()
        .items_center()
        .justify_center()
        .on_mouse_down(MouseButton::Left, |_, _, cx| {
            cx.stop_propagation();
        })
        .child(
            div()
                .bg(theme.surface)
                .border_1()
                .border_color(theme.border)
                .rounded(px(4.))
                .px(px(10.))
                .py(px(4.))
                .text_size(px(11.))
                .text_color(theme.text_muted)
                .child(label.to_owned()),
        )
        .into_any_element()
}
