use std::ops::Range;
use std::sync::LazyLock;

use gpui::prelude::*;
use gpui::{
    EventEmitter, HighlightStyle, Hsla, ScrollHandle, SharedString, StyledText, div, px, rgba,
};
use syntect::highlighting::ThemeSet;
use syntect::parsing::SyntaxSet;

const MAX_FILE_SIZE: u64 = 1_024 * 1_024; // 1 MB

static SYNTAX_SET: LazyLock<SyntaxSet> = LazyLock::new(SyntaxSet::load_defaults_newlines);
static THEME: LazyLock<syntect::highlighting::Theme> = LazyLock::new(|| {
    let ts = ThemeSet::load_defaults();
    ts.themes
        .get("base16-ocean.dark")
        .cloned()
        .unwrap_or_else(|| ts.themes.values().next().cloned().unwrap_or_default())
});

/// Pre-computed highlighted line: full text + byte-range color highlights.
#[derive(Debug, Clone)]
struct HighlightedLine {
    text: SharedString,
    highlights: Vec<(Range<usize>, HighlightStyle)>,
}

#[derive(Debug, Clone)]
pub enum CodeViewerEvent {
    FileOpened(String),
}

impl EventEmitter<CodeViewerEvent> for CodeViewer {}

#[derive(Debug)]
pub struct CodeViewer {
    file_path: Option<String>,
    highlighted: Vec<HighlightedLine>,
    scroll_handle: ScrollHandle,
}

impl Default for CodeViewer {
    fn default() -> Self {
        Self::new()
    }
}

impl CodeViewer {
    #[must_use]
    pub fn new() -> Self {
        Self {
            file_path: None,
            highlighted: Vec::new(),
            scroll_handle: ScrollHandle::new(),
        }
    }

    /// Open and display a file. Caps at 1 MB.
    pub fn open_file(&mut self, path: &str, cx: &mut gpui::Context<Self>) {
        if let Ok(metadata) = std::fs::metadata(path) {
            if metadata.len() > MAX_FILE_SIZE {
                self.file_path = Some(path.to_owned());
                self.highlighted = vec![HighlightedLine {
                    text: "File too large (> 1 MB)".into(),
                    highlights: Vec::new(),
                }];
                cx.emit(CodeViewerEvent::FileOpened(path.to_owned()));
                cx.notify();
                return;
            }
        }

        if let Ok(content) = std::fs::read_to_string(path) {
            self.highlighted = highlight_lines(path, &content);
            self.file_path = Some(path.to_owned());
            cx.emit(CodeViewerEvent::FileOpened(path.to_owned()));
        } else {
            self.highlighted = vec![HighlightedLine {
                text: "Failed to read file".into(),
                highlights: Vec::new(),
            }];
            self.file_path = Some(path.to_owned());
        }
        cx.notify();
    }

    #[must_use]
    pub fn file_path(&self) -> Option<&str> {
        self.file_path.as_deref()
    }

    pub fn close_file(&mut self, cx: &mut gpui::Context<Self>) {
        self.file_path = None;
        self.highlighted.clear();
        cx.notify();
    }
}

/// Highlight source lines using syntect, producing pre-computed byte-range highlights.
fn highlight_lines(path: &str, content: &str) -> Vec<HighlightedLine> {
    let ss = &*SYNTAX_SET;
    let theme = &*THEME;

    let syntax = ss
        .find_syntax_for_file(path)
        .ok()
        .flatten()
        .unwrap_or_else(|| ss.find_syntax_plain_text());

    let mut highlighter = syntect::easy::HighlightLines::new(syntax, theme);
    let mut result = Vec::with_capacity(content.lines().count());

    for line in content.lines() {
        let line_with_nl = format!("{line}\n");
        let spans = highlighter
            .highlight_line(&line_with_nl, ss)
            .unwrap_or_default();

        let mut text = String::new();
        let mut highlights = Vec::new();

        for (style, span_text) in &spans {
            let trimmed = span_text.trim_end_matches('\n');
            if trimmed.is_empty() {
                continue;
            }
            let start = text.len();
            text.push_str(trimmed);
            let end = text.len();
            highlights.push((
                start..end,
                HighlightStyle {
                    color: Some(syntect_color_to_hsla(style.foreground)),
                    ..Default::default()
                },
            ));
        }

        result.push(HighlightedLine {
            text: SharedString::from(text),
            highlights,
        });
    }

    result
}

/// Convert syntect `Color` to GPUI `Hsla`.
fn syntect_color_to_hsla(color: syntect::highlighting::Color) -> Hsla {
    Hsla::from(gpui::Rgba {
        r: f32::from(color.r) / 255.0,
        g: f32::from(color.g) / 255.0,
        b: f32::from(color.b) / 255.0,
        a: f32::from(color.a) / 255.0,
    })
}

/// Estimated height per line in pixels (`text_size` 12px + flex row padding).
const LINE_HEIGHT_ESTIMATE: f32 = 20.0;
/// Extra lines to render above/below the visible viewport for smooth scrolling.
const OVERSCAN: usize = 20;

impl Render for CodeViewer {
    fn render(
        &mut self,
        _window: &mut gpui::Window,
        _cx: &mut gpui::Context<Self>,
    ) -> impl IntoElement {
        if self.highlighted.is_empty() {
            return div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .text_size(px(12.))
                .text_color(rgba(0x5555_55ff))
                .child("No file open")
                .into_any_element();
        }

        let total = self.highlighted.len();
        let viewport_h = f32::from(self.scroll_handle.bounds().size.height);
        let scroll_y = f32::from(self.scroll_handle.offset().y).abs();

        // Compute visible line range with overscan buffer.
        // Line counts and pixel offsets are always non-negative and small enough for f32.
        #[allow(clippy::cast_sign_loss, clippy::cast_precision_loss)]
        let (first, last, top_spacer_h, bottom_spacer_h) = if viewport_h > 0.0 {
            let first = (scroll_y / LINE_HEIGHT_ESTIMATE).floor() as usize;
            let visible_count = (viewport_h / LINE_HEIGHT_ESTIMATE).ceil() as usize + 1;
            let first = first.saturating_sub(OVERSCAN);
            let last = (first + visible_count + 2 * OVERSCAN).min(total);
            let top = first as f32 * LINE_HEIGHT_ESTIMATE;
            let bottom = (total - last) as f32 * LINE_HEIGHT_ESTIMATE;
            (first, last, top, bottom)
        } else {
            (0, total.min(80), 0.0, 0.0)
        };

        let mut container = div()
            .id("code-viewer-scroll")
            .size_full()
            .overflow_scroll()
            .track_scroll(&self.scroll_handle)
            .bg(rgba(0x1e1e_1eff))
            .text_size(px(12.));

        if top_spacer_h > 0.0 {
            container = container.child(div().h(px(top_spacer_h)).w_full());
        }

        for i in first..last {
            container = container.child(render_line(i + 1, &self.highlighted[i]));
        }

        if bottom_spacer_h > 0.0 {
            container = container.child(div().h(px(bottom_spacer_h)).w_full());
        }

        container.into_any_element()
    }
}

fn render_line(line_number: usize, hl: &HighlightedLine) -> gpui::Div {
    let content = if hl.text.is_empty() {
        div().flex_1().pl(px(8.)).text_size(px(12.)).child(" ")
    } else {
        let styled = StyledText::new(hl.text.clone()).with_highlights(hl.highlights.clone());
        div().flex_1().pl(px(8.)).text_size(px(12.)).child(styled)
    };

    div()
        .flex()
        .flex_row()
        .child(
            div()
                .w(px(40.))
                .flex_shrink_0()
                .text_color(rgba(0x5555_55ff))
                .pr(px(8.))
                .text_size(px(12.))
                .border_r_1()
                .border_color(rgba(0x3c3c_3cff))
                .child(
                    div()
                        .w_full()
                        .flex()
                        .justify_end()
                        .child(line_number.to_string()),
                ),
        )
        .child(content)
}
