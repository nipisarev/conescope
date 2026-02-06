use std::io::{Read, Write};
use std::sync::Arc;
use std::sync::mpsc;
use std::thread;

use gpui::{AppContext, SharedString};
use gpui_ghostty_terminal::view::{TerminalInput, TerminalView};
use gpui_ghostty_terminal::{TerminalConfig, TerminalSession, default_terminal_font};
use portable_pty::{CommandBuilder, MasterPty, PtySize, native_pty_system};

/// All state needed for a single terminal pane.
pub struct TerminalPane {
    pub view: gpui::Entity<TerminalView>,
    pub master: Arc<dyn MasterPty + Send>,
    pub stdout_rx: mpsc::Receiver<Vec<u8>>,
    pub stdin_tx: mpsc::Sender<Vec<u8>>,
}

impl std::fmt::Debug for TerminalPane {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TerminalPane")
            .field("view", &"Entity<TerminalView>")
            .finish_non_exhaustive()
    }
}

/// Spawn a PTY-backed terminal pane. Must be called from GPUI app context.
///
/// # Panics
///
/// Panics if the PTY system fails to open, the shell fails to spawn,
/// or the virtual terminal fails to initialize.
pub fn spawn_terminal_pane(
    cwd: Option<&str>,
    window: &mut gpui::Window,
    cx: &mut gpui::App,
) -> TerminalPane {
    let config = TerminalConfig::default();

    let pty_system = native_pty_system();
    let pty_pair = pty_system
        .openpty(PtySize {
            rows: config.rows,
            cols: config.cols,
            pixel_width: 0,
            pixel_height: 0,
        })
        .expect("openpty failed");

    let master: Arc<dyn MasterPty + Send> = Arc::from(pty_pair.master);

    let shell = conescope_core::shell::default_shell();
    let mut cmd = CommandBuilder::new(&shell);
    cmd.arg("-l");
    if let Some(dir) = cwd {
        cmd.cwd(dir);
    }
    let path = conescope_core::shell::augmented_path();
    cmd.env("PATH", &path);
    cmd.env("TERM", "xterm-256color");
    cmd.env("COLORTERM", "truecolor");
    cmd.env("TERM_PROGRAM", "conescope");

    let mut child = pty_pair
        .slave
        .spawn_command(cmd)
        .expect("spawn shell failed");

    thread::spawn(move || {
        let _ = child.wait();
    });

    let mut pty_reader = master.try_clone_reader().expect("pty reader");
    let mut pty_writer = master.take_writer().expect("pty writer");

    let (stdin_tx, stdin_rx) = mpsc::channel::<Vec<u8>>();
    let (stdout_tx, stdout_rx) = mpsc::channel::<Vec<u8>>();

    // Writer thread: stdin_rx -> PTY
    thread::spawn(move || {
        while let Ok(bytes) = stdin_rx.recv() {
            if pty_writer.write_all(&bytes).is_err() {
                break;
            }
            let _ = pty_writer.flush();
        }
    });

    // Reader thread: PTY -> stdout_tx
    thread::spawn(move || {
        let mut buf = [0u8; 8192];
        loop {
            let n = match pty_reader.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => n,
            };
            let _ = stdout_tx.send(buf[..n].to_vec());
        }
    });

    let stdin_tx_for_pane = stdin_tx.clone();

    let view = cx.new(|cx| {
        let focus_handle = cx.focus_handle();
        focus_handle.focus(window, cx);
        let session = TerminalSession::new(config).expect("vt init");
        let input = TerminalInput::new(move |bytes| {
            let _ = stdin_tx.send(bytes.to_vec());
        });
        TerminalView::new_with_input(session, focus_handle, input)
    });

    TerminalPane {
        view,
        master,
        stdout_rx,
        stdin_tx: stdin_tx_for_pane,
    }
}

/// Compute terminal cell metrics from the current window text system.
pub fn compute_cell_metrics(window: &mut gpui::Window) -> Option<(f32, f32)> {
    let mut style = window.text_style();
    let font = default_terminal_font();
    style.font_family = font.family.clone();
    style.font_features = gpui_ghostty_terminal::default_terminal_font_features();
    style.font_fallbacks.clone_from(&font.fallbacks);

    let rem_size = window.rem_size();
    let font_size = style.font_size.to_pixels(rem_size);
    let line_height = style.line_height.to_pixels(style.font_size, rem_size);

    let run = style.to_run(1);
    let lines = window
        .text_system()
        .shape_text(SharedString::from("M"), font_size, &[run], None, Some(1))
        .ok()?;
    let line = lines.first()?;

    let cell_width = f32::from(line.width()).max(1.0);
    let cell_height = f32::from(line_height).max(1.0);
    Some((cell_width, cell_height))
}
