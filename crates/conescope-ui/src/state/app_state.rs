use conescope_core::instance::InstanceUpdate;
use gpui::{AppContext, Entity};

use super::db_worker::DbHandle;
use super::instance_list::InstanceList;
use super::project_store::ProjectStore;
use super::settings_store::{SettingsStore, ViewMode};
use crate::views::text_input::{TextInput, TextInputEvent};

/// Pending confirmation dialog state.
#[derive(Debug, Clone)]
pub struct ConfirmAction {
    pub title: String,
    pub message: String,
    pub instance_id: String,
}

pub struct AppState {
    pub instance_list: Entity<InstanceList>,
    pub project_store: Entity<ProjectStore>,
    pub settings_store: Entity<SettingsStore>,
    pub db: DbHandle,
    pub questions_queue_open: bool,
    pub settings_modal_open: bool,
    pub new_instance_modal_open: bool,
    pub confirm_action: Option<ConfirmAction>,
    /// Error message to display (error boundary).
    pub error_message: Option<String>,
    /// Tile being edited (overview grid inline title editing).
    pub editing_tile_id: Option<String>,
    pub editing_input: Option<Entity<TextInput>>,
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

            Self {
                instance_list,
                project_store,
                settings_store,
                db,
                questions_queue_open: false,
                settings_modal_open: false,
                new_instance_modal_open: false,
                confirm_action: None,
                error_message: None,
                editing_tile_id: None,
                editing_input: None,
            }
        })
    }

    #[must_use]
    pub fn view_mode(&self, cx: &gpui::App) -> ViewMode {
        self.settings_store.read(cx).view_mode()
    }

    #[must_use]
    pub fn focused_instance_id<'a>(&self, cx: &'a gpui::App) -> Option<&'a str> {
        self.settings_store.read(cx).focused_instance_id()
    }

    pub fn focus_instance(&mut self, id: &str, cx: &mut gpui::Context<Self>) {
        let mut session = self.settings_store.read(cx).session().clone();
        session.view_mode = ViewMode::Focus;
        session.focused_instance_id = Some(id.to_owned());
        self.settings_store
            .update(cx, |store, _| store.save_session(session));
        cx.notify();
    }

    pub fn return_to_overview(&mut self, cx: &mut gpui::Context<Self>) {
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

    pub fn toggle_settings_modal(&mut self, cx: &mut gpui::Context<Self>) {
        self.settings_modal_open = !self.settings_modal_open;
        cx.notify();
    }

    pub fn toggle_new_instance_modal(&mut self, cx: &mut gpui::Context<Self>) {
        self.new_instance_modal_open = !self.new_instance_modal_open;
        cx.notify();
    }

    // --- Panel visibility ---

    #[must_use]
    pub fn sidebar_visible(&self, cx: &gpui::App) -> bool {
        self.settings_store.read(cx).session().folder_panel_visible
    }

    #[must_use]
    pub fn editor_visible(&self, cx: &gpui::App) -> bool {
        self.settings_store.read(cx).session().editor_panel_visible
    }

    #[must_use]
    pub fn terminal_visible(&self, cx: &gpui::App) -> bool {
        self.settings_store
            .read(cx)
            .session()
            .terminal_panel_visible
    }

    #[must_use]
    pub fn sidebar_width(&self, cx: &gpui::App) -> f32 {
        self.settings_store.read(cx).session().sidebar_width
    }

    #[must_use]
    pub fn terminal_height(&self, cx: &gpui::App) -> f32 {
        self.settings_store.read(cx).session().terminal_height
    }

    pub fn toggle_sidebar(&mut self, cx: &mut gpui::Context<Self>) {
        let mut session = self.settings_store.read(cx).session().clone();
        session.folder_panel_visible = !session.folder_panel_visible;
        self.settings_store
            .update(cx, |store, _| store.save_session(session));
        cx.notify();
    }

    pub fn toggle_editor(&mut self, cx: &mut gpui::Context<Self>) {
        let mut session = self.settings_store.read(cx).session().clone();
        session.editor_panel_visible = !session.editor_panel_visible;
        self.settings_store
            .update(cx, |store, _| store.save_session(session));
        cx.notify();
    }

    pub fn toggle_terminal(&mut self, cx: &mut gpui::Context<Self>) {
        let mut session = self.settings_store.read(cx).session().clone();
        session.terminal_panel_visible = !session.terminal_panel_visible;
        self.settings_store
            .update(cx, |store, _| store.save_session(session));
        cx.notify();
    }

    pub fn set_sidebar_width(&mut self, width: f32, cx: &mut gpui::Context<Self>) {
        let mut session = self.settings_store.read(cx).session().clone();
        session.sidebar_width = width;
        self.settings_store
            .update(cx, |store, _| store.save_session(session));
        cx.notify();
    }

    pub fn set_terminal_height(&mut self, height: f32, cx: &mut gpui::Context<Self>) {
        let mut session = self.settings_store.read(cx).session().clone();
        session.terminal_height = height;
        self.settings_store
            .update(cx, |store, _| store.save_session(session));
        cx.notify();
    }

    /// Save open editor tabs to session state.
    pub fn save_editor_tabs(
        &mut self,
        tabs: Vec<String>,
        active: Option<String>,
        cx: &mut gpui::Context<Self>,
    ) {
        let mut session = self.settings_store.read(cx).session().clone();
        session.open_editor_tabs = tabs;
        session.active_editor_tab = active;
        self.settings_store
            .update(cx, |store, _| store.save_session(session));
    }

    /// Get saved editor tabs from session state.
    #[must_use]
    pub fn saved_editor_tabs(&self, cx: &gpui::App) -> (Vec<String>, Option<String>) {
        let session = self.settings_store.read(cx).session();
        (
            session.open_editor_tabs.clone(),
            session.active_editor_tab.clone(),
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
}

/// Window position and size for persistence.
#[derive(Debug)]
pub struct WindowBounds {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}
