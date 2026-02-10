use std::collections::{BTreeSet, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use gpui::prelude::*;
use gpui::{
    ClipboardItem, Entity, EventEmitter, FocusHandle, Hsla, ListSizingBehavior, MouseButton,
    Pixels, Point, ScrollHandle, UniformListScrollHandle, div, px, rgba, svg, uniform_list,
};

use gpui_component::input::{Input, InputEvent, InputState};

use conescope_git::status::FileStatus;

use crate::actions::{
    CancelRename, CopyPath, CopyRelativePath, FileCopy, FileCut, FileDelete, FileDuplicate,
    FilePaste, FileRename, FileTrash, NewFile, NewFolder, OpenInTerminal, RevealInFinder,
};
use crate::state::app_state::AppState;
use crate::state::git_store::{GitStore, GitStoreEvent};
use crate::theme::Theme;

// ── Events ───────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum FileTreeEvent {
    OpenFile(String),
}

impl EventEmitter<FileTreeEvent> for FileTree {}

// ── Entry ID ─────────────────────────────────────────────────────

static NEXT_ENTRY_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct EntryId(u64);

impl EntryId {
    fn next() -> Self {
        Self(NEXT_ENTRY_ID.fetch_add(1, Ordering::Relaxed))
    }
}

// ── Entry types ──────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EntryKind {
    Dir,
    File,
}

#[derive(Debug, Clone)]
struct Entry {
    id: EntryId,
    kind: EntryKind,
    abs_path: PathBuf,
    name: String,
    #[allow(dead_code)]
    is_ignored: bool,
    git_status: Option<FileStatus>,
}

#[derive(Debug, Clone)]
struct VisibleEntry {
    entry_id: EntryId,
    depth: usize,
    folded_name: Option<String>,
}

#[derive(Debug, Clone)]
struct EditState {
    entry_id: EntryId,
    is_new: bool,
    #[allow(dead_code)]
    is_dir: bool,
    processing: bool,
}

// ── Context menu ─────────────────────────────────────────────────

#[derive(Debug)]
struct ContextMenuState {
    entry_id: EntryId,
    position: Point<Pixels>,
}

#[derive(Clone, Debug)]
enum ClipboardOp {
    Cut(EntryId),
    Copied(EntryId),
}

// ── FS watcher handle ────────────────────────────────────────────

#[derive(Debug)]
struct FsWatcherHandle {
    /// Kept alive to signal the watcher loop to continue. Dropping stops it.
    #[allow(dead_code)]
    stop_tx: flume::Sender<()>,
}

// ── FileTree ─────────────────────────────────────────────────────

const ROW_HEIGHT: f32 = 24.0;

#[derive(Debug)]
pub struct FileTree {
    app_state: Entity<AppState>,
    git_store: Option<Entity<GitStore>>,

    // Tree data
    entries: HashMap<EntryId, Entry>,
    children: HashMap<EntryId, Vec<EntryId>>,
    root_entries: Vec<EntryId>,

    // Display
    visible_entries: Vec<VisibleEntry>,
    expanded: HashSet<String>,         // abs paths of expanded dirs
    unfolded_dir_ids: HashSet<String>, // abs paths of manually unfolded dirs

    // Selection
    selected: Option<EntryId>,
    marked_entries: BTreeSet<EntryId>,
    last_clicked_index: Option<usize>,

    // Edit
    edit_state: Option<EditState>,
    rename_input: Option<Entity<InputState>>,

    // Scroll / focus
    scroll_handle: UniformListScrollHandle,
    root_handle: ScrollHandle,
    focus_handle: FocusHandle,

    // Context menu / clipboard
    context_menu: Option<ContextMenuState>,
    clipboard: Option<ClipboardOp>,

    // Root
    root_path: Option<String>,

    // Git status cache
    git_status_map: HashMap<String, FileStatus>,
    git_dirty_dirs: HashSet<String>,

    // FS watcher
    fs_watcher: Option<FsWatcherHandle>,
}

impl FileTree {
    #[must_use]
    pub fn new(
        app_state: Entity<AppState>,
        git_store: Option<Entity<GitStore>>,
        cx: &mut gpui::Context<Self>,
    ) -> Self {
        // Subscribe to git store status changes
        if let Some(ref gs) = git_store {
            cx.subscribe(gs, |this: &mut Self, _store, event, cx| {
                if matches!(event, GitStoreEvent::StatusChanged) {
                    this.update_git_status(cx);
                    this.recompute_visible();
                    cx.notify();
                }
            })
            .detach();
        }

        Self {
            app_state,
            git_store,
            entries: HashMap::new(),
            children: HashMap::new(),
            root_entries: Vec::new(),
            visible_entries: Vec::new(),
            expanded: HashSet::new(),
            unfolded_dir_ids: HashSet::new(),
            selected: None,
            marked_entries: BTreeSet::new(),
            last_clicked_index: None,
            edit_state: None,
            rename_input: None,
            scroll_handle: UniformListScrollHandle::new(),
            root_handle: ScrollHandle::new(),
            focus_handle: cx.focus_handle(),
            context_menu: None,
            clipboard: None,
            root_path: None,
            git_status_map: HashMap::new(),
            git_dirty_dirs: HashSet::new(),
            fs_watcher: None,
        }
    }

    // ── Root path management ─────────────────────────────────────

    pub fn set_root(&mut self, path: Option<String>, cx: &mut gpui::Context<Self>) {
        if self.root_path == path {
            return;
        }
        self.root_path = path;
        self.expanded.clear();
        self.unfolded_dir_ids.clear();
        self.rebuild_entries();
        self.update_git_status(cx);
        self.recompute_visible();
        self.startfs_watcher(cx);
    }

    // ── Tree building ────────────────────────────────────────────

    fn rebuild_entries(&mut self) {
        self.entries.clear();
        self.children.clear();
        self.root_entries.clear();

        let Some(root) = self.root_path.clone() else {
            return;
        };
        self.read_dir_into(&root, None);
    }

    fn read_dir_into(&mut self, dir_path: &str, parent_id: Option<EntryId>) {
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
            let abs_path = entry.path();
            let is_dir = entry.file_type().is_ok_and(|ft| ft.is_dir());
            let kind = if is_dir {
                EntryKind::Dir
            } else {
                EntryKind::File
            };

            let rel_str = self.rel_path_str(&abs_path);
            let git_status = self.git_status_map.get(&rel_str).copied();

            let id = EntryId::next();
            let e = Entry {
                id,
                kind,
                abs_path,
                name,
                is_ignored: false,
                git_status,
            };
            if is_dir {
                dirs.push(e);
            } else {
                files.push(e);
            }
        }

        dirs.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        files.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

        let mut child_ids = Vec::new();

        for dir_entry in dirs {
            let did = dir_entry.id;
            let dir_abs = dir_entry.abs_path.to_string_lossy().to_string();
            let expanded = self.expanded.contains(&dir_abs);
            self.entries.insert(did, dir_entry);
            child_ids.push(did);

            if expanded {
                self.read_dir_into(&dir_abs, Some(did));
            }
        }
        for file_entry in files {
            let fid = file_entry.id;
            self.entries.insert(fid, file_entry);
            child_ids.push(fid);
        }

        if let Some(pid) = parent_id {
            self.children.insert(pid, child_ids);
        } else {
            self.root_entries = child_ids;
        }
    }

    // ── Visible entry computation ────────────────────────────────

    fn recompute_visible(&mut self) {
        self.visible_entries.clear();
        self.flatten_children(&self.root_entries.clone(), 0);
    }

    fn entry_abs_str(&self, id: EntryId) -> Option<String> {
        self.entries
            .get(&id)
            .map(|e| e.abs_path.to_string_lossy().to_string())
    }

    fn is_expanded_id(&self, id: EntryId) -> bool {
        self.entry_abs_str(id)
            .is_some_and(|p| self.expanded.contains(&p))
    }

    fn flatten_children(&mut self, ids: &[EntryId], depth: usize) {
        for &id in ids {
            let Some(entry) = self.entries.get(&id) else {
                continue;
            };

            let entry_path = entry.abs_path.to_string_lossy().to_string();
            if entry.kind == EntryKind::Dir && !self.unfolded_dir_ids.contains(&entry_path) {
                // Try auto-fold: single-child dir chains
                if let Some((leaf_id, folded_name, leaf_depth)) = self.compute_fold_chain(id, depth)
                {
                    self.visible_entries.push(VisibleEntry {
                        entry_id: leaf_id,
                        depth,
                        folded_name: Some(folded_name),
                    });
                    // If expanded, show children of the leaf dir
                    if self.is_expanded_id(leaf_id) {
                        if let Some(kids) = self.children.get(&leaf_id).cloned() {
                            self.flatten_children(&kids, leaf_depth + 1);
                        }
                    }
                    continue;
                }
            }

            self.visible_entries.push(VisibleEntry {
                entry_id: id,
                depth,
                folded_name: None,
            });

            if entry.kind == EntryKind::Dir && self.expanded.contains(&entry_path) {
                if let Some(kids) = self.children.get(&id).cloned() {
                    self.flatten_children(&kids, depth + 1);
                }
            }
        }
    }

    /// Walk single-child directory chains for auto-folding.
    /// Returns `(leaf_dir_id, combined_name, leaf_depth)` if foldable.
    fn compute_fold_chain(
        &self,
        start_id: EntryId,
        start_depth: usize,
    ) -> Option<(EntryId, String, usize)> {
        let mut current = start_id;
        let mut names = Vec::new();
        let mut depth = start_depth;

        loop {
            let entry = self.entries.get(&current)?;
            if entry.kind != EntryKind::Dir {
                return None;
            }
            names.push(entry.name.clone());

            let kids = self.children.get(&current)?;
            if kids.len() != 1 {
                break;
            }

            let child = kids[0];
            let child_entry = self.entries.get(&child)?;
            if child_entry.kind != EntryKind::Dir {
                break;
            }

            // Only fold if the intermediate dir is expanded
            if !self.is_expanded_id(current) {
                break;
            }

            current = child;
            depth += 1;
        }

        if names.len() > 1 {
            Some((current, names.join("/"), depth))
        } else {
            None
        }
    }

    // ── Git status ───────────────────────────────────────────────

    fn update_git_status(&mut self, cx: &gpui::Context<Self>) {
        self.git_status_map.clear();
        self.git_dirty_dirs.clear();

        let Some(ref gs) = self.git_store else {
            return;
        };
        let store = gs.read(cx);
        for entry in store.entries() {
            self.git_status_map.insert(entry.path.clone(), entry.status);
            // Propagate dirty to ancestor dirs
            let p = Path::new(&entry.path);
            let mut ancestor = p.parent();
            while let Some(dir) = ancestor {
                let dir_str = dir.to_string_lossy().to_string();
                if dir_str.is_empty() {
                    break;
                }
                self.git_dirty_dirs.insert(dir_str);
                ancestor = dir.parent();
            }
        }

        // Update git_status on existing entries
        let status_map = self.git_status_map.clone();
        let root_path = self.root_path.clone();
        for entry in self.entries.values_mut() {
            let rel = if let Some(ref root) = root_path {
                entry.abs_path.strip_prefix(root).map_or_else(
                    |_| entry.abs_path.to_string_lossy().to_string(),
                    |p| p.to_string_lossy().to_string(),
                )
            } else {
                entry.abs_path.to_string_lossy().to_string()
            };
            entry.git_status = status_map.get(&rel).copied();
        }
    }

    fn rel_path_for_entry(&self, entry: &Entry) -> String {
        self.rel_path_str(&entry.abs_path)
    }

    fn rel_path_str(&self, abs_path: &Path) -> String {
        if let Some(ref root) = self.root_path {
            abs_path.strip_prefix(root).map_or_else(
                |_| abs_path.to_string_lossy().to_string(),
                |p| p.to_string_lossy().to_string(),
            )
        } else {
            abs_path.to_string_lossy().to_string()
        }
    }

    // ── FS watcher ───────────────────────────────────────────────

    fn startfs_watcher(&mut self, cx: &mut gpui::Context<Self>) {
        self.fs_watcher = None;

        let Some(ref root) = self.root_path else {
            return;
        };
        let root_path = PathBuf::from(root);

        let (stop_tx, stop_rx) = flume::bounded::<()>(1);
        let (notify_tx, notify_rx) = flume::unbounded::<()>();

        // Run blocking FS watcher loop on a dedicated OS thread (NOT the main thread)
        std::thread::spawn(move || {
            use notify::{RecursiveMode, Watcher};

            let (tx, rx) = flume::unbounded();

            let Ok(mut watcher) = notify::recommended_watcher(move |_res| {
                let _ = tx.send(());
            }) else {
                return;
            };

            if watcher.watch(&root_path, RecursiveMode::Recursive).is_err() {
                return;
            }

            loop {
                match rx.recv_timeout(std::time::Duration::from_millis(200)) {
                    Ok(()) => {}
                    Err(flume::RecvTimeoutError::Timeout) => {
                        match stop_rx.try_recv() {
                            Ok(()) | Err(flume::TryRecvError::Disconnected) => break,
                            Err(flume::TryRecvError::Empty) => {}
                        }
                        continue;
                    }
                    Err(flume::RecvTimeoutError::Disconnected) => break,
                }

                match stop_rx.try_recv() {
                    Ok(()) | Err(flume::TryRecvError::Disconnected) => break,
                    Err(flume::TryRecvError::Empty) => {}
                }

                // Debounce: drain pending events, wait 100ms
                std::thread::sleep(std::time::Duration::from_millis(100));
                while rx.try_recv().is_ok() {}

                if notify_tx.send(()).is_err() {
                    break;
                }
            }
        });

        // Foreground async task: await notifications non-blockingly and update tree
        cx.spawn(async move |this, cx| {
            while notify_rx.recv_async().await.is_ok() {
                let alive = cx.update(|cx| {
                    if let Some(entity) = this.upgrade() {
                        entity.update(cx, |tree, cx| {
                            // Skip rebuild during active rename — new entry IDs
                            // would invalidate edit_state, causing the input to
                            // vanish and blur-confirm prematurely. The tree will
                            // refresh when the rename finishes.
                            if tree.edit_state.is_some() {
                                return;
                            }
                            tree.rebuild_entries();
                            tree.update_git_status(cx);
                            tree.recompute_visible();
                            cx.notify();
                        });
                        true
                    } else {
                        false
                    }
                });
                if !alive {
                    break;
                }
            }
        })
        .detach();

        self.fs_watcher = Some(FsWatcherHandle { stop_tx });
    }

    // ── Helpers ──────────────────────────────────────────────────

    fn entry_path(&self, id: EntryId) -> Option<String> {
        self.entries
            .get(&id)
            .map(|e| e.abs_path.to_string_lossy().to_string())
    }

    fn entry_abs_path(&self, id: EntryId) -> Option<PathBuf> {
        self.entries.get(&id).map(|e| e.abs_path.clone())
    }

    fn find_entry_id_by_path(&self, path: &str) -> Option<EntryId> {
        let path = Path::new(path);
        self.entries
            .values()
            .find(|e| e.abs_path == path)
            .map(|e| e.id)
    }

    fn selected_paths(&self) -> Vec<String> {
        if !self.marked_entries.is_empty() {
            self.marked_entries
                .iter()
                .filter_map(|id| self.entry_path(*id))
                .collect()
        } else if let Some(id) = self.selected {
            self.entry_path(id).into_iter().collect()
        } else {
            Vec::new()
        }
    }

    fn target_dir(&self) -> Option<String> {
        if let Some(id) = self.selected {
            if let Some(entry) = self.entries.get(&id) {
                if entry.kind == EntryKind::Dir {
                    return Some(entry.abs_path.to_string_lossy().to_string());
                }
                return entry
                    .abs_path
                    .parent()
                    .map(|p| p.to_string_lossy().to_string());
            }
        }
        self.root_path.clone()
    }

    fn unique_path(base: &Path) -> PathBuf {
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

    fn toggle_dir(&mut self, id: EntryId, cx: &mut gpui::Context<Self>) {
        let Some(path) = self.entry_abs_str(id) else {
            return;
        };
        if self.expanded.contains(&path) {
            self.expanded.remove(&path);
        } else {
            self.expanded.insert(path);
        }
        self.rebuild_entries();
        self.update_git_status(cx);
        self.recompute_visible();
        cx.notify();
    }

    fn select_file(&mut self, id: EntryId, cx: &mut gpui::Context<Self>) {
        self.selected = Some(id);
        self.marked_entries.clear();
        if let Some(path) = self.entry_path(id) {
            cx.emit(FileTreeEvent::OpenFile(path));
        }
        cx.notify();
    }

    #[allow(dead_code)]
    fn expand_parent_dirs(&mut self, path: &Path) {
        if let Some(ref root) = self.root_path {
            let mut current = path.parent();
            while let Some(dir) = current {
                let dir_str = dir.to_string_lossy().to_string();
                if dir_str == root.as_str() {
                    break;
                }
                self.expanded.insert(dir_str);
                current = dir.parent();
            }
        }
    }

    fn refresh_tree(&mut self, cx: &mut gpui::Context<Self>) {
        self.rebuild_entries();
        self.update_git_status(cx);
        self.recompute_visible();
        cx.notify();
    }

    // ── Edit state ───────────────────────────────────────────────

    fn start_rename(
        &mut self,
        id: EntryId,
        is_new: bool,
        window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) {
        let Some(entry) = self.entries.get(&id) else {
            return;
        };
        let is_dir = entry.kind == EntryKind::Dir;
        let initial_name = entry.name.clone();

        self.edit_state = Some(EditState {
            entry_id: id,
            is_new,
            is_dir,
            processing: false,
        });

        let state = cx.new(|cx| InputState::new(window, cx).default_value(&initial_name));
        state.update(cx, |s, icx| {
            s.focus(window, icx);
        });

        cx.subscribe(&state, |this, _state, event, cx| match event {
            InputEvent::PressEnter { .. } => {
                this.confirm_rename(cx);
            }
            InputEvent::Blur => {
                // Click-outside confirms rename (matches VSCode/Zed behavior).
                // Escape fires CancelRename action which clears edit_state first,
                // so this guard prevents double-action.
                if this.edit_state.is_some() {
                    this.confirm_rename(cx);
                }
            }
            _ => {}
        })
        .detach();

        self.rename_input = Some(state);
        cx.notify();
    }

    fn confirm_rename(&mut self, cx: &mut gpui::Context<Self>) {
        let Some(ref edit) = self.edit_state.clone() else {
            return;
        };
        if edit.processing {
            return;
        }
        let new_name = self
            .rename_input
            .as_ref()
            .map(|s| s.read(cx).value().to_string())
            .unwrap_or_default()
            .trim()
            .to_owned();
        if !is_valid_rename(new_name.as_str()) {
            self.cancel_rename(cx);
            return;
        }
        let Some(old_path) = self.entry_abs_path(edit.entry_id) else {
            self.cancel_rename(cx);
            return;
        };
        let is_new_file = edit.is_new && !edit.is_dir;

        if let Some(parent) = old_path.parent() {
            let new_path = parent.join(&new_name);
            // Safety: ensure rename stays within project root
            if let Some(ref root) = self.root_path {
                if !rename_stays_within_root(&new_path, Path::new(root)) {
                    self.cancel_rename(cx);
                    return;
                }
            }
            if new_path != old_path && !new_path.exists() {
                let _ = std::fs::rename(&old_path, &new_path);
                let new_path_str = new_path.to_string_lossy().to_string();
                self.edit_state = None;
                self.rename_input = None;
                self.refresh_tree(cx);
                if let Some(new_id) = self.find_entry_id_by_path(&new_path_str) {
                    self.selected = Some(new_id);
                }
                if is_new_file {
                    cx.emit(FileTreeEvent::OpenFile(new_path_str));
                }
                return;
            }
        }

        // Name unchanged or target exists — keep original, still open new files
        let open_path = old_path.to_string_lossy().to_string();
        self.edit_state = None;
        self.rename_input = None;
        self.refresh_tree(cx);
        if is_new_file {
            cx.emit(FileTreeEvent::OpenFile(open_path));
        }
    }

    fn cancel_rename(&mut self, cx: &mut gpui::Context<Self>) {
        // Keep the file/folder with its default name — don't delete, don't open
        self.edit_state = None;
        self.rename_input = None;
        self.refresh_tree(cx);
    }

    // ── Action handlers ──────────────────────────────────────────

    fn on_new_file(
        &mut self,
        _: &NewFile,
        window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) {
        let Some(dir) = self.target_dir() else { return };
        let base = Path::new(&dir).join("untitled");
        let dest = Self::unique_path(&base);
        if std::fs::write(&dest, "").is_ok() {
            self.expanded.insert(dir.clone());
            self.refresh_tree(cx);
            let path_str = dest.to_string_lossy().to_string();
            if let Some(new_id) = self.find_entry_id_by_path(&path_str) {
                self.selected = Some(new_id);
                self.start_rename(new_id, true, window, cx);
            }
        }
    }

    fn on_new_folder(
        &mut self,
        _: &NewFolder,
        window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) {
        let Some(dir) = self.target_dir() else { return };
        let base = Path::new(&dir).join("new-folder");
        let dest = Self::unique_path(&base);
        if std::fs::create_dir_all(&dest).is_ok() {
            self.expanded.insert(dir.clone());
            let dest_str = dest.to_string_lossy().to_string();
            self.expanded.insert(dest_str.clone());
            self.refresh_tree(cx);
            if let Some(new_id) = self.find_entry_id_by_path(&dest_str) {
                self.selected = Some(new_id);
                self.start_rename(new_id, true, window, cx);
            }
        }
    }

    fn on_reveal_in_finder(
        &mut self,
        _: &RevealInFinder,
        _w: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) {
        let path = self
            .selected
            .and_then(|id| self.entry_path(id))
            .or_else(|| self.root_path.clone());
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
        let dir = self.selected.and_then(|id| {
            let entry = self.entries.get(&id)?;
            if entry.kind == EntryKind::Dir {
                Some(entry.abs_path.to_string_lossy().to_string())
            } else {
                entry
                    .abs_path
                    .parent()
                    .map(|p| p.to_string_lossy().to_string())
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
        if let Some(id) = self.selected {
            self.clipboard = Some(ClipboardOp::Cut(id));
        }
        self.context_menu = None;
        cx.notify();
    }

    fn on_copy(&mut self, _: &FileCopy, _w: &mut gpui::Window, cx: &mut gpui::Context<Self>) {
        if let Some(id) = self.selected {
            self.clipboard = Some(ClipboardOp::Copied(id));
        }
        self.context_menu = None;
        cx.notify();
    }

    fn on_paste(&mut self, _: &FilePaste, _w: &mut gpui::Window, cx: &mut gpui::Context<Self>) {
        let Some(ref clip) = self.clipboard.clone() else {
            return;
        };
        let Some(dir) = self.target_dir() else { return };

        let (src_id, is_cut) = match clip {
            ClipboardOp::Cut(id) => (*id, true),
            ClipboardOp::Copied(id) => (*id, false),
        };
        let Some(src_path) = self.entry_abs_path(src_id) else {
            return;
        };
        let file_name = src_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("file");
        let dest_base = Path::new(&dir).join(file_name);

        if is_cut {
            let dest = Self::unique_path(&dest_base);
            let _ = std::fs::rename(&src_path, &dest);
            self.clipboard = None;
        } else {
            let dest = Self::unique_path(&dest_base);
            if src_path.is_dir() {
                copy_dir_recursive(&src_path, &dest);
            } else {
                let _ = std::fs::copy(&src_path, &dest);
            }
        }
        self.expanded.insert(dir.clone());
        self.context_menu = None;
        self.refresh_tree(cx);
    }

    fn on_duplicate(
        &mut self,
        _: &FileDuplicate,
        _w: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) {
        let Some(id) = self.selected else { return };
        let Some(src) = self.entry_abs_path(id) else {
            return;
        };
        let parent = src.parent().unwrap_or(&src);
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
            copy_dir_recursive(&src, &dest);
        } else {
            let _ = std::fs::copy(&src, &dest);
        }
        self.context_menu = None;
        self.refresh_tree(cx);
        let dest_str = dest.to_string_lossy().to_string();
        if let Some(new_id) = self.find_entry_id_by_path(&dest_str) {
            self.selected = Some(new_id);
        }
    }

    fn on_copy_path(&mut self, _: &CopyPath, _w: &mut gpui::Window, cx: &mut gpui::Context<Self>) {
        if let Some(path) = self.selected.and_then(|id| self.entry_path(id)) {
            cx.write_to_clipboard(ClipboardItem::new_string(path));
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
        if let Some(id) = self.selected {
            if let Some(entry) = self.entries.get(&id) {
                let rel = self.rel_path_for_entry(entry);
                cx.write_to_clipboard(ClipboardItem::new_string(rel));
            }
        }
        self.context_menu = None;
        cx.notify();
    }

    fn on_rename(
        &mut self,
        _: &FileRename,
        window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) {
        if let Some(id) = self.selected {
            self.start_rename(id, false, window, cx);
        }
        self.context_menu = None;
        cx.notify();
    }

    fn on_trash(&mut self, _: &FileTrash, _w: &mut gpui::Window, cx: &mut gpui::Context<Self>) {
        let paths = self.selected_paths();
        if paths.is_empty() {
            return;
        }
        self.selected = None;
        self.marked_entries.clear();
        self.context_menu = None;
        cx.notify();

        cx.spawn(async move |this, cx| {
            for path in &paths {
                #[cfg(target_os = "macos")]
                {
                    let _ = std::process::Command::new("osascript")
                        .arg("-e")
                        .arg(format!(
                            "tell application \"Finder\" to delete POSIX file \"{}\"",
                            path.replace('"', "\\\"")
                        ))
                        .output();
                }
                #[cfg(not(target_os = "macos"))]
                {
                    let _ = path; // suppress unused warning
                }
            }
            cx.update(|cx| {
                if let Some(entity) = this.upgrade() {
                    entity.update(cx, Self::refresh_tree);
                }
            });
        })
        .detach();
    }

    fn on_delete(&mut self, _: &FileDelete, _w: &mut gpui::Window, cx: &mut gpui::Context<Self>) {
        let paths = self.selected_paths();
        if paths.is_empty() {
            return;
        }
        self.selected = None;
        self.marked_entries.clear();
        self.context_menu = None;
        cx.notify();

        cx.spawn(async move |this, cx| {
            for path in &paths {
                let p = Path::new(path);
                if p.is_dir() {
                    let _ = std::fs::remove_dir_all(p);
                } else {
                    let _ = std::fs::remove_file(p);
                }
            }
            cx.update(|cx| {
                if let Some(entity) = this.upgrade() {
                    entity.update(cx, Self::refresh_tree);
                }
            });
        })
        .detach();
    }

    fn on_cancel_rename(
        &mut self,
        _: &CancelRename,
        _w: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) {
        if self.edit_state.is_some() {
            self.cancel_rename(cx);
        }
    }

    // ── Click handling with multi-select ─────────────────────────

    fn handle_click(
        &mut self,
        visible_idx: usize,
        cmd: bool,
        shift: bool,
        cx: &mut gpui::Context<Self>,
    ) {
        let Some(ve) = self.visible_entries.get(visible_idx) else {
            return;
        };
        let entry_id = ve.entry_id;
        let Some(entry) = self.entries.get(&entry_id) else {
            return;
        };

        if entry.kind == EntryKind::Dir {
            // Dirs: toggle expand, don't multi-select
            self.selected = Some(entry_id);
            self.marked_entries.clear();
            self.toggle_dir(entry_id, cx);
            self.last_clicked_index = Some(visible_idx);
            return;
        }

        if cmd {
            // Cmd+click: toggle in marked set
            if self.marked_entries.contains(&entry_id) {
                self.marked_entries.remove(&entry_id);
            } else {
                self.marked_entries.insert(entry_id);
            }
            self.selected = Some(entry_id);
        } else if shift {
            // Shift+click: range select
            if let Some(last) = self.last_clicked_index {
                let (start, end) = if last < visible_idx {
                    (last, visible_idx)
                } else {
                    (visible_idx, last)
                };
                self.marked_entries.clear();
                for i in start..=end {
                    if let Some(ve) = self.visible_entries.get(i) {
                        if let Some(e) = self.entries.get(&ve.entry_id) {
                            if e.kind == EntryKind::File {
                                self.marked_entries.insert(ve.entry_id);
                            }
                        }
                    }
                }
            }
            self.selected = Some(entry_id);
        } else {
            // Plain click: single select
            self.marked_entries.clear();
            self.select_file(entry_id, cx);
        }
        self.last_clicked_index = Some(visible_idx);
        cx.notify();
    }

    fn on_key_nav(
        &mut self,
        ev: &gpui::KeyDownEvent,
        _window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) {
        // During rename, suppress tree key nav (CancelRename action handles Escape)
        if self.edit_state.is_some() {
            return;
        }

        let key = ev.keystroke.key.as_ref();
        match key {
            "up" => {
                self.move_selection(-1, cx);
            }
            "down" => {
                self.move_selection(1, cx);
            }
            "right" => {
                // Expand dir or no-op on file
                if let Some(id) = self.selected {
                    if let Some(entry) = self.entries.get(&id) {
                        if entry.kind == EntryKind::Dir && !self.is_expanded_id(id) {
                            self.toggle_dir(id, cx);
                        }
                    }
                }
            }
            "left" => {
                // Collapse dir or select parent
                if let Some(id) = self.selected {
                    if let Some(entry) = self.entries.get(&id) {
                        if entry.kind == EntryKind::Dir && self.is_expanded_id(id) {
                            self.toggle_dir(id, cx);
                        } else {
                            // Select parent dir
                            self.select_parent(id, cx);
                        }
                    }
                }
            }
            "enter" | "return" => {
                if let Some(id) = self.selected {
                    if let Some(entry) = self.entries.get(&id) {
                        if entry.kind == EntryKind::Dir {
                            self.toggle_dir(id, cx);
                        } else {
                            self.select_file(id, cx);
                        }
                    }
                }
            }
            "space" => {
                if let Some(id) = self.selected {
                    if let Some(entry) = self.entries.get(&id) {
                        if entry.kind == EntryKind::Dir {
                            self.toggle_dir(id, cx);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn move_selection(&mut self, delta: i32, cx: &mut gpui::Context<Self>) {
        if self.visible_entries.is_empty() {
            return;
        }
        let current_idx = self
            .selected
            .and_then(|id| self.visible_entries.iter().position(|ve| ve.entry_id == id))
            .unwrap_or(0);

        #[allow(clippy::cast_possible_wrap, clippy::cast_sign_loss)]
        let new_idx = (current_idx as i32 + delta)
            .max(0)
            .min(self.visible_entries.len() as i32 - 1) as usize;

        let new_id = self.visible_entries[new_idx].entry_id;
        self.selected = Some(new_id);
        self.marked_entries.clear();
        self.last_clicked_index = Some(new_idx);
        self.scroll_handle
            .scroll_to_item(new_idx, gpui::ScrollStrategy::Top);
        cx.notify();
    }

    fn select_parent(&mut self, id: EntryId, cx: &mut gpui::Context<Self>) {
        let Some(entry) = self.entries.get(&id) else {
            return;
        };
        if let Some(parent_path) = entry.abs_path.parent() {
            let parent_str = parent_path.to_string_lossy().to_string();
            // Don't go above root
            if self.root_path.as_deref() == Some(&parent_str) {
                return;
            }
            if let Some(parent_id) = self.find_entry_id_by_path(&parent_str) {
                self.selected = Some(parent_id);
                self.marked_entries.clear();
                cx.notify();
            }
        }
    }
}

// ── Render ───────────────────────────────────────────────────────

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
        // Only call set_root if path changed (set_root checks internally too)
        if root != self.root_path {
            self.set_root(root, cx);
        }

        let theme = self.app_state.read(cx).theme().clone();

        if self.visible_entries.is_empty() {
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

        let item_count = self.visible_entries.len();
        let has_clipboard = self.clipboard.is_some();

        let list = uniform_list(
            "file-tree-list",
            item_count,
            cx.processor(move |this, range: std::ops::Range<usize>, _window, cx| {
                let theme = this.app_state.read(cx).theme().clone();
                let context_menu_open = this.context_menu.is_some();
                let edit_entry_id = this.edit_state.as_ref().map(|e| e.entry_id);
                let rename_input = this.rename_input.clone();

                range
                    .map(|ix| {
                        let ve = &this.visible_entries[ix];
                        if edit_entry_id == Some(ve.entry_id) {
                            if let Some(ref input_state) = rename_input {
                                render_rename_row(ve, input_state, &theme)
                            } else {
                                render_row(this, ve, ix, context_menu_open, &theme, cx)
                                    .into_any_element()
                            }
                        } else {
                            render_row(this, ve, ix, context_menu_open, &theme, cx)
                                .into_any_element()
                        }
                    })
                    .collect()
            }),
        )
        .with_sizing_behavior(ListSizingBehavior::Infer)
        .track_scroll(&self.scroll_handle);

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
            .on_action(cx.listener(Self::on_cancel_rename))
            .on_key_down(cx.listener(Self::on_key_nav))
            .child(list);

        // Context menu
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
                            cx.stop_propagation();
                            this.context_menu = None;
                            cx.notify();
                        }),
                    )
                    .on_mouse_down(
                        MouseButton::Right,
                        cx.listener(|this, _, _, cx| {
                            cx.stop_propagation();
                            this.context_menu = None;
                            cx.notify();
                        }),
                    )
                    .child(render_context_menu(menu, has_clipboard, &theme, cx)),
            );
        }

        root.into_any_element()
    }
}

// ── Row rendering ────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
fn render_row(
    tree: &FileTree,
    ve: &VisibleEntry,
    ix: usize,
    context_menu_open: bool,
    theme: &Theme,
    cx: &mut gpui::Context<FileTree>,
) -> gpui::AnyElement {
    let Some(entry) = tree.entries.get(&ve.entry_id) else {
        return div().into_any_element();
    };

    #[allow(clippy::cast_precision_loss)]
    let indent = px(ve.depth as f32 * 16.0 + 8.0);
    let entry_id = ve.entry_id;
    let is_dir = entry.kind == EntryKind::Dir;
    let is_selected = tree.selected == Some(entry_id);
    let is_marked = tree.marked_entries.contains(&entry_id);

    let display_name = ve.folded_name.clone().unwrap_or_else(|| entry.name.clone());

    let icon_path = if is_dir {
        crate::icons::icon_for_dir(tree.is_expanded_id(entry_id))
    } else {
        crate::icons::icon_for_file(&entry.name)
    };

    // Git status coloring
    let fg = entry_color(
        entry,
        is_dir,
        &tree.git_dirty_dirs,
        &tree.rel_path_for_entry(entry),
        theme,
    );
    let icon_color: Hsla = if let Some(status) = entry.git_status {
        git_status_color(status, theme).into()
    } else if is_dir
        && tree
            .git_dirty_dirs
            .contains(&tree.rel_path_for_entry(entry))
    {
        theme.vcs_modified.into()
    } else if is_dir {
        theme.text_muted.into()
    } else {
        file_ext_color(&entry.name).into()
    };

    let bg = if is_selected || is_marked {
        theme.element_selected
    } else {
        rgba(0x0000_0000)
    };
    let surface = theme.surface;

    let row = div()
        .id(ix)
        .pl(indent)
        .pr(px(8.))
        .h(px(ROW_HEIGHT))
        .flex()
        .items_center()
        .text_size(px(13.))
        .text_color(fg)
        .bg(bg)
        .cursor_pointer();

    let row = if context_menu_open {
        row
    } else {
        row.hover(move |s| s.bg(surface))
    };

    row.on_mouse_down(
        MouseButton::Left,
        cx.listener(move |this, event: &gpui::MouseDownEvent, _window, cx| {
            let cmd = event.modifiers.platform;
            let shift = event.modifiers.shift;
            this.handle_click(ix, cmd, shift, cx);
        }),
    )
    .on_mouse_down(
        MouseButton::Right,
        cx.listener(move |this, event: &gpui::MouseDownEvent, _, cx| {
            this.selected = Some(entry_id);
            let root_origin = this.root_handle.bounds().origin;
            this.context_menu = Some(ContextMenuState {
                entry_id,
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
            .child(display_name),
    )
    .into_any_element()
}

fn render_rename_row(
    ve: &VisibleEntry,
    input_state: &Entity<InputState>,
    theme: &Theme,
) -> gpui::AnyElement {
    #[allow(clippy::cast_precision_loss)]
    let indent = px(ve.depth as f32 * 16.0 + 8.0);

    div()
        .pl(indent)
        .pr(px(8.))
        .h(px(ROW_HEIGHT))
        .w_full()
        .flex()
        .items_center()
        .text_size(px(13.))
        .bg(theme.element_selected)
        .child(
            div()
                .flex_1()
                .min_w_0()
                .child(Input::new(input_state).appearance(true).text_size(px(12.))),
        )
        .into_any_element()
}

// ── Entry coloring ───────────────────────────────────────────────

fn entry_color(
    entry: &Entry,
    is_dir: bool,
    dirty_dirs: &HashSet<String>,
    rel_path: &str,
    theme: &Theme,
) -> gpui::Rgba {
    if let Some(status) = entry.git_status {
        return git_status_color(status, theme);
    }
    if is_dir && dirty_dirs.contains(rel_path) {
        return theme.vcs_modified;
    }
    if is_dir { theme.text_muted } else { theme.text }
}

fn git_status_color(status: FileStatus, theme: &Theme) -> gpui::Rgba {
    match status {
        FileStatus::Modified | FileStatus::Renamed => theme.vcs_modified,
        FileStatus::Added | FileStatus::Untracked => theme.vcs_added,
        FileStatus::Deleted => theme.vcs_deleted,
    }
}

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

    let entry_id = menu.entry_id;

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

    items.push(ctx_separator(theme));

    // Group 2: Reveal / Open in Terminal
    items.push(
        ctx_item(
            "Reveal in Finder",
            "\u{2325}\u{2318}R",
            theme,
            cx.listener(move |this, _, _, cx| {
                if let Some(path) = this.entry_path(entry_id) {
                    let _ = std::process::Command::new("open")
                        .arg("-R")
                        .arg(&path)
                        .spawn();
                }
                this.context_menu = None;
                cx.notify();
            }),
        )
        .into_any_element(),
    );
    items.push(
        ctx_item(
            "Open in Terminal",
            "",
            theme,
            cx.listener(move |this, _, _, cx| {
                let dir = this.entries.get(&entry_id).and_then(|e| {
                    if e.kind == EntryKind::Dir {
                        Some(e.abs_path.to_string_lossy().to_string())
                    } else {
                        e.abs_path.parent().map(|p| p.to_string_lossy().to_string())
                    }
                });
                if let Some(d) = dir {
                    let _ = std::process::Command::new("open")
                        .arg("-a")
                        .arg("Terminal")
                        .arg(&d)
                        .spawn();
                }
                this.context_menu = None;
                cx.notify();
            }),
        )
        .into_any_element(),
    );

    items.push(ctx_separator(theme));

    // Group 3: Cut / Copy / Duplicate / Paste
    items.push(
        ctx_item(
            "Cut",
            "\u{2318}X",
            theme,
            cx.listener(move |this, _, _, cx| {
                this.clipboard = Some(ClipboardOp::Cut(entry_id));
                this.context_menu = None;
                cx.notify();
            }),
        )
        .into_any_element(),
    );
    items.push(
        ctx_item(
            "Copy",
            "\u{2318}C",
            theme,
            cx.listener(move |this, _, _, cx| {
                this.clipboard = Some(ClipboardOp::Copied(entry_id));
                this.context_menu = None;
                cx.notify();
            }),
        )
        .into_any_element(),
    );
    items.push(
        ctx_item(
            "Duplicate",
            "\u{2318}D",
            theme,
            cx.listener(move |this, _, window, cx| {
                this.selected = Some(entry_id);
                this.context_menu = None;
                this.on_duplicate(&FileDuplicate, window, cx);
            }),
        )
        .into_any_element(),
    );
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

    items.push(ctx_separator(theme));

    // Group 4: Copy Path / Copy Relative Path
    items.push(
        ctx_item(
            "Copy Path",
            "\u{2325}\u{2318}C",
            theme,
            cx.listener(move |this, _, _, cx| {
                if let Some(path) = this.entry_path(entry_id) {
                    cx.write_to_clipboard(ClipboardItem::new_string(path));
                }
                this.context_menu = None;
                cx.notify();
            }),
        )
        .into_any_element(),
    );
    items.push(
        ctx_item(
            "Copy Relative Path",
            "\u{2325}\u{2318}\u{21e7}C",
            theme,
            cx.listener(move |this, _, _, cx| {
                if let Some(entry) = this.entries.get(&entry_id) {
                    let rel = this.rel_path_for_entry(entry);
                    cx.write_to_clipboard(ClipboardItem::new_string(rel));
                }
                this.context_menu = None;
                cx.notify();
            }),
        )
        .into_any_element(),
    );

    items.push(ctx_separator(theme));

    // Group 5: Rename / Trash / Delete
    items.push(
        ctx_item(
            "Rename",
            "F2",
            theme,
            cx.listener(move |this, _, window, cx| {
                this.context_menu = None;
                this.start_rename(entry_id, false, window, cx);
            }),
        )
        .into_any_element(),
    );
    items.push(
        ctx_item(
            "Trash",
            "\u{232b}",
            theme,
            cx.listener(move |this, _, window, cx| {
                this.selected = Some(entry_id);
                this.on_trash(&FileTrash, window, cx);
            }),
        )
        .into_any_element(),
    );
    items.push(
        ctx_item(
            "Delete",
            "\u{2325}\u{2318}\u{232b}",
            theme,
            cx.listener(move |this, _, window, cx| {
                this.selected = Some(entry_id);
                this.on_delete(&FileDelete, window, cx);
            }),
        )
        .into_any_element(),
    );

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
    handler: impl Fn(&gpui::MouseDownEvent, &mut gpui::Window, &mut gpui::App) + 'static,
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
        .on_mouse_down(MouseButton::Left, move |ev, win, cx| {
            cx.stop_propagation();
            handler(ev, win, cx);
        })
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

// ── Utilities ────────────────────────────────────────────────────

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

/// Validate a rename target name. Rejects empty, path separators, and traversal.
fn is_valid_rename(name: &str) -> bool {
    !name.is_empty() && !name.contains('/') && !name.contains('\0') && name != "." && name != ".."
}

/// Check that `new_path` stays within `root` (no path traversal).
/// Resolves `..` components without requiring paths to exist on disk.
fn rename_stays_within_root(new_path: &Path, root: &Path) -> bool {
    // Try filesystem canonicalization first (most accurate)
    if let (Ok(resolved), Ok(root_resolved)) = (new_path.canonicalize(), root.canonicalize()) {
        return resolved.starts_with(&root_resolved);
    }
    // Fallback: manually resolve `.` and `..` components
    let normalize = |p: &Path| -> PathBuf {
        let mut parts = Vec::new();
        for c in p.components() {
            match c {
                std::path::Component::ParentDir => {
                    parts.pop();
                }
                std::path::Component::CurDir => {}
                other => parts.push(other),
            }
        }
        parts.iter().collect()
    };
    normalize(new_path).starts_with(normalize(root))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    // ── Name validation ─────────────────────────────────────────

    #[test]
    fn valid_rename_names() {
        assert!(is_valid_rename("hello.txt"));
        assert!(is_valid_rename("my-file"));
        assert!(is_valid_rename(".gitignore")); // hidden files OK
        assert!(is_valid_rename("file..txt")); // double dot in name OK
        assert!(is_valid_rename("..."));
        assert!(is_valid_rename("a"));
    }

    #[test]
    fn invalid_rename_names() {
        assert!(!is_valid_rename(""));
        assert!(!is_valid_rename("."));
        assert!(!is_valid_rename(".."));
        assert!(!is_valid_rename("foo/bar")); // path separator
        assert!(!is_valid_rename("../escape")); // traversal with slash
        assert!(!is_valid_rename("foo\0bar")); // null byte
    }

    // ── Path traversal prevention ───────────────────────────────

    #[test]
    fn rename_path_stays_within_root() {
        let root = Path::new("/tmp/project");
        let good = Path::new("/tmp/project/src/file.rs");
        let bad = Path::new("/tmp/other/file.rs");
        let traversal = Path::new("/tmp/project/../other/file.rs");

        assert!(rename_stays_within_root(good, root));
        assert!(!rename_stays_within_root(bad, root));
        // Raw prefix check catches obvious traversal even without canonicalize
        assert!(!rename_stays_within_root(traversal, root));
    }

    // ── unique_path ─────────────────────────────────────────────

    #[test]
    fn unique_path_returns_original_if_not_exists() {
        let tmp = std::env::temp_dir().join("conescope_test_unique_not_exists");
        let _ = std::fs::remove_file(&tmp);
        assert_eq!(FileTree::unique_path(&tmp), tmp);
    }

    #[test]
    fn unique_path_increments_when_exists() {
        let tmp = std::env::temp_dir().join("conescope_test_unique_exists");
        std::fs::write(&tmp, "").unwrap();
        let result = FileTree::unique_path(&tmp);
        assert_ne!(result, tmp);
        assert!(result.to_string_lossy().contains("-1"));
        let _ = std::fs::remove_file(&tmp);
    }

    // ── Data isolation: rename must not cross project boundaries ─

    #[test]
    fn rename_cannot_escape_project_root() {
        // Simulates a rename that would place a file outside the project
        let root = Path::new("/projects/myapp");
        let escaped = Path::new("/projects/other-app/stolen.rs");
        assert!(
            !rename_stays_within_root(escaped, root),
            "rename must not allow files outside project root"
        );
    }
}
