use conescope_core::instance::InstanceUpdate;
use gpui::{AppContext, Entity};

use super::db_worker::DbHandle;
use super::instance_list::InstanceList;
use super::project_store::ProjectStore;
use super::settings_store::{SettingsStore, ViewMode};

pub struct AppState {
    pub instance_list: Entity<InstanceList>,
    pub project_store: Entity<ProjectStore>,
    pub settings_store: Entity<SettingsStore>,
    pub db: DbHandle,
    pub questions_queue_open: bool,
    pub settings_modal_open: bool,
    pub new_instance_modal_open: bool,
    /// Tile being edited (overview grid inline title editing).
    pub editing_tile_id: Option<String>,
    pub editing_buffer: String,
    pub edit_focus: gpui::FocusHandle,
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
            let edit_focus = cx.focus_handle();

            Self {
                instance_list,
                project_store,
                settings_store,
                db,
                questions_queue_open: false,
                settings_modal_open: false,
                new_instance_modal_open: false,
                editing_tile_id: None,
                editing_buffer: String::new(),
                edit_focus,
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
    pub fn start_edit_title(&mut self, id: &str, current_title: &str) {
        self.editing_tile_id = Some(id.to_owned());
        current_title.clone_into(&mut self.editing_buffer);
    }

    /// Save the edited title to the instance and DB.
    pub fn save_edit_title(&mut self, cx: &mut gpui::Context<Self>) {
        let Some(id) = self.editing_tile_id.take() else {
            return;
        };
        let title = self.editing_buffer.clone();
        self.editing_buffer.clear();

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
        self.editing_buffer.clear();
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
}
