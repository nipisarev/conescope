use std::collections::HashSet;

use gpui::prelude::*;
use gpui::{Entity, EventEmitter, Hsla, MouseButton, ScrollHandle, div, px, rgba, svg};

use crate::state::app_state::AppState;
use crate::theme::Theme;
use crate::views::scrollbar::{self, ScrollbarCallbacks, ScrollbarState};

#[derive(Debug, Clone)]
pub enum FileTreeEvent {
    OpenFile(String),
}

impl EventEmitter<FileTreeEvent> for FileTree {}

#[derive(Debug)]
struct FileEntry {
    path: String,
    name: String,
    is_dir: bool,
    depth: usize,
}

#[derive(Debug)]
pub struct FileTree {
    app_state: Entity<AppState>,
    entries: Vec<FileEntry>,
    expanded: HashSet<String>,
    selected: Option<String>,
    root_path: Option<String>,
    scroll_handle: ScrollHandle,
    scrollbar_state: ScrollbarState,
}

impl FileTree {
    #[must_use]
    pub fn new(app_state: Entity<AppState>) -> Self {
        Self {
            app_state,
            entries: Vec::new(),
            expanded: HashSet::new(),
            selected: None,
            root_path: None,
            scroll_handle: ScrollHandle::new(),
            scrollbar_state: ScrollbarState::default(),
        }
    }

    /// Update the root path and rebuild entries.
    pub fn set_root(&mut self, path: Option<String>) {
        if self.root_path == path {
            return;
        }
        self.root_path = path;
        self.expanded.clear();
        self.rebuild_entries();
    }

    fn rebuild_entries(&mut self) {
        self.entries.clear();
        let Some(root) = self.root_path.clone() else {
            return;
        };
        self.read_dir_into(&root, 0);
    }

    fn read_dir_into(&mut self, dir_path: &str, depth: usize) {
        let Ok(read_dir) = std::fs::read_dir(dir_path) else {
            return;
        };

        let mut dirs = Vec::new();
        let mut files = Vec::new();

        for entry in read_dir.flatten() {
            let Ok(name) = entry.file_name().into_string() else {
                continue;
            };
            // Skip hidden files/dirs
            if name.starts_with('.') {
                continue;
            }
            let path = entry.path().to_string_lossy().to_string();
            let is_dir = entry.file_type().is_ok_and(|ft| ft.is_dir());
            let fe = FileEntry {
                path,
                name,
                is_dir,
                depth,
            };
            if is_dir {
                dirs.push(fe);
            } else {
                files.push(fe);
            }
        }

        dirs.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        files.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

        for dir_entry in dirs {
            let expanded = self.expanded.contains(&dir_entry.path);
            let dir_path = dir_entry.path.clone();
            self.entries.push(dir_entry);
            if expanded {
                self.read_dir_into(&dir_path, depth + 1);
            }
        }
        for file_entry in files {
            self.entries.push(file_entry);
        }
    }

    fn toggle_dir(&mut self, path: &str, cx: &mut gpui::Context<Self>) {
        if self.expanded.contains(path) {
            self.expanded.remove(path);
        } else {
            self.expanded.insert(path.to_owned());
        }
        self.rebuild_entries();
        cx.notify();
    }

    fn select_file(&mut self, path: &str, cx: &mut gpui::Context<Self>) {
        self.selected = Some(path.to_owned());
        cx.emit(FileTreeEvent::OpenFile(path.to_owned()));
        cx.notify();
    }
}

impl Render for FileTree {
    fn render(
        &mut self,
        _window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> impl IntoElement {
        // Sync root path from focused instance's project
        let root = {
            let state = self.app_state.read(cx);
            let focused_id = state.focused_instance_id(cx);
            focused_id.and_then(|id| {
                let il = state.instance_list.read(cx);
                il.find_by_id(id, cx).and_then(|entry| {
                    let inst = entry.read(cx);
                    inst.instance
                        .project_id
                        .as_ref()
                        .and_then(|pid| state.project_store.read(cx).get(pid))
                        .map(|p| p.path.clone())
                })
            })
        };
        self.set_root(root);

        let theme = self.app_state.read(cx).theme().clone();

        if self.entries.is_empty() {
            return div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .text_size(px(11.))
                .text_color(theme.text_disabled)
                .child("No project folder")
                .into_any_element();
        }

        let mut scroll_div = div()
            .id("file-tree-scroll")
            .size_full()
            .min_h_0()
            .overflow_y_scroll()
            .track_scroll(&self.scroll_handle)
            .py(px(4.));

        for (i, entry) in self.entries.iter().enumerate() {
            scroll_div = scroll_div.child(render_entry(
                entry,
                i,
                self.selected.as_ref(),
                &self.expanded,
                &theme,
                cx,
            ));
        }

        let scrollbar_el = scrollbar::render_scrollbar(
            "file-tree",
            &self.scroll_handle,
            &self.scrollbar_state,
            ScrollbarCallbacks {
                on_thumb_hover: cx.listener(|this, hovered: &bool, _, _| {
                    this.scrollbar_state.thumb_hovered = *hovered;
                }),
                on_track_click: cx.listener(|this, ev: &gpui::MouseDownEvent, _, cx| {
                    let click_y =
                        f32::from(ev.position.y) - f32::from(this.scroll_handle.bounds().top());
                    scrollbar::apply_track_click(&this.scroll_handle, click_y);
                    cx.notify();
                }),
                on_drag_start: cx.listener(|this, ev: &gpui::MouseDownEvent, _, _| {
                    this.scrollbar_state.drag = Some(scrollbar::ScrollbarDrag {
                        start_mouse_y: f32::from(ev.position.y),
                        start_offset_y: f32::from(this.scroll_handle.offset().y),
                    });
                }),
            },
        );

        div()
            .id("file-tree-container")
            .relative()
            .size_full()
            .min_h_0()
            .on_hover(cx.listener(|this, hovered: &bool, _, _| {
                this.scrollbar_state.container_hovered = *hovered;
            }))
            .on_mouse_move(cx.listener(|this, ev: &gpui::MouseMoveEvent, _, cx| {
                if let Some(drag) = &this.scrollbar_state.drag {
                    let drag = *drag;
                    scrollbar::apply_drag(&this.scroll_handle, &drag, f32::from(ev.position.y));
                    cx.notify();
                }
            }))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, _, _, _| {
                    this.scrollbar_state.drag = None;
                }),
            )
            .on_mouse_up_out(
                MouseButton::Left,
                cx.listener(|this, _, _, _| {
                    this.scrollbar_state.drag = None;
                }),
            )
            .child(scroll_div)
            .children(scrollbar_el)
            .into_any_element()
    }
}

/// Map file extension to a color for visual distinction.
fn file_ext_color(name: &str) -> gpui::Rgba {
    let ext = name.rsplit('.').next().unwrap_or("");
    match ext {
        "rs" => rgba(0xdea5_84ff), // Rust — orange
        "toml" | "yaml" | "yml" | "json" | "json5" => rgba(0x9ece_6aff), // Config — green
        "ts" | "tsx" => rgba(0x7dce_faff), // TypeScript — blue
        "js" | "jsx" => rgba(0xe0c0_46ff), // JavaScript — yellow
        "py" => rgba(0x569c_d6ff), // Python — blue
        "md" | "txt" | "rst" => rgba(0xbbbb_bbff), // Docs — light gray
        "css" | "scss" | "less" => rgba(0xce9f_c9ff), // Styles — purple
        "html" | "htm" => rgba(0xe06c_75ff), // HTML — red
        "sh" | "bash" | "zsh" | "fish" => rgba(0x98c3_79ff), // Shell — green
        "sql" => rgba(0xe5c0_7bff), // SQL — gold
        "lock" => rgba(0x6666_66ff), // Lock files — dim
        _ => rgba(0xcccc_ccff),    // Default — light
    }
}

#[allow(clippy::too_many_arguments)]
fn render_entry(
    entry: &FileEntry,
    _index: usize,
    selected: Option<&String>,
    expanded: &HashSet<String>,
    theme: &Theme,
    cx: &mut gpui::Context<FileTree>,
) -> gpui::Div {
    #[allow(clippy::cast_precision_loss)]
    let indent = px(entry.depth as f32 * 16.0 + 8.0);
    let is_selected = selected == Some(&entry.path);

    let icon_path = if entry.is_dir {
        crate::icons::icon_for_dir(expanded.contains(&entry.path))
    } else {
        crate::icons::icon_for_file(&entry.name)
    };
    let icon_color: Hsla = if entry.is_dir {
        theme.text_muted.into()
    } else {
        file_ext_color(&entry.name).into()
    };

    let fg = if entry.is_dir {
        theme.text_muted
    } else {
        file_ext_color(&entry.name)
    };
    let bg = if is_selected {
        theme.element_selected
    } else {
        rgba(0x0000_0000)
    };
    let surface = theme.surface;

    let path = entry.path.clone();
    let is_dir = entry.is_dir;
    let name = entry.name.clone();

    div()
        .pl(indent)
        .pr(px(8.))
        .py(px(4.))
        .text_size(px(13.))
        .text_color(fg)
        .bg(bg)
        .cursor_pointer()
        .hover(move |s| s.bg(surface))
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |this, _event: &gpui::MouseDownEvent, _window, cx| {
                if is_dir {
                    this.toggle_dir(&path, cx);
                } else {
                    this.select_file(&path, cx);
                }
            }),
        )
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(4.))
                .child(
                    svg()
                        .path(icon_path)
                        .size(px(14.))
                        .text_color(icon_color)
                        .flex_shrink_0(),
                )
                .child(name),
        )
}

/// Shorthand to get project path from focused instance.
#[must_use]
pub fn focused_project_path(app_state: &Entity<AppState>, cx: &gpui::App) -> Option<String> {
    let state = app_state.read(cx);
    let id = state.focused_instance_id(cx)?;
    let il = state.instance_list.read(cx);
    let entry = il.find_by_id(id, cx)?;
    let inst = entry.read(cx);
    let pid = inst.instance.project_id.as_ref()?;
    let project = state.project_store.read(cx).get(pid)?;
    Some(project.path.clone())
}
