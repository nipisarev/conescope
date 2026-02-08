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
    Exited,
}

impl gpui::EventEmitter<InstanceEvent> for InstanceEntry {}

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
    /// Secondary shell terminal (Project instances only, spawned on demand).
    pub shell_terminal_view: Option<gpui::Entity<TerminalView>>,
    pub shell_focus_handle: Option<gpui::FocusHandle>,
    pub shell_stdout_rx: Option<mpsc::Receiver<Vec<u8>>>,
    pub shell_stdin_tx: Option<mpsc::Sender<Vec<u8>>>,
    pub shell_master_pty: Option<Arc<dyn MasterPty + Send>>,
    pub shell_history: TerminalHistory,
    pub shell_alive: bool,
    pub active_tab: TerminalTab,
}

impl std::fmt::Debug for InstanceEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InstanceEntry")
            .field("id", &self.instance.id)
            .field("status", &self.instance.status)
            .field("alive", &self.alive)
            .field("has_terminal", &self.terminal_view.is_some())
            .field("has_shell", &self.shell_terminal_view.is_some())
            .field("active_tab", &self.active_tab)
            .finish_non_exhaustive()
    }
}

impl InstanceEntry {
    /// Create from a DB-loaded instance (no PTY attached yet).
    #[must_use]
    pub fn from_instance(instance: Instance) -> Self {
        Self {
            instance,
            terminal_view: None,
            focus_handle: None,
            stdout_rx: None,
            stdin_tx: None,
            master_pty: None,
            history: TerminalHistory::new(),
            alive: false,
            shell_terminal_view: None,
            shell_focus_handle: None,
            shell_stdout_rx: None,
            shell_stdin_tx: None,
            shell_master_pty: None,
            shell_history: TerminalHistory::new(),
            shell_alive: false,
            active_tab: TerminalTab::Primary,
        }
    }

    /// Attach a spawned terminal pane to this entry.
    pub fn attach_terminal(&mut self, pane: TerminalPane, cx: &mut gpui::Context<Self>) {
        self.terminal_view = Some(pane.view.clone());
        self.focus_handle = Some(pane.focus_handle);
        self.stdout_rx = Some(pane.stdout_rx);
        self.stdin_tx = Some(pane.stdin_tx);
        self.master_pty = Some(pane.master);
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
        // Also kill shell PTY if present
        self.shell_master_pty.take();
        self.shell_stdin_tx.take();
        self.shell_alive = false;
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

    /// Update font on all terminal views (primary + shell).
    pub fn update_font(&mut self, family: &str, cx: &mut gpui::Context<Self>) {
        let font = terminal_font(family);
        if let Some(ref tv) = self.terminal_view {
            tv.update(cx, |v, _| v.set_font(font.clone()));
        }
        if let Some(ref tv) = self.shell_terminal_view {
            tv.update(cx, |v, _| v.set_font(font));
        }
    }

    /// Update background color on all terminal views.
    pub fn update_bg_color(&mut self, color: gpui::Rgba, cx: &mut gpui::Context<Self>) {
        if let Some(ref tv) = self.terminal_view {
            tv.update(cx, |v, _| v.set_bg_color(color));
        }
        if let Some(ref tv) = self.shell_terminal_view {
            tv.update(cx, |v, _| v.set_bg_color(color));
        }
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
            TerminalTab::Shell => self.shell_terminal_view.as_ref(),
        }
    }

    /// Get the focus handle for the currently active tab.
    #[must_use]
    pub fn active_focus_handle(&self) -> Option<&gpui::FocusHandle> {
        match self.active_tab {
            TerminalTab::Primary => self.focus_handle.as_ref(),
            TerminalTab::Shell => self.shell_focus_handle.as_ref(),
        }
    }

    /// Switch the active terminal tab.
    pub fn set_active_tab(&mut self, tab: TerminalTab, cx: &mut gpui::Context<Self>) {
        self.active_tab = tab;
        cx.notify();
    }

    /// Attach a spawned shell terminal pane (secondary PTY).
    pub fn attach_shell_terminal(&mut self, pane: TerminalPane, cx: &mut gpui::Context<Self>) {
        self.shell_terminal_view = Some(pane.view.clone());
        self.shell_focus_handle = Some(pane.focus_handle);
        self.shell_stdout_rx = Some(pane.stdout_rx);
        self.shell_stdin_tx = Some(pane.stdin_tx);
        self.shell_master_pty = Some(pane.master);
        self.shell_alive = true;

        cx.subscribe(&pane.view, |this, _, event: &TerminalViewEvent, _cx| {
            let TerminalViewEvent::ContainerResize(cols, rows) = event;
            this.resize_shell_pty(*cols, *rows);
        })
        .detach();
    }

    /// Start polling `shell_stdout_rx` for the secondary shell terminal.
    pub fn start_shell_output_polling(&mut self, cx: &mut gpui::Context<Self>) {
        let rx = self.shell_stdout_rx.take();
        let tv = self.shell_terminal_view.clone();
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
                        if weak.upgrade().is_none() {
                            break;
                        }
                        continue;
                    }

                    cx.update(|cx| {
                        if let Some(entry) = weak.upgrade() {
                            entry.update(cx, |e, _| e.shell_history.push(batch.clone()));
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

    /// Resize the shell PTY.
    pub fn resize_shell_pty(&self, cols: u16, rows: u16) {
        if let Some(ref master) = self.shell_master_pty {
            let _ = master.resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            });
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
        // Shell fields also unset
        assert!(!entry.shell_alive);
        assert!(entry.shell_terminal_view.is_none());
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
