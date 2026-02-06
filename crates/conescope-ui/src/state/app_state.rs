use gpui::{AppContext, Entity};

use super::db_worker::DbHandle;
use super::instance_list::InstanceList;
use super::project_store::ProjectStore;
use super::settings_store::{SettingsStore, ViewMode};

#[derive(Debug)]
pub struct AppState {
    pub instance_list: Entity<InstanceList>,
    pub project_store: Entity<ProjectStore>,
    pub settings_store: Entity<SettingsStore>,
    pub db: DbHandle,
    pub questions_queue_open: bool,
    pub settings_modal_open: bool,
    pub new_instance_modal_open: bool,
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
