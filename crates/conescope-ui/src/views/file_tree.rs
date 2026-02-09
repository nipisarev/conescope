use std::collections::HashSet;
use std::path::Path;

use gpui::prelude::*;
use gpui::{
    ClipboardItem, Entity, EventEmitter, FocusHandle, Hsla, MouseButton, Pixels, Point,
    ScrollHandle, div, px, rgba, svg,
};

use crate::actions::{
    CopyPath, CopyRelativePath, FileCopy, FileCut, FileDelete, FileDuplicate, FilePaste,
    FileRename, FileTrash, NewFile, NewFolder, OpenInTerminal, RevealInFinder,
};
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

/// Tracks which file's context menu is open and where to render it.
#[derive(Debug)]
struct ContextMenuState {
    path: String,
    position: Point<Pixels>,
}

#[derive(Clone, Debug)]
enum ClipboardOp {
    Cut(String),
    Copied(String),
}

#[derive(Debug)]
pub struct FileTree {
    app_state: Entity<AppState>,
    entries: Vec<FileEntry>,
    expanded: HashSet<String>,
    selected: Option<String>,
    root_path: Option<String>,
    scroll_handle: ScrollHandle,
    root_handle: ScrollHandle,
    scrollbar_state: ScrollbarState,
    focus_handle: FocusHandle,
    context_menu: Option<ContextMenuState>,
    clipboard: Option<ClipboardOp>,
    /// Inline rename state: path being renamed.
    rename_path: Option<String>,
    rename_value: String,
}

impl FileTree {
    #[must_use]
    pub fn new(app_state: Entity<AppState>, cx: &mut gpui::Context<Self>) -> Self {
        Self {
            app_state,
            entries: Vec::new(),
            expanded: HashSet::new(),
            selected: None,
            root_path: None,
            scroll_handle: ScrollHandle::new(),
            root_handle: ScrollHandle::new(),
            scrollbar_state: ScrollbarState::default(),
            focus_handle: cx.focus_handle(),
            context_menu: None,
            clipboard: None,
            rename_path: None,
            rename_value: String::new(),
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

    /// Get the target directory for new file/folder operations.
    fn target_dir(&self) -> Option<String> {
        if let Some(ref sel) = self.selected {
            let p = Path::new(sel);
            if p.is_dir() {
                Some(sel.clone())
            } else {
                p.parent().map(|pp| pp.to_string_lossy().to_string())
            }
        } else {
            self.root_path.clone()
        }
    }

    /// Generate a unique path by appending a suffix if needed.
    fn unique_path(base: &Path) -> std::path::PathBuf {
        if !base.exists() {
            return base.to_path_buf();
        }
        let stem = base.file_stem().and_then(|s| s.to_str()).unwrap_or("file");
        let ext = base.extension().and_then(|e| e.to_str());
        let parent = base.parent().unwrap_or(base);
        for i in 1..100 {
            let name = if let Some(ext) = ext {
                format!("{stem}-{i}.{ext}")
            } else {
                format!("{stem}-{i}")
            };
            let candidate = parent.join(name);
            if !candidate.exists() {
                return candidate;
            }
        }
        base.to_path_buf()
    }

    // ── Action handlers ──────────────────────────────────────────

    fn on_new_file(&mut self, _: &NewFile, _w: &mut gpui::Window, cx: &mut gpui::Context<Self>) {
        let Some(dir) = self.target_dir() else { return };
        let base = Path::new(&dir).join("untitled");
        let dest = Self::unique_path(&base);
        if std::fs::write(&dest, "").is_ok() {
            let path_str = dest.to_string_lossy().to_string();
            self.expanded.insert(dir);
            self.rebuild_entries();
            self.selected = Some(path_str.clone());
            // Enter inline rename immediately
            self.rename_path = Some(path_str);
            dest.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("untitled")
                .clone_into(&mut self.rename_value);
            cx.notify();
        }
    }

    fn on_new_folder(
        &mut self,
        _: &NewFolder,
        _w: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) {
        let Some(dir) = self.target_dir() else { return };
        let base = Path::new(&dir).join("new-folder");
        let dest = Self::unique_path(&base);
        if std::fs::create_dir_all(&dest).is_ok() {
            let path_str = dest.to_string_lossy().to_string();
            self.expanded.insert(dir);
            self.expanded.insert(path_str.clone());
            self.rebuild_entries();
            self.selected = Some(path_str.clone());
            self.rename_path = Some(path_str);
            dest.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("new-folder")
                .clone_into(&mut self.rename_value);
            cx.notify();
        }
    }

    fn on_reveal_in_finder(
        &mut self,
        _: &RevealInFinder,
        _w: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) {
        let path = self.selected.as_deref().or(self.root_path.as_deref());
        if let Some(p) = path {
            let _ = std::process::Command::new("open").arg("-R").arg(p).spawn();
        }
        self.context_menu = None;
        cx.notify();
    }

    fn on_open_in_terminal(
        &mut self,
        _: &OpenInTerminal,
        _w: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) {
        let dir = self.selected.as_deref().and_then(|s| {
            let p = Path::new(s);
            if p.is_dir() {
                Some(s.to_owned())
            } else {
                p.parent().map(|pp| pp.to_string_lossy().to_string())
            }
        });
        if let Some(d) = dir {
            let _ = std::process::Command::new("open")
                .arg("-a")
                .arg("Terminal")
                .arg(&d)
                .spawn();
        }
        self.context_menu = None;
        cx.notify();
    }

    fn on_cut(&mut self, _: &FileCut, _w: &mut gpui::Window, cx: &mut gpui::Context<Self>) {
        if let Some(ref sel) = self.selected {
            self.clipboard = Some(ClipboardOp::Cut(sel.clone()));
        }
        self.context_menu = None;
        cx.notify();
    }

    fn on_copy(&mut self, _: &FileCopy, _w: &mut gpui::Window, cx: &mut gpui::Context<Self>) {
        if let Some(ref sel) = self.selected {
            self.clipboard = Some(ClipboardOp::Copied(sel.clone()));
        }
        self.context_menu = None;
        cx.notify();
    }

    fn on_paste(&mut self, _: &FilePaste, _w: &mut gpui::Window, cx: &mut gpui::Context<Self>) {
        let Some(ref clip) = self.clipboard.clone() else {
            return;
        };
        let Some(dir) = self.target_dir() else { return };

        let (src, is_cut) = match clip {
            ClipboardOp::Cut(p) => (p.clone(), true),
            ClipboardOp::Copied(p) => (p.clone(), false),
        };
        let src_path = Path::new(&src);
        let file_name = src_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("file");
        let dest_base = Path::new(&dir).join(file_name);

        if is_cut {
            let dest = Self::unique_path(&dest_base);
            let _ = std::fs::rename(&src, &dest);
            self.clipboard = None;
        } else {
            let dest = Self::unique_path(&dest_base);
            if src_path.is_dir() {
                copy_dir_recursive(src_path, &dest);
            } else {
                let _ = std::fs::copy(&src, &dest);
            }
        }
        self.expanded.insert(dir);
        self.rebuild_entries();
        self.context_menu = None;
        cx.notify();
    }

    fn on_duplicate(
        &mut self,
        _: &FileDuplicate,
        _w: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) {
        let Some(ref sel) = self.selected.clone() else {
            return;
        };
        let src = Path::new(sel);
        let parent = src.parent().unwrap_or(src);
        let stem = src.file_stem().and_then(|s| s.to_str()).unwrap_or("file");
        let ext = src.extension().and_then(|e| e.to_str());

        let dest_name = if let Some(ext) = ext {
            format!("{stem} copy.{ext}")
        } else {
            format!("{stem} copy")
        };
        let dest_base = parent.join(&dest_name);
        let dest = Self::unique_path(&dest_base);

        if src.is_dir() {
            copy_dir_recursive(src, &dest);
        } else {
            let _ = std::fs::copy(src, &dest);
        }
        self.rebuild_entries();
        self.selected = Some(dest.to_string_lossy().to_string());
        self.context_menu = None;
        cx.notify();
    }

    fn on_copy_path(&mut self, _: &CopyPath, _w: &mut gpui::Window, cx: &mut gpui::Context<Self>) {
        if let Some(ref sel) = self.selected {
            cx.write_to_clipboard(ClipboardItem::new_string(sel.clone()));
        }
        self.context_menu = None;
        cx.notify();
    }

    fn on_copy_relative_path(
        &mut self,
        _: &CopyRelativePath,
        _w: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) {
        if let (Some(sel), Some(root)) = (&self.selected, &self.root_path) {
            let rel = Path::new(sel)
                .strip_prefix(root)
                .map_or_else(|_| sel.clone(), |p| p.to_string_lossy().to_string());
            cx.write_to_clipboard(ClipboardItem::new_string(rel));
        }
        self.context_menu = None;
        cx.notify();
    }

    fn on_rename(&mut self, _: &FileRename, _w: &mut gpui::Window, cx: &mut gpui::Context<Self>) {
        if let Some(ref sel) = self.selected {
            let name = Path::new(sel)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_owned();
            self.rename_path = Some(sel.clone());
            self.rename_value = name;
        }
        self.context_menu = None;
        cx.notify();
    }

    fn on_trash(&mut self, _: &FileTrash, _w: &mut gpui::Window, cx: &mut gpui::Context<Self>) {
        if let Some(ref sel) = self.selected {
            #[cfg(target_os = "macos")]
            {
                let _ = std::process::Command::new("osascript")
                    .arg("-e")
                    .arg(format!(
                        "tell application \"Finder\" to delete POSIX file \"{}\"",
                        sel.replace('"', "\\\"")
                    ))
                    .output();
            }
            self.selected = None;
            self.rebuild_entries();
        }
        self.context_menu = None;
        cx.notify();
    }

    fn on_delete(&mut self, _: &FileDelete, _w: &mut gpui::Window, cx: &mut gpui::Context<Self>) {
        if let Some(ref sel) = self.selected {
            let p = Path::new(sel);
            if p.is_dir() {
                let _ = std::fs::remove_dir_all(p);
            } else {
                let _ = std::fs::remove_file(p);
            }
            self.selected = None;
            self.rebuild_entries();
        }
        self.context_menu = None;
        cx.notify();
    }

    fn confirm_rename(&mut self, cx: &mut gpui::Context<Self>) {
        let Some(ref old_path) = self.rename_path.clone() else {
            return;
        };
        let new_name = self.rename_value.trim();
        if new_name.is_empty() || new_name.contains('/') {
            self.cancel_rename(cx);
            return;
        }
        let old = Path::new(old_path);
        if let Some(parent) = old.parent() {
            let new_path = parent.join(new_name);
            if new_path != old && !new_path.exists() {
                let _ = std::fs::rename(old, &new_path);
                self.selected = Some(new_path.to_string_lossy().to_string());
            }
        }
        self.rename_path = None;
        self.rename_value.clear();
        self.rebuild_entries();
        cx.notify();
    }

    fn cancel_rename(&mut self, cx: &mut gpui::Context<Self>) {
        self.rename_path = None;
        self.rename_value.clear();
        cx.notify();
    }
}

impl Render for FileTree {
    #[allow(clippy::too_many_lines)]
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

        let renaming = self.rename_path.clone();
        let rename_val = self.rename_value.clone();
        let has_clipboard = self.clipboard.is_some();

        let mut scroll_div = div()
            .id("file-tree-scroll")
            .size_full()
            .min_h_0()
            .overflow_y_scroll()
            .track_scroll(&self.scroll_handle)
            .py(px(4.));

        for (i, entry) in self.entries.iter().enumerate() {
            let is_renaming = renaming.as_deref() == Some(&entry.path);
            if is_renaming {
                scroll_div = scroll_div.child(render_rename_entry(entry, &rename_val, &theme, cx));
            } else {
                scroll_div = scroll_div.child(render_entry(
                    entry,
                    i,
                    self.selected.as_ref(),
                    &self.expanded,
                    &theme,
                    cx,
                ));
            }
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

        let mut root = div()
            .id("file-tree-container")
            .key_context("FileTree")
            .track_focus(&self.focus_handle)
            .track_scroll(&self.root_handle)
            .relative()
            .size_full()
            .min_h_0()
            .on_action(cx.listener(Self::on_new_file))
            .on_action(cx.listener(Self::on_new_folder))
            .on_action(cx.listener(Self::on_reveal_in_finder))
            .on_action(cx.listener(Self::on_open_in_terminal))
            .on_action(cx.listener(Self::on_cut))
            .on_action(cx.listener(Self::on_copy))
            .on_action(cx.listener(Self::on_paste))
            .on_action(cx.listener(Self::on_duplicate))
            .on_action(cx.listener(Self::on_copy_path))
            .on_action(cx.listener(Self::on_copy_relative_path))
            .on_action(cx.listener(Self::on_rename))
            .on_action(cx.listener(Self::on_trash))
            .on_action(cx.listener(Self::on_delete))
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
            .children(scrollbar_el);

        // Context menu overlay
        if let Some(ref menu) = self.context_menu {
            root = root.child(
                div()
                    .id("ft-ctx-dismiss")
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
                    ),
            );
            root = root.child(render_context_menu(menu, has_clipboard, &theme, cx));
        }

        root.into_any_element()
    }
}

// ── Context menu ─────────────────────────────────────────────────

#[allow(clippy::too_many_lines)]
fn render_context_menu(
    menu: &ContextMenuState,
    has_clipboard: bool,
    theme: &Theme,
    cx: &mut gpui::Context<FileTree>,
) -> impl IntoElement {
    let x = f32::from(menu.position.x);
    let y = f32::from(menu.position.y);

    let path = menu.path.clone();

    let mut items: Vec<gpui::AnyElement> = Vec::new();

    // Group 1: New File / New Folder
    items.push(
        ctx_item(
            "New File",
            "\u{2318}N",
            theme,
            cx.listener(|this, _, window, cx| {
                this.context_menu = None;
                this.on_new_file(&NewFile, window, cx);
            }),
        )
        .into_any_element(),
    );
    items.push(
        ctx_item(
            "New Folder",
            "\u{2325}\u{2318}N",
            theme,
            cx.listener(|this, _, window, cx| {
                this.context_menu = None;
                this.on_new_folder(&NewFolder, window, cx);
            }),
        )
        .into_any_element(),
    );

    // Separator
    items.push(ctx_separator(theme));

    // Group 2: Reveal / Open in Terminal
    {
        let p = path.clone();
        items.push(
            ctx_item(
                "Reveal in Finder",
                "\u{2325}\u{2318}R",
                theme,
                cx.listener(move |this, _, _, cx| {
                    let _ = std::process::Command::new("open").arg("-R").arg(&p).spawn();
                    this.context_menu = None;
                    cx.notify();
                }),
            )
            .into_any_element(),
        );
    }
    {
        let p = path.clone();
        items.push(
            ctx_item(
                "Open in Terminal",
                "",
                theme,
                cx.listener(move |this, _, _, cx| {
                    let dir = if Path::new(&p).is_dir() {
                        p.clone()
                    } else {
                        Path::new(&p)
                            .parent()
                            .map(|pp| pp.to_string_lossy().to_string())
                            .unwrap_or(p.clone())
                    };
                    let _ = std::process::Command::new("open")
                        .arg("-a")
                        .arg("Terminal")
                        .arg(&dir)
                        .spawn();
                    this.context_menu = None;
                    cx.notify();
                }),
            )
            .into_any_element(),
        );
    }

    // Separator
    items.push(ctx_separator(theme));

    // Group 3: Cut / Copy / Duplicate / Paste
    {
        let p = path.clone();
        items.push(
            ctx_item(
                "Cut",
                "\u{2318}X",
                theme,
                cx.listener(move |this, _, _, cx| {
                    this.clipboard = Some(ClipboardOp::Cut(p.clone()));
                    this.context_menu = None;
                    cx.notify();
                }),
            )
            .into_any_element(),
        );
    }
    {
        let p = path.clone();
        items.push(
            ctx_item(
                "Copy",
                "\u{2318}C",
                theme,
                cx.listener(move |this, _, _, cx| {
                    this.clipboard = Some(ClipboardOp::Copied(p.clone()));
                    this.context_menu = None;
                    cx.notify();
                }),
            )
            .into_any_element(),
        );
    }
    {
        let p = path.clone();
        items.push(
            ctx_item(
                "Duplicate",
                "\u{2318}D",
                theme,
                cx.listener(move |this, _, window, cx| {
                    this.selected = Some(p.clone());
                    this.context_menu = None;
                    this.on_duplicate(&FileDuplicate, window, cx);
                }),
            )
            .into_any_element(),
        );
    }
    // Paste (disabled when no clipboard)
    if has_clipboard {
        items.push(
            ctx_item(
                "Paste",
                "\u{2318}V",
                theme,
                cx.listener(|this, _, window, cx| {
                    this.context_menu = None;
                    this.on_paste(&FilePaste, window, cx);
                }),
            )
            .into_any_element(),
        );
    } else {
        items.push(ctx_item_disabled("Paste", "\u{2318}V", theme));
    }

    // Separator
    items.push(ctx_separator(theme));

    // Group 4: Copy Path / Copy Relative Path
    {
        let p = path.clone();
        items.push(
            ctx_item(
                "Copy Path",
                "\u{2325}\u{2318}C",
                theme,
                cx.listener(move |this, _, _, cx| {
                    cx.write_to_clipboard(ClipboardItem::new_string(p.clone()));
                    this.context_menu = None;
                    cx.notify();
                }),
            )
            .into_any_element(),
        );
    }
    {
        let p = path.clone();
        items.push(
            ctx_item(
                "Copy Relative Path",
                "\u{2325}\u{2318}\u{21e7}C",
                theme,
                cx.listener(move |this, _, _, cx| {
                    let rel = if let Some(ref root) = this.root_path {
                        Path::new(&p)
                            .strip_prefix(root)
                            .map(|r| r.to_string_lossy().to_string())
                            .unwrap_or(p.clone())
                    } else {
                        p.clone()
                    };
                    cx.write_to_clipboard(ClipboardItem::new_string(rel));
                    this.context_menu = None;
                    cx.notify();
                }),
            )
            .into_any_element(),
        );
    }

    // Separator
    items.push(ctx_separator(theme));

    // Group 5: Rename / Trash / Delete
    {
        let p = path.clone();
        items.push(
            ctx_item(
                "Rename",
                "F2",
                theme,
                cx.listener(move |this, _, _, cx| {
                    let name = Path::new(&p)
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("")
                        .to_owned();
                    this.rename_path = Some(p.clone());
                    this.rename_value = name;
                    this.context_menu = None;
                    cx.notify();
                }),
            )
            .into_any_element(),
        );
    }
    {
        let p = path.clone();
        items.push(
            ctx_item(
                "Trash",
                "\u{232b}",
                theme,
                cx.listener(move |this, _, _, cx| {
                    #[cfg(target_os = "macos")]
                    {
                        let _ = std::process::Command::new("osascript")
                            .arg("-e")
                            .arg(format!(
                                "tell application \"Finder\" to delete POSIX file \"{}\"",
                                p.replace('"', "\\\"")
                            ))
                            .output();
                    }
                    this.selected = None;
                    this.context_menu = None;
                    this.rebuild_entries();
                    cx.notify();
                }),
            )
            .into_any_element(),
        );
    }
    {
        let p = path.clone();
        items.push(
            ctx_item(
                "Delete",
                "\u{2325}\u{2318}\u{232b}",
                theme,
                cx.listener(move |this, _, _, cx| {
                    let pp = Path::new(&p);
                    if pp.is_dir() {
                        let _ = std::fs::remove_dir_all(pp);
                    } else {
                        let _ = std::fs::remove_file(pp);
                    }
                    this.selected = None;
                    this.context_menu = None;
                    this.rebuild_entries();
                    cx.notify();
                }),
            )
            .into_any_element(),
        );
    }

    div()
        .absolute()
        .top(px(y))
        .left(px(x))
        .w(px(250.))
        .bg(theme.surface)
        .border_1()
        .border_color(theme.border)
        .rounded(px(4.))
        .py(px(4.))
        .text_size(px(12.))
        .shadow_md()
        .children(items)
}

fn ctx_item(
    label: &str,
    shortcut: &str,
    theme: &Theme,
    on_click: impl Fn(&gpui::MouseDownEvent, &mut gpui::Window, &mut gpui::App) + 'static,
) -> gpui::Div {
    let hover_bg = theme.element_hover;
    let sc = shortcut.to_owned();
    div()
        .flex()
        .flex_row()
        .items_center()
        .justify_between()
        .px(px(10.))
        .py(px(4.))
        .text_color(theme.text)
        .cursor_pointer()
        .hover(move |s| s.bg(hover_bg))
        .on_mouse_down(MouseButton::Left, on_click)
        .child(label.to_owned())
        .child(
            div()
                .text_color(theme.text_faint)
                .text_size(px(11.))
                .child(sc),
        )
}

fn ctx_item_disabled(label: &str, shortcut: &str, theme: &Theme) -> gpui::AnyElement {
    div()
        .flex()
        .flex_row()
        .items_center()
        .justify_between()
        .px(px(10.))
        .py(px(4.))
        .text_color(theme.text_disabled)
        .child(label.to_owned())
        .child(
            div()
                .text_color(theme.text_disabled)
                .text_size(px(11.))
                .child(shortcut.to_owned()),
        )
        .into_any_element()
}

fn ctx_separator(theme: &Theme) -> gpui::AnyElement {
    div()
        .h(px(1.))
        .mx(px(8.))
        .my(px(3.))
        .bg(theme.border)
        .into_any_element()
}

// ── Inline rename entry ──────────────────────────────────────────

fn render_rename_entry(
    entry: &FileEntry,
    value: &str,
    theme: &Theme,
    cx: &mut gpui::Context<FileTree>,
) -> gpui::Div {
    #[allow(clippy::cast_precision_loss)]
    let indent = px(entry.depth as f32 * 16.0 + 8.0);
    let val = value.to_owned();

    div()
        .pl(indent)
        .pr(px(8.))
        .py(px(2.))
        .text_size(px(13.))
        .bg(theme.element_selected)
        .child(
            div()
                .id("ft-rename-input")
                .flex()
                .flex_row()
                .items_center()
                .gap(px(4.))
                .child(
                    div()
                        .px(px(4.))
                        .py(px(1.))
                        .bg(theme.background)
                        .border_1()
                        .border_color(theme.accent)
                        .rounded(px(2.))
                        .text_color(theme.text)
                        .text_size(px(12.))
                        .child(val),
                )
                .on_key_down(cx.listener(|this, ev: &gpui::KeyDownEvent, _, cx| {
                    match ev.keystroke.key.as_ref() {
                        "enter" | "return" => this.confirm_rename(cx),
                        "escape" => this.cancel_rename(cx),
                        "backspace" => {
                            this.rename_value.pop();
                            cx.notify();
                        }
                        _ => {
                            if let Some(ref ch) = ev.keystroke.key_char {
                                this.rename_value.push_str(ch);
                                cx.notify();
                            }
                        }
                    }
                })),
        )
}

// ── File entry rendering ─────────────────────────────────────────

/// Map file extension to a color for visual distinction.
fn file_ext_color(name: &str) -> gpui::Rgba {
    let ext = name.rsplit('.').next().unwrap_or("");
    match ext {
        "rs" => rgba(0xdea5_84ff),
        "toml" | "yaml" | "yml" | "json" | "json5" => rgba(0x9ece_6aff),
        "ts" | "tsx" => rgba(0x7dce_faff),
        "js" | "jsx" => rgba(0xe0c0_46ff),
        "py" => rgba(0x569c_d6ff),
        "md" | "txt" | "rst" => rgba(0xbbbb_bbff),
        "css" | "scss" | "less" => rgba(0xce9f_c9ff),
        "html" | "htm" => rgba(0xe06c_75ff),
        "sh" | "bash" | "zsh" | "fish" => rgba(0x98c3_79ff),
        "sql" => rgba(0xe5c0_7bff),
        "lock" => rgba(0x6666_66ff),
        _ => rgba(0xcccc_ccff),
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
    let path_for_rclick = entry.path.clone();
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
        .on_mouse_down(
            MouseButton::Right,
            cx.listener(move |this, event: &gpui::MouseDownEvent, _, cx| {
                this.selected = Some(path_for_rclick.clone());
                let root_origin = this.root_handle.bounds().origin;
                this.context_menu = Some(ContextMenuState {
                    path: path_for_rclick.clone(),
                    position: Point {
                        x: event.position.x - root_origin.x,
                        y: event.position.y - root_origin.y,
                    },
                });
                cx.notify();
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

/// Recursively copy a directory.
fn copy_dir_recursive(src: &Path, dest: &Path) {
    let _ = std::fs::create_dir_all(dest);
    if let Ok(entries) = std::fs::read_dir(src) {
        for entry in entries.flatten() {
            let src_child = entry.path();
            let dest_child = dest.join(entry.file_name());
            if src_child.is_dir() {
                copy_dir_recursive(&src_child, &dest_child);
            } else {
                let _ = std::fs::copy(&src_child, &dest_child);
            }
        }
    }
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
