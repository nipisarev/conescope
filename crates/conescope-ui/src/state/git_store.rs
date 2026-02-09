use std::sync::{Arc, Mutex};

use gpui::EventEmitter;

use conescope_git::diff::DiffHunk;
use conescope_git::repository::GitRepo;
use conescope_git::status::GitFileEntry;

#[derive(Debug, Clone)]
pub enum GitStoreEvent {
    StatusChanged,
    OpenDiff { path: String, staged: bool },
}

impl EventEmitter<GitStoreEvent> for GitStore {}

pub struct GitStore {
    repo: Option<Arc<Mutex<GitRepo>>>,
    entries: Vec<GitFileEntry>,
    branch: Option<String>,
    current_path: Option<String>,
    work_dir: Option<String>,
}

impl std::fmt::Debug for GitStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GitStore")
            .field("branch", &self.branch)
            .field("entries_len", &self.entries.len())
            .field("has_repo", &self.repo.is_some())
            .finish_non_exhaustive()
    }
}

impl Default for GitStore {
    fn default() -> Self {
        Self::new()
    }
}

impl GitStore {
    #[must_use]
    pub fn new() -> Self {
        Self {
            repo: None,
            entries: Vec::new(),
            branch: None,
            current_path: None,
            work_dir: None,
        }
    }

    /// Open a git repo at `path` (or clear if None). Refreshes status if repo found.
    pub fn set_project(&mut self, path: Option<&str>, cx: &mut gpui::Context<Self>) {
        self.current_path = path.map(str::to_owned);

        if let Some(p) = path {
            match GitRepo::open(std::path::Path::new(p)) {
                Ok(repo) => {
                    self.work_dir = Some(repo.work_dir().to_string_lossy().to_string());
                    self.repo = Some(Arc::new(Mutex::new(repo)));
                    self.refresh(cx);
                }
                Err(e) => {
                    tracing::debug!("no git repo at {p}: {e}");
                    self.repo = None;
                    self.work_dir = None;
                    self.entries.clear();
                    self.branch = None;
                    cx.notify();
                }
            }
        } else {
            self.repo = None;
            self.work_dir = None;
            self.entries.clear();
            self.branch = None;
            cx.notify();
        }
    }

    /// Convert a relative git path to absolute using the repo `work_dir`.
    #[must_use]
    pub fn resolve_path(&self, rel_path: &str) -> String {
        if let Some(ref wd) = self.work_dir {
            let mut abs = wd.clone();
            if !abs.ends_with('/') {
                abs.push('/');
            }
            abs.push_str(rel_path);
            abs
        } else {
            rel_path.to_owned()
        }
    }

    /// Refresh git status + branch from the repo on a background thread.
    ///
    /// # Panics
    /// Panics if the repo mutex is poisoned (should not happen in practice).
    pub fn refresh(&mut self, cx: &mut gpui::Context<Self>) {
        let Some(repo) = self.repo.clone() else {
            return;
        };

        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move {
                    let repo = repo.lock().expect("repo lock poisoned");
                    let entries = repo.status().unwrap_or_default();
                    let branch = repo.head_branch();
                    (entries, branch)
                })
                .await;

            cx.update(|cx| {
                if let Some(store) = this.upgrade() {
                    store.update(cx, |store, cx| {
                        store.entries = result.0;
                        store.branch = result.1;
                        cx.emit(GitStoreEvent::StatusChanged);
                        cx.notify();
                    });
                }
            });
        })
        .detach();
    }

    /// Diff a single file on a background thread, then call `callback` with the hunks.
    ///
    /// # Panics
    /// Panics if the repo mutex is poisoned.
    pub fn diff_file(
        &self,
        path: &str,
        staged: bool,
        cx: &mut gpui::Context<Self>,
        callback: impl FnOnce(Vec<DiffHunk>, &mut gpui::App) + Send + 'static,
    ) {
        let Some(repo) = self.repo.clone() else {
            return;
        };
        let path = path.to_owned();

        cx.spawn(async move |_this, cx| {
            let hunks = cx
                .background_executor()
                .spawn(async move {
                    let repo = repo.lock().expect("repo lock poisoned");
                    repo.diff_file(&path, staged).unwrap_or_default()
                })
                .await;

            cx.update(|cx| {
                callback(hunks, cx);
            });
        })
        .detach();
    }

    /// Stage a file, then refresh status.
    ///
    /// # Panics
    /// Panics if the repo mutex is poisoned.
    pub fn stage_file(&mut self, path: &str, cx: &mut gpui::Context<Self>) {
        let Some(repo) = self.repo.clone() else {
            return;
        };
        let path = path.to_owned();

        cx.spawn(async move |this, cx| {
            cx.background_executor()
                .spawn(async move {
                    let repo = repo.lock().expect("repo lock poisoned");
                    if let Err(e) = repo.stage(&[&path]) {
                        tracing::warn!("git stage failed: {e}");
                    }
                })
                .await;

            cx.update(|cx| {
                if let Some(store) = this.upgrade() {
                    store.update(cx, GitStore::refresh);
                }
            });
        })
        .detach();
    }

    /// Unstage a file, then refresh status.
    ///
    /// # Panics
    /// Panics if the repo mutex is poisoned.
    pub fn unstage_file(&mut self, path: &str, cx: &mut gpui::Context<Self>) {
        let Some(repo) = self.repo.clone() else {
            return;
        };
        let path = path.to_owned();

        cx.spawn(async move |this, cx| {
            cx.background_executor()
                .spawn(async move {
                    let repo = repo.lock().expect("repo lock poisoned");
                    if let Err(e) = repo.unstage(&[&path]) {
                        tracing::warn!("git unstage failed: {e}");
                    }
                })
                .await;

            cx.update(|cx| {
                if let Some(store) = this.upgrade() {
                    store.update(cx, GitStore::refresh);
                }
            });
        })
        .detach();
    }

    /// Discard changes to a file, then refresh status.
    ///
    /// # Panics
    /// Panics if the repo mutex is poisoned.
    pub fn discard_file(&mut self, path: &str, cx: &mut gpui::Context<Self>) {
        let Some(repo) = self.repo.clone() else {
            return;
        };
        let path = path.to_owned();

        cx.spawn(async move |this, cx| {
            cx.background_executor()
                .spawn(async move {
                    let repo = repo.lock().expect("repo lock poisoned");
                    if let Err(e) = repo.discard(&[&path]) {
                        tracing::warn!("git discard failed: {e}");
                    }
                })
                .await;

            cx.update(|cx| {
                if let Some(store) = this.upgrade() {
                    store.update(cx, GitStore::refresh);
                }
            });
        })
        .detach();
    }

    #[must_use]
    pub fn entries(&self) -> &[GitFileEntry] {
        &self.entries
    }

    #[must_use]
    pub fn branch(&self) -> Option<&str> {
        self.branch.as_deref()
    }

    #[must_use]
    pub fn has_repo(&self) -> bool {
        self.repo.is_some()
    }
}
