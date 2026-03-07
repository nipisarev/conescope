#[allow(clippy::module_inception)]
pub mod terminal;
pub mod terminal_element;
pub mod terminal_view;

use std::io::{Read, Write};
use std::sync::Arc;
use std::sync::mpsc;
use std::thread;

use gpui::{AppContext, SharedString};
use portable_pty::{CommandBuilder, MasterPty, PtySize, native_pty_system};

use terminal::Terminal;
pub use terminal::{MenuColors, TerminalColors};
use terminal_view::TerminalView;

/// Default line-height multiplier relative to font size.
pub const DEFAULT_LINE_HEIGHT_RATIO: f32 = 1.2;

/// All state needed for a single terminal pane.
pub struct TerminalPane {
    pub view: gpui::Entity<TerminalView>,
    pub focus_handle: gpui::FocusHandle,
    pub master: Arc<dyn MasterPty + Send>,
    pub stdout_rx: mpsc::Receiver<Vec<u8>>,
    pub stdin_tx: mpsc::Sender<Vec<u8>>,
    pub child_pid: Option<u32>,
}

impl std::fmt::Debug for TerminalPane {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TerminalPane")
            .field("view", &"Entity<TerminalView>")
            .finish_non_exhaustive()
    }
}

/// Build a `gpui::Font` with the given family name and standard terminal fallbacks.
#[must_use]
pub fn terminal_font(family: &str) -> gpui::Font {
    let fallbacks = gpui::FontFallbacks::from_fonts(vec![
        "Menlo".to_owned(),
        "SF Mono".to_owned(),
        "Monaco".to_owned(),
        "Courier New".to_owned(),
        "Courier".to_owned(),
    ]);
    gpui::Font {
        family: SharedString::from(family.to_owned()),
        features: default_terminal_font_features(),
        weight: gpui::FontWeight::NORMAL,
        style: gpui::FontStyle::Normal,
        fallbacks: Some(fallbacks),
    }
}

/// Default terminal font with standard fallbacks.
#[must_use]
pub fn default_terminal_font() -> gpui::Font {
    terminal_font("Menlo")
}

/// Font features disabled for terminal rendering (ligatures off).
#[must_use]
pub fn default_terminal_font_features() -> gpui::FontFeatures {
    gpui::FontFeatures(Arc::new(vec![
        ("calt".to_owned(), 0),
        ("liga".to_owned(), 0),
    ]))
}

/// Spawn a PTY-backed terminal pane. Must be called from GPUI app context.
///
/// If `font_family` is provided, overrides the default terminal font.
///
/// # Panics
///
/// Panics if the PTY system fails to open, the shell fails to spawn,
/// or the virtual terminal fails to initialize.
#[allow(clippy::too_many_arguments)]
pub fn spawn_terminal_pane(
    cwd: Option<&str>,
    font_family: Option<&str>,
    font_size: f32,
    line_height_ratio: f32,
    colors: &TerminalColors,
    window: &mut gpui::Window,
    cx: &mut gpui::App,
) -> TerminalPane {
    // Compute initial PTY size from viewport to avoid 80x24 → actual-size
    // transition that garbles output (shell starts emitting at wrong dimensions).
    // Uses conservative fractions (70% width, 40% height) to account for UI chrome.
    // The resize observer corrects to exact values shortly after.
    let (initial_cols, initial_rows) =
        compute_cell_metrics(window, Some(font_size), font_family, line_height_ratio).map_or(
            (80, 24),
            |(cw, ch)| {
                let vp = window.viewport_size();
                #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
                let cols = ((f32::from(vp.width) * 0.7) / cw).floor().max(80.0) as u16;
                #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
                let rows = ((f32::from(vp.height) * 0.4) / ch).floor().max(24.0) as u16;
                (cols, rows)
            },
        );

    let pty_system = native_pty_system();
    let pty_pair = pty_system
        .openpty(PtySize {
            rows: initial_rows,
            cols: initial_cols,
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

    let child_pid = child.process_id();

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
    let font = font_family.map_or_else(default_terminal_font, terminal_font);
    let font_family_str = font.family.clone();
    let colors = colors.clone();

    let mut pane_focus_handle: Option<gpui::FocusHandle> = None;
    let view = cx.new(|cx| {
        let focus_handle = cx.focus_handle();
        focus_handle.focus(window, cx);
        pane_focus_handle = Some(focus_handle.clone());

        let term =
            cx.new(|_cx| Terminal::new(initial_cols as usize, initial_rows as usize, stdin_tx));

        TerminalView::new(
            term,
            focus_handle,
            font_family_str,
            font_size,
            line_height_ratio,
            colors,
            cx,
        )
    });

    TerminalPane {
        view,
        focus_handle: pane_focus_handle.expect("focus_handle must be set"),
        master,
        stdout_rx,
        stdin_tx: stdin_tx_for_pane,
        child_pid,
    }
}

/// Compute terminal cell metrics from the current window text system.
///
/// If `custom_font_size` is provided, it overrides the window's default text size.
/// If `custom_font_family` is provided, it overrides the default terminal font family.
pub fn compute_cell_metrics(
    window: &mut gpui::Window,
    custom_font_size: Option<f32>,
    custom_font_family: Option<&str>,
    line_height_ratio: f32,
) -> Option<(f32, f32)> {
    let mut style = window.text_style();
    let font = default_terminal_font();
    if let Some(family) = custom_font_family {
        style.font_family = SharedString::from(family.to_owned());
    } else {
        style.font_family = font.family.clone();
    }
    style.font_features = default_terminal_font_features();
    style.font_fallbacks.clone_from(&font.fallbacks);

    let rem_size = window.rem_size();
    let font_size = custom_font_size.map_or_else(|| style.font_size.to_pixels(rem_size), gpui::px);
    style.font_size = font_size.into();
    let line_height = gpui::px(f32::from(font_size) * line_height_ratio);

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
