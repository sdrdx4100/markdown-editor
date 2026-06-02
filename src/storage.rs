use crate::note::Note;
use crate::settings::Settings;
use std::fs;
use std::path::PathBuf;

pub fn data_dir() -> PathBuf {
    if let Ok(appdata) = std::env::var("APPDATA") {
        let p = PathBuf::from(appdata).join("markdown-editor");
        let _ = fs::create_dir_all(&p);
        return p;
    }
    let p = PathBuf::from(".markdown-editor");
    let _ = fs::create_dir_all(&p);
    p
}

fn notes_file() -> PathBuf {
    data_dir().join("notes.json")
}

fn settings_file() -> PathBuf {
    data_dir().join("settings.json")
}

pub fn load_notes() -> Vec<Note> {
    let path = notes_file();
    if !path.exists() {
        return Vec::new();
    }
    fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save_notes(notes: &[Note]) {
    let path = notes_file();
    if let Ok(json) = serde_json::to_string_pretty(notes) {
        let _ = fs::write(path, json);
    }
}

pub fn load_settings() -> Settings {
    let path = settings_file();
    if !path.exists() {
        return Settings::default();
    }
    fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save_settings(settings: &Settings) {
    let path = settings_file();
    if let Ok(json) = serde_json::to_string_pretty(settings) {
        let _ = fs::write(path, json);
    }
}
