use std::collections::HashMap;

pub const DEFAULTS: &[(&str, &str)] = &[
    ("theme", "dark"),
    ("questions_panel_visible", "true"),
    ("editor_font_size", "13"),
    ("terminal_font_size", "13"),
];

#[derive(Debug, Clone, Default)]
pub struct Settings {
    pub map: HashMap<String, String>,
}

impl Settings {
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&str> {
        self.map.get(key).map(String::as_str)
    }

    #[must_use]
    pub fn get_or_default<'a>(&'a self, key: &str) -> &'a str {
        self.map.get(key).map_or_else(
            || {
                DEFAULTS
                    .iter()
                    .find(|(k, _)| *k == key)
                    .map_or("", |(_, v)| *v)
            },
            String::as_str,
        )
    }

    #[must_use]
    pub fn get_i64(&self, key: &str) -> Option<i64> {
        self.get(key).and_then(|v| v.parse().ok())
    }

    #[must_use]
    pub fn get_bool(&self, key: &str) -> Option<bool> {
        self.get(key).map(|v| v == "true" || v == "1")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_returns_value() {
        let mut s = Settings::default();
        s.map.insert("key".into(), "val".into());
        assert_eq!(s.get("key"), Some("val"));
    }

    #[test]
    fn get_or_default_falls_back() {
        let s = Settings::default();
        assert_eq!(s.get_or_default("theme"), "dark");
        assert_eq!(s.get_or_default("nonexistent"), "");
    }

    #[test]
    fn get_i64_parses() {
        let mut s = Settings::default();
        s.map.insert("font_size".into(), "14".into());
        assert_eq!(s.get_i64("font_size"), Some(14));
        assert_eq!(s.get_i64("missing"), None);
    }

    #[test]
    fn get_bool_parses() {
        let mut s = Settings::default();
        s.map.insert("flag".into(), "true".into());
        s.map.insert("num".into(), "1".into());
        s.map.insert("off".into(), "false".into());
        assert_eq!(s.get_bool("flag"), Some(true));
        assert_eq!(s.get_bool("num"), Some(true));
        assert_eq!(s.get_bool("off"), Some(false));
        assert_eq!(s.get_bool("missing"), None);
    }
}
