use conescope_core::settings::Settings;

use super::db_worker::DbHandle;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ViewMode {
    Overview,
    Focus,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SessionState {
    pub view_mode: ViewMode,
    pub focused_instance_id: Option<String>,
    pub terminal_height: f32,
    pub sidebar_width: f32,
    pub folder_panel_visible: bool,
    pub editor_panel_visible: bool,
    pub terminal_panel_visible: bool,
}

impl Default for SessionState {
    fn default() -> Self {
        Self {
            view_mode: ViewMode::Overview,
            focused_instance_id: None,
            terminal_height: 300.0,
            sidebar_width: 240.0,
            folder_panel_visible: true,
            editor_panel_visible: true,
            terminal_panel_visible: true,
        }
    }
}

#[derive(Debug)]
pub struct SettingsStore {
    settings: Settings,
    session: SessionState,
    db: DbHandle,
    loaded: bool,
}

impl SettingsStore {
    #[must_use]
    pub fn new(db: DbHandle) -> Self {
        Self {
            settings: Settings::default(),
            session: SessionState::default(),
            db,
            loaded: false,
        }
    }

    pub fn load(&mut self, all_settings: Vec<(String, String)>) {
        for (key, value) in all_settings {
            if key == "session_state" {
                if let Ok(parsed) = serde_json::from_str::<SessionState>(&value) {
                    self.session = parsed;
                }
            }
            self.settings.map.insert(key, value);
        }
        self.loaded = true;
    }

    pub fn set(&mut self, key: &str, value: &str) {
        self.settings.map.insert(key.to_owned(), value.to_owned());
        self.db.set_setting(key.to_owned(), value.to_owned());
    }

    pub fn save_session(&mut self, session: SessionState) {
        self.session = session;
        if let Ok(json) = serde_json::to_string(&self.session) {
            self.settings
                .map
                .insert("session_state".to_owned(), json.clone());
            self.db.set_setting("session_state".to_owned(), json);
        }
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
    pub fn settings(&self) -> &Settings {
        &self.settings
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
            editor_panel_visible: true,
            terminal_panel_visible: false,
        };
        let json = serde_json::to_string(&state).unwrap();
        let parsed: SessionState = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.view_mode, ViewMode::Focus);
        assert_eq!(parsed.focused_instance_id.as_deref(), Some("abc-123"));
        assert!(!parsed.folder_panel_visible);
    }

    #[test]
    fn session_state_default() {
        let state = SessionState::default();
        assert_eq!(state.view_mode, ViewMode::Overview);
        assert!(state.focused_instance_id.is_none());
        assert!(state.folder_panel_visible);
    }

    #[test]
    fn settings_store_load_parses_session() {
        let db = DbHandle::spawn(":memory:").unwrap();
        let mut store = SettingsStore::new(db);
        assert!(!store.is_loaded());

        let session_json = serde_json::to_string(&SessionState {
            view_mode: ViewMode::Focus,
            focused_instance_id: Some("test-id".into()),
            ..Default::default()
        })
        .unwrap();

        store.load(vec![
            ("theme".into(), "dark".into()),
            ("session_state".into(), session_json),
        ]);

        assert!(store.is_loaded());
        assert_eq!(store.view_mode(), ViewMode::Focus);
        assert_eq!(store.focused_instance_id(), Some("test-id"));
        assert_eq!(store.settings().get("theme"), Some("dark"));
    }
}
