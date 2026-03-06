use conescope_core::instance::InstanceUpdate;
use gpui::{AppContext, Entity};

use super::db_worker::DbHandle;
use super::git_store::GitStore;
use super::instance_list::InstanceList;
use super::project_store::ProjectStore;
use super::pulse_timer::PulseTimer;
use super::session_detector::SessionStatus;
use super::settings_store::{SettingsStore, SidebarMode, SidebarTab, ViewMode};
use crate::theme::{Theme, ThemeMode};
use crate::views::text_input::{TextInput, TextInputEvent};

/// Pending confirmation dialog state.
#[derive(Debug, Clone)]
pub struct ConfirmAction {
    pub title: String,
    pub message: String,
    pub instance_id: String,
}

#[allow(clippy::struct_excessive_bools)]
pub struct AppState {
    pub instance_list: Entity<InstanceList>,
    pub project_store: Entity<ProjectStore>,
    pub settings_store: Entity<SettingsStore>,
    pub git_store: Entity<GitStore>,
    pub pulse_timer: Entity<PulseTimer>,
    pub db: DbHandle,
    pub questions_queue_open: bool,
    pub new_instance_modal_open: bool,
    pub confirm_action: Option<ConfirmAction>,
    /// Error message to display (error boundary).
    pub error_message: Option<String>,
    /// Tile being edited (overview grid inline title editing).
    pub editing_tile_id: Option<String>,
    pub editing_input: Option<Entity<TextInput>>,
    /// Active color theme.
    theme: Theme,
    /// Whether the overlay sidebar is currently visible (transient, not persisted).
    pub sidebar_overlay_visible: bool,
    /// Whether the overlay sidebar content should be visible (staggered animation).
    pub overlay_content_visible: bool,
    /// Generation counter to cancel stale hover-open / auto-hide timers.
    pub sidebar_hover_gen: u32,
    /// Bumped when a debounced terminal resize completes (drives fade-in animation).
    pub terminal_resize_gen: usize,
}

impl std::fmt::Debug for AppState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppState")
            .field("editing_tile_id", &self.editing_tile_id)
            .finish_non_exhaustive()
    }
}

impl AppState {
    #[must_use]
    pub fn new(db: DbHandle, cx: &mut gpui::App) -> Entity<Self> {
        let db2 = db.clone();
        let db3 = db.clone();
        let db4 = db.clone();
        cx.new(|cx| {
            let instance_list = cx.new(|_| InstanceList::new(db2));
            let project_store = cx.new(|_| ProjectStore::new(db3));
            let settings_store = cx.new(|_| SettingsStore::new(db4));
            let git_store = cx.new(|_| GitStore::new());
            let pulse_timer = cx.new(PulseTimer::new);

            // Drive PulseTimer active state from instance pulsing status
            let pt = pulse_timer.clone();
            cx.observe(&instance_list, move |_this, il, cx| {
                let pulsing = il
                    .read(cx)
                    .entries()
                    .iter()
                    .any(|e| e.read(cx).session_status().is_pulsing());
                pt.update(cx, |t, _| t.set_active(pulsing));
            })
            .detach();

            Self {
                instance_list,
                project_store,
                settings_store,
                git_store,
                pulse_timer,
                db,
                questions_queue_open: false,
                new_instance_modal_open: false,
                confirm_action: None,
                error_message: None,
                editing_tile_id: None,
                editing_input: None,
                theme: Theme::load_builtin(ThemeMode::Light),
                sidebar_overlay_visible: false,
                overlay_content_visible: false,
                sidebar_hover_gen: 0,
                terminal_resize_gen: 0,
            }
        })
    }

    #[must_use]
    pub fn theme(&self) -> &Theme {
        &self.theme
    }

    pub fn set_theme(&mut self, mode: ThemeMode, cx: &mut gpui::Context<Self>) {
        self.theme = Theme::load_builtin(mode);
        sync_gpui_component_theme(&self.theme, cx);
        self.settings_store.update(cx, |store, _| {
            mode.as_str().clone_into(&mut store.settings_mut().theme);
        });
        cx.notify();
    }

    pub fn cycle_theme(&mut self, cx: &mut gpui::Context<Self>) {
        let new_mode = self.theme.mode.toggle();
        self.set_theme(new_mode, cx);
    }

    #[must_use]
    pub fn view_mode(&self, cx: &gpui::App) -> ViewMode {
        self.settings_store.read(cx).view_mode()
    }

    #[must_use]
    pub fn focused_instance_id<'a>(&self, cx: &'a gpui::App) -> Option<&'a str> {
        self.settings_store.read(cx).focused_instance_id()
    }

    #[must_use]
    pub fn has_unfocused_questions(&self, cx: &gpui::App) -> bool {
        let focused_id = self.focused_instance_id(cx);
        let il = self.instance_list.read(cx);
        il.entries().iter().any(|e| {
            let entry = e.read(cx);
            let is_focused = focused_id.is_some_and(|fid| fid == entry.id());
            !is_focused && entry.session_status() == SessionStatus::Question
        })
    }

    pub fn focus_instance(&mut self, id: &str, cx: &mut gpui::Context<Self>) {
        tracing::debug!(instance_id = id, "Focusing instance");
        let old_id = self
            .settings_store
            .read(cx)
            .focused_instance_id()
            .map(str::to_owned);
        // Switch per-instance layout: save old, load new
        self.settings_store.update(cx, |store, _| {
            store.switch_instance_layout(old_id.as_deref(), id);
        });
        // Update global session
        let mut session = self.settings_store.read(cx).session().clone();
        session.view_mode = ViewMode::Focus;
        session.focused_instance_id = Some(id.to_owned());
        self.settings_store
            .update(cx, |store, _| store.save_session(session));
        tracing::debug!(instance_id = id, "Instance focused, session saved");
        cx.notify();
    }

    pub fn return_to_overview(&mut self, cx: &mut gpui::Context<Self>) {
        // Persist current layout while focused_instance_id is still set
        self.settings_store
            .update(cx, |store, _| store.persist_current_layout());
        let mut session = self.settings_store.read(cx).session().clone();
        session.view_mode = ViewMode::Overview;
        session.focused_instance_id = None;
        self.settings_store
            .update(cx, |store, _| store.save_session(session));
        cx.notify();
    }

    /// Begin inline editing of a tile's title.
    pub fn start_edit_title(
        &mut self,
        id: &str,
        current_title: &str,
        cx: &mut gpui::Context<Self>,
    ) {
        self.editing_tile_id = Some(id.to_owned());
        let input = cx.new(|cx| TextInput::new(current_title, cx));
        cx.subscribe(&input, |this, _input, event, cx| match event {
            TextInputEvent::Submit(_) => this.save_edit_title(cx),
            TextInputEvent::Cancel => this.cancel_edit_title(cx),
        })
        .detach();
        self.editing_input = Some(input);
        cx.notify();
    }

    /// Save the edited title to the instance and DB.
    pub fn save_edit_title(&mut self, cx: &mut gpui::Context<Self>) {
        let Some(id) = self.editing_tile_id.take() else {
            return;
        };
        let title = self
            .editing_input
            .as_ref()
            .map(|input| input.read(cx).content().to_owned())
            .unwrap_or_default();
        self.editing_input = None;

        // Update in-memory instance
        let il = self.instance_list.clone();
        il.update(cx, |il, cx| {
            let entry = il.find_by_id(&id, cx).cloned();
            if let Some(entry) = entry {
                entry.update(cx, |e, cx| {
                    e.instance.title = Some(title.clone());
                    cx.notify();
                });
            }
        });

        // Persist to DB
        self.db.update_instance(
            id,
            InstanceUpdate {
                title: Some(title),
                ..Default::default()
            },
        );
        cx.notify();
    }

    /// Cancel inline editing without saving.
    pub fn cancel_edit_title(&mut self, cx: &mut gpui::Context<Self>) {
        self.editing_tile_id = None;
        self.editing_input = None;
        cx.notify();
    }

    /// Request confirmation before closing an instance.
    pub fn request_close_instance(&mut self, id: &str, title: &str, cx: &mut gpui::Context<Self>) {
        self.confirm_action = Some(ConfirmAction {
            title: "Close Instance".to_owned(),
            message: format!("Close \"{title}\"? The terminal session will be killed."),
            instance_id: id.to_owned(),
        });
        cx.notify();
    }

    /// Execute the confirmed close action.
    pub fn confirm_close(&mut self, cx: &mut gpui::Context<Self>) {
        let Some(action) = self.confirm_action.take() else {
            return;
        };
        let il = self.instance_list.clone();
        // Return to overview if we're closing the focused instance
        let focused = self.focused_instance_id(cx).map(str::to_owned);
        if focused.as_deref() == Some(&action.instance_id) {
            self.return_to_overview(cx);
        }
        il.update(cx, |list, cx| {
            list.remove_instance(&action.instance_id, cx);
        });
        // Clean up orphaned per-instance layout from cache and DB
        self.settings_store.update(cx, |store, _| {
            store.remove_instance_layout(&action.instance_id);
        });
        cx.notify();
    }

    /// Dismiss the confirm dialog without acting.
    pub fn cancel_confirm(&mut self, cx: &mut gpui::Context<Self>) {
        self.confirm_action = None;
        cx.notify();
    }

    pub fn set_error(&mut self, message: String, cx: &mut gpui::Context<Self>) {
        self.error_message = Some(message);
        cx.notify();
    }

    pub fn dismiss_error(&mut self, cx: &mut gpui::Context<Self>) {
        self.error_message = None;
        cx.notify();
    }

    pub fn toggle_questions_queue(&mut self, cx: &mut gpui::Context<Self>) {
        self.questions_queue_open = !self.questions_queue_open;
        cx.notify();
    }

    pub fn open_settings(&mut self, cx: &mut gpui::Context<Self>) {
        self.settings_store
            .update(cx, |store, _| store.enter_settings_mode());
        cx.notify();
    }

    pub fn close_settings(&mut self, cx: &mut gpui::Context<Self>) {
        // Apply theme from current settings
        let theme_str = self.settings_store.read(cx).settings().theme.clone();
        let mode = crate::theme::ThemeMode::from_str_or_default(&theme_str);
        if mode != self.theme.mode {
            self.theme = Theme::load_builtin(mode);
        }

        self.settings_store
            .update(cx, |store, _| store.exit_settings_mode());
        cx.notify();
    }

    pub fn answer_instance_question(
        &self,
        instance_id: &str,
        choice_index: usize,
        cx: &mut gpui::Context<Self>,
    ) {
        let entry = self
            .instance_list
            .read(cx)
            .find_by_id(instance_id, cx)
            .cloned();
        if let Some(entry) = entry {
            entry.update(cx, |e, _| {
                e.answer_question(choice_index);
            });
        }
    }

    pub fn dismiss_instance_session_event(&self, instance_id: &str, cx: &mut gpui::Context<Self>) {
        let entry = self
            .instance_list
            .read(cx)
            .find_by_id(instance_id, cx)
            .cloned();
        if let Some(entry) = entry {
            entry.update(cx, |e, _| {
                e.dismiss_session_event();
            });
        }
    }

    pub fn toggle_new_instance_modal(&mut self, cx: &mut gpui::Context<Self>) {
        self.new_instance_modal_open = !self.new_instance_modal_open;
        cx.notify();
    }

    // --- Panel visibility ---

    #[must_use]
    pub fn sidebar_tab(&self, cx: &gpui::App) -> SidebarTab {
        self.settings_store.read(cx).layout().sidebar_tab
    }

    #[must_use]
    pub fn sidebar_visible(&self, cx: &gpui::App) -> bool {
        self.settings_store.read(cx).layout().folder_panel_visible
    }

    #[must_use]
    pub fn editor_visible(&self, cx: &gpui::App) -> bool {
        self.settings_store.read(cx).layout().editor_panel_visible
    }

    #[must_use]
    pub fn terminal_visible(&self, cx: &gpui::App) -> bool {
        self.settings_store.read(cx).layout().terminal_panel_visible
    }

    #[must_use]
    pub fn sidebar_width(&self, cx: &gpui::App) -> f32 {
        self.settings_store.read(cx).layout().sidebar_width
    }

    #[must_use]
    pub fn terminal_height(&self, cx: &gpui::App) -> f32 {
        self.settings_store.read(cx).layout().terminal_height
    }

    pub fn toggle_sidebar(&mut self, cx: &mut gpui::Context<Self>) {
        let mut layout = self.settings_store.read(cx).layout().clone();
        if layout.folder_panel_visible && layout.sidebar_tab == SidebarTab::Files {
            layout.folder_panel_visible = false;
        } else {
            layout.folder_panel_visible = true;
            layout.sidebar_tab = SidebarTab::Files;
        }
        self.settings_store
            .update(cx, |store, _| store.save_layout(layout));
        cx.notify();
    }

    pub fn toggle_git_panel(&mut self, cx: &mut gpui::Context<Self>) {
        let mut layout = self.settings_store.read(cx).layout().clone();
        if layout.folder_panel_visible && layout.sidebar_tab == SidebarTab::Git {
            layout.folder_panel_visible = false;
        } else {
            layout.folder_panel_visible = true;
            layout.sidebar_tab = SidebarTab::Git;
        }
        self.settings_store
            .update(cx, |store, _| store.save_layout(layout));
        cx.notify();
    }

    /// Make the editor panel visible (if currently hidden).
    pub fn ensure_editor_visible(&mut self, cx: &mut gpui::Context<Self>) {
        if !self.editor_visible(cx) {
            self.toggle_editor(cx);
        }
    }

    pub fn toggle_editor(&mut self, cx: &mut gpui::Context<Self>) {
        let mut layout = self.settings_store.read(cx).layout().clone();
        layout.editor_panel_visible = !layout.editor_panel_visible;
        self.settings_store
            .update(cx, |store, _| store.save_layout(layout));
        cx.notify();
    }

    pub fn toggle_terminal(&mut self, cx: &mut gpui::Context<Self>) {
        let mut layout = self.settings_store.read(cx).layout().clone();
        layout.terminal_panel_visible = !layout.terminal_panel_visible;
        self.settings_store
            .update(cx, |store, _| store.save_layout(layout));
        cx.notify();
    }

    pub fn set_sidebar_width(&mut self, width: f32, cx: &mut gpui::Context<Self>) {
        let mut layout = self.settings_store.read(cx).layout().clone();
        layout.sidebar_width = width;
        self.settings_store
            .update(cx, |store, _| store.save_layout(layout));
        cx.notify();
    }

    pub fn set_terminal_height(&mut self, height: f32, cx: &mut gpui::Context<Self>) {
        let mut layout = self.settings_store.read(cx).layout().clone();
        layout.terminal_height = height;
        self.settings_store
            .update(cx, |store, _| store.save_layout(layout));
        cx.notify();
    }

    /// Save open editor tabs to the current instance's layout.
    pub fn save_editor_tabs(
        &mut self,
        tabs: Vec<String>,
        active: Option<String>,
        cx: &mut gpui::Context<Self>,
    ) {
        let mut layout = self.settings_store.read(cx).layout().clone();
        layout.open_editor_tabs = tabs;
        layout.active_editor_tab = active;
        self.settings_store
            .update(cx, |store, _| store.save_layout(layout));
    }

    /// Get saved editor tabs from the current instance's layout.
    #[must_use]
    pub fn saved_editor_tabs(&self, cx: &gpui::App) -> (Vec<String>, Option<String>) {
        let layout = self.settings_store.read(cx).layout();
        (
            layout.open_editor_tabs.clone(),
            layout.active_editor_tab.clone(),
        )
    }

    /// Save window bounds to session state.
    pub fn save_window_bounds(&mut self, bounds: &WindowBounds, cx: &mut gpui::Context<Self>) {
        let mut session = self.settings_store.read(cx).session().clone();
        session.window_x = Some(bounds.x);
        session.window_y = Some(bounds.y);
        session.window_width = Some(bounds.width);
        session.window_height = Some(bounds.height);
        self.settings_store
            .update(cx, |store, _| store.save_session(session));
    }

    // --- Sidebar mode ---

    #[must_use]
    pub fn sidebar_mode(&self, cx: &gpui::App) -> SidebarMode {
        self.settings_store.read(cx).session().sidebar_mode
    }

    #[must_use]
    pub fn sidebar_open(&self, cx: &gpui::App) -> bool {
        self.settings_store.read(cx).session().sidebar_open
    }

    pub fn toggle_sidebar_open(&mut self, cx: &mut gpui::Context<Self>) {
        let mut session = self.settings_store.read(cx).session().clone();
        if self.sidebar_overlay_visible {
            // Clicking toggle from overlay → pin sidebar, close overlay
            session.sidebar_open = true;
            self.sidebar_overlay_visible = false;
            self.overlay_content_visible = false;
        } else {
            session.sidebar_open = !session.sidebar_open;
        }
        self.settings_store
            .update(cx, |store, _| store.save_session(session));
        cx.notify();
    }

    pub fn toggle_sidebar_visibility(&mut self, cx: &mut gpui::Context<Self>) {
        if self.sidebar_open(cx) {
            let mut session = self.settings_store.read(cx).session().clone();
            session.sidebar_open = false;
            self.sidebar_overlay_visible = true;
            self.settings_store
                .update(cx, |store, _| store.save_session(session));
        } else if self.sidebar_overlay_visible {
            self.sidebar_overlay_visible = false;
            self.overlay_content_visible = false;
            self.sidebar_hover_gen = self.sidebar_hover_gen.wrapping_add(1);
        } else {
            self.sidebar_overlay_visible = true;
        }
        cx.notify();
    }

    pub fn toggle_sidebar_mode(&mut self, cx: &mut gpui::Context<Self>) {
        let mut session = self.settings_store.read(cx).session().clone();
        session.sidebar_mode = match session.sidebar_mode {
            SidebarMode::Pinned => {
                session.sidebar_open = false;
                self.sidebar_overlay_visible = false;
                self.overlay_content_visible = false;
                SidebarMode::Overlay
            }
            SidebarMode::Overlay => {
                session.sidebar_open = true;
                self.sidebar_overlay_visible = false;
                self.overlay_content_visible = false;
                SidebarMode::Pinned
            }
        };
        self.settings_store
            .update(cx, |store, _| store.save_session(session));
        cx.notify();
    }

    pub fn hide_sidebar_overlay(&mut self, cx: &mut gpui::Context<Self>) {
        self.sidebar_overlay_visible = false;
        self.sidebar_hover_gen = self.sidebar_hover_gen.wrapping_add(1);
        cx.notify();
    }

    pub fn show_sidebar_overlay(&mut self, cx: &mut gpui::Context<Self>) {
        self.sidebar_overlay_visible = true;
        self.sidebar_hover_gen = self.sidebar_hover_gen.wrapping_add(1);
        cx.notify();
    }

    /// Increment hover gen to cancel any pending open/hide timers.
    pub fn bump_sidebar_hover_gen(&mut self) -> u32 {
        self.sidebar_hover_gen = self.sidebar_hover_gen.wrapping_add(1);
        self.sidebar_hover_gen
    }
}

/// Sync our custom theme colors into gpui-component's global theme.
///
/// This ensures the code editor Input widget (line numbers, gutter bg,
/// syntax highlighting colors) matches our app's color scheme.
pub fn sync_gpui_component_theme(theme: &Theme, cx: &mut gpui::App) {
    use std::sync::Arc;

    use gpui::Hsla;

    let editor_bg = Hsla::from(theme.editor_bg);
    let text_muted = Hsla::from(theme.text_muted);

    let gc_theme = gpui_component::theme::Theme::global_mut(cx);
    gc_theme.colors.background = gpui::transparent_black();
    gc_theme.colors.foreground = Hsla::from(theme.text);
    gc_theme.colors.muted_foreground = Hsla::from(theme.text_faint);
    gc_theme.colors.border = Hsla::from(theme.border);
    gc_theme.colors.selection = Hsla::from(theme.selection);

    // Update highlight theme: gutter bg + line number + syntax colors
    let old_ht = &gc_theme.highlight_theme;
    let mut new_style = old_ht.style.clone();
    new_style.editor_background = Some(editor_bg);
    new_style.editor_line_number = Some(text_muted);
    new_style.editor_active_line_number = Some(Hsla::from(theme.text));
    new_style.editor_active_line = Some(Hsla::from(theme.element_selected));

    // Parse syntax colors from the Zed theme JSON into gpui-component's SyntaxColors.
    if let Ok(syntax) = serde_json::from_value::<gpui_component::highlighter::SyntaxColors>(
        theme.syntax_json.clone(),
    ) {
        new_style.syntax = syntax;
    }

    gc_theme.highlight_theme = Arc::new(gpui_component::highlighter::HighlightTheme {
        name: old_ht.name.clone(),
        appearance: old_ht.appearance,
        style: new_style,
    });
}

/// Window position and size for persistence.
#[derive(Debug)]
pub struct WindowBounds {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}
