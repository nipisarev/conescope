use std::ops::RangeInclusive;

use alacritty_terminal::event::{Event as AlacTermEvent, EventListener};
use alacritty_terminal::grid::{Dimensions, Scroll};
use alacritty_terminal::index::{Column, Line, Point as AlacPoint, Side};
use alacritty_terminal::selection::{Selection, SelectionType};
use alacritty_terminal::term::cell::{Cell, Flags as CellFlags};
use alacritty_terminal::term::{Config as TermConfig, Term, TermMode};
use alacritty_terminal::vte::ansi::{CursorShape, CursorStyle, Processor};
use gpui::Rgba;

/// Collected events from alacritty Term callbacks.
#[derive(Clone, Debug)]
pub struct EventCollector {
    events: std::rc::Rc<std::cell::RefCell<Vec<AlacTermEvent>>>,
}

impl EventCollector {
    fn new() -> Self {
        Self {
            events: std::rc::Rc::new(std::cell::RefCell::new(Vec::new())),
        }
    }

    fn take(&self) -> Vec<AlacTermEvent> {
        std::mem::take(&mut *self.events.borrow_mut())
    }
}

impl EventListener for EventCollector {
    fn send_event(&self, event: AlacTermEvent) {
        self.events.borrow_mut().push(event);
    }
}

/// Terminal content snapshot for rendering.
#[derive(Clone, Debug)]
pub struct TerminalContent {
    pub cells: Vec<IndexedCell>,
    pub cursor: CursorInfo,
    pub selection_range: Option<SelectionRange>,
    pub display_offset: usize,
    pub mode: TermMode,
    pub cols: usize,
    pub lines: usize,
}

#[derive(Clone, Debug)]
pub struct IndexedCell {
    pub point: AlacPoint,
    pub c: char,
    pub fg: alacritty_terminal::vte::ansi::Color,
    pub bg: alacritty_terminal::vte::ansi::Color,
    pub flags: CellFlags,
    pub zerowidth: Option<Vec<char>>,
}

#[derive(Clone, Debug)]
pub struct CursorInfo {
    pub point: AlacPoint,
    pub shape: CursorShape,
    pub visible: bool,
}

#[derive(Clone, Debug)]
pub struct SelectionRange {
    pub start: AlacPoint,
    pub end: AlacPoint,
    pub is_block: bool,
}

impl SelectionRange {
    /// Check if the given point falls within this selection range.
    #[must_use]
    pub fn contains(&self, point: AlacPoint) -> bool {
        if self.is_block {
            let col_range = self.col_range();
            point.line >= self.start.line
                && point.line <= self.end.line
                && point.column >= *col_range.start()
                && point.column <= *col_range.end()
        } else if self.start.line == self.end.line {
            point.line == self.start.line
                && point.column >= self.start.column
                && point.column <= self.end.column
        } else if point.line == self.start.line {
            point.column >= self.start.column
        } else if point.line == self.end.line {
            point.column <= self.end.column
        } else {
            point.line > self.start.line && point.line < self.end.line
        }
    }

    fn col_range(&self) -> RangeInclusive<Column> {
        let start_col = self.start.column.min(self.end.column);
        let end_col = self.start.column.max(self.end.column);
        start_col..=end_col
    }
}

impl Default for TerminalContent {
    fn default() -> Self {
        Self {
            cells: Vec::new(),
            cursor: CursorInfo {
                point: AlacPoint::new(Line(0), Column(0)),
                shape: CursorShape::Block,
                visible: true,
            },
            selection_range: None,
            display_offset: 0,
            mode: TermMode::empty(),
            cols: 80,
            lines: 24,
        }
    }
}

/// Simple dimensions struct for Term initialization and resize.
struct TermDimensions {
    cols: usize,
    lines: usize,
}

impl Dimensions for TermDimensions {
    fn total_lines(&self) -> usize {
        self.lines
    }
    fn screen_lines(&self) -> usize {
        self.lines
    }
    fn columns(&self) -> usize {
        self.cols
    }
    fn last_column(&self) -> Column {
        Column(self.cols.saturating_sub(1))
    }
    #[allow(clippy::cast_possible_wrap)]
    fn topmost_line(&self) -> Line {
        Line(-(self.history_size() as i32))
    }
    #[allow(clippy::cast_possible_wrap)]
    fn bottommost_line(&self) -> Line {
        Line(self.screen_lines() as i32 - 1)
    }
    fn history_size(&self) -> usize {
        0
    }
}

/// Core terminal entity wrapping `alacritty_terminal::Term`.
pub struct Terminal {
    term: Term<EventCollector>,
    processor: Processor,
    event_collector: EventCollector,
    pub last_content: TerminalContent,
    pty_writer: std::sync::mpsc::Sender<Vec<u8>>,
    /// Title set by OSC escape sequence.
    pub title: Option<String>,
}

impl std::fmt::Debug for Terminal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Terminal")
            .field("cols", &self.last_content.cols)
            .field("lines", &self.last_content.lines)
            .finish_non_exhaustive()
    }
}

impl Terminal {
    /// Create a new terminal with given grid dimensions and PTY writer.
    #[must_use]
    pub fn new(cols: usize, lines: usize, pty_writer: std::sync::mpsc::Sender<Vec<u8>>) -> Self {
        let event_collector = EventCollector::new();
        let config = TermConfig {
            scrolling_history: 10_000,
            default_cursor_style: CursorStyle {
                shape: CursorShape::Block,
                blinking: false,
            },
            vi_mode_cursor_style: None,
            semantic_escape_chars: String::from(",│`|:\"' ()[]{}<>\t"),
            kitty_keyboard: false,
            osc52: alacritty_terminal::term::Osc52::OnlyCopy,
        };
        let dims = TermDimensions { cols, lines };
        let term = Term::new(config, &dims, event_collector.clone());
        let processor = Processor::new();

        Self {
            term,
            processor,
            event_collector,
            last_content: TerminalContent {
                cols,
                lines,
                ..Default::default()
            },
            pty_writer,
            title: None,
        }
    }

    /// Feed raw bytes from PTY output into the terminal parser.
    pub fn feed_bytes(&mut self, bytes: &[u8], cx: &mut gpui::Context<Self>) {
        self.processor.advance(&mut self.term, bytes);
        self.process_events(cx);
        cx.notify();
    }

    /// Sync the renderable content snapshot from the Term grid.
    pub fn sync(&mut self) {
        let cols = self.term.columns();
        let lines = self.term.screen_lines();
        let content = self.term.renderable_content();
        self.last_content = Self::extract_content(content, cols, lines);
    }

    fn extract_content(
        content: alacritty_terminal::term::RenderableContent<'_>,
        cols: usize,
        lines: usize,
    ) -> TerminalContent {
        let selection_range = content.selection.map(|sel| SelectionRange {
            start: sel.start,
            end: sel.end,
            is_block: sel.is_block,
        });

        let cursor = CursorInfo {
            point: content.cursor.point,
            shape: content.cursor.shape,
            visible: content.mode.contains(TermMode::SHOW_CURSOR),
        };

        let mut cells = Vec::new();
        for indexed in content.display_iter {
            let cell: &Cell = indexed.cell;
            // Skip wide char spacers (second cell of double-wide chars).
            if cell.flags.contains(CellFlags::WIDE_CHAR_SPACER) {
                continue;
            }
            cells.push(IndexedCell {
                point: indexed.point,
                c: cell.c,
                fg: cell.fg,
                bg: cell.bg,
                flags: cell.flags,
                zerowidth: cell.zerowidth().map(<[char]>::to_vec),
            });
        }

        TerminalContent {
            cells,
            cursor,
            selection_range,
            display_offset: content.display_offset,
            mode: content.mode,
            cols,
            lines,
        }
    }

    /// Write bytes to the PTY stdin.
    pub fn input(&self, bytes: &[u8]) {
        let _ = self.pty_writer.send(bytes.to_vec());
    }

    /// Resize the terminal grid.
    pub fn resize(&mut self, cols: usize, lines: usize) {
        let dims = TermDimensions { cols, lines };
        self.term.resize(dims);
        self.last_content.cols = cols;
        self.last_content.lines = lines;
    }

    /// Scroll the terminal viewport.
    pub fn scroll(&mut self, delta: i32) {
        self.term.scroll_display(Scroll::Delta(delta));
    }

    /// Get selected text from the terminal.
    #[must_use]
    pub fn selection_text(&self) -> Option<String> {
        self.term.selection_to_string()
    }

    /// Set a text selection starting at the given grid point.
    pub fn set_selection(&mut self, sel_type: SelectionType, point: AlacPoint, side: Side) {
        let sel = Selection::new(sel_type, point, side);
        self.term.selection = Some(sel);
    }

    /// Update the current selection endpoint.
    pub fn update_selection(&mut self, point: AlacPoint, side: Side) {
        if let Some(ref mut sel) = self.term.selection {
            sel.update(point, side);
        }
    }

    /// Clear the current selection.
    pub fn clear_selection(&mut self) {
        self.term.selection = None;
    }

    /// Select all text in the terminal.
    pub fn select_all(&mut self) {
        let top = AlacPoint::new(self.term.topmost_line(), Column(0));
        let bottom = AlacPoint::new(self.term.bottommost_line(), self.term.last_column());
        let mut sel = Selection::new(SelectionType::Simple, top, Side::Left);
        sel.update(bottom, Side::Right);
        self.term.selection = Some(sel);
    }

    /// Get the current terminal mode flags.
    #[must_use]
    pub fn mode(&self) -> TermMode {
        self.last_content.mode
    }

    /// Process collected events from the term (title changes, bell, etc.).
    fn process_events(&mut self, cx: &mut gpui::Context<Self>) {
        let events = self.event_collector.take();
        for event in events {
            match event {
                AlacTermEvent::Title(title) => {
                    self.title = Some(title);
                }
                AlacTermEvent::ResetTitle => {
                    self.title = None;
                }
                AlacTermEvent::PtyWrite(text) => {
                    self.input(text.as_bytes());
                }
                AlacTermEvent::Bell
                | AlacTermEvent::Wakeup
                | AlacTermEvent::MouseCursorDirty
                | AlacTermEvent::CursorBlinkingChange
                | AlacTermEvent::ClipboardStore(_, _)
                | AlacTermEvent::ClipboardLoad(_, _)
                | AlacTermEvent::ColorRequest(_, _)
                | AlacTermEvent::TextAreaSizeRequest(_)
                | AlacTermEvent::Exit
                | AlacTermEvent::ChildExit(_) => {}
            }
        }
        let _ = cx;
    }
}

/// Default ANSI color palette (dark theme).
pub static ANSI_COLORS: [Rgba; 16] = [
    // Normal colors
    Rgba {
        r: 0.12,
        g: 0.12,
        b: 0.12,
        a: 1.0,
    }, // Black
    Rgba {
        r: 0.80,
        g: 0.22,
        b: 0.22,
        a: 1.0,
    }, // Red
    Rgba {
        r: 0.30,
        g: 0.69,
        b: 0.31,
        a: 1.0,
    }, // Green
    Rgba {
        r: 0.93,
        g: 0.86,
        b: 0.51,
        a: 1.0,
    }, // Yellow
    Rgba {
        r: 0.36,
        g: 0.59,
        b: 0.83,
        a: 1.0,
    }, // Blue
    Rgba {
        r: 0.69,
        g: 0.38,
        b: 0.76,
        a: 1.0,
    }, // Magenta
    Rgba {
        r: 0.30,
        g: 0.73,
        b: 0.75,
        a: 1.0,
    }, // Cyan
    Rgba {
        r: 0.80,
        g: 0.80,
        b: 0.80,
        a: 1.0,
    }, // White
    // Bright colors
    Rgba {
        r: 0.40,
        g: 0.40,
        b: 0.40,
        a: 1.0,
    }, // Bright Black
    Rgba {
        r: 0.96,
        g: 0.36,
        b: 0.36,
        a: 1.0,
    }, // Bright Red
    Rgba {
        r: 0.45,
        g: 0.82,
        b: 0.46,
        a: 1.0,
    }, // Bright Green
    Rgba {
        r: 1.00,
        g: 0.93,
        b: 0.55,
        a: 1.0,
    }, // Bright Yellow
    Rgba {
        r: 0.45,
        g: 0.68,
        b: 0.90,
        a: 1.0,
    }, // Bright Blue
    Rgba {
        r: 0.81,
        g: 0.55,
        b: 0.89,
        a: 1.0,
    }, // Bright Magenta
    Rgba {
        r: 0.45,
        g: 0.86,
        b: 0.88,
        a: 1.0,
    }, // Bright Cyan
    Rgba {
        r: 0.93,
        g: 0.93,
        b: 0.93,
        a: 1.0,
    }, // Bright White
];

const DEFAULT_FG: Rgba = Rgba {
    r: 0.80,
    g: 0.80,
    b: 0.80,
    a: 1.0,
};
const DEFAULT_BG: Rgba = Rgba {
    r: 0.118,
    g: 0.118,
    b: 0.118,
    a: 1.0,
};

/// Convert an alacritty Color to a GPUI Rgba.
#[must_use]
pub fn convert_color(color: &alacritty_terminal::vte::ansi::Color, is_fg: bool) -> Rgba {
    use alacritty_terminal::vte::ansi::{Color, NamedColor};
    match color {
        Color::Named(named) => match named {
            NamedColor::Black => ANSI_COLORS[0],
            NamedColor::Red => ANSI_COLORS[1],
            NamedColor::Green => ANSI_COLORS[2],
            NamedColor::Yellow => ANSI_COLORS[3],
            NamedColor::Blue => ANSI_COLORS[4],
            NamedColor::Magenta => ANSI_COLORS[5],
            NamedColor::Cyan => ANSI_COLORS[6],
            NamedColor::White => ANSI_COLORS[7],
            NamedColor::BrightBlack => ANSI_COLORS[8],
            NamedColor::BrightRed => ANSI_COLORS[9],
            NamedColor::BrightGreen => ANSI_COLORS[10],
            NamedColor::BrightYellow => ANSI_COLORS[11],
            NamedColor::BrightBlue => ANSI_COLORS[12],
            NamedColor::BrightMagenta => ANSI_COLORS[13],
            NamedColor::BrightCyan => ANSI_COLORS[14],
            NamedColor::BrightWhite => ANSI_COLORS[15],
            NamedColor::Foreground | NamedColor::BrightForeground => DEFAULT_FG,
            NamedColor::Background => DEFAULT_BG,
            NamedColor::Cursor => Rgba {
                r: 0.80,
                g: 0.80,
                b: 0.80,
                a: 1.0,
            },
            NamedColor::DimBlack => dim_color(ANSI_COLORS[0]),
            NamedColor::DimRed => dim_color(ANSI_COLORS[1]),
            NamedColor::DimGreen => dim_color(ANSI_COLORS[2]),
            NamedColor::DimYellow => dim_color(ANSI_COLORS[3]),
            NamedColor::DimBlue => dim_color(ANSI_COLORS[4]),
            NamedColor::DimMagenta => dim_color(ANSI_COLORS[5]),
            NamedColor::DimCyan => dim_color(ANSI_COLORS[6]),
            NamedColor::DimWhite => dim_color(ANSI_COLORS[7]),
            NamedColor::DimForeground => dim_color(DEFAULT_FG),
        },
        Color::Spec(rgb) => Rgba {
            r: f32::from(rgb.r) / 255.0,
            g: f32::from(rgb.g) / 255.0,
            b: f32::from(rgb.b) / 255.0,
            a: 1.0,
        },
        Color::Indexed(idx) => indexed_color(*idx, is_fg),
    }
}

fn dim_color(c: Rgba) -> Rgba {
    Rgba {
        r: c.r * 0.66,
        g: c.g * 0.66,
        b: c.b * 0.66,
        a: c.a,
    }
}

/// 256-color palette lookup.
fn indexed_color(idx: u8, _is_fg: bool) -> Rgba {
    match idx {
        0..=15 => ANSI_COLORS[idx as usize],
        // 6x6x6 color cube (indices 16-231)
        16..=231 => {
            let idx = idx - 16;
            let r = idx / 36;
            let g = (idx % 36) / 6;
            let b = idx % 6;
            let to_f = |v: u8| {
                if v == 0 {
                    0.0
                } else {
                    (55.0 + 40.0 * f32::from(v)) / 255.0
                }
            };
            Rgba {
                r: to_f(r),
                g: to_f(g),
                b: to_f(b),
                a: 1.0,
            }
        }
        // Grayscale ramp (indices 232-255)
        232..=255 => {
            let v = (8.0 + 10.0 * f32::from(idx - 232)) / 255.0;
            Rgba {
                r: v,
                g: v,
                b: v,
                a: 1.0,
            }
        }
    }
}
