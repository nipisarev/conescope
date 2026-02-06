use chrono::Utc;
use conescope_core::instance::{Instance, InstanceType, InstanceUpdate};
use gpui::{AppContext, Entity};

use super::db_worker::DbHandle;
use super::instance_entry::{InstanceEntry, InstanceEvent};
use super::project_store::ProjectStore;

#[derive(Debug, Clone)]
pub enum InstanceListEvent {
    Added(String),
    Removed(String),
}

impl gpui::EventEmitter<InstanceListEvent> for InstanceList {}

#[derive(Debug)]
pub struct InstanceList {
    entries: Vec<Entity<InstanceEntry>>,
    db: DbHandle,
}

impl InstanceList {
    #[must_use]
    pub fn new(db: DbHandle) -> Self {
        Self {
            entries: Vec::new(),
            db,
        }
    }

    #[must_use]
    pub fn entries(&self) -> &[Entity<InstanceEntry>] {
        &self.entries
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    #[must_use]
    pub fn find_by_id<'a>(
        &'a self,
        id: &str,
        cx: &'a gpui::App,
    ) -> Option<&'a Entity<InstanceEntry>> {
        self.entries.iter().find(|e| e.read(cx).id() == id)
    }

    /// Find entry by 1-based instance number.
    #[must_use]
    pub fn find_by_number(&self, number: i64, cx: &gpui::App) -> Option<&Entity<InstanceEntry>> {
        self.entries
            .iter()
            .find(|e| e.read(cx).instance.instance_number == Some(number))
    }

    /// Load instances from DB (without PTY — see `restore_terminals` for that).
    pub fn load_from_db(&mut self, instances: Vec<Instance>, cx: &mut gpui::Context<Self>) {
        self.entries.clear();
        for inst in instances {
            let entry = cx.new(|_| InstanceEntry::from_instance(inst));
            self.entries.push(entry);
        }
        cx.notify();
    }

    /// Create a new instance with a live PTY.
    ///
    /// Inserts into DB, spawns terminal, subscribes to events for DB persistence.
    pub fn create_instance(
        &mut self,
        instance: Instance,
        cwd: &str,
        window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) {
        let id = instance.id.clone();

        // Fire-and-forget DB insert
        self.db.insert_instance(instance.clone());

        // Spawn PTY
        let pane = crate::terminal::spawn_terminal_pane(Some(cwd), window, cx);

        // Create entity
        let entry = cx.new(|_| {
            let mut e = InstanceEntry::from_instance(instance);
            e.attach_terminal(pane);
            e
        });

        // Subscribe to events for DB persistence.
        // Context<T>::subscribe gives: FnMut(&mut T, Entity<Emitter>, &Evt, &mut Context<T>)
        let sub = cx.subscribe(&entry, |this, entity, event, cx| {
            let inst = entity.read(cx);
            match event {
                InstanceEvent::StatusChanged(status) => {
                    this.db.update_instance(
                        inst.id().to_owned(),
                        InstanceUpdate {
                            status: Some(*status),
                            ..Default::default()
                        },
                    );
                }
                InstanceEvent::Exited => {
                    let ended = Utc::now().to_rfc3339();
                    this.db.end_instance(inst.id().to_owned(), ended);
                }
            }
        });
        sub.detach();

        // Start output polling for the new entry
        entry.update(cx, |e, cx| {
            e.start_output_polling(cx);
        });

        // For project instances, launch claude
        if entry.read(cx).instance_type() == InstanceType::Project {
            entry.read(cx).send_input(b"claude\r");
        }

        self.entries.push(entry);
        cx.emit(InstanceListEvent::Added(id));
        cx.notify();
    }

    /// Restore terminals for previously loaded DB entries.
    ///
    /// For each entry (loaded via `load_from_db`), spawns a PTY, attaches it,
    /// and starts output polling.
    pub fn restore_terminals(
        &mut self,
        project_store: &Entity<ProjectStore>,
        window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());

        for entry in &self.entries {
            // Extract values while holding the read borrow, then release it
            let (cwd, is_project) = {
                let inst = entry.read(cx);
                let cwd = inst
                    .instance
                    .project_id
                    .as_ref()
                    .and_then(|pid| project_store.read(cx).get(pid).map(|p| p.path.clone()))
                    .unwrap_or_else(|| home.clone());
                let is_project = inst.instance_type() == InstanceType::Project;
                (cwd, is_project)
            };

            let pane = crate::terminal::spawn_terminal_pane(Some(&cwd), window, cx);
            entry.update(cx, |e, cx| {
                e.attach_terminal(pane);
                e.start_output_polling(cx);
            });

            if is_project {
                entry.read(cx).send_input(b"claude\r");
            }
        }
    }

    /// Add a pre-built entry and subscribe for DB persistence.
    ///
    /// Used by action handlers that create entries externally (with Window access).
    pub fn push_entry(&mut self, entry: Entity<InstanceEntry>, cx: &mut gpui::Context<Self>) {
        let id = entry.read(cx).id().to_owned();

        let sub = cx.subscribe(&entry, |this, entity, event, cx| {
            let inst = entity.read(cx);
            match event {
                InstanceEvent::StatusChanged(status) => {
                    this.db.update_instance(
                        inst.id().to_owned(),
                        InstanceUpdate {
                            status: Some(*status),
                            ..Default::default()
                        },
                    );
                }
                InstanceEvent::Exited => {
                    let ended = Utc::now().to_rfc3339();
                    this.db.end_instance(inst.id().to_owned(), ended);
                }
            }
        });
        sub.detach();

        self.entries.push(entry);
        cx.emit(InstanceListEvent::Added(id));
        cx.notify();
    }

    /// Remove an instance: kill PTY, mark exited, end in DB, remove from list.
    pub fn remove_instance(&mut self, id: &str, cx: &mut gpui::Context<Self>) {
        // Kill PTY for the entry being removed
        if let Some(entry) = self.entries.iter().find(|e| e.read(cx).id() == id) {
            entry.update(cx, |e, cx| {
                e.kill_pty();
                e.mark_exited(cx);
            });
        }

        let ended = Utc::now().to_rfc3339();
        self.db.end_instance(id.to_owned(), ended);
        self.entries.retain(|e| e.read(cx).id() != id);
        cx.emit(InstanceListEvent::Removed(id.to_owned()));
        cx.notify();
    }

    /// Access the DB handle (for callers that need to query).
    #[must_use]
    pub fn db(&self) -> &DbHandle {
        &self.db
    }
}
