use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Serialize, Deserialize)]
pub struct Note {
    pub title: String,
    pub content: String,
    pub path: Option<PathBuf>,
    pub modified: bool,
}

impl Note {
    pub fn new(title: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            content: content.into(),
            path: None,
            modified: false,
        }
    }

    pub fn display_title(&self) -> String {
        if self.modified {
            format!("{}*", self.title)
        } else {
            self.title.clone()
        }
    }
}

impl Default for Note {
    fn default() -> Self {
        Self::new("Untitled", "# Untitled\n\n")
    }
}
