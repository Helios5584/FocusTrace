use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum SearchScope { All, Time, App, Title, Prev }

impl Default for SearchScope {
    fn default() -> Self { SearchScope::All }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Settings {
    pub autostart: bool,
    #[serde(default)]
    pub search_scope: SearchScope,
}

impl Default for Settings {
    fn default() -> Self {
        Self { autostart: false, search_scope: SearchScope::All }
    }
}

impl Settings {
    pub fn path() -> PathBuf {
        crate::db::data_dir().join("settings.json")
    }

    pub fn load() -> Self {
        let p = Self::path();
        match std::fs::read_to_string(&p) {
            Ok(s) => serde_json::from_str(&s).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    pub fn save(&self) {
        let p = Self::path();
        if let Some(d) = p.parent() { std::fs::create_dir_all(d).ok(); }
        if let Ok(s) = serde_json::to_string_pretty(self) {
            std::fs::write(&p, s).ok();
        }
    }
}
