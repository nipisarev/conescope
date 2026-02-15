use std::borrow::Cow;
use std::cell::Cell;
use std::rc::Rc;

use alacritty_terminal::index::{Column, Line, Point as AlacPoint, Side};
use alacritty_terminal::selection::SelectionType;
use alacritty_terminal::term::TermMode;
use gpui::prelude::*;
use gpui::{
    ClipboardItem, CursorStyle, ExternalPaths, FocusHandle, KeyDownEvent, MouseButton,
    MouseDownEvent, MouseMoveEvent, MouseUpEvent, Point, Rgba, ScrollWheelEvent, SharedString,
    actions, div, px, rgba,
};

use super::terminal::{Terminal, TerminalColors};
use super::terminal_element::{CachedOrigin, PendingResize, TerminalElement};

actions!(terminal, [Copy, Paste, SelectAll, SendTab, SendBackTab]);

struct TerminalContextMenu {
    position: Point<gpui::Pixels>,
}

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
    line_height_ratio: f32,
    /// Terminal background color (painted by `TerminalElement`).
    bg_color: gpui::Rgba,
    /// Terminal color palette (from theme).
    colors: TerminalColors,
    /// Buffered PTY output waiting to be fed.
    _pending_output: Vec<u8>,
    /// Track mouse drag state for selection.
    selecting: bool,
    /// Shared state for deferred resize from Element prepaint → View render.
    pending_resize: PendingResize,
    /// Element bounds origin (set by Element prepaint, used for mouse coord conversion).
    cached_origin: CachedOrigin,
    /// Task for continuous scrolling during drag selection.
    drag_scroll_task: Option<gpui::Task<()>>,
    /// Last known mouse position during drag (for auto-scroll).
    last_drag_pos: Option<Point<gpui::Pixels>>,
    /// Cached scroll state for scrollbar rendering (updated via observer + `on_scroll`).
    cached_display_offset: usize,
    cached_history_size: usize,
    cached_screen_lines: usize,
    context_menu: Option<TerminalContextMenu>,
    menu_surface: Rgba,
    menu_border: Rgba,
    menu_hover: Rgba,
    menu_text: Rgba,
    menu_text_faint: Rgba,
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
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        terminal: gpui::Entity<Terminal>,
        focus_handle: FocusHandle,
        font_family: SharedString,
        font_size: f32,
        line_height_ratio: f32,
        colors: TerminalColors,
        cx: &mut gpui::Context<Self>,
    ) -> Self {
        let bg_color = colors.bg;
        let screen_lines = terminal.read(cx).screen_lines();

        // Observe terminal for content changes (feed_bytes triggers notify).
        cx.observe(&terminal, |this: &mut Self, terminal, cx| {
            let t = terminal.read(cx);
            this.cached_display_offset = t.display_offset();
            this.cached_history_size = t.history_size();
            this.cached_screen_lines = t.screen_lines();
        })
        .detach();

        Self {
            terminal,
            focus_handle,
            font_family,
            font_size,
            line_height_ratio,
            bg_color,
            colors,
            _pending_output: Vec::new(),
            selecting: false,
            pending_resize: Rc::new(Cell::new(None)),
            cached_origin: Rc::new(Cell::new(gpui::Point::default())),
            drag_scroll_task: None,
            last_drag_pos: None,
            cached_display_offset: 0,
            cached_history_size: 0,
            cached_screen_lines: screen_lines,
            context_menu: None,
            menu_surface: rgba(0x2d2d_2dff),
            menu_border: rgba(0x4040_40ff),
            menu_hover: rgba(0x3a3a_3aff),
            menu_text: rgba(0xe0e0_e0ff),
            menu_text_faint: rgba(0x6666_66ff),
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

    /// Update the terminal background color and palette.
    pub fn set_colors(&mut self, colors: TerminalColors) {
        self.bg_color = colors.bg;
        self.colors = colors;
    }

    /// Update the terminal font.
    pub fn set_font(&mut self, font: gpui::Font) {
        self.font_family = font.family;
        // Font size is controlled separately via text_size cascade.
    }

    /// Update the terminal font size.
    pub fn set_font_size(&mut self, size: f32) {
        self.font_size = size;
    }

    /// Update the terminal line-height ratio.
    pub fn set_line_height_ratio(&mut self, ratio: f32) {
        self.line_height_ratio = ratio;
    }

    #[allow(clippy::too_many_arguments)]
    pub fn set_menu_colors(
        &mut self,
        surface: Rgba,
        border: Rgba,
        hover: Rgba,
        text: Rgba,
        text_faint: Rgba,
    ) {
        self.menu_surface = surface;
        self.menu_border = border;
        self.menu_hover = hover;
        self.menu_text = text;
        self.menu_text_faint = text_faint;
    }

    fn on_key_down(
        &mut self,
        event: &KeyDownEvent,
        _window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) {
        // Let Cmd+ shortcuts propagate to global action handlers
        // (Cmd+C/V/A are handled via on_action, not on_key_down).
        if event.keystroke.modifiers.platform {
            if event.keystroke.key == "backspace" {
                self.terminal.read(cx).input(b"\x15");
                cx.notify();
            }
            return;
        }
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

    fn on_right_click(
        &mut self,
        event: &MouseDownEvent,
        window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) {
        self.focus_handle.focus(window, cx);
        cx.stop_propagation();
        let origin = self.cached_origin.get();
        self.context_menu = Some(TerminalContextMenu {
            position: gpui::Point {
                x: event.position.x - origin.x,
                y: event.position.y - origin.y,
            },
        });
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

        // Store last drag position for auto-scroll
        self.last_drag_pos = Some(event.position);

        let delta = self.drag_line_delta(event.position);
        if delta != 0 {
            self.terminal.update(cx, |term, _| {
                term.scroll_lines(delta);
            });
            let t = self.terminal.read(cx);
            self.cached_display_offset = t.display_offset();
            self.cached_history_size = t.history_size();
            self.cached_screen_lines = t.screen_lines();

            // Start continuous scroll loop if not already running
            if self.drag_scroll_task.is_none() {
                self.drag_scroll_task = Some(Self::spawn_drag_scroll_task(cx));
            }
        } else {
            // Clear task when delta is 0
            self.drag_scroll_task = None;
            self.last_drag_pos = None;
        }

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
        self.drag_scroll_task = None;
        self.last_drag_pos = None;
        cx.notify();
    }

    fn on_scroll(
        &mut self,
        event: &ScrollWheelEvent,
        _window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) {
        cx.stop_propagation();
        let line_height = gpui::px(self.font_size * self.line_height_ratio);
        self.terminal.update(cx, |term, _| {
            term.scroll_wheel(event, line_height);
        });
        // Refresh cached scroll state for scrollbar (scroll_wheel doesn't notify Terminal).
        let t = self.terminal.read(cx);
        self.cached_display_offset = t.display_offset();
        self.cached_history_size = t.history_size();
        self.cached_screen_lines = t.screen_lines();
        cx.notify();
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

    fn send_tab(&mut self, _: &SendTab, _window: &mut gpui::Window, cx: &mut gpui::Context<Self>) {
        self.terminal.read(cx).input(b"\t");
        cx.notify();
    }

    fn send_back_tab(
        &mut self,
        _: &SendBackTab,
        _window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) {
        self.terminal.read(cx).input(b"\x1b[Z");
        cx.notify();
    }

    fn spawn_drag_scroll_task(cx: &mut gpui::Context<Self>) -> gpui::Task<()> {
        cx.spawn(async |this, cx| {
            loop {
                cx.background_executor()
                    .timer(std::time::Duration::from_millis(30))
                    .await;

                let should_continue = this
                    .update(cx, |view, cx| {
                        if !view.selecting {
                            view.drag_scroll_task = None;
                            view.last_drag_pos = None;
                            return false;
                        }

                        if let Some(pos) = view.last_drag_pos {
                            let delta = view.drag_line_delta(pos);
                            if delta != 0 {
                                view.terminal.update(cx, |term, _| {
                                    term.scroll_lines(delta);
                                });
                                let t = view.terminal.read(cx);
                                view.cached_display_offset = t.display_offset();
                                view.cached_history_size = t.history_size();
                                view.cached_screen_lines = t.screen_lines();
                                cx.notify();
                                return true;
                            }
                        }

                        view.drag_scroll_task = None;
                        view.last_drag_pos = None;
                        false
                    })
                    .unwrap_or(false);

                if !should_continue {
                    break;
                }
            }
        })
    }

    fn drag_line_delta(&self, cursor_pos: gpui::Point<gpui::Pixels>) -> i32 {
        let origin = self.cached_origin.get();
        let line_height = gpui::px(self.font_size * self.line_height_ratio);
        let line_height_f = f32::from(line_height);
        #[allow(clippy::cast_precision_loss)]
        let viewport_bottom = origin.y + gpui::px(self.cached_screen_lines as f32 * line_height_f);
        // Threshold: trigger auto-scroll when cursor is within half a line of the edge.
        // GPUI only delivers mouse_move within element bounds, so the cursor can't
        // actually go above origin.y — a proximity zone is needed for the top edge.
        let threshold = line_height * 0.5;
        let top_edge = origin.y + threshold;
        let bottom_edge = viewport_bottom - threshold;

        if cursor_pos.y < top_edge {
            let dist = f32::from(top_edge - cursor_pos.y) + line_height_f;
            (dist.powf(1.1) / line_height_f).clamp(1.0, 3.0) as i32
        } else if cursor_pos.y > bottom_edge {
            let dist = f32::from(cursor_pos.y - bottom_edge) + line_height_f;
            -((dist.powf(1.1) / line_height_f).clamp(1.0, 3.0) as i32)
        } else {
            0
        }
    }

    /// Convert a window-relative pixel position to a terminal grid point.
    fn pixel_to_grid(
        &self,
        position: gpui::Point<gpui::Pixels>,
        window: &mut gpui::Window,
    ) -> (AlacPoint, Side) {
        // Subtract element origin to convert window-relative → element-local coords.
        let origin = self.cached_origin.get();
        let local_x = f32::from(position.x - origin.x).max(0.0);
        let local_y = f32::from(position.y - origin.y).max(0.0);

        let cell_width = f32::from(super::terminal_element::compute_cell_width(
            window,
            &self.font_family,
            gpui::px(self.font_size),
        ))
        .max(1.0);
        let line_height = (self.font_size * self.line_height_ratio).max(1.0);

        #[allow(clippy::cast_sign_loss)]
        let col = (local_x / cell_width).floor() as usize;
        let viewport_line = (local_y / line_height).floor() as i32;

        #[allow(clippy::cast_possible_wrap)]
        let line = viewport_line - self.cached_display_offset as i32;

        let cell_x = local_x % cell_width;
        let side = if cell_x > cell_width / 2.0 {
            Side::Right
        } else {
            Side::Left
        };

        (AlacPoint::new(Line(line), Column(col)), side)
    }
}

impl Render for TerminalView {
    #[allow(clippy::too_many_lines)]
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
        let cached_origin = self.cached_origin.clone();

        // Compute scrollbar thumb from cached scroll state.
        let scrollbar = if self.cached_history_size > 0 {
            let line_height = self.font_size * self.line_height_ratio;
            #[allow(clippy::cast_precision_loss)]
            let viewport_h = self.cached_screen_lines as f32 * line_height;
            #[allow(clippy::cast_precision_loss)]
            let total_h =
                (self.cached_screen_lines + self.cached_history_size) as f32 * line_height;
            let visible_fraction = viewport_h / total_h;
            let thumb_h = (visible_fraction * viewport_h).max(20.0);
            #[allow(clippy::cast_precision_loss)]
            let scroll_fraction = (self.cached_history_size - self.cached_display_offset) as f32
                / self.cached_history_size as f32;
            let scrollable_track = viewport_h - thumb_h;
            let thumb_top = scroll_fraction * scrollable_track;

            Some(
                div()
                    .absolute()
                    .right(px(1.))
                    .top(px(thumb_top))
                    .w(px(6.))
                    .h(px(thumb_h))
                    .rounded(px(3.))
                    .bg(rgba(0x8080_8088))
                    .opacity(0.)
                    .group_hover("terminal-scroll", |s| s.opacity(0.7)),
            )
        } else {
            None
        };

        let mut container = div()
            .group("terminal-scroll")
            .size_full()
            .relative()
            .cursor(CursorStyle::IBeam)
            .track_focus(&self.focus_handle)
            .key_context("Terminal")
            .on_key_down(cx.listener(Self::on_key_down))
            .on_mouse_down(MouseButton::Left, cx.listener(Self::on_mouse_down))
            .on_mouse_down(MouseButton::Right, cx.listener(Self::on_right_click))
            .on_mouse_move(cx.listener(Self::on_mouse_move))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_scroll_wheel(cx.listener(Self::on_scroll))
            .on_action(cx.listener(Self::copy))
            .on_action(cx.listener(Self::paste))
            .on_action(cx.listener(Self::select_all))
            .on_action(cx.listener(Self::send_tab))
            .on_action(cx.listener(Self::send_back_tab))
            .on_drop(
                cx.listener(|this: &mut Self, paths: &ExternalPaths, _window, cx| {
                    let escaped = shell_escape_paths(paths.paths());
                    if !escaped.is_empty() {
                        this.terminal.read(cx).input(escaped.as_bytes());
                        cx.notify();
                    }
                }),
            )
            .text_size(font_size)
            .child(TerminalElement::new(
                terminal,
                focus_handle,
                focused,
                font_family,
                font_size,
                self.line_height_ratio,
                pending_resize,
                cached_origin,
                self.bg_color,
                self.colors.clone(),
            ));

        if let Some(sb) = scrollbar {
            container = container.child(sb);
        }

        if let Some(menu) = &self.context_menu {
            let menu_pos = menu.position;
            let has_selection = self.terminal.read(cx).selection_text().is_some();
            let surface = self.menu_surface;
            let border = self.menu_border;
            let hover = self.menu_hover;
            let text = self.menu_text;
            let text_faint = self.menu_text_faint;

            container = container.child(
                div()
                    .id("terminal-ctx-dismiss")
                    .absolute()
                    .size_full()
                    .top_0()
                    .left_0()
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, _, _, cx| {
                            this.context_menu = None;
                            cx.notify();
                        }),
                    )
                    .on_mouse_down(
                        MouseButton::Right,
                        cx.listener(|this, _, _, cx| {
                            this.context_menu = None;
                            cx.notify();
                        }),
                    )
                    .child(
                        div()
                            .absolute()
                            .top(menu_pos.y)
                            .left(menu_pos.x)
                            .min_w(px(200.))
                            .bg(surface)
                            .border_1()
                            .border_color(border)
                            .rounded(px(6.))
                            .shadow_lg()
                            .py(px(4.))
                            .text_size(px(12.))
                            .child(terminal_ctx_item(
                                "New Terminal",
                                "⌘T",
                                surface,
                                hover,
                                text,
                                cx.listener(|_, _, window, cx| {
                                    window.dispatch_action(
                                        Box::new(crate::actions::NewTerminalTab),
                                        cx,
                                    );
                                }),
                            ))
                            .child(terminal_ctx_separator(border))
                            .when(!has_selection, |div| {
                                div.child(terminal_ctx_item_disabled(
                                    "Copy", "⌘C", surface, text_faint,
                                ))
                            })
                            .when(has_selection, |div| {
                                div.child(terminal_ctx_item(
                                    "Copy",
                                    "⌘C",
                                    surface,
                                    hover,
                                    text,
                                    cx.listener(|this, _, window, cx| {
                                        this.copy(&Copy, window, cx);
                                        this.context_menu = None;
                                        cx.notify();
                                    }),
                                ))
                            })
                            .child(terminal_ctx_item(
                                "Paste",
                                "⌘V",
                                surface,
                                hover,
                                text,
                                cx.listener(|this, _, window, cx| {
                                    this.paste(&Paste, window, cx);
                                    this.context_menu = None;
                                    cx.notify();
                                }),
                            ))
                            .child(terminal_ctx_item(
                                "Select All",
                                "⌘A",
                                surface,
                                hover,
                                text,
                                cx.listener(|this, _, window, cx| {
                                    this.select_all(&SelectAll, window, cx);
                                    this.context_menu = None;
                                    cx.notify();
                                }),
                            ))
                            .child(terminal_ctx_item(
                                "Clear",
                                "⌘K",
                                surface,
                                hover,
                                text,
                                cx.listener(|this, _, _, cx| {
                                    this.terminal.update(cx, |term, _| {
                                        term.clear_screen();
                                    });
                                    this.context_menu = None;
                                    cx.notify();
                                }),
                            ))
                            .child(terminal_ctx_separator(border))
                            .child(terminal_ctx_item(
                                "Close Terminal",
                                "⌘W",
                                surface,
                                hover,
                                text,
                                cx.listener(|this, _, window, cx| {
                                    window.dispatch_action(Box::new(crate::actions::CloseTab), cx);
                                    this.context_menu = None;
                                    cx.notify();
                                }),
                            )),
                    ),
            );
        }

        container
    }
}

/// Encode a GPUI keystroke to terminal escape bytes.
#[allow(clippy::too_many_lines)]
fn encode_keystroke(keystroke: &gpui::Keystroke, mode: TermMode) -> Option<Cow<'static, [u8]>> {
    let key = keystroke.key_char.as_deref().unwrap_or("");
    let modifiers = &keystroke.modifiers;

    // Named keys.
    let app_cursor = mode.contains(TermMode::APP_CURSOR);
    let result: Option<Cow<'static, [u8]>> = match keystroke.key.as_ref() {
        "enter" | "return" => {
            if modifiers.shift {
                Some(Cow::Borrowed(b"\n"))
            } else {
                Some(Cow::Borrowed(b"\r"))
            }
        }
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

    // Ctrl+letter -> control code (0x01-0x1A).
    // Use keystroke.key (physical key) since key_char is None for control combos on macOS.
    if modifiers.control {
        if let Some(ch) = keystroke.key.chars().next() {
            if ch.is_ascii_alphabetic() {
                let code = (ch.to_ascii_lowercase() as u8) - b'a' + 1;
                return Some(Cow::Owned(vec![code]));
            }
        }
    }

    // Alt/Meta prefix (use physical key if key_char unavailable).
    if modifiers.alt {
        let ch = key.chars().next().or_else(|| keystroke.key.chars().next());
        if let Some(ch) = ch {
            let mut buf = vec![b'\x1b'];
            let mut tmp = [0u8; 4];
            let encoded = ch.encode_utf8(&mut tmp);
            buf.extend_from_slice(encoded.as_bytes());
            return Some(Cow::Owned(buf));
        }
    }

    // Character keys.
    if key.is_empty() {
        return None;
    }

    let ch = key.chars().next()?;

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

fn shell_escape_paths(paths: &[std::path::PathBuf]) -> String {
    paths
        .iter()
        .map(|p| {
            let s = p.to_string_lossy();
            format!("'{}'", s.replace('\'', "'\\''"))
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[allow(clippy::too_many_arguments)]
fn terminal_ctx_item(
    label: &str,
    shortcut: &str,
    _surface: Rgba,
    hover: Rgba,
    text: Rgba,
    on_click: impl Fn(&MouseDownEvent, &mut gpui::Window, &mut gpui::App) + 'static,
) -> gpui::Div {
    let sc = shortcut.to_owned();
    div()
        .flex()
        .flex_row()
        .items_center()
        .justify_between()
        .px(px(10.))
        .py(px(4.))
        .text_color(text)
        .cursor_pointer()
        .hover(move |s| s.bg(hover))
        .on_mouse_down(MouseButton::Left, move |ev, win, cx| {
            cx.stop_propagation();
            on_click(ev, win, cx);
        })
        .child(label.to_owned())
        .child(div().text_color(text).child(sc))
}

fn terminal_ctx_item_disabled(
    label: &str,
    shortcut: &str,
    _surface: Rgba,
    text_faint: Rgba,
) -> gpui::AnyElement {
    div()
        .flex()
        .flex_row()
        .items_center()
        .justify_between()
        .px(px(10.))
        .py(px(4.))
        .text_color(text_faint)
        .child(label.to_owned())
        .child(div().text_color(text_faint).child(shortcut.to_owned()))
        .into_any_element()
}

fn terminal_ctx_separator(border: Rgba) -> gpui::AnyElement {
    div()
        .h(px(1.))
        .mx(px(8.))
        .my(px(3.))
        .bg(border)
        .into_any_element()
}
