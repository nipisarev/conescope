use std::borrow::Cow;
use std::cell::Cell;
use std::rc::Rc;

use alacritty_terminal::index::{Column, Line, Point as AlacPoint, Side};
use alacritty_terminal::selection::SelectionType;
use alacritty_terminal::term::TermMode;
use gpui::prelude::*;
use gpui::{
    ClipboardItem, CursorStyle, FocusHandle, KeyDownEvent, MouseButton, MouseDownEvent,
    MouseMoveEvent, MouseUpEvent, ScrollWheelEvent, SharedString, actions, div, px,
};

use super::terminal::Terminal;
use super::terminal_element::{PendingResize, TerminalElement};

actions!(terminal, [Copy, Paste, SelectAll]);

/// Events emitted by `TerminalView`.
#[derive(Debug, Clone)]
pub enum TerminalViewEvent {
    /// Terminal grid was resized to fit container bounds. PTY should be resized too.
    ContainerResize(u16, u16),
}

impl gpui::EventEmitter<TerminalViewEvent> for TerminalView {}

/// GPUI view wrapping a Terminal entity with keyboard/mouse input and rendering.
pub struct TerminalView {
    pub terminal: gpui::Entity<Terminal>,
    focus_handle: FocusHandle,
    font_family: SharedString,
    font_size: f32,
    /// Terminal background color (painted by `TerminalElement`).
    bg_color: gpui::Rgba,
    /// Buffered PTY output waiting to be fed.
    _pending_output: Vec<u8>,
    /// Track mouse drag state for selection.
    selecting: bool,
    /// Cached cell width for mouse position calculations.
    cached_cell_width: f32,
    /// Shared state for deferred resize from Element prepaint → View render.
    pending_resize: PendingResize,
}

impl std::fmt::Debug for TerminalView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TerminalView")
            .field("font_family", &self.font_family)
            .field("font_size", &self.font_size)
            .finish_non_exhaustive()
    }
}

impl TerminalView {
    #[must_use]
    pub fn new(
        terminal: gpui::Entity<Terminal>,
        focus_handle: FocusHandle,
        font_family: SharedString,
        font_size: f32,
    ) -> Self {
        // Default terminal bg: #292828 (matches Gruvbox theme.panel).
        // Constructed to exactly match DEFAULT_BG in terminal.rs.
        let bg_color = gpui::Rgba {
            r: 41.0 / 255.0,
            g: 40.0 / 255.0,
            b: 40.0 / 255.0,
            a: 1.0,
        };
        Self {
            terminal,
            focus_handle,
            font_family,
            font_size,
            bg_color,
            _pending_output: Vec::new(),
            selecting: false,
            cached_cell_width: 8.0,
            pending_resize: Rc::new(Cell::new(None)),
        }
    }

    /// Queue PTY output bytes. They will be fed to the terminal on next update.
    pub fn queue_output_bytes(&mut self, bytes: &[u8], cx: &mut gpui::Context<Self>) {
        self.terminal.update(cx, |term, cx| {
            term.feed_bytes(bytes, cx);
        });
    }

    /// Resize the terminal grid.
    pub fn resize_terminal(&mut self, cols: u16, rows: u16, cx: &mut gpui::Context<Self>) {
        self.terminal.update(cx, |term, _| {
            term.resize(cols as usize, rows as usize);
        });
        cx.notify();
    }

    /// Update the terminal background color.
    pub fn set_bg_color(&mut self, color: gpui::Rgba) {
        self.bg_color = color;
    }

    /// Update the terminal font.
    pub fn set_font(&mut self, font: gpui::Font) {
        self.font_family = font.family;
        // Font size is controlled separately via text_size cascade.
    }

    fn on_key_down(
        &mut self,
        event: &KeyDownEvent,
        _window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) {
        let mode = self.terminal.read(cx).mode();
        if let Some(bytes) = encode_keystroke(&event.keystroke, mode) {
            self.terminal.read(cx).input(bytes.as_ref());
            cx.notify();
        }
    }

    fn on_mouse_down(
        &mut self,
        event: &MouseDownEvent,
        window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) {
        if event.button != MouseButton::Left {
            return;
        }
        self.focus_handle.focus(window, cx);
        cx.stop_propagation();

        let (point, side) = self.pixel_to_grid(event.position, window);
        let sel_type = match event.click_count {
            2 => SelectionType::Semantic,
            3 => SelectionType::Lines,
            _ => SelectionType::Simple,
        };
        self.terminal.update(cx, |term, _| {
            term.set_selection(sel_type, point, side);
        });
        self.selecting = true;
        cx.notify();
    }

    fn on_mouse_move(
        &mut self,
        event: &MouseMoveEvent,
        window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) {
        if !self.selecting {
            return;
        }
        let (point, side) = self.pixel_to_grid(event.position, window);
        self.terminal.update(cx, |term, _| {
            term.update_selection(point, side);
        });
        cx.notify();
    }

    fn on_mouse_up(
        &mut self,
        event: &MouseUpEvent,
        _window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) {
        if event.button != MouseButton::Left {
            return;
        }
        self.selecting = false;
        cx.notify();
    }

    fn on_scroll(
        &mut self,
        event: &ScrollWheelEvent,
        _window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) {
        let delta = event.delta.pixel_delta(px(self.font_size));
        let lines = -(f32::from(delta.y) / self.font_size) as i32;
        if lines != 0 {
            self.terminal.update(cx, |term, _| {
                term.scroll(lines);
            });
            cx.notify();
        }
    }

    fn copy(&mut self, _: &Copy, _window: &mut gpui::Window, cx: &mut gpui::Context<Self>) {
        let text = self.terminal.read(cx).selection_text();
        if let Some(text) = text {
            cx.write_to_clipboard(ClipboardItem::new_string(text));
        }
    }

    fn paste(&mut self, _: &Paste, _window: &mut gpui::Window, cx: &mut gpui::Context<Self>) {
        if let Some(item) = cx.read_from_clipboard() {
            let text = item.text().unwrap_or_default();
            if text.is_empty() {
                return;
            }
            let mode = self.terminal.read(cx).mode();
            if mode.contains(TermMode::BRACKETED_PASTE) {
                self.terminal.read(cx).input(b"\x1b[200~");
                self.terminal.read(cx).input(text.as_bytes());
                self.terminal.read(cx).input(b"\x1b[201~");
            } else {
                self.terminal.read(cx).input(text.as_bytes());
            }
            cx.notify();
        }
    }

    fn select_all(
        &mut self,
        _: &SelectAll,
        _window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) {
        self.terminal.update(cx, |term, _| {
            term.select_all();
        });
        cx.notify();
    }

    /// Convert a pixel position relative to the window to a terminal grid point.
    fn pixel_to_grid(
        &self,
        position: gpui::Point<gpui::Pixels>,
        _window: &mut gpui::Window,
    ) -> (AlacPoint, Side) {
        let cell_width = self.cached_cell_width.max(1.0);
        let line_height = self.font_size.max(1.0);

        #[allow(clippy::cast_sign_loss)]
        let col = (f32::from(position.x) / cell_width).floor().max(0.0) as usize;
        let line = (f32::from(position.y) / line_height).floor().max(0.0) as i32;

        let cell_x = f32::from(position.x) % cell_width;
        let side = if cell_x > cell_width / 2.0 {
            Side::Right
        } else {
            Side::Left
        };

        (AlacPoint::new(Line(line), Column(col)), side)
    }
}

impl Render for TerminalView {
    fn render(
        &mut self,
        window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> impl IntoElement {
        // Apply deferred resize from previous prepaint (container bounds changed).
        if let Some((cols, rows)) = self.pending_resize.take() {
            let (cur_cols, cur_rows) = {
                let t = self.terminal.read(cx);
                (t.last_content.cols, t.last_content.lines)
            };
            if cur_cols != cols || cur_rows != rows {
                let c = cols as u16;
                let r = rows as u16;
                self.resize_terminal(c, r, cx);
                cx.emit(TerminalViewEvent::ContainerResize(c, r));
            }
        }

        let terminal = self.terminal.clone();
        let focus_handle = self.focus_handle.clone();
        let focused = self.focus_handle.is_focused(window);
        let font_family = self.font_family.clone();
        let font_size = px(self.font_size);
        let pending_resize = self.pending_resize.clone();

        div()
            .size_full()
            .cursor(CursorStyle::IBeam)
            .track_focus(&self.focus_handle)
            .key_context("Terminal")
            .on_key_down(cx.listener(Self::on_key_down))
            .on_mouse_down(MouseButton::Left, cx.listener(Self::on_mouse_down))
            .on_mouse_move(cx.listener(Self::on_mouse_move))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_scroll_wheel(cx.listener(Self::on_scroll))
            .on_action(cx.listener(Self::copy))
            .on_action(cx.listener(Self::paste))
            .on_action(cx.listener(Self::select_all))
            .text_size(font_size)
            .child(TerminalElement::new(
                terminal,
                focus_handle,
                focused,
                font_family,
                font_size,
                pending_resize,
                self.bg_color,
            ))
    }
}

/// Encode a GPUI keystroke to terminal escape bytes.
fn encode_keystroke(keystroke: &gpui::Keystroke, mode: TermMode) -> Option<Cow<'static, [u8]>> {
    let key = keystroke.key_char.as_deref().unwrap_or("");
    let modifiers = &keystroke.modifiers;

    // Named keys.
    let app_cursor = mode.contains(TermMode::APP_CURSOR);
    let result: Option<Cow<'static, [u8]>> = match keystroke.key.as_ref() {
        "enter" | "return" => Some(Cow::Borrowed(b"\r")),
        "escape" => Some(Cow::Borrowed(b"\x1b")),
        "tab" => {
            if modifiers.shift {
                Some(Cow::Borrowed(b"\x1b[Z"))
            } else {
                Some(Cow::Borrowed(b"\t"))
            }
        }
        "backspace" => {
            if modifiers.alt {
                Some(Cow::Borrowed(b"\x1b\x7f"))
            } else {
                Some(Cow::Borrowed(b"\x7f"))
            }
        }
        "delete" => Some(encode_with_modifiers(b"\x1b[3~", 3, *modifiers)),
        "up" => {
            if app_cursor && !modifiers.has_any() {
                Some(Cow::Borrowed(b"\x1bOA"))
            } else {
                Some(encode_arrow(b'A', *modifiers))
            }
        }
        "down" => {
            if app_cursor && !modifiers.has_any() {
                Some(Cow::Borrowed(b"\x1bOB"))
            } else {
                Some(encode_arrow(b'B', *modifiers))
            }
        }
        "right" => {
            if app_cursor && !modifiers.has_any() {
                Some(Cow::Borrowed(b"\x1bOC"))
            } else {
                Some(encode_arrow(b'C', *modifiers))
            }
        }
        "left" => {
            if app_cursor && !modifiers.has_any() {
                Some(Cow::Borrowed(b"\x1bOD"))
            } else {
                Some(encode_arrow(b'D', *modifiers))
            }
        }
        "home" => Some(encode_with_modifiers(b"\x1b[H", 0, *modifiers)),
        "end" => Some(encode_with_modifiers(b"\x1b[F", 0, *modifiers)),
        "pageup" => Some(encode_with_modifiers(b"\x1b[5~", 5, *modifiers)),
        "pagedown" => Some(encode_with_modifiers(b"\x1b[6~", 6, *modifiers)),
        "insert" => Some(encode_with_modifiers(b"\x1b[2~", 2, *modifiers)),
        "f1" => Some(Cow::Borrowed(b"\x1bOP")),
        "f2" => Some(Cow::Borrowed(b"\x1bOQ")),
        "f3" => Some(Cow::Borrowed(b"\x1bOR")),
        "f4" => Some(Cow::Borrowed(b"\x1bOS")),
        "f5" => Some(Cow::Borrowed(b"\x1b[15~")),
        "f6" => Some(Cow::Borrowed(b"\x1b[17~")),
        "f7" => Some(Cow::Borrowed(b"\x1b[18~")),
        "f8" => Some(Cow::Borrowed(b"\x1b[19~")),
        "f9" => Some(Cow::Borrowed(b"\x1b[20~")),
        "f10" => Some(Cow::Borrowed(b"\x1b[21~")),
        "f11" => Some(Cow::Borrowed(b"\x1b[23~")),
        "f12" => Some(Cow::Borrowed(b"\x1b[24~")),
        "space" => {
            if modifiers.control {
                Some(Cow::Borrowed(b"\x00"))
            } else {
                Some(Cow::Borrowed(b" "))
            }
        }
        _ => None,
    };

    if result.is_some() {
        return result;
    }

    // Character keys.
    if key.is_empty() {
        return None;
    }

    let ch = key.chars().next()?;

    // Ctrl+letter -> control code (0x01-0x1A).
    if modifiers.control && ch.is_ascii_alphabetic() {
        let code = (ch.to_ascii_lowercase() as u8) - b'a' + 1;
        return Some(Cow::Owned(vec![code]));
    }

    // Alt/Meta prefix.
    if modifiers.alt {
        let mut buf = vec![b'\x1b'];
        let mut tmp = [0u8; 4];
        let encoded = ch.encode_utf8(&mut tmp);
        buf.extend_from_slice(encoded.as_bytes());
        return Some(Cow::Owned(buf));
    }

    // Regular character.
    let mut buf = [0u8; 4];
    let encoded = ch.encode_utf8(&mut buf);
    Some(Cow::Owned(encoded.as_bytes().to_vec()))
}

fn modifier_code(modifiers: gpui::Modifiers) -> u8 {
    let mut code: u8 = 1;
    if modifiers.shift {
        code += 1;
    }
    if modifiers.alt {
        code += 2;
    }
    if modifiers.control {
        code += 4;
    }
    code
}

fn encode_arrow(letter: u8, modifiers: gpui::Modifiers) -> Cow<'static, [u8]> {
    if modifiers.has_any() {
        let code = modifier_code(modifiers);
        Cow::Owned(format!("\x1b[1;{code}{}", letter as char).into_bytes())
    } else {
        Cow::Owned(vec![b'\x1b', b'[', letter])
    }
}

fn encode_with_modifiers(base: &[u8], num: u8, modifiers: gpui::Modifiers) -> Cow<'static, [u8]> {
    if modifiers.has_any() && num > 0 {
        let code = modifier_code(modifiers);
        Cow::Owned(format!("\x1b[{num};{code}~").into_bytes())
    } else {
        Cow::Owned(base.to_vec())
    }
}

trait ModifiersExt {
    fn has_any(&self) -> bool;
}

impl ModifiersExt for gpui::Modifiers {
    fn has_any(&self) -> bool {
        self.shift || self.control || self.alt || self.platform
    }
}
