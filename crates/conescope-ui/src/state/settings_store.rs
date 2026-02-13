use std::collections::HashMap;

use conescope_core::settings::SettingsJson;

use super::db_worker::DbHandle;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ViewMode {
    #[default]
    Overview,
    Focus,
    #[serde(other)]
    Settings,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SidebarTab {
    Git,
    #[default]
    #[serde(other)]
    Files,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SidebarMode {
    #[default]
    Pinned,
    #[serde(other)]
    Overlay,
}

/// Per-instance layout state. Each instance remembers its own panel configuration.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct InstanceLayoutState {
    #[serde(default = "default_sidebar_width")]
    pub sidebar_width: f32,
    #[serde(default = "default_terminal_height")]
    pub terminal_height: f32,
    #[serde(default = "default_true")]
    pub folder_panel_visible: bool,
    #[serde(default)]
    pub sidebar_tab: SidebarTab,
    #[serde(default = "default_true")]
    pub editor_panel_visible: bool,
    #[serde(default = "default_true")]
    pub terminal_panel_visible: bool,
    #[serde(default)]
    pub open_editor_tabs: Vec<String>,
    #[serde(default)]
    pub active_editor_tab: Option<String>,
}

fn default_sidebar_width() -> f32 {
    240.0
}
fn default_terminal_height() -> f32 {
    300.0
}
fn default_true() -> bool {
    true
}

impl Default for InstanceLayoutState {
    fn default() -> Self {
        Self {
            sidebar_width: 240.0,
            terminal_height: 300.0,
            folder_panel_visible: true,
            sidebar_tab: SidebarTab::Files,
            editor_panel_visible: true,
            terminal_panel_visible: true,
            open_editor_tabs: Vec::new(),
            active_editor_tab: None,
        }
    }
}

/// Global session state (shared across all instances).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SessionState {
    #[serde(default)]
    pub view_mode: ViewMode,
    #[serde(default)]
    pub focused_instance_id: Option<String>,
    #[serde(default)]
    pub window_x: Option<f32>,
    #[serde(default)]
    pub window_y: Option<f32>,
    #[serde(default)]
    pub window_width: Option<f32>,
    #[serde(default)]
    pub window_height: Option<f32>,
    #[serde(default)]
    pub sidebar_mode: SidebarMode,
    #[serde(default = "default_true")]
    pub sidebar_open: bool,
}

impl Default for SessionState {
    fn default() -> Self {
        Self {
            view_mode: ViewMode::Overview,
            focused_instance_id: None,
            window_x: None,
            window_y: None,
            window_width: None,
            window_height: None,
            sidebar_mode: SidebarMode::Pinned,
            sidebar_open: true,
        }
    }
}

/// Transitional struct for deserializing old session state that mixed global + per-instance fields.
/// After the first save cycle, only the global `SessionState` format is written.
#[derive(serde::Deserialize)]
struct LegacySessionState {
    // Global fields
    #[serde(default)]
    view_mode: ViewMode,
    #[serde(default)]
    focused_instance_id: Option<String>,
    #[serde(default)]
    window_x: Option<f32>,
    #[serde(default)]
    window_y: Option<f32>,
    #[serde(default)]
    window_width: Option<f32>,
    #[serde(default)]
    window_height: Option<f32>,
    #[serde(default)]
    sidebar_mode: SidebarMode,
    #[serde(default = "default_true")]
    sidebar_open: bool,
    // Per-instance fields (present in old data, absent in new format)
    #[serde(default)]
    terminal_height: Option<f32>,
    #[serde(default)]
    sidebar_width: Option<f32>,
    #[serde(default)]
    folder_panel_visible: Option<bool>,
    #[serde(default)]
    sidebar_tab: Option<SidebarTab>,
    #[serde(default)]
    editor_panel_visible: Option<bool>,
    #[serde(default)]
    terminal_panel_visible: Option<bool>,
    #[serde(default)]
    open_editor_tabs: Option<Vec<String>>,
    #[serde(default)]
    active_editor_tab: Option<String>,
}

impl LegacySessionState {
    fn split(self) -> (SessionState, Option<InstanceLayoutState>) {
        let session = SessionState {
            view_mode: self.view_mode,
            focused_instance_id: self.focused_instance_id,
            window_x: self.window_x,
            window_y: self.window_y,
            window_width: self.window_width,
            window_height: self.window_height,
            sidebar_mode: self.sidebar_mode,
            sidebar_open: self.sidebar_open,
        };
        let has_layout = self.terminal_height.is_some()
            || self.sidebar_width.is_some()
            || self.folder_panel_visible.is_some()
            || self.sidebar_tab.is_some()
            || self.editor_panel_visible.is_some()
            || self.terminal_panel_visible.is_some();
        let layout = if has_layout {
            Some(InstanceLayoutState {
                sidebar_width: self.sidebar_width.unwrap_or(240.0),
                terminal_height: self.terminal_height.unwrap_or(300.0),
                folder_panel_visible: self.folder_panel_visible.unwrap_or(true),
                sidebar_tab: self.sidebar_tab.unwrap_or_default(),
                editor_panel_visible: self.editor_panel_visible.unwrap_or(true),
                terminal_panel_visible: self.terminal_panel_visible.unwrap_or(true),
                open_editor_tabs: self.open_editor_tabs.unwrap_or_default(),
                active_editor_tab: self.active_editor_tab,
            })
        } else {
            None
        };
        (session, layout)
    }
}

const INSTANCE_LAYOUT_PREFIX: &str = "instance_layout:";

#[derive(Debug)]
pub struct SettingsStore {
    settings: SettingsJson,
    session: SessionState,
    /// Current focused instance's layout.
    layout: InstanceLayoutState,
    /// Cached layouts for all known instances.
    instance_layouts: HashMap<String, InstanceLayoutState>,
    /// Previous view mode before entering Settings (for restoring on exit).
    previous_view_mode: Option<ViewMode>,
    db: DbHandle,
    loaded: bool,
}

impl SettingsStore {
    #[must_use]
    pub fn new(db: DbHandle) -> Self {
        Self {
            settings: SettingsJson::default(),
            session: SessionState::default(),
            layout: InstanceLayoutState::default(),
            instance_layouts: HashMap::new(),
            previous_view_mode: None,
            db,
            loaded: false,
        }
    }

    /// Load session state from DB key-value pairs.
    /// Handles both old (monolithic) and new (split) formats.
    #[allow(clippy::needless_pass_by_value)]
    pub fn load_session(&mut self, all_settings: Vec<(String, String)>) {
        let mut legacy_layout = None;

        for (key, value) in &all_settings {
            if key == "session_state" {
                if let Ok(legacy) = serde_json::from_str::<LegacySessionState>(value) {
                    let (mut session, layout) = legacy.split();
                    // Never restore Settings view mode from DB
                    if session.view_mode == ViewMode::Settings {
                        session.view_mode = ViewMode::Overview;
                    }
                    self.session = session;
                    legacy_layout = layout;
                }
            } else if let Some(instance_id) = key.strip_prefix(INSTANCE_LAYOUT_PREFIX) {
                if let Ok(layout) = serde_json::from_str::<InstanceLayoutState>(value) {
                    self.instance_layouts.insert(instance_id.to_owned(), layout);
                }
            }
        }

        // Set current layout for the focused instance
        if let Some(ref id) = self.session.focused_instance_id {
            if let Some(layout) = self.instance_layouts.get(id) {
                self.layout = layout.clone();
            } else if let Some(layout) = legacy_layout {
                // Migrate legacy per-instance data to the focused instance
                self.instance_layouts.insert(id.clone(), layout.clone());
                self.layout = layout;
            }
        } else if let Some(layout) = legacy_layout {
            // No focused instance but legacy data exists — use as default layout
            self.layout = layout;
        }

        self.loaded = true;
    }

    /// Load user settings from a `SettingsJson` (loaded from file).
    pub fn load_settings(&mut self, settings: SettingsJson) {
        self.settings = settings;
    }

    /// Save global session state to DB.
    pub fn save_session(&mut self, session: SessionState) {
        self.session = session;
        // If we're in Settings mode, serialize the previous_view_mode instead
        let mut session_to_save = self.session.clone();
        if session_to_save.view_mode == ViewMode::Settings {
            session_to_save.view_mode = self.previous_view_mode.unwrap_or(ViewMode::Overview);
        }
        if let Ok(json) = serde_json::to_string(&session_to_save) {
            self.db.set_setting("session_state".to_owned(), json);
        }
    }

    /// Save the current instance's layout and persist to DB.
    pub fn save_layout(&mut self, layout: InstanceLayoutState) {
        self.layout = layout;
        self.persist_current_layout();
    }

    /// Persist current layout to DB under the focused instance's key.
    fn persist_current_layout(&self) {
        if let Some(ref id) = self.session.focused_instance_id {
            self.persist_layout_for(id, &self.layout);
        }
    }

    fn persist_layout_for(&self, instance_id: &str, layout: &InstanceLayoutState) {
        if let Ok(json) = serde_json::to_string(layout) {
            let key = format!("{INSTANCE_LAYOUT_PREFIX}{instance_id}");
            self.db.set_setting(key, json);
        }
    }

    /// Switch layout when focusing a different instance.
    /// Saves the current layout under `old_id`, loads layout for `new_id`.
    pub fn switch_instance_layout(&mut self, old_id: Option<&str>, new_id: &str) {
        // Save current layout under old instance
        if let Some(old) = old_id {
            self.instance_layouts
                .insert(old.to_owned(), self.layout.clone());
            self.persist_layout_for(old, &self.layout);
        }
        // Load layout for new instance (or default)
        self.layout = self
            .instance_layouts
            .get(new_id)
            .cloned()
            .unwrap_or_default();
    }

    /// Remove a deleted instance's layout from cache and DB.
    pub fn remove_instance_layout(&mut self, instance_id: &str) {
        self.instance_layouts.remove(instance_id);
        let key = format!("{INSTANCE_LAYOUT_PREFIX}{instance_id}");
        self.db.delete_setting(key);
    }

    /// Enter settings editing mode, stashing current view mode.
    pub fn enter_settings_mode(&mut self) {
        if self.session.view_mode != ViewMode::Settings {
            self.previous_view_mode = Some(self.session.view_mode);
        }
        self.session.view_mode = ViewMode::Settings;
    }

    /// Exit settings editing mode, restoring previous view mode.
    pub fn exit_settings_mode(&mut self) {
        self.session.view_mode = self.previous_view_mode.unwrap_or(ViewMode::Overview);
        self.previous_view_mode = None;
    }

    #[must_use]
    pub fn view_mode(&self) -> ViewMode {
        self.session.view_mode
    }

    #[must_use]
    pub fn focused_instance_id(&self) -> Option<&str> {
        self.session.focused_instance_id.as_deref()
    }

    #[must_use]
    pub fn session(&self) -> &SessionState {
        &self.session
    }

    #[must_use]
    pub fn layout(&self) -> &InstanceLayoutState {
        &self.layout
    }

    #[must_use]
    pub fn settings(&self) -> &SettingsJson {
        &self.settings
    }

    pub fn settings_mut(&mut self) -> &mut SettingsJson {
        &mut self.settings
    }

    #[must_use]
    pub fn is_loaded(&self) -> bool {
        self.loaded
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn session_state_serde_roundtrip() {
        let state = SessionState {
            view_mode: ViewMode::Focus,
            focused_instance_id: Some("abc-123".into()),
            window_x: Some(200.0),
            window_y: Some(150.0),
            window_width: Some(1600.0),
            window_height: Some(1000.0),
            ..Default::default()
        };
        let json = serde_json::to_string(&state).unwrap();
        let parsed: SessionState = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.view_mode, ViewMode::Focus);
        assert_eq!(parsed.focused_instance_id.as_deref(), Some("abc-123"));
        assert_eq!(parsed.window_x, Some(200.0));
    }

    #[test]
    fn instance_layout_serde_roundtrip() {
        let layout = InstanceLayoutState {
            sidebar_width: 280.0,
            terminal_height: 400.0,
            folder_panel_visible: false,
            sidebar_tab: SidebarTab::Git,
            editor_panel_visible: true,
            terminal_panel_visible: false,
            open_editor_tabs: vec!["/foo/bar.rs".into(), "/baz/qux.ts".into()],
            active_editor_tab: Some("/foo/bar.rs".into()),
        };
        let json = serde_json::to_string(&layout).unwrap();
        let parsed: InstanceLayoutState = serde_json::from_str(&json).unwrap();
        assert!(!parsed.folder_panel_visible);
        assert_eq!(parsed.sidebar_tab, SidebarTab::Git);
        assert_eq!(parsed.open_editor_tabs.len(), 2);
        assert_eq!(parsed.active_editor_tab.as_deref(), Some("/foo/bar.rs"));
    }

    #[test]
    fn legacy_session_state_backward_compat() {
        // Old monolithic JSON with ALL fields
        let old_json = r#"{
            "view_mode":"focus",
            "focused_instance_id":"test-id",
            "terminal_height":400.0,
            "sidebar_width":280.0,
            "folder_panel_visible":false,
            "editor_panel_visible":true,
            "terminal_panel_visible":true,
            "window_x":100.0,
            "window_y":200.0,
            "window_width":1400.0,
            "window_height":900.0
        }"#;
        let legacy: LegacySessionState = serde_json::from_str(old_json).unwrap();
        let (session, layout) = legacy.split();

        assert_eq!(session.view_mode, ViewMode::Focus);
        assert_eq!(session.focused_instance_id.as_deref(), Some("test-id"));
        assert_eq!(session.window_x, Some(100.0));

        let layout = layout.expect("should extract per-instance layout from old format");
        assert_eq!(layout.terminal_height, 400.0);
        assert_eq!(layout.sidebar_width, 280.0);
        assert!(!layout.folder_panel_visible);
    }

    #[test]
    fn legacy_session_state_new_format() {
        // New global-only JSON — no per-instance fields
        let new_json = r#"{
            "view_mode":"overview",
            "focused_instance_id":null,
            "window_x":100.0,
            "window_y":200.0
        }"#;
        let legacy: LegacySessionState = serde_json::from_str(new_json).unwrap();
        let (session, layout) = legacy.split();

        assert_eq!(session.view_mode, ViewMode::Overview);
        assert!(layout.is_none(), "no per-instance fields in new format");
    }

    #[test]
    fn session_state_default() {
        let state = SessionState::default();
        assert_eq!(state.view_mode, ViewMode::Overview);
        assert!(state.focused_instance_id.is_none());
    }

    #[test]
    fn instance_layout_default() {
        let layout = InstanceLayoutState::default();
        assert!(layout.folder_panel_visible);
        assert!(layout.editor_panel_visible);
        assert!(layout.terminal_panel_visible);
        assert_eq!(layout.sidebar_width, 240.0);
        assert_eq!(layout.terminal_height, 300.0);
        assert!(layout.open_editor_tabs.is_empty());
    }

    #[test]
    fn settings_store_load_session() {
        let db = DbHandle::spawn(":memory:").unwrap();
        let mut store = SettingsStore::new(db);
        assert!(!store.is_loaded());

        let session_json = serde_json::to_string(&SessionState {
            view_mode: ViewMode::Focus,
            focused_instance_id: Some("test-id".into()),
            ..Default::default()
        })
        .unwrap();

        store.load_session(vec![
            ("theme".into(), "dark".into()),
            ("session_state".into(), session_json),
        ]);

        assert!(store.is_loaded());
        assert_eq!(store.view_mode(), ViewMode::Focus);
        assert_eq!(store.focused_instance_id(), Some("test-id"));
    }

    #[test]
    fn settings_store_load_legacy_migrates_layout() {
        let db = DbHandle::spawn(":memory:").unwrap();
        let mut store = SettingsStore::new(db);

        // Old monolithic format with per-instance fields
        let old_json = r#"{
            "view_mode":"focus",
            "focused_instance_id":"inst-1",
            "terminal_height":400.0,
            "sidebar_width":280.0,
            "folder_panel_visible":false,
            "editor_panel_visible":true,
            "terminal_panel_visible":true
        }"#;

        store.load_session(vec![("session_state".into(), old_json.into())]);

        // Global state loaded
        assert_eq!(store.view_mode(), ViewMode::Focus);
        assert_eq!(store.focused_instance_id(), Some("inst-1"));

        // Per-instance layout migrated for focused instance
        assert_eq!(store.layout().terminal_height, 400.0);
        assert_eq!(store.layout().sidebar_width, 280.0);
        assert!(!store.layout().folder_panel_visible);
    }

    #[test]
    fn settings_store_load_per_instance_layouts() {
        let db = DbHandle::spawn(":memory:").unwrap();
        let mut store = SettingsStore::new(db);

        let session_json = r#"{"view_mode":"focus","focused_instance_id":"inst-2"}"#;
        let layout_1 = serde_json::to_string(&InstanceLayoutState {
            sidebar_width: 300.0,
            ..Default::default()
        })
        .unwrap();
        let layout_2 = serde_json::to_string(&InstanceLayoutState {
            sidebar_width: 200.0,
            terminal_height: 500.0,
            ..Default::default()
        })
        .unwrap();

        store.load_session(vec![
            ("session_state".into(), session_json.into()),
            ("instance_layout:inst-1".into(), layout_1),
            ("instance_layout:inst-2".into(), layout_2),
        ]);

        // Current layout matches focused instance (inst-2)
        assert_eq!(store.layout().sidebar_width, 200.0);
        assert_eq!(store.layout().terminal_height, 500.0);
    }

    #[test]
    fn switch_instance_layout_saves_and_loads() {
        let db = DbHandle::spawn(":memory:").unwrap();
        let mut store = SettingsStore::new(db);
        store.session.focused_instance_id = Some("inst-A".into());

        // Set up instance A's layout
        store.layout.sidebar_width = 350.0;
        store.layout.folder_panel_visible = false;

        // Pre-load instance B's layout in cache
        store.instance_layouts.insert(
            "inst-B".into(),
            InstanceLayoutState {
                sidebar_width: 200.0,
                terminal_height: 500.0,
                ..Default::default()
            },
        );

        // Switch from A to B
        store.switch_instance_layout(Some("inst-A"), "inst-B");

        // Current layout is now B's
        assert_eq!(store.layout().sidebar_width, 200.0);
        assert_eq!(store.layout().terminal_height, 500.0);

        // A's layout was cached
        let cached_a = store.instance_layouts.get("inst-A").unwrap();
        assert_eq!(cached_a.sidebar_width, 350.0);
        assert!(!cached_a.folder_panel_visible);
    }

    #[test]
    fn switch_to_unknown_instance_gets_defaults() {
        let db = DbHandle::spawn(":memory:").unwrap();
        let mut store = SettingsStore::new(db);
        store.session.focused_instance_id = Some("inst-A".into());
        store.layout.sidebar_width = 350.0;

        // Switch to an instance with no saved layout
        store.switch_instance_layout(Some("inst-A"), "inst-NEW");

        // Gets default layout
        let defaults = InstanceLayoutState::default();
        assert_eq!(store.layout().sidebar_width, defaults.sidebar_width);
        assert_eq!(store.layout().terminal_height, defaults.terminal_height);
        assert!(store.layout().folder_panel_visible);
    }

    #[test]
    fn settings_mode_stash_restore() {
        let db = DbHandle::spawn(":memory:").unwrap();
        let mut store = SettingsStore::new(db);
        store.load_session(vec![]);

        assert_eq!(store.view_mode(), ViewMode::Overview);
        store.enter_settings_mode();
        assert_eq!(store.view_mode(), ViewMode::Settings);
        store.exit_settings_mode();
        assert_eq!(store.view_mode(), ViewMode::Overview);
    }

    #[test]
    fn settings_view_mode_never_persisted() {
        // Old format with Settings view mode
        let session_json = r#"{
            "view_mode":"settings",
            "focused_instance_id":null,
            "terminal_height":300.0,
            "sidebar_width":240.0,
            "folder_panel_visible":true,
            "editor_panel_visible":true,
            "terminal_panel_visible":true
        }"#;

        let db = DbHandle::spawn(":memory:").unwrap();
        let mut store = SettingsStore::new(db);
        store.load_session(vec![("session_state".into(), session_json.into())]);
        assert_eq!(store.view_mode(), ViewMode::Overview);
    }

    #[test]
    fn typed_settings_access() {
        let db = DbHandle::spawn(":memory:").unwrap();
        let mut store = SettingsStore::new(db);
        store.load_settings(SettingsJson {
            theme: "light".into(),
            font_family: "SF Mono".into(),
            editor_font_size: 14,
            terminal_font_size: 16,
            ..Default::default()
        });

        assert_eq!(store.settings().theme, "light");
        assert_eq!(store.settings().font_family, "SF Mono");
        assert_eq!(store.settings().editor_font_size, 14);
        assert_eq!(store.settings().terminal_font_size, 16);
    }

    #[test]
    fn remove_instance_layout_cleans_cache() {
        let db = DbHandle::spawn(":memory:").unwrap();
        let mut store = SettingsStore::new(db);
        store.session.focused_instance_id = Some("inst-A".into());

        // Pre-populate cache with two instance layouts
        store.instance_layouts.insert(
            "inst-A".into(),
            InstanceLayoutState {
                sidebar_width: 350.0,
                ..Default::default()
            },
        );
        store.instance_layouts.insert(
            "inst-B".into(),
            InstanceLayoutState {
                sidebar_width: 200.0,
                ..Default::default()
            },
        );

        // Remove inst-A's layout
        store.remove_instance_layout("inst-A");
        assert!(
            !store.instance_layouts.contains_key("inst-A"),
            "inst-A layout should be removed from cache"
        );
        assert!(
            store.instance_layouts.contains_key("inst-B"),
            "inst-B layout should remain"
        );

        // Removing non-existent key is a no-op
        store.remove_instance_layout("nonexistent");
    }

    #[test]
    fn legacy_sidebar_tab_detected_as_layout() {
        // Verify: sidebar_tab alone triggers has_layout detection
        let json = r#"{
            "view_mode":"focus",
            "focused_instance_id":"test-id",
            "sidebar_tab":"git"
        }"#;
        let legacy: LegacySessionState = serde_json::from_str(json).unwrap();
        let (_session, layout) = legacy.split();
        assert!(
            layout.is_some(),
            "sidebar_tab alone should produce a layout"
        );
        assert_eq!(layout.unwrap().sidebar_tab, SidebarTab::Git);
    }

    #[test]
    fn instance_layout_isolation() {
        // Verify: changing one instance's layout never affects another
        let db = DbHandle::spawn(":memory:").unwrap();
        let mut store = SettingsStore::new(db);
        store.session.focused_instance_id = Some("inst-A".into());

        // Set up A with custom layout
        store.save_layout(InstanceLayoutState {
            sidebar_width: 999.0,
            folder_panel_visible: false,
            ..Default::default()
        });

        // Switch to B
        store.switch_instance_layout(Some("inst-A"), "inst-B");

        // B gets defaults, not A's values
        assert_eq!(store.layout().sidebar_width, 240.0);
        assert!(store.layout().folder_panel_visible);

        // Modify B
        store.save_layout(InstanceLayoutState {
            terminal_height: 777.0,
            ..Default::default()
        });

        // Switch back to A
        store.switch_instance_layout(Some("inst-B"), "inst-A");

        // A still has its original values
        assert_eq!(store.layout().sidebar_width, 999.0);
        assert!(!store.layout().folder_panel_visible);
        // A does NOT have B's terminal_height
        assert_eq!(store.layout().terminal_height, 300.0);
    }
}
