use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Typed settings persisted as `~/.conescope/settings.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettingsJson {
    #[serde(
        default = "default_comment_font_family",
        rename = "_comment_font_family"
    )]
    pub comment_font_family: String,

    #[serde(default = "default_theme")]
    pub theme: String,

    #[serde(default = "default_font_family")]
    pub font_family: String,

    #[serde(default = "default_font_size")]
    pub editor_font_size: i64,

    #[serde(default = "default_font_size")]
    pub terminal_font_size: i64,

    #[serde(default = "default_terminal_line_height")]
    pub terminal_line_height: f64,
}

fn default_comment_font_family() -> String {
    "Requires app restart for existing terminals".into()
}
fn default_theme() -> String {
    "dark".into()
}
fn default_font_family() -> String {
    "Menlo".into()
}
fn default_font_size() -> i64 {
    13
}
fn default_terminal_line_height() -> f64 {
    1.2
}

impl Default for SettingsJson {
    fn default() -> Self {
        Self {
            comment_font_family: default_comment_font_family(),
            theme: default_theme(),
            font_family: default_font_family(),
            editor_font_size: default_font_size(),
            terminal_font_size: default_font_size(),
            terminal_line_height: default_terminal_line_height(),
        }
    }
}

/// Maximum font size for UI chrome (tabs, file tree, git panel, overview tiles).
/// Terminal and editor content use the uncapped value.
const MAX_UI_FONT_SIZE: f32 = 20.0;
const MIN_UI_FONT_SIZE: f32 = 10.0;

impl SettingsJson {
    /// Font size for UI chrome elements, clamped to a readable range.
    /// Terminal/editor content should use the raw `terminal_font_size`/`editor_font_size`.
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn ui_font_size(&self) -> f32 {
        (self.terminal_font_size as f32).clamp(MIN_UI_FONT_SIZE, MAX_UI_FONT_SIZE)
    }

    /// Editor chrome font size (tabs), clamped to a readable range.
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn ui_editor_font_size(&self) -> f32 {
        (self.editor_font_size as f32).clamp(MIN_UI_FONT_SIZE, MAX_UI_FONT_SIZE)
    }

    /// Returns `$HOME/.conescope`.
    #[must_use]
    pub fn settings_dir() -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
        PathBuf::from(home).join(".conescope")
    }

    /// Returns `$HOME/.conescope/scratch/{project_id}/` for project-scoped scratch files.
    /// Uses `_no_project` for instances without a project.
    #[must_use]
    pub fn scratch_dir(project_id: Option<&str>) -> PathBuf {
        let base = Self::settings_dir().join("scratch");
        base.join(project_id.unwrap_or("_no_project"))
    }

    /// Base scratch directory `$HOME/.conescope/scratch/` (for `is_scratch_file` checks).
    #[must_use]
    pub fn scratch_base_dir() -> PathBuf {
        Self::settings_dir().join("scratch")
    }

    /// Returns `$HOME/.conescope/archives/scratches/{project_id}/`.
    #[must_use]
    pub fn archive_dir(project_id: Option<&str>) -> PathBuf {
        Self::settings_dir()
            .join("archives")
            .join("scratches")
            .join(project_id.unwrap_or("_no_project"))
    }

    /// Returns true if the path is inside the scratch directory.
    #[must_use]
    pub fn is_scratch_file(path: &Path) -> bool {
        path.starts_with(Self::scratch_base_dir())
    }

    /// Generate an untitled filename: `Untitled-{n+1}`.
    #[must_use]
    pub fn next_untitled_name(counter: usize) -> String {
        format!("Untitled-{}", counter + 1)
    }

    /// Returns the path to `settings.json` inside `dir`.
    #[must_use]
    pub fn file_path(dir: &Path) -> PathBuf {
        dir.join("settings.json")
    }

    /// Load settings from `dir/settings.json`. Returns `Default` on any error.
    #[must_use]
    pub fn load_from_file(dir: &Path) -> Self {
        let path = Self::file_path(dir);
        match std::fs::read_to_string(&path) {
            Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    /// Write settings as pretty JSON, atomically (write tmp then rename).
    ///
    /// # Errors
    /// Returns I/O error on failure.
    pub fn save_to_file(&self, dir: &Path) -> io::Result<()> {
        std::fs::create_dir_all(dir)?;
        let path = Self::file_path(dir);
        let tmp = dir.join("settings.json.tmp");
        let json = serde_json::to_string_pretty(self).map_err(io::Error::other)?;
        std::fs::write(&tmp, json)?;
        std::fs::rename(&tmp, &path)
    }

    /// One-time migration: build `SettingsJson` from old DB key-value pairs.
    #[must_use]
    pub fn migrate_from_db(pairs: &[(String, String)]) -> Self {
        let mut settings = Self::default();
        for (key, value) in pairs {
            match key.as_str() {
                "theme" => settings.theme.clone_from(value),
                "font_family" => settings.font_family.clone_from(value),
                "editor_font_size" => {
                    if let Ok(v) = value.parse() {
                        settings.editor_font_size = v;
                    }
                }
                "terminal_font_size" => {
                    if let Ok(v) = value.parse() {
                        settings.terminal_font_size = v;
                    }
                }
                "terminal_line_height" => {
                    if let Ok(v) = value.parse() {
                        settings.terminal_line_height = v;
                    }
                }
                _ => {} // skip session_state, questions_panel_visible, etc.
            }
        }
        settings
    }
}

// Keep old DEFAULTS const for DB migration compatibility (used in database.rs insert_default_settings).
pub const DEFAULTS: &[(&str, &str)] = &[
    ("theme", "dark"),
    ("questions_panel_visible", "true"),
    ("editor_font_size", "13"),
    ("terminal_font_size", "13"),
    ("font_family", "Menlo"),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_values() {
        let s = SettingsJson::default();
        assert_eq!(s.theme, "dark");
        assert_eq!(s.font_family, "Menlo");
        assert_eq!(s.editor_font_size, 13);
        assert_eq!(s.terminal_font_size, 13);
        assert!((s.terminal_line_height - 1.2).abs() < f64::EPSILON);
    }

    #[test]
    fn serde_roundtrip() {
        let s = SettingsJson {
            theme: "light".into(),
            font_family: "SF Mono".into(),
            editor_font_size: 14,
            terminal_font_size: 16,
            ..Default::default()
        };
        let json = serde_json::to_string_pretty(&s).unwrap();
        let parsed: SettingsJson = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.theme, "light");
        assert_eq!(parsed.font_family, "SF Mono");
        assert_eq!(parsed.editor_font_size, 14);
        assert_eq!(parsed.terminal_font_size, 16);
    }

    #[test]
    fn load_missing_file_returns_defaults() {
        let dir = PathBuf::from("/tmp/conescope-test-nonexistent-dir-xyz");
        let s = SettingsJson::load_from_file(&dir);
        assert_eq!(s.theme, "dark");
        assert_eq!(s.font_family, "Menlo");
    }

    #[test]
    fn save_and_load_roundtrip() {
        let dir = std::env::temp_dir().join("conescope-settings-test");
        let _ = std::fs::remove_dir_all(&dir);

        let s = SettingsJson {
            theme: "light".into(),
            font_family: "Fira Code".into(),
            editor_font_size: 15,
            terminal_font_size: 12,
            ..Default::default()
        };
        s.save_to_file(&dir).unwrap();

        let loaded = SettingsJson::load_from_file(&dir);
        assert_eq!(loaded.theme, "light");
        assert_eq!(loaded.font_family, "Fira Code");
        assert_eq!(loaded.editor_font_size, 15);
        assert_eq!(loaded.terminal_font_size, 12);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_corrupt_file_returns_defaults() {
        let dir = std::env::temp_dir().join("conescope-corrupt-settings-test");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("settings.json"), "not valid json!!!").unwrap();

        let s = SettingsJson::load_from_file(&dir);
        assert_eq!(s.theme, "dark");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn migrate_from_db_pairs() {
        let pairs = vec![
            ("theme".into(), "light".into()),
            ("font_family".into(), "Hack".into()),
            ("terminal_font_size".into(), "16".into()),
            ("session_state".into(), "{}".into()), // should be ignored
        ];
        let s = SettingsJson::migrate_from_db(&pairs);
        assert_eq!(s.theme, "light");
        assert_eq!(s.font_family, "Hack");
        assert_eq!(s.terminal_font_size, 16);
        assert_eq!(s.editor_font_size, 13); // default, not in pairs
    }

    #[test]
    fn comment_field_survives_roundtrip() {
        let s = SettingsJson::default();
        let json = serde_json::to_string_pretty(&s).unwrap();
        assert!(json.contains("_comment_font_family"));
        let parsed: SettingsJson = serde_json::from_str(&json).unwrap();
        assert!(parsed.comment_font_family.contains("restart"));
    }

    #[test]
    fn partial_json_uses_defaults() {
        let json = r#"{"theme": "light"}"#;
        let s: SettingsJson = serde_json::from_str(json).unwrap();
        assert_eq!(s.theme, "light");
        assert_eq!(s.font_family, "Menlo");
        assert_eq!(s.editor_font_size, 13);
    }

    #[test]
    fn scratch_dir_is_inside_settings_dir() {
        let scratch = SettingsJson::scratch_dir(Some("proj-1"));
        let settings = SettingsJson::settings_dir();
        assert!(scratch.starts_with(&settings));
        assert!(scratch.ends_with("proj-1"));

        let no_proj = SettingsJson::scratch_dir(None);
        assert!(no_proj.ends_with("_no_project"));
    }

    #[test]
    fn is_scratch_file_detects_scratch_paths() {
        let scratch = SettingsJson::scratch_dir(Some("proj-1"));
        let path = scratch.join("Untitled-1");
        assert!(SettingsJson::is_scratch_file(&path));

        let no_proj = SettingsJson::scratch_dir(None);
        assert!(SettingsJson::is_scratch_file(&no_proj.join("Untitled-2")));

        assert!(!SettingsJson::is_scratch_file(Path::new("/tmp/foo.rs")));
    }

    #[test]
    fn archive_dir_structure() {
        let archive = SettingsJson::archive_dir(Some("proj-1"));
        let settings = SettingsJson::settings_dir();
        assert!(archive.starts_with(&settings));
        assert!(
            archive
                .to_string_lossy()
                .contains("archives/scratches/proj-1")
        );

        let no_proj = SettingsJson::archive_dir(None);
        assert!(no_proj.to_string_lossy().contains("_no_project"));
    }

    #[test]
    fn next_untitled_name_increments() {
        assert_eq!(SettingsJson::next_untitled_name(0), "Untitled-1");
        assert_eq!(SettingsJson::next_untitled_name(4), "Untitled-5");
    }
}
