#![allow(clippy::missing_errors_doc)]

use std::collections::HashMap;
use std::io::{Read, Write};
use std::thread;

use anyhow::{Context, Result};
use portable_pty::{CommandBuilder, MasterPty, PtySize, native_pty_system};
use tracing::{error, info};

use crate::event::PtyEvent;
use crate::history::TerminalHistory;

/// Per-instance PTY state.
pub struct PtyHandle {
    master: Box<dyn MasterPty + Send>,
    writer: Option<Box<dyn Write + Send>>,
    event_rx: flume::Receiver<PtyEvent>,
    pub history: TerminalHistory,
}

impl std::fmt::Debug for PtyHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PtyHandle")
            .field("history", &self.history)
            .field("has_writer", &self.writer.is_some())
            .finish_non_exhaustive()
    }
}

impl PtyHandle {
    /// Drain all pending events without blocking.
    #[must_use]
    pub fn drain_events(&self) -> Vec<PtyEvent> {
        let mut events = Vec::new();
        while let Ok(event) = self.event_rx.try_recv() {
            events.push(event);
        }
        events
    }

    /// Send bytes to the PTY stdin.
    pub fn send_input(&mut self, data: &[u8]) -> Result<()> {
        if let Some(writer) = &mut self.writer {
            writer.write_all(data).context("Failed to write to PTY")?;
            writer.flush().context("Failed to flush PTY")?;
        }
        Ok(())
    }

    /// Resize the PTY.
    pub fn resize(&self, cols: u16, rows: u16) -> Result<()> {
        self.master
            .resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .context("Failed to resize PTY")
    }
}

/// Manages all PTY instances.
#[derive(Debug, Default)]
pub struct PtyManager {
    handles: HashMap<String, PtyHandle>,
}

impl PtyManager {
    #[must_use]
    pub fn new() -> Self {
        Self {
            handles: HashMap::new(),
        }
    }

    /// Spawn a shell and immediately send `claude\r` to start Claude CLI.
    pub fn spawn_claude(&mut self, id: &str, cwd: &str) -> Result<()> {
        self.spawn_internal(id, cwd, Some("claude"))?;
        Ok(())
    }

    /// Spawn a plain interactive shell.
    pub fn spawn_shell(&mut self, id: &str, cwd: &str) -> Result<()> {
        self.spawn_internal(id, cwd, None)?;
        Ok(())
    }

    fn spawn_internal(&mut self, id: &str, cwd: &str, initial_command: Option<&str>) -> Result<()> {
        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(PtySize {
                rows: 24,
                cols: 80,
                pixel_width: 0,
                pixel_height: 0,
            })
            .context("Failed to open PTY")?;

        let shell = conescope_core::shell::default_shell();
        let mut cmd = CommandBuilder::new(&shell);
        cmd.cwd(cwd);

        // Augment PATH
        let path = conescope_core::shell::augmented_path();
        cmd.env("PATH", &path);
        cmd.env("TERM", "xterm-256color");

        let _child = pair
            .slave
            .spawn_command(cmd)
            .context("Failed to spawn shell")?;
        // Drop slave — master holds the connection
        drop(pair.slave);

        let master = pair.master;
        let mut writer = master.take_writer().context("Failed to take PTY writer")?;

        // Send initial command if specified
        if let Some(initial_cmd) = initial_command {
            writer
                .write_all(format!("{initial_cmd}\r").as_bytes())
                .context("Failed to send initial command")?;
            writer.flush()?;
        }

        let (event_tx, event_rx) = flume::unbounded();

        // Spawn reader thread — blocking I/O, not async
        let mut reader = master
            .try_clone_reader()
            .context("Failed to clone PTY reader")?;
        let reader_id = id.to_string();
        thread::spawn(move || {
            let mut buf = [0u8; 4096];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => {
                        let _ = event_tx.send(PtyEvent::Exit(None));
                        break;
                    }
                    Ok(n) => {
                        let _ = event_tx.send(PtyEvent::Output(buf[..n].to_vec()));
                    }
                    Err(e) => {
                        let _ = event_tx.send(PtyEvent::Error(e.to_string()));
                        let _ = event_tx.send(PtyEvent::Exit(None));
                        break;
                    }
                }
            }
            info!("PTY reader thread exited for {reader_id}");
        });

        self.handles.insert(
            id.to_string(),
            PtyHandle {
                master,
                writer: Some(writer),
                event_rx,
                history: TerminalHistory::new(),
            },
        );

        Ok(())
    }

    #[must_use]
    pub fn get(&self, id: &str) -> Option<&PtyHandle> {
        self.handles.get(id)
    }

    pub fn get_mut(&mut self, id: &str) -> Option<&mut PtyHandle> {
        self.handles.get_mut(id)
    }

    pub fn send_input(&mut self, id: &str, data: &[u8]) -> Result<()> {
        self.handles
            .get_mut(id)
            .context("PTY handle not found")?
            .send_input(data)
    }

    pub fn resize(&self, id: &str, cols: u16, rows: u16) -> Result<()> {
        self.handles
            .get(id)
            .context("PTY handle not found")?
            .resize(cols, rows)
    }

    pub fn kill(&mut self, id: &str) {
        if let Some(mut handle) = self.handles.remove(id) {
            // Drop writer to signal EOF to the PTY
            handle.writer.take();
            info!("Killed PTY {id}");
        }
    }

    pub fn cleanup(&mut self) {
        let ids: Vec<String> = self.handles.keys().cloned().collect();
        for id in ids {
            self.kill(&id);
        }
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.handles.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.handles.is_empty()
    }
}

impl Drop for PtyManager {
    fn drop(&mut self) {
        if !self.handles.is_empty() {
            error!(
                "PtyManager dropped with {} active handles — cleaning up",
                self.handles.len()
            );
            self.cleanup();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn spawn_shell_and_read_output() {
        let mut mgr = PtyManager::new();
        mgr.spawn_shell("test1", "/tmp").unwrap();

        // Give the shell time to start
        thread::sleep(Duration::from_millis(500));

        // Send echo command
        mgr.send_input("test1", b"echo PTYTEST_MARKER\r").unwrap();
        thread::sleep(Duration::from_millis(500));

        let handle = mgr.get("test1").unwrap();
        let events = handle.drain_events();
        let output: Vec<u8> = events
            .iter()
            .filter_map(|e| match e {
                PtyEvent::Output(data) => Some(data.clone()),
                _ => None,
            })
            .flatten()
            .collect();
        let output_str = String::from_utf8_lossy(&output);
        assert!(
            output_str.contains("PTYTEST_MARKER"),
            "Expected output to contain PTYTEST_MARKER, got: {output_str}"
        );

        mgr.kill("test1");
        assert!(mgr.is_empty());
    }

    #[test]
    fn resize_works() {
        let mut mgr = PtyManager::new();
        mgr.spawn_shell("test2", "/tmp").unwrap();
        thread::sleep(Duration::from_millis(200));

        // Should not error
        mgr.resize("test2", 120, 40).unwrap();
        mgr.cleanup();
    }

    #[test]
    fn cleanup_removes_all() {
        let mut mgr = PtyManager::new();
        mgr.spawn_shell("a", "/tmp").unwrap();
        mgr.spawn_shell("b", "/tmp").unwrap();
        assert_eq!(mgr.len(), 2);
        mgr.cleanup();
        assert!(mgr.is_empty());
    }
}
