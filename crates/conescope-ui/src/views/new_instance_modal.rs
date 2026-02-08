use gpui::prelude::*;
use gpui::{AppContext, Entity, MouseButton, PathPromptOptions, ScrollHandle, div, px};

use conescope_core::instance::{Instance, InstanceStatus, InstanceType};
use conescope_core::project::Project;

use crate::state::app_state::AppState;
use crate::state::instance_entry::InstanceEntry;
use crate::terminal::spawn_terminal_pane;
use crate::theme::Theme;
use crate::views::scrollbar::{self, ScrollbarCallbacks, ScrollbarState};

const MAX_RECENT_PROJECTS: usize = 5;

#[derive(Debug)]
pub struct NewInstanceModal {
    app_state: Entity<AppState>,
    scroll_handle: ScrollHandle,
    scrollbar_state: ScrollbarState,
}

impl NewInstanceModal {
    #[must_use]
    pub fn new(app_state: Entity<AppState>) -> Self {
        Self {
            app_state,
            scroll_handle: ScrollHandle::new(),
            scrollbar_state: ScrollbarState::default(),
        }
    }
}

struct NewInstanceParams<'a> {
    instance_type: InstanceType,
    cwd: &'a str,
    project_id: Option<String>,
    title_prefix: &'a str,
}

fn create_instance_at(
    params: NewInstanceParams<'_>,
    app_state: &Entity<AppState>,
    window: &mut gpui::Window,
    cx: &mut gpui::App,
) {
    let NewInstanceParams {
        instance_type,
        cwd,
        project_id,
        title_prefix,
    } = params;
    let id = uuid::Uuid::new_v4().to_string();
    let started_at = chrono::Utc::now().to_rfc3339();

    let instance = Instance {
        id: id.clone(),
        project_id,
        title: Some(title_prefix.to_string()),
        status: InstanceStatus::Starting,
        tokens_used: 0,
        cost_estimate: 0.0,
        started_at,
        ended_at: None,
        instance_type,
        color: None,
    };

    app_state.read(cx).db.insert_instance(instance.clone());

    let font_family = Some(
        app_state
            .read(cx)
            .settings_store
            .read(cx)
            .settings()
            .font_family
            .clone(),
    );
    let pane = spawn_terminal_pane(Some(cwd), font_family.as_deref(), window, cx);
    let is_project = instance_type == InstanceType::Project;

    let entry = cx.new(|cx| {
        let mut e = InstanceEntry::from_instance(instance);
        e.attach_terminal(pane, cx);
        e
    });

    entry.update(cx, InstanceEntry::start_output_polling);

    if is_project {
        entry.read(cx).send_input(b"claude\r");
    }

    let entry_for_focus = entry.clone();
    let instance_list = app_state.read(cx).instance_list.clone();
    instance_list.update(cx, |list, cx| list.push_entry(entry, cx));

    app_state.update(cx, |s, cx| {
        s.new_instance_modal_open = false;
        s.focus_instance(&id, cx);
    });

    if let Some(fh) = entry_for_focus.read(cx).focus_handle.clone() {
        fh.focus(window, cx);
    }
}

fn create_terminal(app_state: &Entity<AppState>, window: &mut gpui::Window, cx: &mut gpui::App) {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
    create_instance_at(
        NewInstanceParams {
            instance_type: InstanceType::Terminal,
            cwd: &home,
            project_id: None,
            title_prefix: "Terminal",
        },
        app_state,
        window,
        cx,
    );
}

fn create_project_at_home(
    app_state: &Entity<AppState>,
    window: &mut gpui::Window,
    cx: &mut gpui::App,
) {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
    create_instance_at(
        NewInstanceParams {
            instance_type: InstanceType::Project,
            cwd: &home,
            project_id: None,
            title_prefix: "Project",
        },
        app_state,
        window,
        cx,
    );
}

fn create_project_for(
    project: &Project,
    app_state: &Entity<AppState>,
    window: &mut gpui::Window,
    cx: &mut gpui::App,
) {
    let now = chrono::Utc::now().to_rfc3339();
    let ps = app_state.read(cx).project_store.clone();
    let pid = project.id.clone();
    ps.update(cx, |store, _| store.update_last_used(&pid, &now));

    create_instance_at(
        NewInstanceParams {
            instance_type: InstanceType::Project,
            cwd: &project.path,
            project_id: Some(project.id.clone()),
            title_prefix: &project.display_name,
        },
        app_state,
        window,
        cx,
    );
}

fn shorten_path(path: &str) -> String {
    let home = std::env::var("HOME").unwrap_or_default();
    if !home.is_empty() && path.starts_with(&home) {
        format!("~{}", &path[home.len()..])
    } else {
        path.to_owned()
    }
}

fn render_modal_body(
    app_state: &Entity<AppState>,
    recent_projects_section: Option<gpui::Stateful<gpui::Div>>,
    theme: &Theme,
) -> gpui::Stateful<gpui::Div> {
    let app_state_close = app_state.clone();
    let app_state_terminal = app_state.clone();
    let app_state_project = app_state.clone();
    let app_state_browse = app_state.clone();

    div()
        .id("new-instance-modal")
        .w(px(360.))
        .max_h(px(500.))
        .bg(theme.surface)
        .rounded(px(8.))
        .border_1()
        .border_color(theme.border_variant)
        .flex()
        .flex_col()
        .overflow_hidden()
        .on_mouse_down(MouseButton::Left, |_, _, cx| {
            cx.stop_propagation();
        })
        .child(modal_header(app_state_close, theme))
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
                    ActionKind::Terminal,
                    theme,
                ))
                .child(modal_button(
                    "New Project (~/)",
                    "Launch Claude Code in home directory",
                    app_state_project,
                    ActionKind::ProjectHome,
                    theme,
                ))
                .child(modal_button(
                    "Browse...",
                    "Choose a project directory",
                    app_state_browse,
                    ActionKind::Browse,
                    theme,
                )),
        )
        .children(recent_projects_section)
}

fn modal_header(app_state: Entity<AppState>, theme: &Theme) -> gpui::Div {
    let text = theme.text;
    div()
        .px(px(16.))
        .py(px(12.))
        .flex()
        .flex_row()
        .items_center()
        .border_b_1()
        .border_color(theme.border)
        .child(div().flex_1().text_color(theme.text).child("New Instance"))
        .child(
            div()
                .cursor_pointer()
                .text_color(theme.text_muted)
                .hover(move |s| s.text_color(text))
                .child("\u{00d7}")
                .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                    app_state.update(cx, |s, cx| {
                        s.new_instance_modal_open = false;
                        cx.notify();
                    });
                }),
        )
}

fn recent_projects_list(
    projects: &[Project],
    app_state: &Entity<AppState>,
    theme: &Theme,
) -> gpui::Div {
    let mut list = div().flex().flex_col().gap(px(4.));
    for project in projects {
        list = list.child(recent_project_row(project, app_state, theme));
    }
    list
}

impl Render for NewInstanceModal {
    #[allow(clippy::too_many_lines)]
    fn render(
        &mut self,
        _window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> impl IntoElement {
        let theme = self.app_state.read(cx).theme().clone();
        let app_state_backdrop = self.app_state.clone();

        let recent_projects: Vec<Project> = {
            let state = self.app_state.read(cx);
            let ps = state.project_store.read(cx);
            let mut projects: Vec<Project> = ps.projects().to_vec();
            projects.sort_by(|a, b| b.last_used_at.cmp(&a.last_used_at));
            projects.truncate(MAX_RECENT_PROJECTS);
            projects
        };

        let recent_section = if recent_projects.is_empty() {
            None
        } else {
            let list = recent_projects_list(&recent_projects, &self.app_state, &theme);

            let scroll_div = div()
                .id("recent-projects-scroll")
                .max_h(px(200.))
                .overflow_y_scroll()
                .track_scroll(&self.scroll_handle)
                .child(list);

            let scrollbar_el = scrollbar::render_scrollbar(
                "recent-projects",
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
                .id("recent-scroll-container")
                .relative()
                .max_h(px(200.))
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

            Some(
                div()
                    .id("recent-projects-section")
                    .px(px(16.))
                    .pb(px(12.))
                    .flex()
                    .flex_col()
                    .gap(px(4.))
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(theme.text_faint)
                            .pb(px(4.))
                            .child("RECENT PROJECTS"),
                    )
                    .child(scroll_container),
            )
        };

        div()
            .id("modal-backdrop")
            .absolute()
            .size_full()
            .top_0()
            .left_0()
            .bg(theme.backdrop)
            .flex()
            .items_center()
            .justify_center()
            .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                app_state_backdrop.update(cx, |s, cx| {
                    s.new_instance_modal_open = false;
                    cx.notify();
                });
            })
            .child(render_modal_body(&self.app_state, recent_section, &theme))
    }
}

#[derive(Clone, Copy)]
enum ActionKind {
    Terminal,
    ProjectHome,
    Browse,
}

fn modal_button(
    label: &str,
    description: &str,
    app_state: Entity<AppState>,
    action: ActionKind,
    theme: &Theme,
) -> gpui::Div {
    let label = label.to_owned();
    let description = description.to_owned();
    let border_variant = theme.border_variant;
    div()
        .px(px(12.))
        .py(px(10.))
        .rounded(px(6.))
        .cursor_pointer()
        .bg(theme.border)
        .hover(move |s| s.bg(border_variant))
        .flex()
        .flex_col()
        .gap(px(2.))
        .child(div().text_color(theme.text).child(label))
        .child(
            div()
                .text_size(px(11.))
                .text_color(theme.text_muted)
                .child(description),
        )
        .on_mouse_down(MouseButton::Left, move |_, window, cx| match action {
            ActionKind::Terminal => create_terminal(&app_state, window, cx),
            ActionKind::ProjectHome => create_project_at_home(&app_state, window, cx),
            ActionKind::Browse => browse_for_directory(&app_state, window, cx),
        })
}

fn browse_for_directory(
    app_state: &Entity<AppState>,
    _window: &mut gpui::Window,
    cx: &mut gpui::App,
) {
    let receiver = cx.prompt_for_paths(PathPromptOptions {
        files: false,
        directories: true,
        multiple: false,
        prompt: Some("Select project directory".into()),
    });

    let app_state = app_state.clone();
    cx.spawn(async move |cx| {
        if let Ok(Ok(Some(paths))) = receiver.await {
            if let Some(path) = paths.first() {
                let path_str = path.display().to_string();
                let display_name = path
                    .file_name()
                    .map_or_else(|| path_str.clone(), |n| n.to_string_lossy().into_owned());

                cx.update(|cx| {
                    // Add project to store if not already there
                    let ps = app_state.read(cx).project_store.clone();
                    let existing_id = {
                        let store = ps.read(cx);
                        store
                            .projects()
                            .iter()
                            .find(|p| p.path == path_str)
                            .map(|p| p.id.clone())
                    };

                    let project_id = if let Some(id) = existing_id {
                        id
                    } else {
                        let new_project = Project {
                            id: uuid::Uuid::new_v4().to_string(),
                            path: path_str.clone(),
                            display_name: display_name.clone(),
                            color: ps.read(cx).next_color().to_owned(),
                            created_at: chrono::Utc::now().to_rfc3339(),
                            last_used_at: chrono::Utc::now().to_rfc3339(),
                        };
                        let id = new_project.id.clone();
                        ps.update(cx, |store, _| store.add(new_project));
                        id
                    };

                    // We need Window context for PTY spawning — use update_window
                    // Since we can't get the window handle here, we use a workaround:
                    // store the pending path and handle it on next render.
                    // Actually, the simpler approach: we can't spawn PTY without Window.
                    // Instead, store pending project and let the modal handle it.
                    // But that adds complexity. Let's just set up the instance without
                    // a PTY and let restore_terminals handle it... no, that's wrong.

                    // The correct approach: we need to get a window handle.
                    // The prompt_for_paths is called from App context already.
                    // We need to find our window. Let's iterate windows.
                    let windows = cx.windows();
                    if let Some(window_handle) = windows.first() {
                        let _ = window_handle.update(cx, |_view, window, cx| {
                            create_instance_at(
                                NewInstanceParams {
                                    instance_type: InstanceType::Project,
                                    cwd: &path_str,
                                    project_id: Some(project_id),
                                    title_prefix: &display_name,
                                },
                                &app_state,
                                window,
                                cx,
                            );
                        });
                    }
                });
            }
        }
    })
    .detach();
}

fn recent_project_row(project: &Project, app_state: &Entity<AppState>, theme: &Theme) -> gpui::Div {
    let project = project.clone();
    let app_state = app_state.clone();
    let short_path = shorten_path(&project.path);
    let border = theme.border;

    div()
        .px(px(8.))
        .py(px(6.))
        .rounded(px(4.))
        .cursor_pointer()
        .hover(move |s| s.bg(border))
        .flex()
        .flex_row()
        .items_center()
        .gap(px(8.))
        .child(
            div()
                .flex_1()
                .flex()
                .flex_col()
                .child(
                    div()
                        .text_color(theme.text)
                        .text_size(px(13.))
                        .child(project.display_name.clone()),
                )
                .child(
                    div()
                        .text_color(theme.text_faint)
                        .text_size(px(11.))
                        .child(short_path),
                ),
        )
        .on_mouse_down(MouseButton::Left, move |_, window, cx| {
            create_project_for(&project, &app_state, window, cx);
        })
}
