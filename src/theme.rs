use egui::{Color32, FontFamily, FontId, Style, TextStyle, Visuals};

pub fn apply(ctx: &egui::Context) {
    let mut style = Style::default();

    style.text_styles = [
        (TextStyle::Small, FontId::new(11.0, FontFamily::Proportional)),
        (TextStyle::Body, FontId::new(14.0, FontFamily::Proportional)),
        (TextStyle::Button, FontId::new(13.0, FontFamily::Proportional)),
        (TextStyle::Heading, FontId::new(18.0, FontFamily::Proportional)),
        (TextStyle::Monospace, FontId::new(13.0, FontFamily::Monospace)),
    ]
    .into();

    style.spacing.item_spacing = egui::vec2(6.0, 4.0);
    style.spacing.button_padding = egui::vec2(8.0, 4.0);

    let mut visuals = Visuals::dark();

    // Background layers (darkest → lightest)
    visuals.window_fill = Color32::from_rgb(28, 30, 34);
    visuals.panel_fill = Color32::from_rgb(28, 30, 34);
    visuals.faint_bg_color = Color32::from_rgb(36, 38, 44);
    visuals.extreme_bg_color = Color32::from_rgb(22, 24, 28); // editor bg

    // Accent
    visuals.selection.bg_fill = Color32::from_rgb(45, 180, 140);
    visuals.hyperlink_color = Color32::from_rgb(80, 200, 160);

    // Widget colors
    visuals.widgets.noninteractive.bg_fill = Color32::from_rgb(36, 38, 44);
    visuals.widgets.inactive.bg_fill = Color32::from_rgb(44, 46, 54);
    visuals.widgets.hovered.bg_fill = Color32::from_rgb(55, 58, 68);
    visuals.widgets.active.bg_fill = Color32::from_rgb(45, 180, 140);

    visuals.widgets.noninteractive.fg_stroke.color = Color32::from_rgb(180, 185, 195);
    visuals.widgets.inactive.fg_stroke.color = Color32::from_rgb(200, 205, 215);
    visuals.widgets.hovered.fg_stroke.color = Color32::WHITE;
    visuals.widgets.active.fg_stroke.color = Color32::WHITE;

    // Borders
    visuals.widgets.noninteractive.bg_stroke.color = Color32::from_rgb(50, 53, 62);
    visuals.widgets.noninteractive.bg_stroke.width = 1.0;

    visuals.window_rounding = 8.0.into();
    visuals.widgets.noninteractive.rounding = 4.0.into();
    visuals.widgets.inactive.rounding = 4.0.into();
    visuals.widgets.hovered.rounding = 4.0.into();
    visuals.widgets.active.rounding = 4.0.into();

    style.visuals = visuals;
    ctx.set_style(style);
}

pub const SIDEBAR_BG: Color32 = Color32::from_rgb(24, 26, 30);
pub const EDITOR_BG: Color32 = Color32::from_rgb(30, 32, 38);
pub const PREVIEW_BG: Color32 = Color32::from_rgb(34, 36, 42);
pub const SELECTED_ITEM_BG: Color32 = Color32::from_rgb(40, 160, 120);
pub const ACCENT: Color32 = Color32::from_rgb(45, 180, 140);
pub const TEXT_DIM: Color32 = Color32::from_rgb(120, 125, 140);
pub const TEXT_NORMAL: Color32 = Color32::from_rgb(200, 205, 215);
pub const LINE_NUMBER_COLOR: Color32 = Color32::from_rgb(80, 85, 100);
