use std::path::Path;

use gpui::prelude::*;
use gpui::{Entity, MouseButton, div, px, svg};

use conescope_core::instance::InstanceType;
use conescope_core::settings::SettingsJson;

use crate::state::app_state::AppState;
use crate::theme::Theme;
use crate::views::colors::worktree_color_variant;
use crate::views::new_instance_modal::{NewInstanceParams, create_instance_at};
use crate::views::text_input::{TextInput, TextInputEvent};

pub struct WorktreeModal {
    app_state: Entity<AppState>,
    branch_input: Entity<TextInput>,
    copy_ignored: bool,
    placeholder: String,
}

impl std::fmt::Debug for WorktreeModal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WorktreeModal")
            .field("copy_ignored", &self.copy_ignored)
            .finish_non_exhaustive()
    }
}

impl WorktreeModal {
    #[must_use]
    pub fn new(app_state: Entity<AppState>, cx: &mut gpui::Context<Self>) -> Self {
        let placeholder = names::Generator::default()
            .next()
            .unwrap_or_else(|| "feature".into());

        let branch_input = cx.new(|cx| TextInput::new("", cx));

        cx.subscribe(&branch_input, |this, _input, event, cx| match event {
            TextInputEvent::Submit(_) => {
                cx.spawn(async move |this, cx| {
                    cx.update(|cx| {
                        let windows = cx.windows();
                        if let Some(wh) = windows.first() {
                            let _ = wh.update(cx, |_view, window, cx| {
                                let _ = this.update(cx, |modal, cx| modal.on_create(window, cx));
                            });
                        }
                    });
                })
                .detach();
            }
            TextInputEvent::Cancel => {
                this.app_state.update(cx, AppState::close_worktree_modal);
            }
        })
        .detach();

        Self {
            app_state,
            branch_input,
            copy_ignored: true,
            placeholder,
        }
    }

    fn on_create(&mut self, _window: &mut gpui::Window, cx: &mut gpui::Context<Self>) {
        let Some(create_data) = self.gather_create_data(cx) else {
            tracing::warn!("worktree: gather_create_data returned None");
            return;
        };

        tracing::info!(
            src = %create_data.source_path,
            wt = %create_data.worktree_path_str,
            branch = %create_data.branch,
            "worktree: creating"
        );

        self.app_state.update(cx, AppState::close_worktree_modal);

        let copy_ignored = self.copy_ignored;
        let app_state = self.app_state.clone();

        cx.spawn(async move |_this, cx| {
            let result = cx
                .background_executor()
                .spawn({
                    let src = create_data.source_path.clone();
                    let wt = create_data.worktree_path_str.clone();
                    let br = create_data.branch.clone();
                    async move {
                        // Ensure parent directories exist for the worktree path
                        let wt_path = Path::new(&wt);
                        if let Some(parent) = wt_path.parent() {
                            std::fs::create_dir_all(parent)?;
                        }
                        let repo = conescope_git::repository::GitRepo::open(Path::new(&src))?;
                        repo.worktree_add(&wt, &br)?;
                        // Verify the worktree directory was actually created
                        if !wt_path.exists() {
                            anyhow::bail!("worktree dir not created at {wt}");
                        }
                        if copy_ignored {
                            repo.copy_ignored_files(wt_path)?;
                        }
                        Ok::<_, anyhow::Error>(())
                    }
                })
                .await;

            if let Err(ref e) = result {
                tracing::error!("worktree creation failed: {e}");
            }

            cx.update(|cx| {
                if let Err(e) = result {
                    app_state.update(cx, |s, cx| {
                        s.set_error(format!("Worktree error: {e}"), cx);
                    });
                    return;
                }
                let windows = cx.windows();
                if let Some(wh) = windows.first() {
                    if let Err(e) = wh.update(cx, |_view, window, cx| {
                        tracing::info!(
                            cwd = %create_data.worktree_path_str,
                            "worktree: spawning instance"
                        );
                        create_data.spawn_instance(&app_state, window, cx);
                    }) {
                        tracing::error!("worktree: window update failed: {e}");
                    }
                } else {
                    tracing::error!("worktree: no windows found for instance creation");
                }
            });
        })
        .detach();
    }

    fn gather_create_data(&self, cx: &gpui::App) -> Option<CreateData> {
        let state = self.app_state.read(cx);
        let source_id = state.worktree_modal_source_id.clone()?;

        let source_entry = state
            .instance_list
            .read(cx)
            .find_by_id(&source_id, cx)
            .cloned()?;

        let entry = source_entry.read(cx);
        let source_path = entry.instance.current_cwd.clone().unwrap_or_default();
        let project_id = entry.instance.project_id.clone();
        let parent_color = entry
            .instance
            .color
            .clone()
            .unwrap_or_else(|| "#4FC3F7".to_string());

        let typed = self.branch_input.read(cx).content().to_owned();
        let branch = if typed.is_empty() {
            self.placeholder.clone()
        } else {
            typed
        };

        let project_display_name = project_id
            .as_deref()
            .and_then(|pid| {
                state
                    .project_store
                    .read(cx)
                    .get(pid)
                    .map(|p| p.display_name.clone())
            })
            .unwrap_or_else(|| {
                Path::new(&source_path).file_name().map_or_else(
                    || "project".to_string(),
                    |n| n.to_string_lossy().into_owned(),
                )
            });

        let worktree_path = SettingsJson::worktree_dir(&project_display_name, &branch);
        let worktree_path_str = worktree_path.display().to_string();

        let worktree_index = state
            .instance_list
            .read(cx)
            .entries()
            .iter()
            .filter(|e| {
                let inst = &e.read(cx).instance;
                inst.parent_instance_id.as_deref() == Some(source_id.as_str())
                    || (project_id.is_some()
                        && inst.project_id == project_id
                        && inst.worktree_path.is_some())
            })
            .count();

        let color = worktree_color_variant(&parent_color, worktree_index);
        let title = format!("{project_display_name} [{branch}]");

        Some(CreateData {
            source_path,
            branch,
            worktree_path_str,
            project_id,
            source_id,
            color,
            title,
        })
    }
}

struct CreateData {
    source_path: String,
    branch: String,
    worktree_path_str: String,
    project_id: Option<String>,
    source_id: String,
    color: String,
    title: String,
}

impl CreateData {
    fn spawn_instance(
        self,
        app_state: &Entity<AppState>,
        window: &mut gpui::Window,
        cx: &mut gpui::App,
    ) {
        create_instance_at(
            NewInstanceParams {
                instance_type: InstanceType::Project,
                cwd: &self.worktree_path_str,
                project_id: self.project_id,
                title_prefix: &self.title,
                color_override: Some(self.color),
                worktree_path: Some(self.worktree_path_str.clone()),
                worktree_branch: Some(self.branch),
                parent_instance_id: Some(self.source_id),
            },
            app_state,
            window,
            cx,
        );
    }
}

impl Render for WorktreeModal {
    #[allow(clippy::too_many_lines)]
    fn render(
        &mut self,
        _window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> impl IntoElement {
        let state = self.app_state.read(cx);
        let theme = state.theme().clone();

        let source_branch = state
            .worktree_modal_source_id
            .as_deref()
            .and_then(|id| {
                state.instance_list.read(cx).find_by_id(id, cx).map(|e| {
                    e.read(cx)
                        .git_summary
                        .branch
                        .clone()
                        .unwrap_or_else(|| "HEAD".to_string())
                })
            })
            .unwrap_or_else(|| "HEAD".to_string());

        let app_state_backdrop = self.app_state.clone();
        let app_state_cancel = self.app_state.clone();
        let placeholder = self.placeholder.clone();

        div()
            .id("worktree-backdrop")
            .absolute()
            .size_full()
            .top_0()
            .left_0()
            .bg(theme.backdrop)
            .flex()
            .items_center()
            .justify_center()
            .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                app_state_backdrop.update(cx, AppState::close_worktree_modal);
            })
            .child(render_modal_card(
                &self.app_state,
                &self.branch_input,
                self.copy_ignored,
                &source_branch,
                &placeholder,
                app_state_cancel,
                &theme,
                cx,
            ))
    }
}

#[allow(clippy::too_many_arguments)]
fn render_modal_card(
    app_state: &Entity<AppState>,
    branch_input: &Entity<TextInput>,
    copy_ignored: bool,
    source_branch: &str,
    placeholder: &str,
    app_state_cancel: Entity<AppState>,
    theme: &Theme,
    cx: &mut gpui::Context<'_, WorktreeModal>,
) -> gpui::Stateful<gpui::Div> {
    let border = theme.border;
    let border_variant = theme.border_variant;
    let text_color = theme.text;
    let accent = theme.accent;
    let element_hover = theme.element_hover;

    div()
        .id("worktree-modal")
        .w(px(380.))
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
        .child(render_header(app_state, theme))
        .child(render_body(
            branch_input,
            copy_ignored,
            source_branch,
            placeholder,
            theme,
            cx,
        ))
        .child(render_footer(
            app_state_cancel,
            border,
            border_variant,
            text_color,
            accent,
            element_hover,
            theme,
            cx,
        ))
}

fn render_header(app_state: &Entity<AppState>, theme: &Theme) -> gpui::Div {
    let close_state = app_state.clone();
    let text = theme.text;
    let text_muted = theme.text_muted;

    div()
        .px(px(16.))
        .py(px(12.))
        .flex()
        .flex_row()
        .items_center()
        .border_b_1()
        .border_color(theme.border)
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(6.))
                .flex_1()
                .child(
                    svg()
                        .path(crate::icons::ICON_GIT)
                        .size(px(14.))
                        .text_color(theme.text)
                        .flex_shrink_0(),
                )
                .child(div().text_color(theme.text).child("New Worktree")),
        )
        .child(
            div()
                .cursor_pointer()
                .text_color(text_muted)
                .hover(move |s| s.text_color(text))
                .child(svg().path(crate::icons::ICON_CLOSE).size(px(14.)))
                .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                    close_state.update(cx, AppState::close_worktree_modal);
                }),
        )
}

#[allow(clippy::too_many_arguments)]
fn render_body(
    branch_input: &Entity<TextInput>,
    copy_ignored: bool,
    source_branch: &str,
    placeholder: &str,
    theme: &Theme,
    cx: &mut gpui::Context<'_, WorktreeModal>,
) -> gpui::Div {
    let text_muted = theme.text_muted;
    div()
        .px(px(16.))
        .py(px(12.))
        .flex()
        .flex_col()
        .gap(px(12.))
        .child(
            div()
                .text_size(px(12.))
                .text_color(theme.text_muted)
                .child(format!("From: {source_branch}")),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(4.))
                .child(
                    div()
                        .text_size(px(11.))
                        .text_color(theme.text_faint)
                        .child("BRANCH NAME"),
                )
                .child(
                    div()
                        .px(px(8.))
                        .py(px(6.))
                        .rounded(px(4.))
                        .border_1()
                        .border_color(theme.border)
                        .bg(theme.editor_bg)
                        .child(branch_input.clone()),
                )
                .child(
                    div()
                        .text_size(px(11.))
                        .text_color(theme.text_faint)
                        .child(format!("Empty = \"{placeholder}\"")),
                ),
        )
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(8.))
                .cursor_pointer()
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|this, _, _, cx| {
                        this.copy_ignored = !this.copy_ignored;
                        cx.notify();
                    }),
                )
                .child(
                    div()
                        .text_size(px(14.))
                        .text_color(text_muted)
                        .child(if copy_ignored { "\u{2611}" } else { "\u{2610}" }),
                )
                .child(
                    div()
                        .text_size(px(12.))
                        .text_color(text_muted)
                        .child("Copy .gitignore'd files"),
                ),
        )
}

#[allow(clippy::too_many_arguments)]
fn render_footer(
    app_state_cancel: Entity<AppState>,
    border: gpui::Rgba,
    border_variant: gpui::Rgba,
    text_color: gpui::Rgba,
    accent: gpui::Rgba,
    element_hover: gpui::Rgba,
    theme: &Theme,
    cx: &mut gpui::Context<'_, WorktreeModal>,
) -> gpui::Div {
    div()
        .px(px(16.))
        .py(px(12.))
        .border_t_1()
        .border_color(theme.border)
        .flex()
        .flex_row()
        .justify_end()
        .gap(px(8.))
        .child(
            div()
                .id("worktree-cancel")
                .px(px(12.))
                .py(px(6.))
                .rounded(px(4.))
                .cursor_pointer()
                .bg(border)
                .hover(move |s| s.bg(border_variant))
                .text_color(text_color)
                .text_size(px(13.))
                .child("Cancel")
                .on_click(move |_, _, cx| {
                    app_state_cancel.update(cx, AppState::close_worktree_modal);
                }),
        )
        .child(
            div()
                .id("worktree-create")
                .px(px(12.))
                .py(px(6.))
                .rounded(px(4.))
                .cursor_pointer()
                .bg(accent)
                .hover(move |s| s.bg(element_hover))
                .text_color(gpui::rgba(0xffff_ffff))
                .text_size(px(13.))
                .child("Create")
                .on_click(cx.listener(|this, _, window, cx| {
                    this.on_create(window, cx);
                })),
        )
}
