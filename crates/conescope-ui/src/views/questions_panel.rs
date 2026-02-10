use gpui::prelude::*;
use gpui::{Entity, MouseButton, ScrollHandle, div, px};

use conescope_core::question::QuestionWithContext;

use crate::state::app_state::AppState;
use crate::theme::Theme;
use crate::views::colors::{default_instance_color, hex_to_rgba};
use crate::views::scrollbar::{self, ScrollbarCallbacks, ScrollbarState};

#[derive(Debug)]
pub struct QuestionsPanel {
    app_state: Entity<AppState>,
    questions: Vec<QuestionWithContext>,
    scroll_handle: ScrollHandle,
    scrollbar_state: ScrollbarState,
}

impl QuestionsPanel {
    #[must_use]
    pub fn new(app_state: Entity<AppState>) -> Self {
        Self {
            app_state,
            questions: Vec::new(),
            scroll_handle: ScrollHandle::new(),
            scrollbar_state: ScrollbarState::default(),
        }
    }

    /// Reload pending questions from the DB.
    pub fn refresh(&mut self, questions: Vec<QuestionWithContext>) {
        self.questions = questions;
    }
}

fn render_question_card(
    q: &QuestionWithContext,
    app_state: &Entity<AppState>,
    theme: &Theme,
) -> gpui::Stateful<gpui::Div> {
    let id_str = q.question.id.clone();
    let app_state = app_state.clone();

    let instance_label = q
        .instance_title
        .as_deref()
        .unwrap_or("Unknown instance")
        .to_owned();
    let project_color = q
        .project_color
        .as_deref()
        .map_or_else(default_instance_color, hex_to_rgba);
    let project_label = q.project_name.clone().unwrap_or_default();

    let element_id = format!("question-{}", q.question.id);
    let bg = theme.background;

    div()
        .id(gpui::SharedString::from(element_id))
        .p(px(12.))
        .rounded(px(6.))
        .bg(theme.surface)
        .border_1()
        .border_color(theme.border)
        .flex()
        .flex_col()
        .gap(px(6.))
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(6.))
                .child(
                    div()
                        .text_size(px(11.))
                        .text_color(project_color)
                        .child(project_label),
                )
                .child(
                    div()
                        .text_size(px(11.))
                        .text_color(theme.text_muted)
                        .child(instance_label),
                ),
        )
        .child(
            div()
                .text_size(px(13.))
                .text_color(theme.text)
                .child(q.question.question_text.clone()),
        )
        .when(q.question.context.is_some(), |el| {
            let ctx = q.question.context.clone().unwrap_or_default();
            el.child(
                div()
                    .text_size(px(11.))
                    .text_color(theme.text_muted)
                    .px(px(8.))
                    .py(px(4.))
                    .rounded(px(4.))
                    .bg(bg)
                    .child(ctx),
            )
        })
        .child(
            div()
                .flex()
                .flex_row()
                .gap(px(8.))
                .justify_end()
                .child(dismiss_button(id_str, app_state, theme)),
        )
}

fn dismiss_button(
    question_id: String,
    app_state: Entity<AppState>,
    theme: &Theme,
) -> gpui::Stateful<gpui::Div> {
    let element_id = format!("dismiss-{question_id}");
    let border_variant = theme.border_variant;
    div()
        .id(gpui::SharedString::from(element_id))
        .px(px(8.))
        .py(px(4.))
        .rounded(px(4.))
        .cursor_pointer()
        .bg(theme.border)
        .hover(move |s| s.bg(border_variant))
        .text_size(px(11.))
        .text_color(theme.text)
        .child("Dismiss")
        .on_click(move |_, _, cx| {
            let now = chrono::Utc::now().to_rfc3339();
            let db = app_state.read(cx).db.clone();
            db.answer_question(question_id.clone(), "dismissed".into(), now);
        })
}

impl Render for QuestionsPanel {
    #[allow(clippy::too_many_lines)]
    fn render(
        &mut self,
        _window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> impl IntoElement {
        let theme = self.app_state.read(cx).theme().clone();
        let app_state_close = self.app_state.clone();
        let app_state_header = self.app_state.clone();

        let mut body = div()
            .id("questions-panel")
            .w(px(420.))
            .max_h(px(600.))
            .bg(theme.panel)
            .rounded(px(8.))
            .border_1()
            .border_color(theme.border_variant)
            .flex()
            .flex_col()
            .overflow_hidden()
            .on_mouse_down(MouseButton::Left, |_, _, cx| {
                cx.stop_propagation();
            })
            .child(panel_header(app_state_header, &theme));

        if self.questions.is_empty() {
            body = body.child(
                div()
                    .p(px(24.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_size(px(13.))
                    .text_color(theme.text_faint)
                    .child("No pending questions"),
            );
        } else {
            let mut list = div().p(px(12.)).flex().flex_col().gap(px(8.));
            for q in &self.questions {
                list = list.child(render_question_card(q, &self.app_state, &theme));
            }

            let scroll_div = div()
                .id("questions-list-scroll")
                .flex_1()
                .min_h_0()
                .overflow_y_scroll()
                .track_scroll(&self.scroll_handle)
                .child(list);

            let scrollbar_el = scrollbar::render_scrollbar(
                "questions",
                &self.scroll_handle,
                &self.scrollbar_state,
                ScrollbarCallbacks {
                    on_thumb_hover: cx.listener(|this, hovered: &bool, _, _| {
                        this.scrollbar_state.thumb_hovered = *hovered;
                    }),
                    on_track_click: cx.listener(|this, ev: &gpui::MouseDownEvent, _, cx| {
                        let click_y =
                            f32::from(ev.position.y) - f32::from(this.scroll_handle.bounds().top());
                        scrollbar::apply_track_click(&this.scroll_handle, click_y);
                        cx.notify();
                    }),
                    on_drag_start: cx.listener(|this, ev: &gpui::MouseDownEvent, _, _| {
                        this.scrollbar_state.drag = Some(scrollbar::ScrollbarDrag {
                            start_mouse_y: f32::from(ev.position.y),
                            start_offset_y: f32::from(this.scroll_handle.offset().y),
                        });
                    }),
                },
            );

            let scroll_container = div()
                .id("questions-scroll-container")
                .relative()
                .flex_1()
                .min_h_0()
                .on_hover(cx.listener(|this, hovered: &bool, _, _| {
                    this.scrollbar_state.container_hovered = *hovered;
                }))
                .on_mouse_move(cx.listener(|this, ev: &gpui::MouseMoveEvent, _, cx| {
                    if let Some(drag) = &this.scrollbar_state.drag {
                        let drag = *drag;
                        scrollbar::apply_drag(&this.scroll_handle, &drag, f32::from(ev.position.y));
                        cx.notify();
                    }
                }))
                .on_mouse_up(
                    MouseButton::Left,
                    cx.listener(|this, _, _, _| {
                        this.scrollbar_state.drag = None;
                    }),
                )
                .on_mouse_up_out(
                    MouseButton::Left,
                    cx.listener(|this, _, _, _| {
                        this.scrollbar_state.drag = None;
                    }),
                )
                .child(scroll_div)
                .children(scrollbar_el);

            body = body.child(scroll_container);
        }

        // Backdrop
        div()
            .id("questions-backdrop")
            .absolute()
            .size_full()
            .top_0()
            .left_0()
            .bg(theme.backdrop)
            .flex()
            .items_center()
            .justify_center()
            .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                app_state_close.update(cx, |s, cx| {
                    s.questions_queue_open = false;
                    cx.notify();
                });
            })
            .child(body)
    }
}

fn panel_header(app_state: Entity<AppState>, theme: &Theme) -> gpui::Div {
    let text = theme.text;
    div()
        .px(px(16.))
        .py(px(12.))
        .flex()
        .flex_row()
        .items_center()
        .border_b_1()
        .border_color(theme.border)
        .child(div().flex_1().text_color(theme.text).child("Questions"))
        .child(
            div()
                .cursor_pointer()
                .text_color(theme.text_muted)
                .hover(move |s| s.text_color(text))
                .child("\u{00d7}")
                .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                    app_state.update(cx, |s, cx| {
                        s.questions_queue_open = false;
                        cx.notify();
                    });
                }),
        )
}
