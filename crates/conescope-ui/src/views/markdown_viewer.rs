use gpui::prelude::*;
use gpui::{AnyElement, Entity, FontWeight, ScrollHandle, SharedString, div, px};
use pulldown_cmark::{Event, HeadingLevel, Parser, Tag, TagEnd};

use crate::state::app_state::AppState;
use crate::theme::Theme;

pub struct MarkdownViewer {
    app_state: Entity<AppState>,
    file_path: Option<String>,
    content: String,
    scroll_handle: ScrollHandle,
}

impl std::fmt::Debug for MarkdownViewer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MarkdownViewer")
            .field("file_path", &self.file_path)
            .finish_non_exhaustive()
    }
}

impl MarkdownViewer {
    #[must_use]
    pub fn new(app_state: Entity<AppState>) -> Self {
        Self {
            app_state,
            file_path: None,
            content: String::new(),
            scroll_handle: ScrollHandle::new(),
        }
    }

    pub fn show_preview(&mut self, path: &str, content: &str, cx: &mut gpui::Context<Self>) {
        self.file_path = Some(path.to_owned());
        content.clone_into(&mut self.content);
        cx.notify();
    }

    pub fn update_content(&mut self, content: &str, cx: &mut gpui::Context<Self>) {
        content.clone_into(&mut self.content);
        cx.notify();
    }

    pub fn clear(&mut self, cx: &mut gpui::Context<Self>) {
        self.file_path = None;
        self.content.clear();
        cx.notify();
    }

    #[must_use]
    pub fn file_path(&self) -> Option<&str> {
        self.file_path.as_deref()
    }
}

struct BlockBuilder {
    blocks: Vec<AnyElement>,
    text_buf: String,
    in_code_block: bool,
    in_heading: Option<HeadingLevel>,
    in_blockquote: bool,
    list_stack: Vec<Option<u64>>,
    item_index: u64,
}

impl BlockBuilder {
    fn new() -> Self {
        Self {
            blocks: Vec::new(),
            text_buf: String::new(),
            in_code_block: false,
            in_heading: None,
            in_blockquote: false,
            list_stack: Vec::new(),
            item_index: 0,
        }
    }

    fn flush_heading(&mut self, level: HeadingLevel, theme: &Theme) {
        if self.text_buf.is_empty() {
            return;
        }
        let text = std::mem::take(&mut self.text_buf);
        let el = match level {
            HeadingLevel::H1 => div()
                .text_size(px(24.))
                .font_weight(FontWeight::BOLD)
                .mb(px(12.))
                .mt(px(16.))
                .text_color(theme.text)
                .child(text)
                .into_any_element(),
            HeadingLevel::H2 => div()
                .text_size(px(20.))
                .font_weight(FontWeight::BOLD)
                .mb(px(10.))
                .mt(px(14.))
                .pb(px(4.))
                .border_b_1()
                .border_color(theme.border)
                .text_color(theme.text)
                .child(text)
                .into_any_element(),
            HeadingLevel::H3 => div()
                .text_size(px(16.))
                .font_weight(FontWeight::SEMIBOLD)
                .mb(px(8.))
                .mt(px(12.))
                .text_color(theme.text)
                .child(text)
                .into_any_element(),
            _ => div()
                .text_size(px(14.))
                .font_weight(FontWeight::SEMIBOLD)
                .mb(px(6.))
                .mt(px(10.))
                .text_color(theme.text)
                .child(text)
                .into_any_element(),
        };
        self.blocks.push(el);
    }

    fn flush_paragraph(&mut self, theme: &Theme) {
        if self.text_buf.is_empty() {
            return;
        }
        let text = std::mem::take(&mut self.text_buf);
        if self.in_blockquote {
            let el = div()
                .border_l_2()
                .border_color(theme.border)
                .pl(px(12.))
                .mb(px(8.))
                .text_color(theme.text_muted)
                .child(text)
                .into_any_element();
            self.blocks.push(el);
        } else {
            let el = div()
                .mb(px(8.))
                .text_color(theme.text)
                .child(text)
                .into_any_element();
            self.blocks.push(el);
        }
    }

    fn flush_code_block(&mut self, theme: &Theme) {
        let text = std::mem::take(&mut self.text_buf);
        let el = div()
            .bg(theme.panel)
            .rounded(px(4.))
            .p(px(8.))
            .mb(px(8.))
            .font_family(SharedString::from("monospace"))
            .text_size(px(13.))
            .text_color(theme.text_muted)
            .overflow_hidden()
            .child(text)
            .into_any_element();
        self.blocks.push(el);
    }

    fn flush_list_item(&mut self, theme: &Theme) {
        if self.text_buf.is_empty() {
            return;
        }
        let text = std::mem::take(&mut self.text_buf);
        let prefix = if let Some(start) = self.list_stack.last().copied().flatten() {
            let idx = start + self.item_index;
            format!("{idx}. ")
        } else {
            String::from("\u{2022} ")
        };
        let el = div()
            .pl(px(20.))
            .mb(px(4.))
            .text_color(theme.text)
            .flex()
            .child(
                div()
                    .flex_shrink_0()
                    .text_color(theme.text_muted)
                    .child(prefix),
            )
            .child(div().child(text))
            .into_any_element();
        self.blocks.push(el);
    }
}

fn render_markdown(content: &str, theme: &Theme) -> Vec<AnyElement> {
    let parser = Parser::new(content);
    let mut bb = BlockBuilder::new();

    for event in parser {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                bb.in_heading = Some(level);
                bb.text_buf.clear();
            }
            Event::End(TagEnd::Heading(level)) => {
                bb.flush_heading(level, theme);
                bb.in_heading = None;
            }
            Event::Start(Tag::Paragraph) => {
                bb.text_buf.clear();
            }
            Event::End(TagEnd::Paragraph) => {
                if bb.in_heading.is_none() && !bb.in_code_block {
                    bb.flush_paragraph(theme);
                }
            }
            Event::Start(Tag::CodeBlock(_)) => {
                bb.in_code_block = true;
                bb.text_buf.clear();
            }
            Event::End(TagEnd::CodeBlock) => {
                bb.flush_code_block(theme);
                bb.in_code_block = false;
            }
            Event::Start(Tag::BlockQuote(_)) => {
                bb.in_blockquote = true;
            }
            Event::End(TagEnd::BlockQuote(_)) => {
                bb.in_blockquote = false;
            }
            Event::Start(Tag::List(start)) => {
                bb.list_stack.push(start);
            }
            Event::End(TagEnd::List(_)) => {
                bb.list_stack.pop();
            }
            Event::Start(Tag::Item) => {
                bb.item_index = 0;
                bb.text_buf.clear();
            }
            Event::End(TagEnd::Item) => {
                bb.flush_list_item(theme);
                bb.item_index += 1;
            }
            Event::Rule => {
                bb.blocks.push(
                    div()
                        .h(px(1.))
                        .w_full()
                        .bg(theme.border)
                        .my(px(12.))
                        .into_any_element(),
                );
            }
            Event::Text(text) => {
                bb.text_buf.push_str(&text);
            }
            Event::Code(code) => {
                bb.text_buf.push('`');
                bb.text_buf.push_str(&code);
                bb.text_buf.push('`');
            }
            Event::SoftBreak => {
                bb.text_buf.push(' ');
            }
            Event::HardBreak => {
                bb.text_buf.push('\n');
            }
            _ => {}
        }
    }

    bb.blocks
}

impl Render for MarkdownViewer {
    fn render(
        &mut self,
        _window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> impl IntoElement {
        let state = self.app_state.read(cx);
        let theme = state.theme().clone();

        if self.file_path.is_none() {
            return div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .text_color(theme.text_disabled)
                .child("No preview")
                .into_any_element();
        }

        let blocks = render_markdown(&self.content, &theme);

        let mut container = div()
            .id("md-preview")
            .size_full()
            .overflow_y_scroll()
            .track_scroll(&self.scroll_handle)
            .p(px(16.))
            .bg(theme.editor_bg);

        for block in blocks {
            container = container.child(block);
        }

        container.into_any_element()
    }
}
