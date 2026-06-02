use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThemeMode {
    Dark,
    Light,
}

impl Default for ThemeMode {
    fn default() -> Self {
        ThemeMode::Dark
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default)]
    pub theme: ThemeMode,
    #[serde(default = "default_font_size")]
    pub editor_font_size: f32,
    #[serde(default = "default_true")]
    pub show_line_numbers: bool,
    #[serde(default = "default_true")]
    pub word_wrap: bool,
    #[serde(default = "default_true")]
    pub auto_save: bool,
    #[serde(default = "default_true")]
    pub show_sidebar: bool,
    #[serde(default = "default_true")]
    pub show_preview: bool,
    #[serde(default = "default_true")]
    pub syntax_highlight: bool,
}

fn default_font_size() -> f32 {
    13.0
}

fn default_true() -> bool {
    true
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: ThemeMode::Dark,
            editor_font_size: 13.0,
            show_line_numbers: true,
            word_wrap: true,
            auto_save: true,
            show_sidebar: true,
            show_preview: true,
            syntax_highlight: true,
        }
    }
}
