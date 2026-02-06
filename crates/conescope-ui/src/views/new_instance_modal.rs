use gpui::prelude::*;
use gpui::{AppContext, Entity, MouseButton, div, px, rgba};

use conescope_core::instance::{Instance, InstanceStatus, InstanceType};

use crate::state::app_state::AppState;
use crate::state::instance_entry::InstanceEntry;
use crate::terminal::spawn_terminal_pane;

#[derive(Debug)]
pub struct NewInstanceModal {
    app_state: Entity<AppState>,
}

impl NewInstanceModal {
    #[must_use]
    pub fn new(app_state: Entity<AppState>) -> Self {
        Self { app_state }
    }
}

fn create_instance(
    instance_type: InstanceType,
    app_state: &Entity<AppState>,
    window: &mut gpui::Window,
    cx: &mut gpui::App,
) {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
    let cwd = home.as_str();

    // Compute instance number from current count
    let instance_number = {
        let state = app_state.read(cx);
        let il = state.instance_list.read(cx);
        #[allow(clippy::cast_possible_wrap)]
        let n = il.len() as i64 + 1;
        n
    };

    let id = uuid::Uuid::new_v4().to_string();
    let started_at = chrono::Utc::now().to_rfc3339();

    let instance = Instance {
        id: id.clone(),
        project_id: None,
        title: Some(match instance_type {
            InstanceType::Project => format!("Project #{instance_number}"),
            InstanceType::Terminal => format!("Terminal #{instance_number}"),
        }),
        status: InstanceStatus::Starting,
        instance_number: Some(instance_number),
        tokens_used: 0,
        cost_estimate: 0.0,
        started_at,
        ended_at: None,
        instance_type,
        color: None,
    };

    // DB insert (fire-and-forget)
    app_state.read(cx).db.insert_instance(instance.clone());

    // Spawn PTY (needs Window)
    let pane = spawn_terminal_pane(Some(cwd), window, cx);
    let is_project = instance_type == InstanceType::Project;

    // Create InstanceEntry
    let entry = cx.new(|_| {
        let mut e = InstanceEntry::from_instance(instance);
        e.attach_terminal(pane);
        e
    });

    // Start output polling
    entry.update(cx, InstanceEntry::start_output_polling);

    // Send "claude\r" for project instances
    if is_project {
        entry.read(cx).send_input(b"claude\r");
    }

    // Push to instance list
    let instance_list = app_state.read(cx).instance_list.clone();
    instance_list.update(cx, |list, cx| list.push_entry(entry, cx));

    // Close modal
    app_state.update(cx, |s, cx| {
        s.new_instance_modal_open = false;
        cx.notify();
    });
}

impl Render for NewInstanceModal {
    fn render(
        &mut self,
        _window: &mut gpui::Window,
        _cx: &mut gpui::Context<Self>,
    ) -> impl IntoElement {
        let app_state_close = self.app_state.clone();
        let app_state_terminal = self.app_state.clone();
        let app_state_project = self.app_state.clone();

        // Backdrop
        div()
            .absolute()
            .size_full()
            .top_0()
            .left_0()
            .bg(rgba(0x0000_0080))
            .flex()
            .items_center()
            .justify_center()
            // Modal box
            .child(
                div()
                    .w(px(320.))
                    .bg(rgba(0x2d2d_2dff))
                    .rounded(px(8.))
                    .border_1()
                    .border_color(rgba(0x4c4c_4cff))
                    .flex()
                    .flex_col()
                    .overflow_hidden()
                    // Header
                    .child(
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
                                    .child("New Instance"),
                            )
                            .child(
                                div()
                                    .cursor_pointer()
                                    .text_color(rgba(0x8888_88ff))
                                    .hover(|s| s.text_color(rgba(0xcccc_ccff)))
                                    .child("\u{00d7}") // ×
                                    .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                                        app_state_close
                                            .update(cx, AppState::toggle_new_instance_modal);
                                    }),
                            ),
                    )
                    // Buttons
                    .child(
                        div()
                            .p(px(16.))
                            .flex()
                            .flex_col()
                            .gap(px(8.))
                            .child(modal_button(
                                "New Terminal",
                                "Open a shell terminal",
                                app_state_terminal,
                                InstanceType::Terminal,
                            ))
                            .child(modal_button(
                                "New Project (~/)",
                                "Launch Claude Code in home directory",
                                app_state_project,
                                InstanceType::Project,
                            )),
                    ),
            )
    }
}

fn modal_button(
    label: &str,
    description: &str,
    app_state: Entity<AppState>,
    instance_type: InstanceType,
) -> gpui::Div {
    let label = label.to_owned();
    let description = description.to_owned();
    div()
        .px(px(12.))
        .py(px(10.))
        .rounded(px(6.))
        .cursor_pointer()
        .bg(rgba(0x3c3c_3cff))
        .hover(|s| s.bg(rgba(0x4c4c_4cff)))
        .flex()
        .flex_col()
        .gap(px(2.))
        .child(div().text_color(rgba(0xdddd_ddff)).child(label))
        .child(
            div()
                .text_size(px(11.))
                .text_color(rgba(0x8888_88ff))
                .child(description),
        )
        .on_mouse_down(MouseButton::Left, move |_, window, cx| {
            create_instance(instance_type, &app_state, window, cx);
        })
}
