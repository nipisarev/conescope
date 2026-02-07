use gpui::prelude::*;
use gpui::{Entity, MouseButton, div, px, rgba};

use conescope_core::question::QuestionWithContext;

use crate::state::app_state::AppState;
use crate::views::colors::hex_to_rgba;

#[derive(Debug)]
pub struct QuestionsPanel {
    app_state: Entity<AppState>,
    questions: Vec<QuestionWithContext>,
}

impl QuestionsPanel {
    #[must_use]
    pub fn new(app_state: Entity<AppState>) -> Self {
        Self {
            app_state,
            questions: Vec::new(),
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
        .map_or_else(|| rgba(0x6464_b5f6), hex_to_rgba);
    let project_label = q.project_name.clone().unwrap_or_default();

    let element_id = format!("question-{}", q.question.id);

    div()
        .id(gpui::SharedString::from(element_id))
        .p(px(12.))
        .rounded(px(6.))
        .bg(rgba(0x2d2d_2dff))
        .border_1()
        .border_color(rgba(0x3c3c_3cff))
        .flex()
        .flex_col()
        .gap(px(6.))
        // Source line: instance + project
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
                        .text_color(rgba(0x8888_88ff))
                        .child(instance_label),
                ),
        )
        // Question text
        .child(
            div()
                .text_size(px(13.))
                .text_color(rgba(0xdddd_ddff))
                .child(q.question.question_text.clone()),
        )
        // Context (if any)
        .when(q.question.context.is_some(), |el| {
            let ctx = q.question.context.clone().unwrap_or_default();
            el.child(
                div()
                    .text_size(px(11.))
                    .text_color(rgba(0x8888_88ff))
                    .px(px(8.))
                    .py(px(4.))
                    .rounded(px(4.))
                    .bg(rgba(0x1e1e_1eff))
                    .child(ctx),
            )
        })
        // Actions
        .child(
            div()
                .flex()
                .flex_row()
                .gap(px(8.))
                .justify_end()
                .child(dismiss_button(id_str, app_state)),
        )
}

fn dismiss_button(question_id: String, app_state: Entity<AppState>) -> gpui::Stateful<gpui::Div> {
    let element_id = format!("dismiss-{question_id}");
    div()
        .id(gpui::SharedString::from(element_id))
        .px(px(8.))
        .py(px(4.))
        .rounded(px(4.))
        .cursor_pointer()
        .bg(rgba(0x3c3c_3cff))
        .hover(|s| s.bg(rgba(0x4c4c_4cff)))
        .text_size(px(11.))
        .text_color(rgba(0xcccc_ccff))
        .child("Dismiss")
        .on_click(move |_, _, cx| {
            let now = chrono::Utc::now().to_rfc3339();
            let db = app_state.read(cx).db.clone();
            db.answer_question(question_id.clone(), "dismissed".into(), now);
        })
}

impl Render for QuestionsPanel {
    fn render(
        &mut self,
        _window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> impl IntoElement {
        let app_state_close = self.app_state.clone();

        // Backdrop
        div()
            .id("questions-backdrop")
            .absolute()
            .size_full()
            .top_0()
            .left_0()
            .bg(rgba(0x0000_0080))
            .flex()
            .items_center()
            .justify_center()
            .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                app_state_close.update(cx, |s, cx| {
                    s.questions_queue_open = false;
                    cx.notify();
                });
            })
            .child(render_panel_body(&self.questions, &self.app_state, cx))
    }
}

fn render_panel_body(
    questions: &[QuestionWithContext],
    app_state: &Entity<AppState>,
    _cx: &gpui::Context<QuestionsPanel>,
) -> gpui::Stateful<gpui::Div> {
    let app_state_header = app_state.clone();

    let mut body = div()
        .id("questions-panel")
        .w(px(420.))
        .max_h(px(600.))
        .bg(rgba(0x2525_26ff))
        .rounded(px(8.))
        .border_1()
        .border_color(rgba(0x4c4c_4cff))
        .flex()
        .flex_col()
        .overflow_hidden()
        .on_mouse_down(MouseButton::Left, |_, _, cx| {
            cx.stop_propagation();
        })
        // Header
        .child(panel_header(app_state_header));

    // Content
    if questions.is_empty() {
        body = body.child(
            div()
                .p(px(24.))
                .flex()
                .items_center()
                .justify_center()
                .text_size(px(13.))
                .text_color(rgba(0x6666_66ff))
                .child("No pending questions"),
        );
    } else {
        let mut list = div().p(px(12.)).flex().flex_col().gap(px(8.));
        for q in questions {
            list = list.child(render_question_card(q, app_state));
        }
        body = body.child(
            div()
                .id("questions-list-scroll")
                .flex_1()
                .min_h_0()
                .overflow_y_scroll()
                .child(list),
        );
    }

    body
}

fn panel_header(app_state: Entity<AppState>) -> gpui::Div {
    div()
        .px(px(16.))
        .py(px(12.))
        .flex()
        .flex_row()
        .items_center()
        .border_b_1()
        .border_color(rgba(0x3c3c_3cff))
        .child(
            div()
                .flex_1()
                .text_color(rgba(0xdddd_ddff))
                .child("Questions"),
        )
        .child(
            div()
                .cursor_pointer()
                .text_color(rgba(0x8888_88ff))
                .hover(|s| s.text_color(rgba(0xcccc_ccff)))
                .child("\u{00d7}")
                .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                    app_state.update(cx, |s, cx| {
                        s.questions_queue_open = false;
                        cx.notify();
                    });
                }),
        )
}
