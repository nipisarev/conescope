use std::sync::Arc;
use std::sync::mpsc;
use std::time::Duration;

use crate::terminal::terminal_view::{TerminalView, TerminalViewEvent};
use conescope_core::instance::{Instance, InstanceStatus, InstanceType, TerminalTab};
use conescope_pty::history::TerminalHistory;
use portable_pty::{MasterPty, PtySize};

use crate::terminal::{TerminalPane, terminal_font};

#[derive(Debug, Clone)]
pub enum InstanceEvent {
    StatusChanged(InstanceStatus),
    CwdChanged(String),
    Exited,
}

impl gpui::EventEmitter<InstanceEvent> for InstanceEntry {}

/// Per-shell-tab state bundle.
#[allow(missing_debug_implementations)]
pub struct ShellTab {
    pub id: usize,
    pub terminal_view: gpui::Entity<TerminalView>,
    pub focus_handle: gpui::FocusHandle,
    pub stdin_tx: mpsc::Sender<Vec<u8>>,
    pub master_pty: Arc<dyn MasterPty + Send>,
    pub history: TerminalHistory,
    pub alive: bool,
}

/// Cached git summary for an instance's project directory.
#[derive(Debug, Clone, Default)]
pub struct GitSummary {
    pub branch: Option<String>,
    pub insertions: usize,
    pub deletions: usize,
}

pub struct InstanceEntry {
    pub instance: Instance,
    /// Primary terminal (Claude CLI for Project, shell for Terminal).
    pub terminal_view: Option<gpui::Entity<TerminalView>>,
    pub focus_handle: Option<gpui::FocusHandle>,
    pub stdout_rx: Option<mpsc::Receiver<Vec<u8>>>,
    pub stdin_tx: Option<mpsc::Sender<Vec<u8>>>,
    pub master_pty: Option<Arc<dyn MasterPty + Send>>,
    pub history: TerminalHistory,
    pub alive: bool,
    /// Shell tabs (Project instances only, spawned on demand). Unlimited count.
    pub shell_tabs: Vec<ShellTab>,
    shell_next_id: usize,
    pub active_tab: TerminalTab,
    /// Cached git info for the project directory.
    pub git_summary: GitSummary,
    /// PID of the child shell process (for CWD detection).
    pub child_pid: Option<u32>,
    /// Last known CWD (updated by background polling).
    pub current_cwd: Option<String>,
}

impl std::fmt::Debug for InstanceEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InstanceEntry")
            .field("id", &self.instance.id)
            .field("status", &self.instance.status)
            .field("alive", &self.alive)
            .field("has_terminal", &self.terminal_view.is_some())
            .field("shell_tab_count", &self.shell_tabs.len())
            .field("active_tab", &self.active_tab)
            .finish_non_exhaustive()
    }
}

impl InstanceEntry {
    /// Create from a DB-loaded instance (no PTY attached yet).
    #[must_use]
    pub fn from_instance(instance: Instance) -> Self {
        let current_cwd = instance.current_cwd.clone();
        Self {
            instance,
            terminal_view: None,
            focus_handle: None,
            stdout_rx: None,
            stdin_tx: None,
            master_pty: None,
            history: TerminalHistory::new(),
            alive: false,
            shell_tabs: Vec::new(),
            shell_next_id: 0,
            active_tab: TerminalTab::Primary,
            git_summary: GitSummary::default(),
            child_pid: None,
            current_cwd,
        }
    }

    /// Attach a spawned terminal pane to this entry.
    pub fn attach_terminal(&mut self, pane: TerminalPane, cx: &mut gpui::Context<Self>) {
        self.terminal_view = Some(pane.view.clone());
        self.focus_handle = Some(pane.focus_handle);
        self.stdout_rx = Some(pane.stdout_rx);
        self.stdin_tx = Some(pane.stdin_tx);
        self.master_pty = Some(pane.master);
        self.child_pid = pane.child_pid;
        self.alive = true;

        // Subscribe to container-driven resizes so PTY stays in sync.
        cx.subscribe(&pane.view, |this, _, event: &TerminalViewEvent, _cx| {
            let TerminalViewEvent::ContainerResize(cols, rows) = event;
            this.resize_pty(*cols, *rows);
        })
        .detach();
    }

    #[must_use]
    pub fn id(&self) -> &str {
        &self.instance.id
    }

    #[must_use]
    pub fn status(&self) -> InstanceStatus {
        self.instance.status
    }

    #[must_use]
    pub fn instance_type(&self) -> InstanceType {
        self.instance.instance_type
    }

    pub fn set_status(&mut self, status: InstanceStatus, cx: &mut gpui::Context<Self>) {
        self.instance.status = status;
        cx.emit(InstanceEvent::StatusChanged(status));
        cx.notify();
    }

    pub fn update_tokens(&mut self, tokens: i64, cost: f64, cx: &mut gpui::Context<Self>) {
        self.instance.tokens_used = tokens;
        self.instance.cost_estimate = cost;
        cx.notify();
    }

    /// Kill the PTY process. Drops the master PTY handle which sends SIGHUP.
    pub fn kill_pty(&mut self) {
        self.master_pty.take(); // Drop sends SIGHUP to child
        self.stdin_tx.take(); // Close input channel
        self.alive = false;
        // Kill all shell tabs
        self.shell_tabs.clear();
    }

    /// Add a new shell tab from a spawned terminal pane. Returns (id, `stdout_rx`).
    pub fn add_shell_tab(
        &mut self,
        pane: TerminalPane,
        cx: &mut gpui::Context<Self>,
    ) -> (usize, mpsc::Receiver<Vec<u8>>) {
        let id = self.shell_next_id;
        self.shell_next_id += 1;

        let tab = ShellTab {
            id,
            terminal_view: pane.view.clone(),
            focus_handle: pane.focus_handle,
            stdin_tx: pane.stdin_tx,
            master_pty: pane.master,
            history: TerminalHistory::new(),
            alive: true,
        };
        self.shell_tabs.push(tab);

        // Subscribe to resize events for this shell tab's PTY.
        let tab_id = id;
        cx.subscribe(
            &pane.view,
            move |this, _, event: &TerminalViewEvent, _cx| {
                let TerminalViewEvent::ContainerResize(cols, rows) = event;
                this.resize_shell_pty_by_id(tab_id, *cols, *rows);
            },
        )
        .detach();

        (id, pane.stdout_rx)
    }

    /// Close a shell tab by ID. Fixes `active_tab` if needed.
    pub fn close_shell_tab(&mut self, id: usize, cx: &mut gpui::Context<Self>) {
        self.shell_tabs.retain(|t| t.id != id);
        // If we closed the active tab, switch to last shell or primary
        if self.active_tab == TerminalTab::Shell(id) {
            self.active_tab = self
                .shell_tabs
                .last()
                .map_or(TerminalTab::Primary, |t| TerminalTab::Shell(t.id));
        }
        cx.notify();
    }

    pub fn mark_exited(&mut self, cx: &mut gpui::Context<Self>) {
        self.instance.status = InstanceStatus::Stopped;
        self.alive = false;
        cx.emit(InstanceEvent::Exited);
        cx.notify();
    }

    /// Focus this instance's terminal view so keyboard input goes to the PTY.
    pub fn focus_terminal(&self, window: &mut gpui::Window, cx: &mut gpui::App) {
        if let Some(ref fh) = self.focus_handle {
            fh.focus(window, cx);
        }
    }

    /// Update font on all terminal views (primary + all shells).
    pub fn update_font(&mut self, family: &str, cx: &mut gpui::Context<Self>) {
        let font = terminal_font(family);
        if let Some(ref tv) = self.terminal_view {
            tv.update(cx, |v, _| v.set_font(font.clone()));
        }
        for tab in &self.shell_tabs {
            let f = font.clone();
            tab.terminal_view.update(cx, |v, _| v.set_font(f));
        }
    }

    /// Update font size on all terminal views.
    pub fn update_font_size(&mut self, size: f32, cx: &mut gpui::Context<Self>) {
        if let Some(ref tv) = self.terminal_view {
            tv.update(cx, |v, cx| {
                v.set_font_size(size);
                cx.notify();
            });
        }
        for tab in &self.shell_tabs {
            tab.terminal_view.update(cx, |v, cx| {
                v.set_font_size(size);
                cx.notify();
            });
        }
    }

    /// Update terminal line-height ratio on all terminal views.
    pub fn update_line_height(&mut self, ratio: f32, cx: &mut gpui::Context<Self>) {
        if let Some(ref tv) = self.terminal_view {
            tv.update(cx, |v, cx| {
                v.set_line_height_ratio(ratio);
                cx.notify();
            });
        }
        for tab in &self.shell_tabs {
            tab.terminal_view.update(cx, |v, cx| {
                v.set_line_height_ratio(ratio);
                cx.notify();
            });
        }
    }

    /// Update terminal color palette on all terminal views.
    pub fn update_colors(
        &mut self,
        colors: &crate::terminal::TerminalColors,
        cx: &mut gpui::Context<Self>,
    ) {
        if let Some(ref tv) = self.terminal_view {
            let c = colors.clone();
            tv.update(cx, |v, _| v.set_colors(c));
        }
        for tab in &self.shell_tabs {
            let c = colors.clone();
            tab.terminal_view.update(cx, |v, _| v.set_colors(c));
        }
    }

    /// Start periodic polling: detect CWD changes (every 3s) and refresh git summary.
    pub fn start_background_polling(&mut self, cx: &mut gpui::Context<Self>) {
        let pid = self.child_pid;
        let weak = cx.weak_entity();

        cx.spawn(async move |_this, cx| {
            let mut tick: u32 = 0;
            loop {
                cx.background_executor().timer(Duration::from_secs(3)).await;

                // Detect CWD from child PID
                let new_cwd = pid.and_then(conescope_core::process::get_process_cwd);

                let alive = cx.update(|cx| {
                    let Some(entry) = weak.upgrade() else {
                        return false;
                    };
                    entry.update(cx, |e, cx| {
                        let cwd_changed =
                            new_cwd.is_some() && new_cwd.as_ref() != e.current_cwd.as_ref();

                        if cwd_changed {
                            e.current_cwd.clone_from(&new_cwd);
                            if let Some(ref cwd) = new_cwd {
                                cx.emit(InstanceEvent::CwdChanged(cwd.clone()));
                            }
                            cx.notify();
                        }

                        // Refresh git: on CWD change or every 3rd tick (~9s)
                        if cwd_changed || tick % 3 == 0 {
                            if let Some(cwd) = e.current_cwd.clone() {
                                e.refresh_git_summary(&cwd, cx);
                            }
                        }
                    });
                    true
                });

                if !alive {
                    break;
                }
                tick = tick.wrapping_add(1);
            }
        })
        .detach();
    }

    /// Refresh cached git summary (branch + diff stats) for the given project path.
    pub fn refresh_git_summary(&mut self, path: &str, cx: &mut gpui::Context<Self>) {
        let path = path.to_owned();
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move {
                    let repo =
                        conescope_git::repository::GitRepo::open(std::path::Path::new(&path))
                            .ok()?;
                    let branch = repo.head_branch();
                    let (ins, del) = repo.diff_stats().unwrap_or((0, 0));
                    Some(GitSummary {
                        branch,
                        insertions: ins,
                        deletions: del,
                    })
                })
                .await;

            cx.update(|cx| {
                if let Some(entry) = this.upgrade() {
                    entry.update(cx, |e, cx| {
                        e.git_summary = result.unwrap_or_default();
                        cx.notify();
                    });
                }
            });
        })
        .detach();
    }

    pub fn resize_pty(&self, cols: u16, rows: u16) {
        if let Some(ref master) = self.master_pty {
            let _ = master.resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            });
        }
    }

    /// Resize all shell PTYs.
    pub fn resize_all_shell_ptys(&self, cols: u16, rows: u16) {
        for tab in &self.shell_tabs {
            let _ = tab.master_pty.resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            });
        }
    }

    fn resize_shell_pty_by_id(&self, id: usize, cols: u16, rows: u16) {
        if let Some(tab) = self.shell_tabs.iter().find(|t| t.id == id) {
            let _ = tab.master_pty.resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            });
        }
    }

    pub fn send_input(&self, data: &[u8]) {
        if let Some(ref tx) = self.stdin_tx {
            let _ = tx.send(data.to_vec());
        }
    }

    /// Get the terminal view for the currently active tab.
    #[must_use]
    pub fn active_terminal_view(&self) -> Option<&gpui::Entity<TerminalView>> {
        match self.active_tab {
            TerminalTab::Primary => self.terminal_view.as_ref(),
            TerminalTab::Shell(id) => self
                .shell_tabs
                .iter()
                .find(|t| t.id == id)
                .map(|t| &t.terminal_view),
        }
    }

    /// Get the focus handle for the currently active tab.
    #[must_use]
    pub fn active_focus_handle(&self) -> Option<&gpui::FocusHandle> {
        match self.active_tab {
            TerminalTab::Primary => self.focus_handle.as_ref(),
            TerminalTab::Shell(id) => self
                .shell_tabs
                .iter()
                .find(|t| t.id == id)
                .map(|t| &t.focus_handle),
        }
    }

    /// Switch the active terminal tab.
    pub fn set_active_tab(&mut self, tab: TerminalTab, cx: &mut gpui::Context<Self>) {
        self.active_tab = tab;
        cx.notify();
    }

    /// Shell tab metadata: (id, alive) pairs for rendering.
    #[must_use]
    pub fn shell_tab_info(&self) -> Vec<(usize, bool)> {
        self.shell_tabs.iter().map(|t| (t.id, t.alive)).collect()
    }

    #[must_use]
    pub fn has_shell_tabs(&self) -> bool {
        !self.shell_tabs.is_empty()
    }

    /// Get focus handle for a specific shell tab by ID.
    #[must_use]
    pub fn shell_focus_handle(&self, id: usize) -> Option<&gpui::FocusHandle> {
        self.shell_tabs
            .iter()
            .find(|t| t.id == id)
            .map(|t| &t.focus_handle)
    }

    /// Start polling stdout for a specific shell tab.
    pub fn start_shell_output_polling(
        &mut self,
        id: usize,
        rx: mpsc::Receiver<Vec<u8>>,
        cx: &mut gpui::Context<Self>,
    ) {
        let tv = self
            .shell_tabs
            .iter()
            .find(|t| t.id == id)
            .map(|t| t.terminal_view.clone());
        let weak = cx.weak_entity();

        if let Some(tv) = tv {
            cx.spawn(async move |_weak_self, cx| {
                loop {
                    cx.background_executor()
                        .timer(Duration::from_millis(16))
                        .await;
                    let mut batch = Vec::new();
                    while let Ok(chunk) = rx.try_recv() {
                        batch.extend_from_slice(&chunk);
                    }
                    if batch.is_empty() {
                        if weak.upgrade().is_none() {
                            break;
                        }
                        continue;
                    }

                    let tab_id = id;
                    cx.update(|cx| {
                        if let Some(entry) = weak.upgrade() {
                            entry.update(cx, |e, _| {
                                if let Some(tab) = e.shell_tabs.iter_mut().find(|t| t.id == tab_id)
                                {
                                    tab.history.push(batch.clone());
                                }
                            });
                        }
                        tv.update(cx, |view, cx| {
                            view.queue_output_bytes(&batch, cx);
                        });
                    });
                }
            })
            .detach();
        }
    }

    /// Start polling `stdout_rx` every 16ms, feed bytes to `terminal_view` and history.
    ///
    /// Takes ownership of `stdout_rx`. The polling task stops when the entity is dropped.
    pub fn start_output_polling(&mut self, cx: &mut gpui::Context<Self>) {
        let rx = self.stdout_rx.take();
        let tv = self.terminal_view.clone();
        let weak = cx.weak_entity();

        if let (Some(rx), Some(tv)) = (rx, tv) {
            cx.spawn(async move |_weak_self, cx| {
                loop {
                    cx.background_executor()
                        .timer(Duration::from_millis(16))
                        .await;
                    let mut batch = Vec::new();
                    while let Ok(chunk) = rx.try_recv() {
                        batch.extend_from_slice(&chunk);
                    }
                    if batch.is_empty() {
                        // Stop polling if the entity was dropped.
                        if weak.upgrade().is_none() {
                            break;
                        }
                        continue;
                    }

                    cx.update(|cx| {
                        if let Some(entry) = weak.upgrade() {
                            entry.update(cx, |e, _| e.history.push(batch.clone()));
                        }
                        tv.update(cx, |view, cx| {
                            view.queue_output_bytes(&batch, cx);
                        });
                    });
                }
            })
            .detach();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use conescope_core::instance::Instance;

    fn test_instance() -> Instance {
        Instance {
            id: "test-1".into(),
            project_id: None,
            title: Some("Test".into()),
            status: InstanceStatus::Starting,
            tokens_used: 0,
            cost_estimate: 0.0,
            started_at: "2025-01-01T00:00:00Z".into(),
            ended_at: None,
            instance_type: InstanceType::Terminal,
            color: None,
            current_cwd: None,
        }
    }

    #[test]
    fn from_instance_has_no_terminal() {
        let entry = InstanceEntry::from_instance(test_instance());
        assert!(!entry.alive);
        assert!(entry.terminal_view.is_none());
        assert!(entry.stdout_rx.is_none());
        assert!(entry.stdin_tx.is_none());
        assert!(entry.master_pty.is_none());
        assert!(entry.history.is_empty());
        assert!(entry.shell_tabs.is_empty());
        assert_eq!(entry.active_tab, TerminalTab::Primary);
    }

    #[test]
    fn accessors_work() {
        let entry = InstanceEntry::from_instance(test_instance());
        assert_eq!(entry.id(), "test-1");
        assert_eq!(entry.status(), InstanceStatus::Starting);
        assert_eq!(entry.instance_type(), InstanceType::Terminal);
    }
}
