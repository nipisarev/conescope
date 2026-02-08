/// Icon asset path for a file based on its extension.
#[must_use]
pub fn icon_for_file(name: &str) -> &'static str {
    let ext = name.rsplit('.').next().unwrap_or("");
    match ext {
        "rs" => "icons/file-rs-thin.svg",
        "js" => "icons/file-js-thin.svg",
        "ts" => "icons/file-ts-thin.svg",
        "jsx" => "icons/file-jsx-thin.svg",
        "tsx" => "icons/file-tsx-thin.svg",
        "py" => "icons/file-py-thin.svg",
        "c" => "icons/file-c-thin.svg",
        "cpp" | "cc" => "icons/file-cpp-thin.svg",
        "cs" => "icons/file-c-sharp-thin.svg",
        "html" | "htm" => "icons/file-html-thin.svg",
        "css" | "scss" | "less" => "icons/file-css-thin.svg",
        "sql" => "icons/file-sql-thin.svg",
        "vue" => "icons/file-vue-thin.svg",
        "md" => "icons/file-md-thin.svg",
        "png" | "jpg" | "jpeg" | "gif" | "svg" | "ico" | "webp" => "icons/image-thin.svg",
        "mp4" | "mov" | "avi" | "mkv" => "icons/video-camera-thin.svg",
        "mp3" | "wav" | "flac" | "ogg" => "icons/music-notes-thin.svg",
        "sh" | "bash" | "zsh" | "fish" | "bat" => "icons/file-code-thin.svg",
        "txt" | "log" => "icons/file-text-thin.svg",
        "toml" | "yaml" | "yml" | "json" | "json5" | "env" | "ini" | "cfg" => "icons/gear-thin.svg",
        "zip" | "tar" | "gz" | "bz2" | "xz" | "7z" | "rar" => "icons/file-archive-thin.svg",
        _ => "icons/file-thin.svg",
    }
}

/// Icon asset path for a directory (open or closed).
#[must_use]
pub fn icon_for_dir(expanded: bool) -> &'static str {
    if expanded {
        "icons/folder-open-thin.svg"
    } else {
        "icons/folder-simple-thin.svg"
    }
}

// UI icon paths
pub const ICON_GRID: &str = "icons/squares-four-thin.svg";
pub const ICON_CONESCOPE_OUTLINE: &str = "icons/conescope-outline.svg";
pub const ICON_CONESCOPE_SOLID: &str = "icons/conescope-solid.svg";
pub const ICON_SIDEBAR: &str = "icons/sidebar-simple-thin.svg";
pub const ICON_EDITOR: &str = "icons/note-pencil-thin.svg";
pub const ICON_TERMINAL: &str = "icons/terminal-window-thin.svg";
pub const ICON_COMMAND: &str = "icons/command-thin.svg";
pub const ICON_PLUS: &str = "icons/plus-thin.svg";
pub const ICON_CLOSE: &str = "icons/x-thin.svg";
pub const ICON_QUESTION: &str = "icons/question-thin.svg";
pub const ICON_SETTINGS: &str = "icons/gear-thin.svg";
pub const ICON_BACK: &str = "icons/minus-circle-thin.svg";
pub const ICON_CLOSE_CIRCLE: &str = "icons/x-circle-thin.svg";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icon_for_known_extensions() {
        assert_eq!(icon_for_file("main.rs"), "icons/file-rs-thin.svg");
        assert_eq!(icon_for_file("app.tsx"), "icons/file-tsx-thin.svg");
        assert_eq!(icon_for_file("Cargo.toml"), "icons/gear-thin.svg");
        assert_eq!(icon_for_file("photo.png"), "icons/image-thin.svg");
    }

    #[test]
    fn icon_for_unknown_extension_is_default() {
        assert_eq!(icon_for_file("something.xyz"), "icons/file-thin.svg");
        assert_eq!(icon_for_file("noext"), "icons/file-thin.svg");
    }

    #[test]
    fn dir_icons_toggle() {
        assert_eq!(icon_for_dir(false), "icons/folder-simple-thin.svg");
        assert_eq!(icon_for_dir(true), "icons/folder-open-thin.svg");
    }
}
