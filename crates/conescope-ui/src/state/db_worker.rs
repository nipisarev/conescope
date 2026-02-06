use std::thread;

use anyhow::Result;
use conescope_core::database::Database;
use conescope_core::instance::{Instance, InstanceUpdate};
use conescope_core::project::Project;
use tracing::{error, info};

pub enum DbCommand {
    // Instance ops
    GetAllInstances {
        reply: flume::Sender<Result<Vec<Instance>>>,
    },
    InsertInstance {
        instance: Instance,
        reply: flume::Sender<Result<()>>,
    },
    UpdateInstance {
        id: String,
        updates: InstanceUpdate,
        reply: flume::Sender<Result<()>>,
    },
    EndInstance {
        id: String,
        ended_at: String,
        reply: flume::Sender<Result<()>>,
    },
    GetNextInstanceNumber {
        reply: flume::Sender<Result<i64>>,
    },
    // Project ops
    GetAllProjects {
        reply: flume::Sender<Result<Vec<Project>>>,
    },
    InsertProject {
        project: Project,
        reply: flume::Sender<Result<()>>,
    },
    UpdateProjectLastUsed {
        id: String,
        timestamp: String,
        reply: flume::Sender<Result<()>>,
    },
    DeleteProject {
        id: String,
        reply: flume::Sender<Result<()>>,
    },
    // Settings ops
    GetAllSettings {
        reply: flume::Sender<Result<Vec<(String, String)>>>,
    },
    SetSetting {
        key: String,
        value: String,
        reply: flume::Sender<Result<()>>,
    },
    // Lifecycle
    Shutdown,
}

impl std::fmt::Debug for DbCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::GetAllInstances { .. } => write!(f, "GetAllInstances"),
            Self::InsertInstance { .. } => write!(f, "InsertInstance"),
            Self::UpdateInstance { id, .. } => write!(f, "UpdateInstance({id})"),
            Self::EndInstance { id, .. } => write!(f, "EndInstance({id})"),
            Self::GetNextInstanceNumber { .. } => write!(f, "GetNextInstanceNumber"),
            Self::GetAllProjects { .. } => write!(f, "GetAllProjects"),
            Self::InsertProject { .. } => write!(f, "InsertProject"),
            Self::UpdateProjectLastUsed { id, .. } => {
                write!(f, "UpdateProjectLastUsed({id})")
            }
            Self::DeleteProject { id, .. } => write!(f, "DeleteProject({id})"),
            Self::GetAllSettings { .. } => write!(f, "GetAllSettings"),
            Self::SetSetting { key, .. } => write!(f, "SetSetting({key})"),
            Self::Shutdown => write!(f, "Shutdown"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct DbHandle {
    tx: flume::Sender<DbCommand>,
}

impl DbHandle {
    /// Spawn a background DB worker thread.
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be opened.
    ///
    /// # Panics
    ///
    /// Panics if the OS cannot spawn a new thread.
    pub fn spawn(db_path: &str) -> Result<Self> {
        let db = Database::open(db_path)?;
        let (tx, rx) = flume::unbounded::<DbCommand>();

        thread::Builder::new()
            .name("db-worker".into())
            .spawn(move || {
                info!("DB worker started");
                Self::run_loop(&db, &rx);
                info!("DB worker stopped");
            })
            .expect("spawn db-worker thread");

        Ok(Self { tx })
    }

    fn run_loop(db: &Database, rx: &flume::Receiver<DbCommand>) {
        while let Ok(cmd) = rx.recv() {
            match cmd {
                DbCommand::Shutdown => break,

                DbCommand::GetAllInstances { reply } => {
                    let _ = reply.send(db.get_all_instances());
                }
                DbCommand::InsertInstance { instance, reply } => {
                    let _ = reply.send(db.insert_instance(&instance));
                }
                DbCommand::UpdateInstance { id, updates, reply } => {
                    let _ = reply.send(db.update_instance(&id, &updates));
                }
                DbCommand::EndInstance {
                    id,
                    ended_at,
                    reply,
                } => {
                    let _ = reply.send(db.end_instance(&id, &ended_at));
                }
                DbCommand::GetNextInstanceNumber { reply } => {
                    let _ = reply.send(db.get_next_instance_number());
                }

                DbCommand::GetAllProjects { reply } => {
                    let _ = reply.send(db.get_all_projects());
                }
                DbCommand::InsertProject { project, reply } => {
                    let _ = reply.send(db.insert_project(&project));
                }
                DbCommand::UpdateProjectLastUsed {
                    id,
                    timestamp,
                    reply,
                } => {
                    let _ = reply.send(db.update_project_last_used(&id, &timestamp));
                }
                DbCommand::DeleteProject { id, reply } => {
                    let _ = reply.send(db.delete_project(&id));
                }

                DbCommand::GetAllSettings { reply } => {
                    let _ = reply.send(db.get_all_settings());
                }
                DbCommand::SetSetting { key, value, reply } => {
                    let _ = reply.send(db.set_setting(&key, &value));
                }
            }
        }
    }

    // --- Typed convenience methods ---
    // Each returns a oneshot Receiver for the caller to await.

    #[must_use]
    pub fn get_all_instances(&self) -> flume::Receiver<Result<Vec<Instance>>> {
        let (tx, rx) = flume::bounded(1);
        if self
            .tx
            .send(DbCommand::GetAllInstances { reply: tx })
            .is_err()
        {
            error!("DB worker channel closed");
        }
        rx
    }

    pub fn insert_instance(&self, instance: Instance) {
        let (tx, _rx) = flume::bounded(1);
        let _ = self.tx.send(DbCommand::InsertInstance {
            instance,
            reply: tx,
        });
    }

    pub fn update_instance(&self, id: String, updates: InstanceUpdate) {
        let (tx, _rx) = flume::bounded(1);
        let _ = self.tx.send(DbCommand::UpdateInstance {
            id,
            updates,
            reply: tx,
        });
    }

    pub fn end_instance(&self, id: String, ended_at: String) {
        let (tx, _rx) = flume::bounded(1);
        let _ = self.tx.send(DbCommand::EndInstance {
            id,
            ended_at,
            reply: tx,
        });
    }

    #[must_use]
    pub fn get_next_instance_number(&self) -> flume::Receiver<Result<i64>> {
        let (tx, rx) = flume::bounded(1);
        if self
            .tx
            .send(DbCommand::GetNextInstanceNumber { reply: tx })
            .is_err()
        {
            error!("DB worker channel closed");
        }
        rx
    }

    #[must_use]
    pub fn get_all_projects(&self) -> flume::Receiver<Result<Vec<Project>>> {
        let (tx, rx) = flume::bounded(1);
        if self
            .tx
            .send(DbCommand::GetAllProjects { reply: tx })
            .is_err()
        {
            error!("DB worker channel closed");
        }
        rx
    }

    pub fn insert_project(&self, project: Project) {
        let (tx, _rx) = flume::bounded(1);
        let _ = self
            .tx
            .send(DbCommand::InsertProject { project, reply: tx });
    }

    pub fn update_project_last_used(&self, id: String, timestamp: String) {
        let (tx, _rx) = flume::bounded(1);
        let _ = self.tx.send(DbCommand::UpdateProjectLastUsed {
            id,
            timestamp,
            reply: tx,
        });
    }

    pub fn delete_project(&self, id: String) {
        let (tx, _rx) = flume::bounded(1);
        let _ = self.tx.send(DbCommand::DeleteProject { id, reply: tx });
    }

    #[must_use]
    pub fn get_all_settings(&self) -> flume::Receiver<Result<Vec<(String, String)>>> {
        let (tx, rx) = flume::bounded(1);
        if self
            .tx
            .send(DbCommand::GetAllSettings { reply: tx })
            .is_err()
        {
            error!("DB worker channel closed");
        }
        rx
    }

    pub fn set_setting(&self, key: String, value: String) {
        let (tx, _rx) = flume::bounded(1);
        let _ = self.tx.send(DbCommand::SetSetting {
            key,
            value,
            reply: tx,
        });
    }

    pub fn shutdown(&self) {
        let _ = self.tx.send(DbCommand::Shutdown);
    }
}

// NOTE: No Drop impl. The flume channel disconnect handles cleanup:
// when all DbHandle clones are dropped, all Senders drop, rx.recv()
// returns Err(Disconnected), and the worker loop exits naturally.
// Use shutdown() explicitly only when you need synchronous cleanup (e.g. tests).

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp_db_path() -> String {
        let dir = std::env::temp_dir().join("conescope-test");
        fs::create_dir_all(&dir).ok();
        let path = dir.join(format!("test-{}.db", uuid::Uuid::new_v4()));
        path.to_string_lossy().into_owned()
    }

    #[test]
    fn db_handle_roundtrip() {
        let path = temp_db_path();
        let handle = DbHandle::spawn(&path).unwrap();

        // Settings should be populated with defaults
        let settings = handle.get_all_settings().recv().unwrap().unwrap();
        assert!(settings.len() >= 4);

        // Insert + get instances
        let inst = Instance {
            id: "dw1".into(),
            project_id: None,
            title: Some("Worker Test".into()),
            status: conescope_core::instance::InstanceStatus::Starting,
            instance_number: Some(1),
            tokens_used: 0,
            cost_estimate: 0.0,
            started_at: "2025-01-01T00:00:00Z".into(),
            ended_at: None,
            instance_type: conescope_core::instance::InstanceType::Terminal,
            color: None,
        };
        handle.insert_instance(inst);

        // Give worker thread time to process
        std::thread::sleep(std::time::Duration::from_millis(50));

        let instances = handle.get_all_instances().recv().unwrap().unwrap();
        assert_eq!(instances.len(), 1);
        assert_eq!(instances[0].title.as_deref(), Some("Worker Test"));

        handle.shutdown();
        fs::remove_file(&path).ok();
    }
}
