use alacritty_terminal::term::cell::Flags as CellFlags;
use alacritty_terminal::vte::ansi::CursorShape;
use std::cell::Cell;
use std::rc::Rc;

use gpui::{
    Bounds, Element, FocusHandle, GlobalElementId, Hitbox, InspectorElementId, IntoElement,
    LayoutId, Pixels, Point, Rgba, SharedString, Style, TextStyle, WrappedLine, px, relative, size,
};

use super::terminal::{IndexedCell, Terminal, TerminalColors, TerminalContent};

/// Styling info for a contiguous run of same-styled cells.
#[derive(Clone, PartialEq)]
#[allow(clippy::struct_excessive_bools)]
struct RunStyle {
    fg: Rgba,
    bg: Rgba,
    bold: bool,
    italic: bool,
    underline: bool,
    strikethrough: bool,
}

/// Prepaint state holding shaped text and paint commands.
#[allow(missing_debug_implementations)]
pub struct TerminalElementState {
    _hitbox: Hitbox,
    _cell_width: Pixels,
    line_height: Pixels,
    bg_rects: Vec<(Bounds<Pixels>, Rgba)>,
    selection_rects: Vec<Bounds<Pixels>>,
    cursor_rect: Option<(Bounds<Pixels>, CursorShape)>,
    text_runs: Vec<ShapedRun>,
}

struct ShapedRun {
    origin: Point<Pixels>,
    line: WrappedLine,
}

/// Shared state for deferred resize: Element writes desired size in prepaint,
/// View reads it in render and applies resize outside the render pipeline.
pub type PendingResize = Rc<Cell<Option<(usize, usize)>>>;

/// GPUI Element that renders terminal cells.
#[allow(missing_debug_implementations)]
pub struct TerminalElement {
    terminal: gpui::Entity<Terminal>,
    _focus_handle: FocusHandle,
    focused: bool,
    font_family: SharedString,
    font_size: Pixels,
    pending_resize: PendingResize,
    bg_color: Rgba,
    colors: TerminalColors,
}

impl TerminalElement {
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        terminal: gpui::Entity<Terminal>,
        focus_handle: FocusHandle,
        focused: bool,
        font_family: SharedString,
        font_size: Pixels,
        pending_resize: PendingResize,
        bg_color: Rgba,
        colors: TerminalColors,
    ) -> Self {
        Self {
            terminal,
            _focus_handle: focus_handle,
            focused,
            font_family,
            font_size,
            pending_resize,
            bg_color,
            colors,
        }
    }
}

impl IntoElement for TerminalElement {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}

impl Element for TerminalElement {
    type RequestLayoutState = ();
    type PrepaintState = TerminalElementState;

    fn id(&self) -> Option<gpui::ElementId> {
        Some(gpui::ElementId::from((
            "terminal",
            self.terminal.entity_id(),
        )))
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut gpui::Window,
        cx: &mut gpui::App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let mut style = Style::default();
        style.size.width = relative(1.).into();
        style.size.height = relative(1.).into();
        let layout_id = window.request_layout(style, [], cx);
        (layout_id, ())
    }

    #[allow(clippy::too_many_lines)]
    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        window: &mut gpui::Window,
        cx: &mut gpui::App,
    ) -> Self::PrepaintState {
        let hitbox = window.insert_hitbox(bounds, gpui::HitboxBehavior::Normal);

        // Compute cell metrics.
        let font_size = self.font_size;
        let line_height = font_size;
        let cell_width = compute_cell_width(window, &self.font_family, font_size);

        // Compute desired cols/rows from bounds and store for deferred resize.
        // The TerminalView will read this in render() and resize the terminal
        // outside the render pipeline (avoiding infinite re-render loops).
        let bw = f32::from(bounds.size.width);
        let bh = f32::from(bounds.size.height);
        let cw = f32::from(cell_width);
        let lh = f32::from(line_height);
        if bw > cw && bh > lh {
            #[allow(clippy::cast_sign_loss)]
            let fit_cols = (bw / cw).floor() as usize;
            #[allow(clippy::cast_sign_loss)]
            let fit_rows = (bh / lh).floor() as usize;
            if fit_cols > 0 && fit_rows > 0 {
                self.pending_resize.set(Some((fit_cols, fit_rows)));
            }
        }

        // Sync terminal content.
        let content = self.terminal.update(cx, |t, _| {
            t.sync();
            t.last_content.clone()
        });

        let mut bg_rects: Vec<(Bounds<Pixels>, Rgba)> = Vec::new();
        let mut selection_rects: Vec<Bounds<Pixels>> = Vec::new();
        let mut text_runs: Vec<ShapedRun> = Vec::new();

        // Group cells by line for rendering.
        let mut line_cells: Vec<Vec<&IndexedCell>> = Vec::new();
        let mut current_line: i32 = i32::MIN;

        for cell in &content.cells {
            let line_idx = cell.point.line.0;
            if line_idx != current_line {
                line_cells.push(Vec::new());
                current_line = line_idx;
            }
            if let Some(last) = line_cells.last_mut() {
                last.push(cell);
            }
        }

        for line in &line_cells {
            if line.is_empty() {
                continue;
            }

            // Process cells into styled runs.
            let mut run_start_col: usize = 0;
            let mut run_text = String::new();
            let mut run_style: Option<RunStyle> = None;
            let row = line[0].point.line.0;
            #[allow(clippy::cast_precision_loss)]
            let y = px(f32::from(font_size) * row as f32);

            for cell in line {
                let (fg, bg) = resolve_cell_colors(cell, &content, &self.colors);
                let style = RunStyle {
                    fg,
                    bg,
                    bold: cell.flags.contains(CellFlags::BOLD),
                    italic: cell.flags.contains(CellFlags::ITALIC),
                    underline: cell.flags.intersects(CellFlags::ALL_UNDERLINES),
                    strikethrough: cell.flags.contains(CellFlags::STRIKEOUT),
                };

                let col = cell.point.column.0;
                #[allow(clippy::cast_precision_loss)]
                let cell_x = px(f32::from(cell_width) * col as f32);

                // Paint cell background (skip cells matching terminal bg).
                if bg != self.bg_color {
                    bg_rects.push((
                        Bounds::new(
                            bounds.origin + Point::new(cell_x, y),
                            size(cell_width, line_height),
                        ),
                        bg,
                    ));
                }

                // Selection highlight.
                if let Some(ref sel) = content.selection_range {
                    if sel.contains(cell.point) {
                        selection_rects.push(Bounds::new(
                            bounds.origin + Point::new(cell_x, y),
                            size(cell_width, line_height),
                        ));
                    }
                }

                // Check if we need to start a new text run.
                let should_flush = run_style.as_ref().is_some_and(|s| *s != style)
                    || (run_style.is_some() && col != run_start_col + run_text.chars().count());

                if should_flush {
                    if let Some(ref rs) = run_style {
                        if let Some(shaped) =
                            shape_run(&run_text, rs, &self.font_family, font_size, window)
                        {
                            #[allow(clippy::cast_precision_loss)]
                            let origin = bounds.origin
                                + Point::new(px(f32::from(cell_width) * run_start_col as f32), y);
                            text_runs.push(ShapedRun {
                                origin,
                                line: shaped,
                            });
                        }
                    }
                    run_text.clear();
                    run_start_col = col;
                    run_style = Some(style.clone());
                } else if run_style.is_none() {
                    run_start_col = col;
                    run_style = Some(style.clone());
                }

                // Append character.
                if cell.c != ' ' || cell.flags.contains(CellFlags::WIDE_CHAR) {
                    run_text.push(cell.c);
                } else {
                    run_text.push(' ');
                }

                // Append zero-width chars.
                if let Some(ref zw) = cell.zerowidth {
                    for &ch in zw {
                        run_text.push(ch);
                    }
                }
            }

            // Flush last run.
            if let Some(ref rs) = run_style {
                if !run_text.is_empty() {
                    if let Some(shaped) =
                        shape_run(&run_text, rs, &self.font_family, font_size, window)
                    {
                        #[allow(clippy::cast_precision_loss)]
                        let origin = bounds.origin
                            + Point::new(px(f32::from(cell_width) * run_start_col as f32), y);
                        text_runs.push(ShapedRun {
                            origin,
                            line: shaped,
                        });
                    }
                }
            }
        }

        // Cursor rect.
        #[allow(clippy::cast_precision_loss)]
        let cursor_rect = if content.cursor.visible && self.focused {
            let cursor_x = px(f32::from(cell_width) * content.cursor.point.column.0 as f32);
            let cursor_y = px(f32::from(font_size) * content.cursor.point.line.0 as f32);
            Some((
                Bounds::new(
                    bounds.origin + Point::new(cursor_x, cursor_y),
                    size(cell_width, line_height),
                ),
                content.cursor.shape,
            ))
        } else {
            None
        };

        TerminalElementState {
            _hitbox: hitbox,
            _cell_width: cell_width,
            line_height,
            bg_rects,
            selection_rects,
            cursor_rect,
            text_runs,
        }
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut gpui::Window,
        cx: &mut gpui::App,
    ) {
        // Paint full terminal background.
        window.paint_quad(gpui::fill(bounds, self.bg_color));

        // Paint cell backgrounds.
        for (rect, color) in &prepaint.bg_rects {
            window.paint_quad(gpui::fill(*rect, *color));
        }

        // Paint selection overlay.
        let sel_color = gpui::rgba(0x2644_6ECC);
        for rect in &prepaint.selection_rects {
            window.paint_quad(gpui::fill(*rect, sel_color));
        }

        // Paint text.
        for run in &prepaint.text_runs {
            run.line
                .paint(
                    run.origin,
                    prepaint.line_height,
                    gpui::TextAlign::Left,
                    None,
                    window,
                    cx,
                )
                .ok();
        }

        // Paint cursor.
        if let Some((rect, shape)) = &prepaint.cursor_rect {
            let cursor_color = gpui::rgba(0xCCCC_CCFF);
            match shape {
                CursorShape::Block => {
                    window.paint_quad(gpui::fill(*rect, cursor_color));
                }
                CursorShape::Beam => {
                    let beam = Bounds::new(rect.origin, size(px(2.0), rect.size.height));
                    window.paint_quad(gpui::fill(beam, cursor_color));
                }
                CursorShape::Underline => {
                    let ul = Bounds::new(
                        Point::new(rect.origin.x, rect.origin.y + rect.size.height - px(2.0)),
                        size(rect.size.width, px(2.0)),
                    );
                    window.paint_quad(gpui::fill(ul, cursor_color));
                }
                CursorShape::HollowBlock => {
                    window.paint_quad(gpui::outline(*rect, cursor_color, gpui::BorderStyle::Solid));
                }
                CursorShape::Hidden => {}
            }
        }
    }
}

fn compute_cell_width(
    window: &mut gpui::Window,
    family: &SharedString,
    font_size: Pixels,
) -> Pixels {
    let style = TextStyle {
        font_family: family.clone(),
        font_size: font_size.into(),
        ..TextStyle::default()
    };
    let run = style.to_run(1);
    let text = SharedString::from("M");
    if let Ok(lines) = window
        .text_system()
        .shape_text(text, font_size, &[run], None, Some(1))
    {
        if let Some(line) = lines.first() {
            return line.width().max(px(1.0));
        }
    }
    px(8.0)
}

fn resolve_cell_colors(
    cell: &IndexedCell,
    _content: &TerminalContent,
    colors: &TerminalColors,
) -> (Rgba, Rgba) {
    let mut fg = colors.convert(&cell.fg, true);
    let mut bg = colors.convert(&cell.bg, false);

    // Handle inverse/reverse video.
    if cell.flags.contains(CellFlags::INVERSE) {
        std::mem::swap(&mut fg, &mut bg);
    }

    // Dim flag.
    if cell.flags.contains(CellFlags::DIM) {
        fg.r *= 0.66;
        fg.g *= 0.66;
        fg.b *= 0.66;
    }

    // Hidden flag.
    if cell.flags.contains(CellFlags::HIDDEN) {
        fg = bg;
    }

    (fg, bg)
}

fn shape_run(
    text: &str,
    style: &RunStyle,
    font_family: &SharedString,
    font_size: Pixels,
    window: &mut gpui::Window,
) -> Option<WrappedLine> {
    if text.is_empty() {
        return None;
    }

    let mut text_style = TextStyle {
        font_family: font_family.clone(),
        font_size: font_size.into(),
        color: gpui::Hsla::from(style.fg),
        font_weight: if style.bold {
            gpui::FontWeight::BOLD
        } else {
            gpui::FontWeight::default()
        },
        font_style: if style.italic {
            gpui::FontStyle::Italic
        } else {
            gpui::FontStyle::default()
        },
        ..TextStyle::default()
    };

    if style.underline {
        text_style.underline = Some(gpui::UnderlineStyle {
            thickness: px(1.0),
            color: Some(gpui::Hsla::from(style.fg)),
            wavy: false,
        });
    }
    if style.strikethrough {
        text_style.strikethrough = Some(gpui::StrikethroughStyle {
            thickness: px(1.0),
            color: Some(gpui::Hsla::from(style.fg)),
        });
    }

    let run = text_style.to_run(text.len());
    let shared = SharedString::from(text.to_owned());
    window
        .text_system()
        .shape_text(shared, font_size, &[run], None, Some(1))
        .ok()
        .and_then(|lines| lines.into_iter().next())
}
