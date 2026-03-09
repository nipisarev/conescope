use gpui::Rgba;
use indexmap::IndexMap;

pub use serde_json::Value as JsonValue;

/// Active theme mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeMode {
    Dark,
    Light,
}

impl ThemeMode {
    #[must_use]
    pub fn toggle(self) -> Self {
        match self {
            Self::Dark => Self::Light,
            Self::Light => Self::Dark,
        }
    }

    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Dark => "dark",
            Self::Light => "light",
        }
    }

    #[must_use]
    pub fn from_str_or_default(s: &str) -> Self {
        if s.eq_ignore_ascii_case("light") {
            Self::Light
        } else {
            Self::Dark
        }
    }
}

/// Semantic color palette used by all views.
#[derive(Debug, Clone)]
pub struct Theme {
    pub name: String,
    pub mode: ThemeMode,

    // Backgrounds
    pub background: Rgba,
    pub editor_bg: Rgba,
    pub panel: Rgba,
    pub surface: Rgba,

    // Interactive
    pub element_hover: Rgba,
    pub element_selected: Rgba,
    pub accent: Rgba,
    pub selection: Rgba,

    // Borders
    pub border: Rgba,
    pub border_variant: Rgba,

    // Divider / scrollbar
    pub divider_active: Rgba,

    // Text
    pub text: Rgba,
    pub text_muted: Rgba,
    pub text_faint: Rgba,
    pub text_disabled: Rgba,

    // Destructive
    pub destructive: Rgba,
    pub destructive_hover: Rgba,

    // Status
    pub warning: Rgba,

    // Overlay
    pub backdrop: Rgba,

    // Terminal
    pub terminal_bg: Rgba,
    pub terminal_fg: Rgba,
    pub terminal_ansi: [Rgba; 16],

    // Version control
    pub vcs_modified: Rgba,
    pub vcs_added: Rgba,
    pub vcs_deleted: Rgba,
    pub vcs_conflict: Rgba,

    // Sidebar glass
    pub sidebar_glass_bg: Rgba,
    pub sidebar_glass_border: Rgba,

    // Tab-specific
    pub tab_active_bg: Rgba,
    pub tab_inactive_bg: Rgba,

    // Diff viewer
    pub diff_add_bg: Rgba,
    pub diff_add_text: Rgba,
    pub diff_delete_bg: Rgba,
    pub diff_delete_text: Rgba,
    pub diff_hunk_header_bg: Rgba,

    // Syntax highlighting (raw JSON for gpui-component SyntaxColors deserialization)
    pub syntax_json: JsonValue,
}

impl Theme {
    /// Build a `TerminalColors` palette from this theme.
    #[must_use]
    pub fn terminal_colors(&self) -> crate::terminal::TerminalColors {
        crate::terminal::TerminalColors {
            fg: self.terminal_fg,
            bg: self.terminal_bg,
            ansi: self.terminal_ansi,
        }
    }

    /// Load a built-in theme.
    #[must_use]
    pub fn load_builtin(mode: ThemeMode) -> Self {
        match mode {
            ThemeMode::Dark => {
                let json = include_str!("../assets/themes/gruvbox-material.json");
                parse_zed_theme(json, true)
            }
            ThemeMode::Light => {
                let json = include_str!("../assets/themes/vscode-light-plus.json");
                parse_zed_theme(json, false)
            }
        }
    }
}

/// Parse a Zed theme JSON, using the first variant (`themes[0]`).
#[must_use]
pub fn parse_zed_theme(json: &str, fallback_dark: bool) -> Theme {
    if let Some((_, theme)) = parse_zed_theme_variant(json, 0) {
        theme
    } else if fallback_dark {
        dark_theme(
            "Unknown".into(),
            ThemeMode::Dark,
            &|_| None,
            JsonValue::Null,
        )
    } else {
        light_theme(
            "Unknown".into(),
            ThemeMode::Light,
            &|_| None,
            JsonValue::Null,
        )
    }
}

/// Parse a specific variant from a Zed theme JSON.
///
/// Returns `(variant_name, Theme)` or `None` if the variant index is out of range.
#[must_use]
pub fn parse_zed_theme_variant(json: &str, variant_index: usize) -> Option<(String, Theme)> {
    let val: serde_json::Value = serde_json::from_str(json).ok()?;
    let theme_entry = val["themes"].get(variant_index)?;
    let style = &theme_entry["style"];

    let raw_name = theme_entry["name"]
        .as_str()
        .or_else(|| val["name"].as_str())
        .unwrap_or("Unknown");
    let variant_name = raw_name
        .strip_prefix("Zed Legacy: ")
        .unwrap_or(raw_name)
        .to_owned();

    let appearance = theme_entry["appearance"].as_str().unwrap_or("dark");
    let is_dark = !appearance.eq_ignore_ascii_case("light");

    let get = |key: &str| -> Option<Rgba> { style[key].as_str().and_then(hex_color) };
    let syntax_json = style["syntax"].clone();

    let mode = if is_dark {
        ThemeMode::Dark
    } else {
        ThemeMode::Light
    };

    let theme = if is_dark {
        dark_theme(variant_name.clone(), mode, &get, syntax_json)
    } else {
        light_theme(variant_name.clone(), mode, &get, syntax_json)
    };

    Some((variant_name, theme))
}

/// Blend a foreground color over a background at the given opacity (0.0–1.0).
fn blend_over(fg: Rgba, bg: Rgba, opacity: f32) -> Rgba {
    Rgba {
        r: fg.r * opacity + bg.r * (1.0 - opacity),
        g: fg.g * opacity + bg.g * (1.0 - opacity),
        b: fg.b * opacity + bg.b * (1.0 - opacity),
        a: 1.0,
    }
}

/// Count how many theme variants a Zed theme JSON contains.
fn count_variants(json: &str) -> usize {
    serde_json::from_str::<serde_json::Value>(json)
        .ok()
        .and_then(|v| v["themes"].as_array().map(Vec::len))
        .unwrap_or(0)
}

/// Entry returned by `ThemeRegistry::list()`.
#[derive(Debug, Clone)]
pub struct ThemeEntry {
    pub name: String,
    pub mode: ThemeMode,
}

/// Registry of all available themes (bundled + user).
#[derive(Debug)]
pub struct ThemeRegistry {
    themes: IndexMap<String, Theme>,
}

impl ThemeRegistry {
    #[must_use]
    pub fn new() -> Self {
        let mut registry = Self {
            themes: IndexMap::new(),
        };

        let bundled: &[&str] = &[
            include_str!("../assets/themes/gruvbox-material.json"),
            include_str!("../assets/themes/vscode-light-plus.json"),
            include_str!("../assets/themes/catppuccin.json"),
            include_str!("../assets/themes/dracula.json"),
            include_str!("../assets/themes/one.json"),
            include_str!("../assets/themes/solarized.json"),
            include_str!("../assets/themes/nord.json"),
            include_str!("../assets/themes/rose-pine.json"),
            include_str!("../assets/themes/tokyo-night.json"),
            include_str!("../assets/themes/ayu.json"),
            include_str!("../assets/themes/github.json"),
            include_str!("../assets/themes/kanagawa.json"),
            include_str!("../assets/themes/monokai.json"),
            include_str!("../assets/themes/macos-classic.json"),
            include_str!("../assets/themes/night-owl.json"),
            include_str!("../assets/themes/vitesse.json"),
            include_str!("../assets/themes/horizon.json"),
            include_str!("../assets/themes/vesper.json"),
        ];

        for json in bundled {
            let n = count_variants(json);
            for i in 0..n {
                if let Some((name, theme)) = parse_zed_theme_variant(json, i) {
                    registry.themes.insert(name, theme);
                }
            }
        }

        registry
    }

    pub fn load_user_themes(&mut self) {
        let home = std::env::var("HOME").unwrap_or_default();
        let dir = std::path::PathBuf::from(home)
            .join(".conescope")
            .join("themes");
        let Ok(entries) = std::fs::read_dir(&dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "json") {
                let json = match std::fs::read_to_string(&path) {
                    Ok(j) => j,
                    Err(e) => {
                        tracing::warn!("failed to read user theme {:?}: {e}", path);
                        continue;
                    }
                };
                let n = count_variants(&json);
                if n == 0 {
                    tracing::warn!("skipping user theme {:?}: no valid variants", path);
                    continue;
                }
                for i in 0..n {
                    if let Some((name, theme)) = parse_zed_theme_variant(&json, i) {
                        if self.themes.contains_key(&name) {
                            tracing::info!("user theme overrides bundled: {name}");
                        }
                        self.themes.insert(name, theme);
                    }
                }
            }
        }
    }

    #[must_use]
    pub fn get(&self, name: &str) -> Option<&Theme> {
        self.themes.get(name)
    }

    #[must_use]
    pub fn list(&self) -> Vec<ThemeEntry> {
        self.themes
            .values()
            .map(|t| ThemeEntry {
                name: t.name.clone(),
                mode: t.mode,
            })
            .collect()
    }

    /// Map legacy "dark"/"light" setting values to concrete theme names.
    #[must_use]
    pub fn resolve_legacy(s: &str) -> String {
        match s {
            "dark" => "Gruvbox Material".to_owned(),
            "light" => "VSCode Light+".to_owned(),
            other => other.to_owned(),
        }
    }
}

impl Default for ThemeRegistry {
    fn default() -> Self {
        Self::new()
    }
}

fn dark_theme(
    name: String,
    mode: ThemeMode,
    get: &dyn Fn(&str) -> Option<Rgba>,
    syntax_json: JsonValue,
) -> Theme {
    Theme {
        name,
        mode,
        background: get("background").unwrap_or(rgba(0x1e1e_1eff)),
        editor_bg: get("editor.background").unwrap_or(rgba(0x2525_26ff)),
        panel: get("panel.background")
            .or_else(|| get("title_bar.background"))
            .unwrap_or(rgba(0x2525_26ff)),
        surface: get("elevated_surface.background").unwrap_or(rgba(0x2d2d_2dff)),
        element_hover: get("element.hover").unwrap_or(rgba(0x3333_33ff)),
        element_selected: get("element.selected").unwrap_or(rgba(0x0944_61ff)),
        accent: get("text.accent").unwrap_or(rgba(0x0e63_9cff)),
        selection: rgba(0x264f_78ff),
        border: get("border").unwrap_or(rgba(0x3c3c_3cff)),
        border_variant: get("border.variant").unwrap_or(rgba(0x4c4c_4cff)),
        divider_active: rgba(0x007a_ccff),
        text: get("text").unwrap_or(rgba(0xd4d4_d4ff)),
        text_muted: get("text.muted").unwrap_or(rgba(0x8888_88ff)),
        text_faint: get("text.disabled").unwrap_or(rgba(0x6666_66ff)),
        text_disabled: rgba(0x5555_55ff),
        destructive: rgba(0xcc44_44ff),
        destructive_hover: rgba(0xdd55_55ff),
        warning: rgba(0xccaa_33ff),
        backdrop: rgba(0x0000_0080),
        terminal_bg: get("terminal.background").unwrap_or(rgba(0x2525_26ff)),
        terminal_fg: get("terminal.foreground").unwrap_or(rgba(0xd4d4_d4ff)),
        terminal_ansi: parse_ansi_colors(get, true),
        vcs_modified: get("version_control.modified")
            .or_else(|| get("modified"))
            .unwrap_or(rgba(0xe0c0_46ff)),
        vcs_added: get("version_control.created")
            .or_else(|| get("created"))
            .unwrap_or(rgba(0x73d2_75ff)),
        vcs_deleted: get("version_control.deleted")
            .or_else(|| get("deleted"))
            .unwrap_or(rgba(0xf55c_5cff)),
        vcs_conflict: get("version_control.conflict")
            .or_else(|| get("conflict"))
            .unwrap_or(rgba(0xcf8c_e3ff)),
        sidebar_glass_bg: rgba(0x1a1a_1ab0),
        sidebar_glass_border: rgba(0x4c4c_4c50),
        tab_active_bg: get("tab.active_background").unwrap_or(rgba(0x2d2d_2dff)),
        tab_inactive_bg: get("tab.inactive_background").unwrap_or(rgba(0x1e1e_1eff)),
        diff_add_text: get("version_control.created")
            .or_else(|| get("created"))
            .unwrap_or(rgba(0x98c3_79ff)),
        diff_add_bg: {
            let add_color = get("version_control.created")
                .or_else(|| get("created"))
                .unwrap_or(rgba(0x98c3_79ff));
            let ed_bg = get("editor.background").unwrap_or(rgba(0x2525_26ff));
            blend_over(add_color, ed_bg, 0.15)
        },
        diff_delete_text: get("version_control.deleted")
            .or_else(|| get("deleted"))
            .unwrap_or(rgba(0xe06c_75ff)),
        diff_delete_bg: {
            let del_color = get("version_control.deleted")
                .or_else(|| get("deleted"))
                .unwrap_or(rgba(0xe06c_75ff));
            let ed_bg = get("editor.background").unwrap_or(rgba(0x2525_26ff));
            blend_over(del_color, ed_bg, 0.15)
        },
        diff_hunk_header_bg: {
            let mod_color = get("version_control.modified")
                .or_else(|| get("modified"))
                .unwrap_or(rgba(0x6666_bbff));
            let ed_bg = get("editor.background").unwrap_or(rgba(0x2525_26ff));
            blend_over(mod_color, ed_bg, 0.12)
        },
        syntax_json,
    }
}

fn light_theme(
    name: String,
    mode: ThemeMode,
    get: &dyn Fn(&str) -> Option<Rgba>,
    syntax_json: JsonValue,
) -> Theme {
    Theme {
        name,
        mode,
        background: get("background").unwrap_or(rgba(0xffff_ffff)),
        editor_bg: get("editor.background").unwrap_or(rgba(0xffff_ffff)),
        panel: get("panel.background")
            .or_else(|| get("title_bar.background"))
            .unwrap_or(rgba(0xf3f3_f3ff)),
        surface: get("elevated_surface.background").unwrap_or(rgba(0xffff_ffff)),
        element_hover: rgba(0xe8e8_e8ff),
        element_selected: rgba(0xcce5_ffff),
        accent: get("text.accent").unwrap_or(rgba(0x0066_b8ff)),
        selection: rgba(0xadd6_ffff),
        border: get("border").unwrap_or(rgba(0xdddd_ddff)),
        border_variant: rgba(0xcccc_ccff),
        divider_active: rgba(0x007a_ccff),
        text: get("text").unwrap_or(rgba(0x3333_33ff)),
        text_muted: get("text.muted").unwrap_or(rgba(0x6666_66ff)),
        text_faint: rgba(0x9999_99ff),
        text_disabled: rgba(0xaaaa_aaff),
        destructive: rgba(0xd32f_2fff),
        destructive_hover: rgba(0xe53e_3eff),
        warning: rgba(0xf9a8_25ff),
        backdrop: rgba(0x0000_0060),
        terminal_bg: get("terminal.background").unwrap_or(rgba(0xffff_ffff)),
        terminal_fg: get("terminal.foreground").unwrap_or(rgba(0x3333_33ff)),
        terminal_ansi: parse_ansi_colors(get, false),
        vcs_modified: get("version_control.modified")
            .or_else(|| get("modified"))
            .unwrap_or(rgba(0x9498_00ff)),
        vcs_added: get("version_control.created")
            .or_else(|| get("created"))
            .unwrap_or(rgba(0x107c_10ff)),
        vcs_deleted: get("version_control.deleted")
            .or_else(|| get("deleted"))
            .unwrap_or(rgba(0xcd31_31ff)),
        vcs_conflict: get("version_control.conflict")
            .or_else(|| get("conflict"))
            .unwrap_or(rgba(0xbc05_bcff)),
        sidebar_glass_bg: rgba(0xf0f0_f0b0),
        sidebar_glass_border: rgba(0xcccc_cc50),
        tab_active_bg: get("tab.active_background").unwrap_or(rgba(0xffff_ffff)),
        tab_inactive_bg: get("tab.inactive_background").unwrap_or(rgba(0xecec_ecff)),
        diff_add_text: get("version_control.created")
            .or_else(|| get("created"))
            .unwrap_or(rgba(0x107c_10ff)),
        diff_add_bg: {
            let add_color = get("version_control.created")
                .or_else(|| get("created"))
                .unwrap_or(rgba(0x107c_10ff));
            let ed_bg = get("editor.background").unwrap_or(rgba(0xffff_ffff));
            blend_over(add_color, ed_bg, 0.10)
        },
        diff_delete_text: get("version_control.deleted")
            .or_else(|| get("deleted"))
            .unwrap_or(rgba(0xcd31_31ff)),
        diff_delete_bg: {
            let del_color = get("version_control.deleted")
                .or_else(|| get("deleted"))
                .unwrap_or(rgba(0xcd31_31ff));
            let ed_bg = get("editor.background").unwrap_or(rgba(0xffff_ffff));
            blend_over(del_color, ed_bg, 0.10)
        },
        diff_hunk_header_bg: {
            let mod_color = get("version_control.modified")
                .or_else(|| get("modified"))
                .unwrap_or(rgba(0x6666_bbff));
            let ed_bg = get("editor.background").unwrap_or(rgba(0xffff_ffff));
            blend_over(mod_color, ed_bg, 0.08)
        },
        syntax_json,
    }
}

/// Parse 16 ANSI terminal colors from theme JSON.
fn parse_ansi_colors(get: &dyn Fn(&str) -> Option<Rgba>, dark: bool) -> [Rgba; 16] {
    let keys = [
        "terminal.ansi.black",
        "terminal.ansi.red",
        "terminal.ansi.green",
        "terminal.ansi.yellow",
        "terminal.ansi.blue",
        "terminal.ansi.magenta",
        "terminal.ansi.cyan",
        "terminal.ansi.white",
        "terminal.ansi.bright_black",
        "terminal.ansi.bright_red",
        "terminal.ansi.bright_green",
        "terminal.ansi.bright_yellow",
        "terminal.ansi.bright_blue",
        "terminal.ansi.bright_magenta",
        "terminal.ansi.bright_cyan",
        "terminal.ansi.bright_white",
    ];
    let defaults_dark: [Rgba; 16] = [
        rgba(0x1e1e_1eff),
        rgba(0xcc44_44ff),
        rgba(0x4db0_4fff),
        rgba(0xeddb_82ff),
        rgba(0x5c97_d4ff),
        rgba(0xb061_c2ff),
        rgba(0x4dba_bfff),
        rgba(0xcccc_ccff),
        rgba(0x6666_66ff),
        rgba(0xf55c_5cff),
        rgba(0x73d2_75ff),
        rgba(0xffee_8cff),
        rgba(0x73ae_e6ff),
        rgba(0xcf8c_e3ff),
        rgba(0x73dc_e0ff),
        rgba(0xeded_edff),
    ];
    let defaults_light: [Rgba; 16] = [
        rgba(0x0000_00ff),
        rgba(0xcd31_31ff),
        rgba(0x107c_10ff),
        rgba(0x9498_00ff),
        rgba(0x0451_a5ff),
        rgba(0xbc05_bcff),
        rgba(0x0598_bcff),
        rgba(0x5555_55ff),
        rgba(0x6666_66ff),
        rgba(0xcd31_31ff),
        rgba(0x14ce_14ff),
        rgba(0xb5ba_00ff),
        rgba(0x0451_a5ff),
        rgba(0xbc05_bcff),
        rgba(0x0598_bcff),
        rgba(0xa5a5_a5ff),
    ];
    let defaults = if dark {
        &defaults_dark
    } else {
        &defaults_light
    };
    let mut colors = [rgba(0x0000_00ff); 16];
    for (i, key) in keys.iter().enumerate() {
        colors[i] = get(key).unwrap_or(defaults[i]);
    }
    colors
}

/// Parse a hex color string like "#rrggbb" or "#rrggbbaa" into `Rgba`.
#[allow(clippy::many_single_char_names)]
fn hex_color(s: &str) -> Option<Rgba> {
    let s = s.strip_prefix('#')?;
    let len = s.len();
    if len != 6 && len != 8 {
        return None;
    }
    let r = u8::from_str_radix(&s[0..2], 16).ok()?;
    let g = u8::from_str_radix(&s[2..4], 16).ok()?;
    let b = u8::from_str_radix(&s[4..6], 16).ok()?;
    let a = if len == 8 {
        u8::from_str_radix(&s[6..8], 16).ok()?
    } else {
        255
    };
    Some(rgba(u32::from_be_bytes([r, g, b, a])))
}

/// Shorthand to build `Rgba` from a `u32` like `0xRRGG_BBAA`.
#[must_use]
#[allow(clippy::many_single_char_names)]
const fn rgba(c: u32) -> Rgba {
    let [r, g, b, a] = c.to_be_bytes();
    Rgba {
        r: r as f32 / 255.0,
        g: g as f32 / 255.0,
        b: b as f32 / 255.0,
        a: a as f32 / 255.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_gruvbox_dark() {
        let json = include_str!("../assets/themes/gruvbox-material.json");
        let theme = parse_zed_theme(json, true);
        assert_eq!(theme.mode, ThemeMode::Dark);
        assert!(theme.name.contains("Gruvbox"));
        // background should be parsed from JSON
        assert!(theme.background.a > 0.9);
    }

    #[test]
    fn parse_vscode_light() {
        let json = include_str!("../assets/themes/vscode-light-plus.json");
        let theme = parse_zed_theme(json, false);
        assert_eq!(theme.mode, ThemeMode::Light);
        // Light bg should be bright (r > 0.9)
        assert!(theme.background.r > 0.9);
        assert!(theme.background.g > 0.9);
    }

    #[test]
    fn hex_color_parsing() {
        let c = hex_color("#ff0000").unwrap();
        assert!((c.r - 1.0).abs() < 0.01);
        assert!(c.g.abs() < 0.01);
        assert!(c.b.abs() < 0.01);
        assert!((c.a - 1.0).abs() < 0.01);

        let c = hex_color("#00ff0080").unwrap();
        assert!(c.g > 0.9);
        assert!((c.a - 0.502).abs() < 0.01);

        assert!(hex_color("invalid").is_none());
        assert!(hex_color("#xyz").is_none());
    }

    #[test]
    fn theme_mode_toggle() {
        assert_eq!(ThemeMode::Dark.toggle(), ThemeMode::Light);
        assert_eq!(ThemeMode::Light.toggle(), ThemeMode::Dark);
    }

    #[test]
    fn theme_mode_from_str() {
        assert_eq!(ThemeMode::from_str_or_default("light"), ThemeMode::Light);
        assert_eq!(ThemeMode::from_str_or_default("Light"), ThemeMode::Light);
        assert_eq!(ThemeMode::from_str_or_default("dark"), ThemeMode::Dark);
        assert_eq!(ThemeMode::from_str_or_default("anything"), ThemeMode::Dark);
    }

    #[test]
    fn parse_variant_auto_detects_appearance() {
        let json = include_str!("../assets/themes/gruvbox-material.json");
        let (name, theme) = parse_zed_theme_variant(json, 0).unwrap();
        assert_eq!(theme.mode, ThemeMode::Dark);
        assert!(!name.is_empty());
    }

    #[test]
    fn parse_variant_out_of_range_returns_none() {
        let json = include_str!("../assets/themes/gruvbox-material.json");
        assert!(parse_zed_theme_variant(json, 99).is_none());
    }

    #[test]
    fn diff_colors_are_set() {
        let json = include_str!("../assets/themes/gruvbox-material.json");
        let theme = parse_zed_theme(json, true);
        assert!(theme.diff_add_bg.a > 0.9);
        assert!(theme.diff_add_text.a > 0.9);
        assert!(theme.diff_delete_bg.a > 0.9);
        assert!(theme.diff_delete_text.a > 0.9);
        assert!(theme.diff_hunk_header_bg.a > 0.9);
    }

    #[test]
    fn registry_loads_bundled_themes() {
        let reg = ThemeRegistry::new();
        let list = reg.list();
        assert!(list.len() >= 2, "expected at least 2 bundled themes");
        assert!(reg.get("Gruvbox Material").is_some());
    }

    #[test]
    fn registry_resolve_legacy() {
        assert_eq!(ThemeRegistry::resolve_legacy("dark"), "Gruvbox Material");
        assert_eq!(ThemeRegistry::resolve_legacy("light"), "VSCode Light+");
        assert_eq!(ThemeRegistry::resolve_legacy("Custom"), "Custom");
    }

    #[test]
    fn multi_variant_themes_load_all() {
        let json = include_str!("../assets/themes/catppuccin.json");
        let n = count_variants(json);
        assert!(n >= 2, "catppuccin should have multiple variants");
        for i in 0..n {
            assert!(parse_zed_theme_variant(json, i).is_some());
        }
    }

    #[test]
    fn solarized_names_strip_zed_legacy_prefix() {
        let json = include_str!("../assets/themes/solarized.json");
        let reg = ThemeRegistry::new();
        let (name, _) = parse_zed_theme_variant(json, 0).unwrap();
        assert!(
            !name.starts_with("Zed Legacy:"),
            "expected stripped name, got: {name}"
        );
        assert!(reg.get("Solarized Dark").is_some());
        assert!(reg.get("Solarized Light").is_some());
    }
}
