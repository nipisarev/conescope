/// Icon asset path for a file based on its extension.
#[must_use]
pub fn icon_for_file(name: &str) -> &'static str {
    let ext = name.rsplit('.').next().unwrap_or("");
    match ext {
        "rs" => "icons/file-rs.svg",
        "js" => "icons/file-js.svg",
        "ts" => "icons/file-ts.svg",
        "jsx" => "icons/file-jsx.svg",
        "tsx" => "icons/file-tsx.svg",
        "py" => "icons/file-py.svg",
        "c" => "icons/file-c.svg",
        "cpp" | "cc" => "icons/file-cpp.svg",
        "cs" => "icons/file-c-sharp.svg",
        "html" | "htm" => "icons/file-html.svg",
        "css" | "scss" | "less" => "icons/file-css.svg",
        "sql" => "icons/file-sql.svg",
        "vue" => "icons/file-vue.svg",
        "md" => "icons/file-md.svg",
        "png" | "jpg" | "jpeg" | "gif" | "svg" | "ico" | "webp" => "icons/image.svg",
        "mp4" | "mov" | "avi" | "mkv" => "icons/video-camera.svg",
        "mp3" | "wav" | "flac" | "ogg" => "icons/music-notes.svg",
        "sh" | "bash" | "zsh" | "fish" | "bat" => "icons/file-code.svg",
        "txt" | "log" => "icons/file-text.svg",
        "toml" | "yaml" | "yml" | "json" | "json5" | "env" | "ini" | "cfg" => "icons/gear.svg",
        "zip" | "tar" | "gz" | "bz2" | "xz" | "7z" | "rar" => "icons/file-archive.svg",
        _ => "icons/file.svg",
    }
}

/// Icon asset path for a directory (open or closed).
#[must_use]
pub fn icon_for_dir(expanded: bool) -> &'static str {
    if expanded {
        "icons/folder-open.svg"
    } else {
        "icons/folder-simple.svg"
    }
}

// UI icon paths (all regular weight)
pub const ICON_GRID: &str = "icons/squares-four.svg";
pub const ICON_CONESCOPE_OUTLINE: &str = "icons/conescope-outline.svg";
pub const ICON_CONESCOPE_SOLID: &str = "icons/conescope-solid.svg";
pub const ICON_SIDEBAR: &str = "icons/sidebar-simple.svg";
pub const ICON_EDITOR: &str = "icons/note-pencil.svg";
pub const ICON_TERMINAL: &str = "icons/terminal-window.svg";
pub const ICON_COMMAND: &str = "icons/command.svg";
pub const ICON_PLUS: &str = "icons/plus.svg";
pub const ICON_CLOSE: &str = "icons/x.svg";
pub const ICON_QUESTION: &str = "icons/question.svg";
pub const ICON_SETTINGS: &str = "icons/gear.svg";
pub const ICON_BACK: &str = "icons/minus-circle.svg";
pub const ICON_CLOSE_CIRCLE: &str = "icons/x-circle.svg";
pub const ICON_TRASH: &str = "icons/trash.svg";
pub const ICON_ARROW_LEFT: &str = "icons/arrow-left.svg";
pub const ICON_GIT: &str = "icons/git-branch.svg";
pub const ICON_FOLDER_PLUS: &str = "icons/folder-plus.svg";

// Aliases for tile headers (same as above, all regular weight now)
pub const ICON_TERMINAL_WINDOW: &str = ICON_TERMINAL;
pub const ICON_COMMAND_REG: &str = ICON_COMMAND;
pub const ICON_FOLDER_REG: &str = "icons/folder-simple.svg";
pub const ICON_GIT_REG: &str = ICON_GIT;
pub const ICON_CLOSE_REG: &str = ICON_CLOSE;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icon_for_known_extensions() {
        assert_eq!(icon_for_file("main.rs"), "icons/file-rs.svg");
        assert_eq!(icon_for_file("app.tsx"), "icons/file-tsx.svg");
        assert_eq!(icon_for_file("Cargo.toml"), "icons/gear.svg");
        assert_eq!(icon_for_file("photo.png"), "icons/image.svg");
    }

    #[test]
    fn icon_for_unknown_extension_is_default() {
        assert_eq!(icon_for_file("something.xyz"), "icons/file.svg");
        assert_eq!(icon_for_file("noext"), "icons/file.svg");
    }

    #[test]
    fn dir_icons_toggle() {
        assert_eq!(icon_for_dir(false), "icons/folder-simple.svg");
        assert_eq!(icon_for_dir(true), "icons/folder-open.svg");
    }
}
