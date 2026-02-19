use gpui::prelude::*;
use gpui::{AnyElement, Entity, FocusHandle, KeyDownEvent, MouseButton, SharedString, div, px};

use crate::state::app_state::AppState;
use crate::state::session_detector::SessionEvent;
use crate::theme::Theme;
use crate::views::overview_grid::OverviewGrid;

#[must_use]
#[allow(clippy::too_many_arguments)]
pub fn render_question_overlay(
    event: &SessionEvent,
    instance_id: &str,
    app_state: &Entity<AppState>,
    theme: &Theme,
    font_size: f32,
    focus_handle: Option<&FocusHandle>,
    selected_index: usize,
    grid_entity: Option<&Entity<OverviewGrid>>,
) -> AnyElement {
    match event {
        SessionEvent::Question { text, choices, .. } => render_question_card(
            text,
            choices,
            instance_id,
            app_state.clone(),
            theme,
            font_size,
            focus_handle,
            selected_index,
            grid_entity.cloned(),
        ),
        SessionEvent::WaitingForInput => render_badge("Waiting...", theme, font_size),
        SessionEvent::Finished => render_badge("Finished", theme, font_size),
    }
}

#[allow(
    clippy::needless_pass_by_value,
    clippy::too_many_arguments,
    clippy::too_many_lines
)]
fn render_question_card(
    text: &str,
    choices: &[String],
    instance_id: &str,
    app_state: Entity<AppState>,
    theme: &Theme,
    font_size: f32,
    focus_handle: Option<&FocusHandle>,
    selected_index: usize,
    grid_entity: Option<Entity<OverviewGrid>>,
) -> AnyElement {
    // total_choices = actual choices + 1 for "More context"
    let total_choices = choices.len() + 1;

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
        .max_w(px(font_size * 24.0))
        .child(
            div()
                .text_size(px(font_size))
                .text_color(theme.text)
                .child(text.to_owned()),
        );

    for (i, choice) in choices.iter().enumerate() {
        let label = format!("{}. {}", i + 1, choice);
        let iid = instance_id.to_owned();
        let state = app_state.clone();
        let btn_id = SharedString::from(format!("choice-{iid}-{i}"));
        let is_selected = i == selected_index;

        let bg = if is_selected {
            theme.accent
        } else {
            theme.element_hover
        };

        card = card.child(
            div()
                .id(btn_id)
                .cursor_pointer()
                .px(px(6.))
                .py(px(3.))
                .rounded(px(4.))
                .text_size(px(font_size - 1.0))
                .text_color(theme.text)
                .bg(bg)
                .when(!is_selected, |s| s.hover(|s| s.bg(theme.accent)))
                .on_click(move |_, _, cx| {
                    state.update(cx, |s, cx| {
                        s.answer_instance_question(&iid, i, cx);
                    });
                })
                .child(label),
        );
    }

    // "More context →" as the last selectable choice
    let focus_id = instance_id.to_owned();
    let focus_state = app_state.clone();
    let more_ctx_selected = selected_index == choices.len();
    let more_ctx_bg = if more_ctx_selected {
        theme.accent
    } else {
        theme.element_hover
    };
    card = card.child(
        div()
            .id(SharedString::from(format!("more-ctx-{focus_id}")))
            .cursor_pointer()
            .px(px(6.))
            .py(px(3.))
            .rounded(px(4.))
            .text_size(px(font_size - 1.0))
            .text_color(theme.text)
            .bg(more_ctx_bg)
            .when(!more_ctx_selected, |s| s.hover(|s| s.bg(theme.accent)))
            .on_click(move |_, _, cx| {
                focus_state.update(cx, |s, cx| {
                    s.focus_instance(&focus_id, cx);
                });
            })
            .child("More context \u{2192}"),
    );

    // Backdrop + centered card
    let mut backdrop = div()
        .id("overlay-keyboard-target")
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
        });

    if let Some(fh) = focus_handle {
        let key_state = app_state.clone();
        let key_iid = instance_id.to_owned();
        let num_real_choices = choices.len();
        backdrop =
            backdrop
                .track_focus(fh)
                .on_key_down(move |event: &KeyDownEvent, _window, cx| {
                    let key = event.keystroke.key.as_ref();
                    match key {
                        "up" => {
                            if let Some(ref grid) = grid_entity {
                                let new_idx = (selected_index + total_choices - 1) % total_choices;
                                grid.update(cx, |g, cx| {
                                    g.set_overlay_selected_index(new_idx, cx);
                                });
                            }
                        }
                        "down" => {
                            if let Some(ref grid) = grid_entity {
                                let new_idx = (selected_index + 1) % total_choices;
                                grid.update(cx, |g, cx| {
                                    g.set_overlay_selected_index(new_idx, cx);
                                });
                            }
                        }
                        "enter" => {
                            let iid = key_iid.clone();
                            if selected_index < num_real_choices {
                                key_state.update(cx, |s, cx| {
                                    s.answer_instance_question(&iid, selected_index, cx);
                                });
                            } else {
                                key_state.update(cx, |s, cx| {
                                    s.focus_instance(&iid, cx);
                                });
                            }
                        }
                        "escape" => {
                            let iid = key_iid.clone();
                            key_state.update(cx, |s, cx| {
                                s.dismiss_instance_session_event(&iid, cx);
                            });
                        }
                        _ => {}
                    }
                    cx.stop_propagation();
                });
    }

    backdrop.child(card).into_any_element()
}

fn render_badge(label: &str, theme: &Theme, font_size: f32) -> AnyElement {
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
                .text_size(px(font_size - 1.0))
                .text_color(theme.text_muted)
                .child(label.to_owned()),
        )
        .into_any_element()
}
