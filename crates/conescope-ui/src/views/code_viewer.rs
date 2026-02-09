use gpui::prelude::*;
use gpui::{Entity, EventEmitter, div, px, rgba};
use gpui_component::input::{Input, InputEvent, InputState};

const MAX_FILE_SIZE: u64 = 10 * 1_024 * 1_024; // 10 MB (ropey handles large files)

#[derive(Debug, Clone)]
pub enum CodeEditorEvent {
    FileOpened(String),
    FileSavedAs(String),
}

impl EventEmitter<CodeEditorEvent> for CodeEditor {}

pub struct CodeEditor {
    file_path: Option<String>,
    editor_state: Option<Entity<InputState>>,
    /// Pending file to open once the editor state is initialized.
    pending_open: Option<String>,
}

impl std::fmt::Debug for CodeEditor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CodeEditor")
            .field("file_path", &self.file_path)
            .finish_non_exhaustive()
    }
}

impl Default for CodeEditor {
    fn default() -> Self {
        Self::new()
    }
}

impl CodeEditor {
    #[must_use]
    pub fn new() -> Self {
        Self {
            file_path: None,
            editor_state: None,
            pending_open: None,
        }
    }

    /// Lazily create the `InputState` entity on first render (requires `Window`).
    fn ensure_editor(
        &mut self,
        window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> Entity<InputState> {
        if let Some(ref state) = self.editor_state {
            return state.clone();
        }
        let state = cx.new(|cx| {
            InputState::new(window, cx)
                .code_editor("rust")
                .line_number(true)
                .searchable(true)
        });

        cx.subscribe(&state, |this, _state, event, cx| {
            if let InputEvent::Change = event {
                if let Some(ref path) = this.file_path {
                    if conescope_core::settings::SettingsJson::is_scratch_file(
                        std::path::Path::new(path),
                    ) {
                        if let Some(ref editor) = this.editor_state {
                            let content = editor.read(cx).value().to_string();
                            let _ = std::fs::write(path, content);
                        }
                    }
                }
            }
        })
        .detach();

        self.editor_state = Some(state.clone());
        state
    }

    /// Open and display a file. Defers to render if editor not yet initialized.
    pub fn open_file(&mut self, path: &str, cx: &mut gpui::Context<Self>) {
        self.pending_open = Some(path.to_owned());
        cx.notify();
    }

    fn load_file_into_state(
        &mut self,
        path: &str,
        state: &Entity<InputState>,
        window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) {
        if let Ok(metadata) = std::fs::metadata(path) {
            if metadata.len() > MAX_FILE_SIZE {
                self.file_path = Some(path.to_owned());
                cx.emit(CodeEditorEvent::FileOpened(path.to_owned()));
                cx.notify();
                return;
            }
        }

        if let Ok(content) = std::fs::read_to_string(path) {
            let lang = lang_from_path(path);
            state.update(cx, |s, cx| {
                s.set_highlighter(lang, cx);
                s.set_value(&content, window, cx);
            });
            self.file_path = Some(path.to_owned());
            cx.emit(CodeEditorEvent::FileOpened(path.to_owned()));
        } else {
            self.file_path = Some(path.to_owned());
        }
        cx.notify();
    }

    #[must_use]
    pub fn file_path(&self) -> Option<&str> {
        self.file_path.as_deref()
    }

    /// Get the current editor text content.
    #[must_use]
    pub fn value(&self, cx: &gpui::App) -> Option<String> {
        self.editor_state
            .as_ref()
            .map(|s| s.read(cx).value().to_string())
    }

    /// Save the current content to the open file path.
    ///
    /// # Errors
    /// Returns `std::io::Error` if the file cannot be written.
    pub fn save_file(&self, cx: &gpui::App) -> Result<(), std::io::Error> {
        let Some(path) = &self.file_path else {
            return Ok(());
        };
        let Some(content) = self.value(cx) else {
            return Ok(());
        };
        std::fs::write(path, content)
    }

    /// Open a native "Save As" dialog. On success, writes content to the chosen path,
    /// deletes the scratch file, and emits `FileSavedAs`.
    pub fn save_as(&mut self, cx: &mut gpui::Context<Self>) {
        let Some(ref current_path) = self.file_path else {
            return;
        };
        let content = self
            .editor_state
            .as_ref()
            .map(|s| s.read(cx).value().to_string())
            .unwrap_or_default();
        let old_path = current_path.clone();

        cx.spawn(async move |this, cx| {
            let handle = rfd::AsyncFileDialog::new()
                .set_file_name("Untitled")
                .save_file()
                .await;

            if let Some(file) = handle {
                let new_path = file.path().to_string_lossy().to_string();
                // Write content to new location
                if std::fs::write(&new_path, &content).is_ok() {
                    // Delete old scratch file
                    if conescope_core::settings::SettingsJson::is_scratch_file(
                        std::path::Path::new(&old_path),
                    ) {
                        let _ = std::fs::remove_file(&old_path);
                    }
                    cx.update(|cx| {
                        if let Some(editor) = this.upgrade() {
                            editor.update(cx, |editor, cx| {
                                editor.file_path = Some(new_path.clone());
                                cx.emit(CodeEditorEvent::FileSavedAs(new_path));
                                cx.notify();
                            });
                        }
                    });
                }
            }
        })
        .detach();
    }

    pub fn close_file(&mut self, cx: &mut gpui::Context<Self>) {
        self.file_path = None;
        self.pending_open = None;
        cx.notify();
    }
}

/// Map file extension to tree-sitter language name.
fn lang_from_path(path: &str) -> String {
    let ext = path.rsplit('.').next().unwrap_or("");
    match ext {
        "ts" | "tsx" => "typescript",
        "js" | "jsx" => "javascript",
        "py" => "python",
        "go" => "go",
        "rb" => "ruby",
        "c" | "h" => "c",
        "cpp" | "cc" | "cxx" | "hpp" => "cpp",
        "java" => "java",
        "cs" => "c_sharp",
        "swift" => "swift",
        "kt" | "kts" => "kotlin",
        "json" | "json5" => "json",
        "toml" => "toml",
        "yaml" | "yml" => "yaml",
        "html" | "htm" => "html",
        "css" => "css",
        "scss" => "scss",
        "md" | "markdown" => "markdown",
        "sh" | "bash" | "zsh" => "bash",
        "sql" => "sql",
        "lua" => "lua",
        "zig" => "zig",
        "ex" | "exs" => "elixir",
        "erl" | "hrl" => "erlang",
        "hs" => "haskell",
        _ => "rust", // fallback
    }
    .to_owned()
}

impl Render for CodeEditor {
    fn render(
        &mut self,
        window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> impl IntoElement {
        let state = self.ensure_editor(window, cx);

        // Process pending file open (deferred from before editor was initialized)
        if let Some(path) = self.pending_open.take() {
            self.load_file_into_state(&path, &state, window, cx);
        }

        if self.file_path.is_none() {
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

        div()
            .size_full()
            .child(
                Input::new(&state)
                    .h_full()
                    .appearance(false)
                    .focus_bordered(false),
            )
            .into_any_element()
    }
}
