use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FontChoice {
    YuGothic,
    Meiryo,
    MsGothic,
    BizUdGothic,
}

impl Default for FontChoice {
    fn default() -> Self {
        FontChoice::YuGothic
    }
}

impl FontChoice {
    pub fn display_name(self) -> &'static str {
        match self {
            FontChoice::YuGothic => "游ゴシック",
            FontChoice::Meiryo => "メイリオ",
            FontChoice::MsGothic => "MS ゴシック",
            FontChoice::BizUdGothic => "BIZ UDP ゴシック",
        }
    }

    pub fn font_candidates(self) -> &'static [&'static str] {
        match self {
            FontChoice::YuGothic => &[
                r"C:\Windows\Fonts\YuGothM.ttc",
                r"C:\Windows\Fonts\YuGothR.ttc",
            ],
            FontChoice::Meiryo => &[
                r"C:\Windows\Fonts\meiryo.ttc",
                r"C:\Windows\Fonts\MeiryoUI.ttc",
            ],
            FontChoice::MsGothic => &[r"C:\Windows\Fonts\msgothic.ttc"],
            FontChoice::BizUdGothic => &[r"C:\Windows\Fonts\BIZ-UDGothicR.ttc"],
        }
    }

    pub fn all() -> &'static [FontChoice] {
        &[
            FontChoice::YuGothic,
            FontChoice::Meiryo,
            FontChoice::MsGothic,
            FontChoice::BizUdGothic,
        ]
    }
}

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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ViewMode {
    EditorOnly,
    Split,
    PreviewOnly,
}

impl Default for ViewMode {
    fn default() -> Self {
        ViewMode::Split
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default)]
    pub theme: ThemeMode,
    #[serde(default = "default_font_size")]
    pub editor_font_size: f32,
    #[serde(default)]
    pub font_choice: FontChoice,
    #[serde(default = "default_true")]
    pub show_line_numbers: bool,
    #[serde(default = "default_true")]
    pub word_wrap: bool,
    #[serde(default = "default_true")]
    pub auto_save: bool,
    #[serde(default = "default_true")]
    pub show_sidebar: bool,
    #[serde(default)]
    pub view_mode: ViewMode,
    #[serde(default = "default_true")]
    pub syntax_highlight: bool,
    #[serde(default)]
    pub show_toc: bool,
    #[serde(default = "default_true")]
    pub sync_scroll: bool,
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
            font_choice: FontChoice::default(),
            show_line_numbers: true,
            word_wrap: true,
            auto_save: true,
            show_sidebar: true,
            view_mode: ViewMode::Split,
            syntax_highlight: true,
            show_toc: false,
            sync_scroll: true,
        }
    }
}
