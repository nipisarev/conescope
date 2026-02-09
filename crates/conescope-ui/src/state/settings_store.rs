use conescope_core::settings::SettingsJson;

use super::db_worker::DbHandle;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ViewMode {
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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SessionState {
    pub view_mode: ViewMode,
    pub focused_instance_id: Option<String>,
    pub terminal_height: f32,
    pub sidebar_width: f32,
    pub folder_panel_visible: bool,
    #[serde(default)]
    pub sidebar_tab: SidebarTab,
    pub editor_panel_visible: bool,
    pub terminal_panel_visible: bool,
    #[serde(default)]
    pub open_editor_tabs: Vec<String>,
    #[serde(default)]
    pub active_editor_tab: Option<String>,
    #[serde(default)]
    pub window_x: Option<f32>,
    #[serde(default)]
    pub window_y: Option<f32>,
    #[serde(default)]
    pub window_width: Option<f32>,
    #[serde(default)]
    pub window_height: Option<f32>,
}

impl Default for SessionState {
    fn default() -> Self {
        Self {
            view_mode: ViewMode::Overview,
            focused_instance_id: None,
            terminal_height: 300.0,
            sidebar_width: 240.0,
            folder_panel_visible: true,
            sidebar_tab: SidebarTab::Files,
            editor_panel_visible: true,
            terminal_panel_visible: true,
            open_editor_tabs: Vec::new(),
            active_editor_tab: None,
            window_x: None,
            window_y: None,
            window_width: None,
            window_height: None,
        }
    }
}

#[derive(Debug)]
pub struct SettingsStore {
    settings: SettingsJson,
    session: SessionState,
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
            previous_view_mode: None,
            db,
            loaded: false,
        }
    }

    /// Load session state from DB key-value pairs (`session_state` key only).
    pub fn load_session(&mut self, all_settings: Vec<(String, String)>) {
        for (key, value) in all_settings {
            if key == "session_state" {
                if let Ok(mut parsed) = serde_json::from_str::<SessionState>(&value) {
                    // Never restore Settings view mode from DB
                    if parsed.view_mode == ViewMode::Settings {
                        parsed.view_mode = ViewMode::Overview;
                    }
                    self.session = parsed;
                }
            }
        }
        self.loaded = true;
    }

    /// Load user settings from a `SettingsJson` (loaded from file).
    pub fn load_settings(&mut self, settings: SettingsJson) {
        self.settings = settings;
    }

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
mod tests {
    use super::*;

    #[test]
    fn session_state_serde_roundtrip() {
        let state = SessionState {
            view_mode: ViewMode::Focus,
            focused_instance_id: Some("abc-123".into()),
            terminal_height: 400.0,
            sidebar_width: 280.0,
            folder_panel_visible: false,
            sidebar_tab: SidebarTab::Files,
            editor_panel_visible: true,
            terminal_panel_visible: false,
            open_editor_tabs: vec!["/foo/bar.rs".into(), "/baz/qux.ts".into()],
            active_editor_tab: Some("/foo/bar.rs".into()),
            window_x: Some(200.0),
            window_y: Some(150.0),
            window_width: Some(1600.0),
            window_height: Some(1000.0),
        };
        let json = serde_json::to_string(&state).unwrap();
        let parsed: SessionState = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.view_mode, ViewMode::Focus);
        assert_eq!(parsed.focused_instance_id.as_deref(), Some("abc-123"));
        assert!(!parsed.folder_panel_visible);
        assert_eq!(parsed.open_editor_tabs.len(), 2);
        assert_eq!(parsed.active_editor_tab.as_deref(), Some("/foo/bar.rs"));
    }

    #[test]
    fn session_state_backward_compat() {
        // Old JSON without new fields should still deserialize
        let old_json = r#"{
            "view_mode":"focus",
            "focused_instance_id":"test-id",
            "terminal_height":300.0,
            "sidebar_width":240.0,
            "folder_panel_visible":true,
            "editor_panel_visible":true,
            "terminal_panel_visible":true
        }"#;
        let parsed: SessionState = serde_json::from_str(old_json).unwrap();
        assert!(parsed.open_editor_tabs.is_empty());
        assert!(parsed.active_editor_tab.is_none());
    }

    #[test]
    fn session_state_default() {
        let state = SessionState::default();
        assert_eq!(state.view_mode, ViewMode::Overview);
        assert!(state.focused_instance_id.is_none());
        assert!(state.folder_panel_visible);
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
        // If view_mode is Settings in DB JSON, it should deserialize as Settings
        // but load_session should override to Overview
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
        // Should NOT restore to Settings
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
}
