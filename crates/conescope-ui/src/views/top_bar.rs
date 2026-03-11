use gpui::prelude::*;
use gpui::{Entity, Hsla, MouseButton, div, px, svg};

use crate::icons;
use crate::state::app_state::{AppState, WorktreeDropdownItem};
use crate::state::settings_store::ViewMode;
use crate::theme::Theme;
use crate::views::colors::{default_instance_color, hex_to_rgba};

type BadgeClickHandler =
    Box<dyn Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static>;

#[derive(Debug)]
pub struct TopBar {
    app_state: Entity<AppState>,
    cached_has_unfocused_questions: bool,
    cached_pulse_opacity: f32,
}

impl TopBar {
    #[must_use]
    pub fn new(app_state: Entity<AppState>, cx: &mut gpui::Context<Self>) -> Self {
        let pulse_timer = app_state.read(cx).pulse_timer.clone();
        let instance_list = app_state.read(cx).instance_list.clone();
        let settings_store = app_state.read(cx).settings_store.clone();

        cx.observe(&pulse_timer, |this, timer, cx| {
            this.cached_pulse_opacity = timer.read(cx).opacity();
            cx.notify();
        })
        .detach();

        cx.observe(&instance_list, {
            let app_state = app_state.clone();
            move |this, _, cx| {
                this.cached_has_unfocused_questions =
                    app_state.read(cx).has_unfocused_questions(cx);
                cx.notify();
            }
        })
        .detach();

        cx.observe(&settings_store, {
            let app_state = app_state.clone();
            move |this, _, cx| {
                this.cached_has_unfocused_questions =
                    app_state.read(cx).has_unfocused_questions(cx);
                cx.notify();
            }
        })
        .detach();

        Self {
            app_state,
            cached_has_unfocused_questions: false,
            cached_pulse_opacity: 1.0,
        }
    }

    fn toggle_worktree_menu(&mut self, click_x: f32, cx: &mut gpui::Context<Self>) {
        if self.app_state.read(cx).worktree_dropdown.is_some() {
            self.app_state.update(cx, |s, cx| {
                s.close_worktree_dropdown(cx);
            });
            return;
        }

        let state = self.app_state.read(cx);
        let fi = focused_info(state, cx);
        let Some(fi) = fi else {
            return;
        };

        let siblings = collect_worktree_siblings(state, &fi, cx);
        if siblings.is_empty() {
            let focused_id = fi.id.clone();
            self.app_state.update(cx, |s, cx| {
                s.open_worktree_modal(&focused_id, cx);
            });
        } else {
            let focused_id = Some(fi.id.clone());
            self.app_state.update(cx, |s, cx| {
                s.open_worktree_dropdown(siblings, focused_id, click_x, cx);
            });
        }
    }
}

struct FocusedInfo {
    id: String,
    num: usize,
    title: String,
    color: gpui::Rgba,
    git_branch: Option<String>,
    git_insertions: usize,
    git_deletions: usize,
    project_id: Option<String>,
    parent_instance_id: Option<String>,
}

/// Resolve the focused instance title, positional number, project color, and git info.
fn focused_info(state: &AppState, cx: &gpui::App) -> Option<FocusedInfo> {
    let id = state.focused_instance_id(cx)?;
    let il = state.instance_list.read(cx);
    let pos = il.entries().iter().position(|e| e.read(cx).id() == id)?;
    let entry = &il.entries()[pos];
    let inst = entry.read(cx);
    let title = inst
        .instance
        .title
        .as_deref()
        .unwrap_or("Untitled")
        .to_owned();
    let color = inst
        .instance
        .color
        .as_deref()
        .map_or_else(default_instance_color, hex_to_rgba);
    let git = &inst.git_summary;
    Some(FocusedInfo {
        id: id.to_owned(),
        num: pos + 1,
        title,
        color,
        git_branch: git.branch.clone(),
        git_insertions: git.insertions,
        git_deletions: git.deletions,
        project_id: inst.instance.project_id.clone(),
        parent_instance_id: inst.parent_instance_id().map(str::to_owned),
    })
}

/// Build the focus-mode left additions: back arrow + title + git badge.
fn render_focus_left(
    fi: &FocusedInfo,
    app_state_for_back: Entity<AppState>,
    font_size: f32,
    theme: &Theme,
    on_badge_click: Option<impl Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static>,
) -> gpui::Div {
    let icon_size = px(font_size + 1.0);
    let text_hover: Hsla = theme.text.into();
    let text_muted: Hsla = theme.text_muted.into();

    let title_text = format!("\u{2318}{} {}", fi.num, fi.title);

    let mut section = div()
        .flex()
        .flex_row()
        .items_center()
        // Back arrow (return to overview)
        .child(
            div()
                .ml(px(4.))
                .px(px(6.))
                .py(px(4.))
                .rounded(px(4.))
                .cursor_pointer()
                .hover(move |s| s.text_color(text_hover))
                .child(
                    svg()
                        .path(icons::ICON_ARROW_LEFT)
                        .size(icon_size)
                        .text_color(text_muted)
                        .flex_shrink_0(),
                )
                .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                    app_state_for_back.update(cx, AppState::return_to_overview);
                }),
        )
        // Title
        .child(div().ml(px(8.)).text_color(fi.color).child(title_text));

    // Git badge
    if let Some(branch) = &fi.git_branch {
        let green = gpui::rgba(0x4ec9_4eff);
        let red = gpui::rgba(0xf851_49ff);
        let secondary_size = px(font_size - 1.0);
        let accent_hover: Hsla = theme.accent.into();

        let mut badge = div()
            .id("git-badge")
            .ml(px(10.))
            .flex()
            .flex_row()
            .items_center()
            .gap(px(3.))
            .text_size(secondary_size)
            .text_color(theme.text_muted)
            .cursor_pointer()
            .rounded(px(4.))
            .px(px(4.))
            .py(px(2.))
            .hover(move |s| s.text_color(accent_hover))
            .child(
                svg()
                    .path(icons::ICON_GIT)
                    .size(icon_size)
                    .text_color(theme.text_muted)
                    .flex_shrink_0(),
            )
            .child(branch.clone());

        if fi.git_insertions > 0 {
            badge = badge.child(
                div()
                    .text_color(green)
                    .child(format!("+{}", fi.git_insertions)),
            );
        }
        if fi.git_deletions > 0 {
            badge = badge.child(
                div()
                    .text_color(red)
                    .child(format!("-{}", fi.git_deletions)),
            );
        }

        if let Some(handler) = on_badge_click {
            badge = badge.on_click(move |ev, win, cx| {
                cx.stop_propagation();
                handler(ev, win, cx);
            });
        }

        section = section.child(badge);
    }

    section
}

impl Render for TopBar {
    #[allow(clippy::too_many_lines)]
    fn render(
        &mut self,
        _window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> impl IntoElement {
        let state = self.app_state.read(cx);
        let theme = state.theme().clone();
        let view_mode = state.view_mode(cx);
        let font_size = state.settings_store.read(cx).settings().ui_font_size();
        let sidebar_open = state.sidebar_open(cx);

        let focus_info = if view_mode == ViewMode::Focus {
            focused_info(state, cx)
        } else {
            None
        };

        let title_color: gpui::Rgba = focus_info.as_ref().map_or(theme.text_muted, |fi| fi.color);
        let center_text = match view_mode {
            ViewMode::Settings => "SETTINGS".to_string(),
            _ => String::new(),
        };

        let app_state_for_close = self.app_state.clone();
        let app_state_for_back = self.app_state.clone();
        let app_state_for_sidebar = self.app_state.clone();

        let right_section = match view_mode {
            ViewMode::Focus => render_focus_close_button(app_state_for_close, font_size, &theme),
            ViewMode::Overview => render_overview_buttons(&self.app_state, font_size, &theme),
            ViewMode::Settings => render_settings_buttons(&self.app_state, font_size, &theme),
        };

        let bar_height = px(font_size * 2.0 + 6.0);
        let icon_size = px(font_size + 1.0);
        let sidebar_icon_color: Hsla = if sidebar_open {
            theme.accent.into()
        } else {
            theme.text_muted.into()
        };
        let sidebar_hover: Hsla = theme.text.into();

        let show_notification_dot =
            view_mode == ViewMode::Focus && self.cached_has_unfocused_questions;
        let pulse_opacity = self.cached_pulse_opacity;

        let sidebar_btn = (!sidebar_open).then(|| {
            let mut btn = div()
                .relative()
                .px(px(6.))
                .py(px(4.))
                .rounded(px(4.))
                .cursor_pointer()
                .hover(move |s| s.text_color(sidebar_hover))
                .child(
                    svg()
                        .path(icons::ICON_SIDEBAR)
                        .size(icon_size)
                        .text_color(sidebar_icon_color)
                        .flex_shrink_0(),
                )
                .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                    app_state_for_sidebar.update(cx, AppState::toggle_sidebar_open);
                });
            if show_notification_dot {
                btn = btn.child(
                    div()
                        .absolute()
                        .top(px(-2.))
                        .right(px(-2.))
                        .w(px(8.))
                        .h(px(8.))
                        .rounded(px(4.))
                        .bg(gpui::rgba(0xf871_71ff))
                        .opacity(pulse_opacity),
                );
            }
            btn
        });

        let mut left_section = div()
            .flex()
            .flex_row()
            .items_center()
            .child(div().w(px(8.)))
            .children(sidebar_btn);

        if let Some(fi) = &focus_info {
            let badge_handler: Option<BadgeClickHandler> = if fi.git_branch.is_some() {
                let this = cx.entity().clone();
                Some(Box::new(
                    move |ev: &gpui::ClickEvent, _: &mut gpui::Window, cx: &mut gpui::App| {
                        let click_x = f32::from(ev.position().x);
                        this.update(cx, |bar, cx| bar.toggle_worktree_menu(click_x, cx));
                    },
                ))
            } else {
                None
            };
            left_section = left_section.child(render_focus_left(
                fi,
                app_state_for_back,
                font_size,
                &theme,
                badge_handler,
            ));
        }

        div()
            .id("top-bar")
            .relative()
            .h(bar_height)
            .w_full()
            .flex()
            .flex_row()
            .items_center()
            .text_size(px(font_size))
            .bg(theme.panel)
            .rounded_tl(px(10.))
            .rounded_tr(px(10.))
            .border_b_1()
            .border_color(theme.border)
            .on_click(|event, window, _cx| {
                if event.click_count() == 2 {
                    window.titlebar_double_click();
                }
            })
            .child(left_section)
            .child(
                div()
                    .flex_1()
                    .flex()
                    .justify_center()
                    .text_color(title_color)
                    .child(center_text),
            )
            .child(right_section)
    }
}

fn collect_worktree_siblings(
    state: &AppState,
    fi: &FocusedInfo,
    cx: &gpui::App,
) -> Vec<WorktreeDropdownItem> {
    let il = state.instance_list.read(cx);
    let entries = il.entries();

    let focused_id = &fi.id;
    let project_id = &fi.project_id;
    let parent_id = &fi.parent_instance_id;

    let root_id = parent_id.as_deref().unwrap_or(focused_id.as_str());

    entries
        .iter()
        .filter_map(|e| {
            let inst = e.read(cx);
            let id = inst.id().to_owned();

            let is_sibling = id == root_id
                || inst.parent_instance_id() == Some(root_id)
                || (project_id.is_some()
                    && inst.instance.project_id.as_ref() == project_id.as_ref()
                    && inst.is_worktree());

            if !is_sibling || id == *focused_id {
                return None;
            }

            let title = inst
                .instance
                .title
                .clone()
                .unwrap_or_else(|| "Untitled".into());
            let branch = inst
                .worktree_branch()
                .map(str::to_owned)
                .or_else(|| inst.git_summary.branch.clone());

            Some(WorktreeDropdownItem { id, title, branch })
        })
        .collect()
}

fn render_overview_buttons(
    _app_state: &Entity<AppState>,
    _font_size: f32,
    _theme: &Theme,
) -> gpui::Div {
    div()
}

fn render_settings_buttons(
    app_state: &Entity<AppState>,
    font_size: f32,
    theme: &Theme,
) -> gpui::Div {
    let text_muted: Hsla = theme.text_muted.into();
    let text: Hsla = theme.text.into();
    let border = theme.border;
    let border_variant = theme.border_variant;
    let app_state = app_state.clone();

    div()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(8.))
        .pr(px(12.))
        .child(
            div()
                .id("settings-save-btn")
                .flex()
                .flex_row()
                .items_center()
                .px(px(10.))
                .py(px(3.))
                .rounded(px(16.))
                .border_1()
                .border_color(theme.text_disabled)
                .cursor_pointer()
                .text_size(px(font_size - 1.0))
                .text_color(text_muted)
                .hover(move |s| s.bg(border).border_color(border_variant).text_color(text))
                .on_click(move |_, _, cx| {
                    app_state.update(cx, AppState::close_settings);
                })
                .child("Save"),
        )
}

fn render_focus_close_button(
    app_state_for_close: Entity<AppState>,
    font_size: f32,
    theme: &Theme,
) -> gpui::Div {
    let text_muted: Hsla = theme.text_muted.into();
    let destructive_hover: Hsla = theme.destructive_hover.into();
    let icon_size = px(font_size + 1.0);

    div()
        .flex()
        .flex_row()
        .pr(px(12.))
        // Close button (trash icon, red on hover)
        .child(
            div()
                .px(px(6.))
                .py(px(4.))
                .rounded(px(4.))
                .cursor_pointer()
                .hover(move |s| s.text_color(destructive_hover))
                .child(
                    svg()
                        .path(icons::ICON_TRASH)
                        .size(icon_size)
                        .text_color(text_muted)
                        .flex_shrink_0(),
                )
                .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                    let (id, title) = {
                        let s = app_state_for_close.read(cx);
                        let fid = s
                            .focused_instance_id(cx)
                            .unwrap_or_default()
                            .to_owned();
                        let il = s.instance_list.read(cx);
                        let t = il
                            .find_by_id(&fid, cx)
                            .map(|e| {
                                e.read(cx)
                                    .instance
                                    .title
                                    .clone()
                                    .unwrap_or_else(|| "Untitled".into())
                            })
                            .unwrap_or_default();
                        (fid, t)
                    };
                    app_state_for_close.update(cx, |s, cx| {
                        s.request_close_instance(&id, &title, cx);
                    });
                }),
        )
}
