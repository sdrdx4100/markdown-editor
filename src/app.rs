use crate::attachments;
use crate::editor_actions::{self, EditorAction};
use crate::export;
use crate::find_replace::{self, FindReplaceState};
use crate::highlight;
use crate::note::Note;
use crate::settings::{FontChoice, Settings, ThemeMode, ViewMode};
use crate::storage;
use crate::theme::{self, ThemeColors};
use crate::toc;
use crate::wikilinks::{self, QuickSwitcherState};
use egui::{Color32, FontFamily, FontId, RichText, ScrollArea, TextEdit, Ui};
use egui_commonmark::{CommonMarkCache, CommonMarkViewer};
use std::fs;
use std::time::Instant;

const EDITOR_ID: &str = "main_editor";
const AUTOSAVE_INTERVAL_SECS: u64 = 3;

pub struct MarkdownApp {
    notes: Vec<Note>,
    selected: usize,
    cache: CommonMarkCache,
    search_query: String,
    pending_action: Option<EditorAction>,
    pending_line_move: Option<bool>, // true = up, false = down
    pending_list_continuation: bool,
    settings: Settings,
    last_save_at: Instant,
    notes_dirty: bool,
    show_settings: bool,
    show_trash: bool,
    find: FindReplaceState,
    quick_switcher: QuickSwitcherState,
    show_backlinks: bool,
    status_override: Option<String>,
    status_override_at: Option<Instant>,
}

impl MarkdownApp {
    pub fn new(cc: &eframe::CreationContext) -> Self {
        let settings = storage::load_settings();
        load_japanese_font(&cc.egui_ctx, settings.font_choice);
        theme::apply(&cc.egui_ctx, settings.theme, settings.editor_font_size);

        let mut notes = storage::load_notes();
        if notes.is_empty() {
            notes = default_notes();
        }

        Self {
            notes,
            selected: 0,
            cache: CommonMarkCache::default(),
            search_query: String::new(),
            pending_action: None,
            pending_line_move: None,
            pending_list_continuation: false,
            settings,
            last_save_at: Instant::now(),
            notes_dirty: false,
            show_settings: false,
            show_trash: false,
            find: FindReplaceState::default(),
            quick_switcher: QuickSwitcherState::default(),
            show_backlinks: true,
            status_override: None,
            status_override_at: None,
        }
    }

    fn colors(&self) -> ThemeColors {
        theme::colors(self.settings.theme)
    }

    fn save_notes(&mut self) {
        storage::save_notes(&self.notes);
        self.notes_dirty = false;
        self.last_save_at = Instant::now();
    }

    fn save_settings(&self) {
        storage::save_settings(&self.settings);
    }

    fn mark_dirty(&mut self) {
        self.notes_dirty = true;
    }

    fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        let (
            ctrl,
            shift,
            alt,
            key_s,
            key_n,
            key_o,
            key_b,
            key_i,
            key_k,
            key_e,
            key_q,
            key_l,
            key_t,
            key_f,
            key_h,
            key_up,
            key_down,
            key_esc,
        ) = ctx.input(|i| {
            (
                i.modifiers.ctrl,
                i.modifiers.shift,
                i.modifiers.alt,
                i.key_pressed(egui::Key::S),
                i.key_pressed(egui::Key::N),
                i.key_pressed(egui::Key::O),
                i.key_pressed(egui::Key::B),
                i.key_pressed(egui::Key::I),
                i.key_pressed(egui::Key::K),
                i.key_pressed(egui::Key::E),
                i.key_pressed(egui::Key::Q),
                i.key_pressed(egui::Key::L),
                i.key_pressed(egui::Key::T),
                i.key_pressed(egui::Key::F),
                i.key_pressed(egui::Key::H),
                i.key_pressed(egui::Key::ArrowUp),
                i.key_pressed(egui::Key::ArrowDown),
                i.key_pressed(egui::Key::Escape),
            )
        });

        if ctrl && key_s && !shift {
            self.save_current_to_file();
        }
        if ctrl && key_n && !shift {
            self.new_note();
        }
        if ctrl && key_o && !shift {
            self.open_file();
        }
        if ctrl && key_b && !shift {
            self.pending_action = Some(EditorAction::Wrap { prefix: "**", suffix: "**" });
        }
        if ctrl && key_i && !shift {
            self.pending_action = Some(EditorAction::Wrap { prefix: "*", suffix: "*" });
        }
        if ctrl && key_k && !shift {
            self.pending_action = Some(EditorAction::Insert("[リンクテキスト](https://)"));
        }
        if ctrl && key_e && !shift {
            self.pending_action = Some(EditorAction::Wrap { prefix: "`", suffix: "`" });
        }
        if ctrl && shift && key_q {
            self.pending_action = Some(EditorAction::LinePrefix("> "));
        }
        if ctrl && shift && key_l {
            self.pending_action = Some(EditorAction::LinePrefix("- "));
        }
        if ctrl && shift && key_t {
            self.pending_action = Some(EditorAction::LinePrefix("- [ ] "));
        }
        if ctrl && key_f && !shift {
            self.find.open_find();
        }
        if ctrl && key_h && !shift {
            self.find.open_replace();
        }
        if key_esc && self.find.visible {
            self.find.close();
        }
        if alt && key_up {
            self.pending_line_move = Some(true);
        }
        if alt && key_down {
            self.pending_line_move = Some(false);
        }
        // Image paste: TextEdit consumes Ctrl+V before we can see it, so
        // listen for the paste *event* itself, which still fires after the
        // widget handles it. When a paste happens, also check whether the
        // clipboard carries an image — if so, attach it.
        let paste_event = ctx.input(|i| {
            i.events
                .iter()
                .any(|e| matches!(e, egui::Event::Paste(_)))
        });
        if paste_event {
            self.try_paste_image(false);
        }
        // Ctrl+Shift+V also works as an explicit trigger (e.g. when the
        // focus is not on the text editor).
        let key_v = ctx.input(|i| i.key_pressed(egui::Key::V));
        if ctrl && shift && key_v {
            self.try_paste_image(true);
        }
        // Ctrl+P: Quick switcher
        let key_p = ctx.input(|i| i.key_pressed(egui::Key::P));
        if ctrl && key_p && !shift {
            self.quick_switcher.open();
        }
        if key_esc && self.quick_switcher.visible {
            self.quick_switcher.close();
        }
        // Ctrl+\ to cycle view mode
        let key_backslash = ctx.input(|i| i.key_pressed(egui::Key::Backslash));
        if ctrl && key_backslash {
            self.settings.view_mode = match self.settings.view_mode {
                ViewMode::EditorOnly => ViewMode::Split,
                ViewMode::Split => ViewMode::PreviewOnly,
                ViewMode::PreviewOnly => ViewMode::EditorOnly,
            };
            self.save_settings();
        }
    }

    fn toolbar_button(&mut self, ui: &mut Ui, label: &str, tooltip: &str, action: EditorAction) {
        let c = self.colors();
        let resp = ui.add(
            egui::Button::new(RichText::new(label).size(13.0).color(c.text_normal))
                .min_size(egui::vec2(32.0, 28.0))
                .fill(c.button_bg),
        );
        if resp.on_hover_text(tooltip).clicked() {
            self.pending_action = Some(action);
        }
    }

    fn draw_toolbar(&mut self, ui: &mut Ui) {
        let c = self.colors();
        egui::Frame::none()
            .fill(c.toolbar_bg)
            .inner_margin(egui::Margin::symmetric(12.0, 8.0))
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    ui.spacing_mut().item_spacing.y = 4.0;
                    self.toolbar_button(ui, "B", "太字 (Ctrl+B)", EditorAction::Wrap { prefix: "**", suffix: "**" });
                    self.toolbar_button(ui, "I", "斜体 (Ctrl+I)", EditorAction::Wrap { prefix: "*", suffix: "*" });
                    self.toolbar_button(ui, "S", "打ち消し", EditorAction::Wrap { prefix: "~~", suffix: "~~" });
                    ui.separator();
                    self.toolbar_button(ui, "H1", "見出し1", EditorAction::LinePrefix("# "));
                    self.toolbar_button(ui, "H2", "見出し2", EditorAction::LinePrefix("## "));
                    self.toolbar_button(ui, "H3", "見出し3", EditorAction::LinePrefix("### "));
                    ui.separator();
                    self.toolbar_button(ui, "</>", "インラインコード (Ctrl+E)", EditorAction::Wrap { prefix: "`", suffix: "`" });
                    self.toolbar_button(ui, "{ }", "コードブロック", EditorAction::CodeBlock(""));
                    self.toolbar_button(ui, "🔗", "リンク (Ctrl+K)", EditorAction::Insert("[リンクテキスト](https://)"));
                    self.toolbar_button(ui, "🖼", "画像", EditorAction::Insert("![alt](path/to/image.png)"));
                    ui.separator();
                    self.toolbar_button(ui, "•", "箇条書き (Ctrl+Shift+L)", EditorAction::LinePrefix("- "));
                    self.toolbar_button(ui, "1.", "番号付きリスト", EditorAction::LinePrefix("1. "));
                    self.toolbar_button(ui, "☑", "ToDo (Ctrl+Shift+T)", EditorAction::LinePrefix("- [ ] "));
                    self.toolbar_button(ui, "❝", "引用 (Ctrl+Shift+Q)", EditorAction::LinePrefix("> "));
                    self.toolbar_button(ui, "—", "水平線", EditorAction::Insert("\n---\n"));
                    self.toolbar_button(ui, "⊞", "テーブル", EditorAction::Table { rows: 2, cols: 3 });
                });
            });
    }

    fn save_current_to_file(&mut self) {
        let note = &mut self.notes[self.selected];
        let path = if let Some(p) = &note.path {
            p.clone()
        } else {
            let Some(path) = rfd::FileDialog::new()
                .add_filter("Markdown", &["md", "markdown"])
                .set_file_name(format!("{}.md", note.title))
                .save_file()
            else {
                return;
            };
            path
        };

        if fs::write(&path, &note.content).is_ok() {
            note.path = Some(path);
            note.modified = false;
            self.mark_dirty();
        }
    }

    fn open_file(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("Markdown", &["md", "markdown"])
            .pick_file()
        else {
            return;
        };

        if let Ok(content) = fs::read_to_string(&path) {
            let title = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("Untitled")
                .to_string();
            let mut note = Note::new(title, content);
            note.path = Some(path);
            self.notes.push(note);
            self.selected = self.notes.len() - 1;
            self.mark_dirty();
        }
    }

    fn new_note(&mut self) {
        self.notes.push(Note::default());
        self.selected = self.notes.len() - 1;
        self.mark_dirty();
    }

    fn move_to_trash(&mut self, idx: usize) {
        if idx >= self.notes.len() {
            return;
        }
        self.notes[idx].trashed = true;
        self.notes[idx].touch();
        // Move selection to first non-trashed note
        if self.selected == idx {
            let next = self.notes.iter().position(|n| !n.trashed).unwrap_or(0);
            self.selected = next;
        }
        self.mark_dirty();
    }

    fn restore_from_trash(&mut self, idx: usize) {
        if idx >= self.notes.len() {
            return;
        }
        self.notes[idx].trashed = false;
        self.notes[idx].touch();
        self.mark_dirty();
    }

    fn delete_permanently(&mut self, idx: usize) {
        if idx >= self.notes.len() {
            return;
        }
        self.notes.remove(idx);
        if self.selected >= self.notes.len() {
            self.selected = self.notes.len().saturating_sub(1);
        }
        self.mark_dirty();
    }

    fn toggle_star(&mut self, idx: usize) {
        if idx >= self.notes.len() {
            return;
        }
        self.notes[idx].starred = !self.notes[idx].starred;
        self.notes[idx].touch();
        self.mark_dirty();
    }

    fn ensure_valid_selection(&mut self) {
        let visible: Vec<usize> = self
            .notes
            .iter()
            .enumerate()
            .filter(|(_, n)| n.trashed == self.show_trash)
            .map(|(i, _)| i)
            .collect();

        if !visible.contains(&self.selected) {
            if let Some(&first) = visible.first() {
                self.selected = first;
            } else {
                // No notes in current view — create one if we're in normal view
                if !self.show_trash {
                    self.notes.push(Note::default());
                    self.selected = self.notes.len() - 1;
                    self.mark_dirty();
                }
            }
        }
    }

    fn draw_sidebar(&mut self, ui: &mut Ui) {
        let c = self.colors();

        // Header
        egui::Frame::none()
            .fill(c.sidebar_bg)
            .inner_margin(egui::Margin::symmetric(12.0, 10.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("☰  Markdown Editor")
                            .color(c.text_strong)
                            .size(15.0)
                            .strong(),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .button(RichText::new("+").size(18.0).color(c.accent))
                            .on_hover_text("新規ノート (Ctrl+N)")
                            .clicked()
                        {
                            self.new_note();
                        }
                    });
                });
            });

        ui.add(egui::Separator::default().spacing(0.0).grow(0.0));

        // Search
        egui::Frame::none()
            .fill(c.sidebar_bg)
            .inner_margin(egui::Margin::symmetric(10.0, 8.0))
            .show(ui, |ui| {
                ui.add(
                    TextEdit::singleline(&mut self.search_query)
                        .hint_text("🔍  検索...")
                        .desired_width(f32::INFINITY)
                        .font(FontId::new(13.0, FontFamily::Proportional)),
                );
            });

        ui.add(egui::Separator::default().spacing(0.0).grow(0.0));

        // View tabs: All / Starred / Trash
        egui::Frame::none()
            .fill(c.sidebar_bg)
            .inner_margin(egui::Margin::symmetric(8.0, 4.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let all_btn = egui::SelectableLabel::new(
                        !self.show_trash,
                        RichText::new("📝 ノート").size(12.0).color(c.text_normal),
                    );
                    if ui.add(all_btn).clicked() {
                        self.show_trash = false;
                    }
                    let trash_btn = egui::SelectableLabel::new(
                        self.show_trash,
                        RichText::new(format!(
                            "🗑 ゴミ箱 ({})",
                            self.notes.iter().filter(|n| n.trashed).count()
                        ))
                        .size(12.0)
                        .color(c.text_normal),
                    );
                    if ui.add(trash_btn).clicked() {
                        self.show_trash = true;
                    }
                });
            });

        ui.add(egui::Separator::default().spacing(0.0).grow(0.0));

        // Note list
        egui::Frame::none().fill(c.sidebar_bg).show(ui, |ui| {
            ui.add_space(4.0);
            ScrollArea::vertical().show(ui, |ui| {
                ui.set_width(ui.available_width());
                let query = self.search_query.to_lowercase();
                let show_trash = self.show_trash;
                let indices: Vec<usize> = (0..self.notes.len())
                    .filter(|&i| self.notes[i].trashed == show_trash)
                    .filter(|&i| {
                        query.is_empty()
                            || self.notes[i].title.to_lowercase().contains(&query)
                            || self.notes[i].content.to_lowercase().contains(&query)
                            || self.notes[i].tags.iter().any(|t| t.to_lowercase().contains(&query))
                    })
                    .collect();

                let mut new_selected = self.selected;
                let mut toggle_star_idx: Option<usize> = None;
                let mut trash_idx: Option<usize> = None;
                let mut restore_idx: Option<usize> = None;
                let mut delete_idx: Option<usize> = None;

                for i in indices {
                    let is_selected = i == self.selected;
                    let title = self.notes[i].display_title();
                    let starred = self.notes[i].starred;
                    let preview: String = self.notes[i]
                        .content
                        .lines()
                        .find(|l| !l.trim_start_matches('#').trim().is_empty())
                        .unwrap_or("")
                        .trim_start_matches('#')
                        .trim()
                        .chars()
                        .take(40)
                        .collect();

                    let desired_height = if preview.is_empty() { 36.0 } else { 52.0 };
                    let (rect, response) = ui.allocate_exact_size(
                        egui::vec2(ui.available_width(), desired_height),
                        egui::Sense::click(),
                    );

                    if response.clicked() {
                        new_selected = i;
                    }

                    let bg = if is_selected {
                        c.selected_item_bg
                    } else if response.hovered() {
                        c.hover_item_bg
                    } else {
                        c.sidebar_bg
                    };
                    ui.painter().rect_filled(rect, 4.0, bg);

                    // Star icon
                    let star_rect = egui::Rect::from_min_size(
                        rect.right_top() + egui::vec2(-30.0, 8.0),
                        egui::vec2(20.0, 20.0),
                    );
                    let star_resp = ui.interact(
                        star_rect,
                        egui::Id::new(("star", i)),
                        egui::Sense::click(),
                    );
                    let star_color = if starred {
                        Color32::from_rgb(255, 200, 60)
                    } else {
                        c.text_dim
                    };
                    ui.painter().text(
                        star_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        if starred { "★" } else { "☆" },
                        egui::FontId::new(14.0, FontFamily::Proportional),
                        star_color,
                    );
                    if star_resp.clicked() {
                        toggle_star_idx = Some(i);
                    }

                    let title_color = if is_selected {
                        Color32::WHITE
                    } else {
                        c.text_normal
                    };
                    let title_pos = rect.min + egui::vec2(12.0, 10.0);
                    ui.painter().text(
                        title_pos,
                        egui::Align2::LEFT_TOP,
                        &title,
                        egui::FontId::new(13.0, FontFamily::Proportional),
                        title_color,
                    );

                    if !preview.is_empty() {
                        let preview_color = if is_selected {
                            Color32::from_rgb(220, 245, 235)
                        } else {
                            c.text_dim
                        };
                        let preview_pos = rect.min + egui::vec2(12.0, 28.0);
                        ui.painter().text(
                            preview_pos,
                            egui::Align2::LEFT_TOP,
                            &preview,
                            egui::FontId::new(11.0, FontFamily::Proportional),
                            preview_color,
                        );
                    }

                    // Right-click context menu
                    response.context_menu(|ui| {
                        if !show_trash {
                            if ui.button(if starred { "★ お気に入り解除" } else { "☆ お気に入りに追加" }).clicked() {
                                toggle_star_idx = Some(i);
                                ui.close_menu();
                            }
                            if ui.button("🗑 ゴミ箱へ").clicked() {
                                trash_idx = Some(i);
                                ui.close_menu();
                            }
                        } else {
                            if ui.button("↩ 元に戻す").clicked() {
                                restore_idx = Some(i);
                                ui.close_menu();
                            }
                            if ui.button("❌ 完全に削除").clicked() {
                                delete_idx = Some(i);
                                ui.close_menu();
                            }
                        }
                    });

                    ui.add_space(2.0);
                }

                self.selected = new_selected;
                if let Some(i) = toggle_star_idx {
                    self.toggle_star(i);
                }
                if let Some(i) = trash_idx {
                    self.move_to_trash(i);
                }
                if let Some(i) = restore_idx {
                    self.restore_from_trash(i);
                }
                if let Some(i) = delete_idx {
                    self.delete_permanently(i);
                }
            });
        });
    }

    fn draw_editor(&mut self, ui: &mut Ui) {
        let c = self.colors();
        let path_label = self.notes[self.selected]
            .path
            .as_ref()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "[未保存]".to_string());

        egui::Frame::none()
            .fill(c.header_bg)
            .inner_margin(egui::Margin::symmetric(12.0, 6.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    // Title editor (single-line)
                    let note = &mut self.notes[self.selected];
                    let title_resp = ui.add(
                        TextEdit::singleline(&mut note.title)
                            .desired_width(220.0)
                            .frame(false)
                            .font(FontId::new(14.0, FontFamily::Proportional))
                            .text_color(c.text_strong),
                    );
                    if title_resp.changed() {
                        note.modified = true;
                        note.touch();
                        self.notes_dirty = true;
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // View mode toggle buttons (right side)
                        let mode = self.settings.view_mode;
                        if ui
                            .selectable_label(mode == ViewMode::PreviewOnly, "👁")
                            .on_hover_text("プレビューのみ")
                            .clicked()
                        {
                            self.settings.view_mode = ViewMode::PreviewOnly;
                            self.save_settings();
                        }
                        if ui
                            .selectable_label(mode == ViewMode::Split, "⫼")
                            .on_hover_text("分割表示")
                            .clicked()
                        {
                            self.settings.view_mode = ViewMode::Split;
                            self.save_settings();
                        }
                        if ui
                            .selectable_label(mode == ViewMode::EditorOnly, "📝")
                            .on_hover_text("編集のみ")
                            .clicked()
                        {
                            self.settings.view_mode = ViewMode::EditorOnly;
                            self.save_settings();
                        }
                        ui.add_space(8.0);
                        ui.label(RichText::new(path_label).color(c.text_dim).size(11.0));
                    });
                });

                // Tag editing row
                ui.horizontal(|ui| {
                    ui.label(RichText::new("🏷").color(c.text_dim).size(11.0));
                    let note = &mut self.notes[self.selected];
                    let mut tags_joined = note.tags.join(", ");
                    let resp = ui.add(
                        TextEdit::singleline(&mut tags_joined)
                            .hint_text("タグをカンマ区切りで追加...")
                            .desired_width(f32::INFINITY)
                            .font(FontId::new(11.0, FontFamily::Proportional))
                            .text_color(c.text_dim),
                    );
                    if resp.changed() {
                        note.tags = tags_joined
                            .split(',')
                            .map(|s| s.trim().to_string())
                            .filter(|s| !s.is_empty())
                            .collect();
                        note.touch();
                        self.notes_dirty = true;
                    }
                });
            });

        self.draw_toolbar(ui);

        ui.add(egui::Separator::default().spacing(0.0).grow(0.0));

        let font_size = self.settings.editor_font_size;
        let show_line_numbers = self.settings.show_line_numbers;
        let word_wrap = self.settings.word_wrap;

        let scroll = if word_wrap {
            ScrollArea::vertical()
        } else {
            ScrollArea::both()
        };

        scroll
            .id_salt("editor_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.horizontal_top(|ui| {
                    if show_line_numbers {
                        let note = &self.notes[self.selected];
                        let line_count = note.content.lines().count().max(1);
                        let line_nums: String =
                            (1..=line_count).map(|n| format!("{}\n", n)).collect();
                        ui.add(
                            TextEdit::multiline(&mut line_nums.as_str())
                                .desired_width(36.0)
                                .frame(false)
                                .interactive(false)
                                .font(FontId::new(font_size, FontFamily::Monospace))
                                .text_color(c.line_number),
                        );
                        ui.add(
                            egui::Separator::default().vertical().spacing(0.0).grow(0.0),
                        );
                    }

                    let note = &mut self.notes[self.selected];
                    let prev_content = note.content.clone();
                    let editor_id = egui::Id::new(EDITOR_ID);
                    let syntax_highlight = self.settings.syntax_highlight;
                    let theme_mode = self.settings.theme;
                    let text_color = c.text_normal;

                    let mut layouter = |ui: &Ui, text: &str, wrap_width: f32| {
                        let mut job = if syntax_highlight {
                            highlight::layout_markdown(text, font_size, theme_mode, text_color)
                        } else {
                            let mut j = egui::text::LayoutJob::default();
                            j.append(
                                text,
                                0.0,
                                egui::text::TextFormat {
                                    font_id: FontId::new(font_size, FontFamily::Monospace),
                                    color: text_color,
                                    ..Default::default()
                                },
                            );
                            j
                        };
                        if word_wrap {
                            job.wrap.max_width = wrap_width;
                        }
                        ui.fonts(|f| f.layout_job(job))
                    };

                    let mut editor = TextEdit::multiline(&mut note.content)
                        .id(editor_id)
                        .desired_rows(40)
                        .frame(false)
                        .font(FontId::new(font_size, FontFamily::Monospace))
                        .text_color(c.text_normal)
                        .lock_focus(true)
                        .layouter(&mut layouter);
                    if word_wrap {
                        editor = editor.desired_width(f32::INFINITY);
                    } else {
                        editor = editor.desired_width(2000.0);
                    }
                    let output = editor.show(ui);

                    if note.content != prev_content {
                        note.modified = true;
                        note.touch();
                        self.notes_dirty = true;
                    }

                    // Detect Enter for list continuation
                    let enter_pressed = ui.ctx().input(|i| {
                        i.key_pressed(egui::Key::Enter)
                            && !i.modifiers.ctrl
                            && !i.modifiers.shift
                            && !i.modifiers.alt
                    });
                    if enter_pressed && output.response.has_focus() {
                        self.pending_list_continuation = true;
                    }

                    // Apply pending markdown action (toolbar/shortcut)
                    if let Some(action) = self.pending_action.take() {
                        let (sel_start, sel_end) = if let Some(range) = output.cursor_range {
                            (range.primary.ccursor.index, range.secondary.ccursor.index)
                        } else {
                            let end = note.content.chars().count();
                            (end, end)
                        };
                        let result =
                            editor_actions::apply(action, &note.content, sel_start, sel_end);
                        note.content = result.new_content;
                        note.modified = true;
                        note.touch();
                        self.notes_dirty = true;

                        let mut state = output.state.clone();
                        let new_range = egui::text::CCursorRange::two(
                            egui::text::CCursor::new(result.new_cursor_start),
                            egui::text::CCursor::new(result.new_cursor_end),
                        );
                        state.cursor.set_char_range(Some(new_range));
                        state.store(ui.ctx(), editor_id);
                        ui.ctx().memory_mut(|m| m.request_focus(editor_id));
                    }

                    // Apply line move (Alt+Up/Down)
                    if let Some(up) = self.pending_line_move.take() {
                        let (sel_start, sel_end) = if let Some(range) = output.cursor_range {
                            (range.primary.ccursor.index, range.secondary.ccursor.index)
                        } else {
                            (0, 0)
                        };
                        if let Some(result) =
                            editor_actions::move_lines(&note.content, sel_start, sel_end, up)
                        {
                            note.content = result.new_content;
                            note.modified = true;
                            note.touch();
                            self.notes_dirty = true;

                            let mut state = output.state.clone();
                            let new_range = egui::text::CCursorRange::two(
                                egui::text::CCursor::new(result.new_cursor_start),
                                egui::text::CCursor::new(result.new_cursor_end),
                            );
                            state.cursor.set_char_range(Some(new_range));
                            state.store(ui.ctx(), editor_id);
                            ui.ctx().memory_mut(|m| m.request_focus(editor_id));
                        }
                    }

                    // Apply list continuation (Enter pressed)
                    if self.pending_list_continuation {
                        self.pending_list_continuation = false;
                        // Cursor is now after the inserted newline; we need to handle
                        // the case AFTER egui inserted the '\n'. Re-derive cursor.
                        if let Some(range) = output.cursor_range {
                            let cursor = range.primary.ccursor.index;
                            // We want to inspect the line that ended at cursor-1 (the just-broken line).
                            // After egui inserts \n, the previous line is the marker line.
                            // We need to look at line BEFORE cursor.
                            let chars: Vec<char> = note.content.chars().collect();
                            if cursor > 0 && cursor <= chars.len() && chars[cursor - 1] == '\n' {
                                // Find prev line start
                                let mut prev_start = cursor - 1;
                                while prev_start > 0 && chars[prev_start - 1] != '\n' {
                                    prev_start -= 1;
                                }
                                let prev_line: String = chars[prev_start..cursor - 1].iter().collect();
                                if let Some(marker) = detect_list_marker(&prev_line) {
                                    if marker.content_empty {
                                        // Remove the marker on prev line AND the newline (exit list)
                                        let new_content: String = chars[..prev_start]
                                            .iter()
                                            .chain(chars[cursor..].iter())
                                            .collect();
                                        let new_pos = prev_start;
                                        note.content = new_content;
                                        let mut state = output.state.clone();
                                        let new_range = egui::text::CCursorRange::two(
                                            egui::text::CCursor::new(new_pos),
                                            egui::text::CCursor::new(new_pos),
                                        );
                                        state.cursor.set_char_range(Some(new_range));
                                        state.store(ui.ctx(), editor_id);
                                    } else {
                                        // Insert marker at cursor
                                        let insertion: String = format!("{}{}", marker.indent, marker.next_marker);
                                        let new_content: String = chars[..cursor]
                                            .iter()
                                            .collect::<String>()
                                            + &insertion
                                            + &chars[cursor..].iter().collect::<String>();
                                        let new_pos = cursor + insertion.chars().count();
                                        note.content = new_content;
                                        let mut state = output.state.clone();
                                        let new_range = egui::text::CCursorRange::two(
                                            egui::text::CCursor::new(new_pos),
                                            egui::text::CCursor::new(new_pos),
                                        );
                                        state.cursor.set_char_range(Some(new_range));
                                        state.store(ui.ctx(), editor_id);
                                    }
                                    note.modified = true;
                                    note.touch();
                                    self.notes_dirty = true;
                                }
                            }
                        }
                    }
                });
            });
    }

    fn draw_preview(&mut self, ui: &mut Ui) {
        let c = self.colors();
        let raw = &self.notes[self.selected].content;
        let content = wikilinks::render_for_preview(raw);

        egui::Frame::none().fill(c.preview_bg).show(ui, |ui| {
            egui::Frame::none()
                .fill(c.header_bg)
                .inner_margin(egui::Margin::symmetric(12.0, 6.0))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new("👁  Preview")
                                .color(c.text_dim)
                                .size(12.0),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let mode = self.settings.view_mode;
                            if ui
                                .selectable_label(mode == ViewMode::PreviewOnly, "👁")
                                .on_hover_text("プレビューのみ")
                                .clicked()
                            {
                                self.settings.view_mode = ViewMode::PreviewOnly;
                                self.save_settings();
                            }
                            if ui
                                .selectable_label(mode == ViewMode::Split, "⫼")
                                .on_hover_text("分割表示")
                                .clicked()
                            {
                                self.settings.view_mode = ViewMode::Split;
                                self.save_settings();
                            }
                            if ui
                                .selectable_label(mode == ViewMode::EditorOnly, "📝")
                                .on_hover_text("編集のみ")
                                .clicked()
                            {
                                self.settings.view_mode = ViewMode::EditorOnly;
                                self.save_settings();
                            }
                        });
                    });
                });

            ui.add(egui::Separator::default().spacing(0.0).grow(0.0));

            ScrollArea::vertical()
                .id_salt("preview_scroll")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.set_max_width(ui.available_width());
                    egui::Frame::none()
                        .inner_margin(egui::Margin::symmetric(20.0, 16.0))
                        .show(ui, |ui| {
                            ui.set_max_width(ui.available_width());
                            CommonMarkViewer::new()
                                .max_image_width(Some(600))
                                .show(ui, &mut self.cache, &content);
                        });
                });
        });
    }

    fn draw_settings_window(&mut self, ctx: &egui::Context) {
        if !self.show_settings {
            return;
        }
        let mut open = self.show_settings;
        let mut settings_changed = false;
        let mut font_changed = false;
        egui::Window::new("⚙ 設定")
            .open(&mut open)
            .resizable(false)
            .default_width(360.0)
            .show(ctx, |ui| {
                ui.label(RichText::new("外観").strong().size(14.0));
                ui.horizontal(|ui| {
                    ui.label("テーマ:");
                    if ui
                        .selectable_label(self.settings.theme == ThemeMode::Dark, "🌙 ダーク")
                        .clicked()
                    {
                        self.settings.theme = ThemeMode::Dark;
                        settings_changed = true;
                    }
                    if ui
                        .selectable_label(self.settings.theme == ThemeMode::Light, "☀ ライト")
                        .clicked()
                    {
                        self.settings.theme = ThemeMode::Light;
                        settings_changed = true;
                    }
                });

                ui.add_space(8.0);
                ui.label(RichText::new("エディタ").strong().size(14.0));

                ui.horizontal(|ui| {
                    ui.label("フォント:");
                    let current = self.settings.font_choice;
                    egui::ComboBox::from_id_salt("font_choice")
                        .selected_text(current.display_name())
                        .show_ui(ui, |ui| {
                            for &choice in FontChoice::all() {
                                if ui
                                    .selectable_label(current == choice, choice.display_name())
                                    .clicked()
                                {
                                    self.settings.font_choice = choice;
                                    font_changed = true;
                                    settings_changed = true;
                                }
                            }
                        });
                });

                ui.horizontal(|ui| {
                    ui.label("フォントサイズ:");
                    if ui
                        .add(
                            egui::DragValue::new(&mut self.settings.editor_font_size)
                                .range(8.0..=32.0)
                                .speed(0.5)
                                .suffix(" px"),
                        )
                        .changed()
                    {
                        settings_changed = true;
                    }
                });
                if ui
                    .checkbox(&mut self.settings.show_line_numbers, "行番号を表示")
                    .changed()
                {
                    settings_changed = true;
                }
                if ui
                    .checkbox(&mut self.settings.word_wrap, "テキスト折り返し")
                    .changed()
                {
                    settings_changed = true;
                }
                if ui
                    .checkbox(&mut self.settings.auto_save, "自動保存")
                    .changed()
                {
                    settings_changed = true;
                }
                if ui
                    .checkbox(&mut self.settings.syntax_highlight, "シンタックスハイライト")
                    .changed()
                {
                    settings_changed = true;
                }

                ui.add_space(8.0);
                ui.label(RichText::new("ストレージ").strong().size(14.0));
                ui.horizontal(|ui| {
                    ui.label("データ保存先:");
                    ui.monospace(storage::data_dir().display().to_string());
                });
                if ui.button("📂 データフォルダを開く").clicked() {
                    let path = storage::data_dir();
                    #[cfg(target_os = "windows")]
                    let _ = std::process::Command::new("explorer").arg(path).spawn();
                }
            });
        self.show_settings = open;
        if font_changed {
            load_japanese_font(ctx, self.settings.font_choice);
        }
        if settings_changed {
            theme::apply(ctx, self.settings.theme, self.settings.editor_font_size);
            self.save_settings();
        }
    }

    fn draw_find_bar(&mut self, ctx: &egui::Context) {
        if !self.find.visible {
            return;
        }
        let c = self.colors();
        egui::TopBottomPanel::top("find_bar")
            .frame(
                egui::Frame::none()
                    .fill(c.toolbar_bg)
                    .inner_margin(egui::Margin::symmetric(8.0, 6.0)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("🔍").color(c.text_dim));
                    let query_resp = ui.add(
                        TextEdit::singleline(&mut self.find.query)
                            .hint_text("検索...")
                            .desired_width(200.0),
                    );
                    if self.find.focus_query {
                        query_resp.request_focus();
                        self.find.focus_query = false;
                    }

                    // Match count
                    let matches = find_replace::find_all(
                        &self.notes[self.selected].content,
                        &self.find.query,
                        self.find.case_sensitive,
                    );
                    let total = matches.len();
                    let current_display = if total == 0 {
                        "0/0".to_string()
                    } else {
                        let cur = self.find.current_match.min(total.saturating_sub(1)) + 1;
                        format!("{}/{}", cur, total)
                    };
                    ui.label(RichText::new(current_display).color(c.text_dim).size(11.0));

                    if ui.button("▲").on_hover_text("前へ (Shift+Enter)").clicked() {
                        if total > 0 {
                            self.find.current_match = (self.find.current_match + total - 1) % total;
                            self.jump_to_match(ctx, &matches);
                        }
                    }
                    if ui.button("▼").on_hover_text("次へ (Enter)").clicked() {
                        if total > 0 {
                            self.find.current_match = (self.find.current_match + 1) % total;
                            self.jump_to_match(ctx, &matches);
                        }
                    }
                    ui.checkbox(&mut self.find.case_sensitive, "Aa")
                        .on_hover_text("大文字小文字を区別");

                    if ui.button("✖").on_hover_text("閉じる (Esc)").clicked() {
                        self.find.close();
                    }
                });

                if self.find.show_replace {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("↳").color(c.text_dim));
                        ui.add(
                            TextEdit::singleline(&mut self.find.replace_with)
                                .hint_text("置換後...")
                                .desired_width(200.0),
                        );
                        if ui.button("置換").clicked() {
                            self.replace_current_match();
                        }
                        if ui.button("すべて置換").clicked() {
                            self.replace_all();
                        }
                    });
                }
            });

        // Enter key while focused on the search bar jumps to next match
        if self.find.visible {
            let enter = ctx.input(|i| i.key_pressed(egui::Key::Enter));
            let shift = ctx.input(|i| i.modifiers.shift);
            if enter {
                let matches = find_replace::find_all(
                    &self.notes[self.selected].content,
                    &self.find.query,
                    self.find.case_sensitive,
                );
                let total = matches.len();
                if total > 0 {
                    if shift {
                        self.find.current_match = (self.find.current_match + total - 1) % total;
                    } else {
                        self.find.current_match = (self.find.current_match + 1) % total;
                    }
                    self.jump_to_match(ctx, &matches);
                }
            }
        }
    }

    fn jump_to_match(&self, ctx: &egui::Context, matches: &[(usize, usize)]) {
        if matches.is_empty() {
            return;
        }
        let (b_start, b_end) = matches[self.find.current_match.min(matches.len() - 1)];
        let content = &self.notes[self.selected].content;
        let char_start = content[..b_start].chars().count();
        let char_end = content[..b_end].chars().count();
        let editor_id = egui::Id::new(EDITOR_ID);
        if let Some(mut state) = egui::TextEdit::load_state(ctx, editor_id) {
            let new_range = egui::text::CCursorRange::two(
                egui::text::CCursor::new(char_start),
                egui::text::CCursor::new(char_end),
            );
            state.cursor.set_char_range(Some(new_range));
            state.store(ctx, editor_id);
        }
    }

    fn replace_current_match(&mut self) {
        let matches = find_replace::find_all(
            &self.notes[self.selected].content,
            &self.find.query,
            self.find.case_sensitive,
        );
        if matches.is_empty() {
            return;
        }
        let idx = self.find.current_match.min(matches.len() - 1);
        let (b_start, b_end) = matches[idx];
        let content = &self.notes[self.selected].content;
        let new_content = format!(
            "{}{}{}",
            &content[..b_start],
            self.find.replace_with,
            &content[b_end..]
        );
        let note = &mut self.notes[self.selected];
        note.content = new_content;
        note.modified = true;
        note.touch();
        self.notes_dirty = true;
    }

    fn replace_all(&mut self) {
        let (new_content, count) = find_replace::replace_all(
            &self.notes[self.selected].content,
            &self.find.query,
            &self.find.replace_with,
            self.find.case_sensitive,
        );
        if count > 0 {
            let note = &mut self.notes[self.selected];
            note.content = new_content;
            note.modified = true;
            note.touch();
            self.notes_dirty = true;
        }
    }

    fn select_note_by_index(&mut self, idx: usize) {
        if idx < self.notes.len() {
            self.selected = idx;
        }
    }

    fn navigate_to_wikilink(&mut self, target: &str) {
        let matches = wikilinks::resolve(&self.notes, target);
        if let Some(&idx) = matches.first() {
            self.selected = idx;
        } else {
            // Create a new note with that title
            let mut note = Note::new(target.to_string(), format!("# {}\n\n", target));
            note.modified = true;
            self.notes.push(note);
            self.selected = self.notes.len() - 1;
            self.notes_dirty = true;
        }
    }

    fn draw_backlinks_panel(&mut self, ui: &mut Ui) {
        let c = self.colors();
        let selected = self.selected;
        let current_title = self.notes[selected].title.clone();

        let backlinks_index = wikilinks::build_backlink_index(&self.notes);
        let backlinks = backlinks_index.get(&selected).cloned().unwrap_or_default();
        let outgoing = wikilinks::extract(&self.notes[selected].content);

        egui::Frame::none()
            .fill(c.header_bg)
            .inner_margin(egui::Margin::symmetric(12.0, 6.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("🔗  リンク").color(c.text_dim).size(12.0));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button("✖").on_hover_text("閉じる").clicked() {
                            self.show_backlinks = false;
                        }
                    });
                });
            });
        ui.add(egui::Separator::default().spacing(0.0).grow(0.0));

        let mut nav_target: Option<NavTarget> = None;

        ScrollArea::vertical()
            .id_salt("backlinks_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.add_space(4.0);

                // Outgoing links
                ui.label(
                    RichText::new(format!("  📤 アウトゴーイング ({})", outgoing.len()))
                        .color(c.text_dim)
                        .size(11.0)
                        .strong(),
                );
                if outgoing.is_empty() {
                    ui.label(
                        RichText::new("    (なし)")
                            .color(c.text_dim)
                            .size(11.0)
                            .italics(),
                    );
                } else {
                    for link in &outgoing {
                        let display = link.alias.as_deref().unwrap_or(&link.target);
                        let exists = !wikilinks::resolve(&self.notes, &link.target).is_empty();
                        let color = if exists { c.accent } else { c.text_dim };
                        let prefix = if exists { "  → " } else { "  ✚ " };
                        let resp = ui.add(
                            egui::Label::new(
                                RichText::new(format!("{}{}", prefix, display))
                                    .color(color)
                                    .size(11.0),
                            )
                            .sense(egui::Sense::click()),
                        );
                        if resp.clicked() {
                            nav_target = Some(NavTarget::Wiki(link.target.clone()));
                        }
                        if !exists {
                            resp.on_hover_text("クリックで新規ノート作成");
                        }
                    }
                }

                ui.add_space(8.0);
                ui.label(
                    RichText::new(format!("  📥 バックリンク ({})", backlinks.len()))
                        .color(c.text_dim)
                        .size(11.0)
                        .strong(),
                );
                if backlinks.is_empty() {
                    ui.label(
                        RichText::new("    (なし)")
                            .color(c.text_dim)
                            .size(11.0)
                            .italics(),
                    );
                } else {
                    for (src_idx, link) in &backlinks {
                        let src_title = &self.notes[*src_idx].title;
                        let label = if let Some(alias) = &link.alias {
                            format!("  ← {} ({})", src_title, alias)
                        } else {
                            format!("  ← {}", src_title)
                        };
                        let resp = ui.add(
                            egui::Label::new(
                                RichText::new(label).color(c.text_normal).size(11.0),
                            )
                            .sense(egui::Sense::click()),
                        );
                        if resp.clicked() {
                            nav_target = Some(NavTarget::Index(*src_idx));
                        }
                    }
                }
                let _ = current_title;
            });

        match nav_target {
            Some(NavTarget::Wiki(target)) => self.navigate_to_wikilink(&target),
            Some(NavTarget::Index(idx)) => self.select_note_by_index(idx),
            None => {}
        }
    }

    fn draw_quick_switcher(&mut self, ctx: &egui::Context) {
        if !self.quick_switcher.visible {
            return;
        }
        let c = self.colors();
        let mut close = false;
        let mut navigate_to: Option<usize> = None;

        // Build filtered candidates
        let query = self.quick_switcher.query.clone();
        let mut candidates: Vec<(i32, usize)> = self
            .notes
            .iter()
            .enumerate()
            .filter(|(_, n)| !n.trashed)
            .filter_map(|(i, n)| wikilinks::fuzzy_match(&n.title, &query).map(|s| (s, i)))
            .collect();
        candidates.sort_by_key(|(score, _)| *score);
        candidates.truncate(20);

        if self.quick_switcher.selected >= candidates.len() {
            self.quick_switcher.selected = 0;
        }

        // Key navigation
        let (key_up, key_down, key_enter) = ctx.input(|i| {
            (
                i.key_pressed(egui::Key::ArrowUp),
                i.key_pressed(egui::Key::ArrowDown),
                i.key_pressed(egui::Key::Enter),
            )
        });
        if !candidates.is_empty() {
            if key_down {
                self.quick_switcher.selected =
                    (self.quick_switcher.selected + 1) % candidates.len();
            }
            if key_up {
                self.quick_switcher.selected = (self.quick_switcher.selected + candidates.len() - 1)
                    % candidates.len();
            }
            if key_enter {
                navigate_to = Some(candidates[self.quick_switcher.selected].1);
                close = true;
            }
        } else if key_enter && !query.is_empty() {
            // Create new note with the query as title
            let mut note = Note::new(query.clone(), format!("# {}\n\n", query));
            note.modified = true;
            self.notes.push(note);
            navigate_to = Some(self.notes.len() - 1);
            self.notes_dirty = true;
            close = true;
        }

        egui::Window::new("クイックスイッチャー")
            .title_bar(false)
            .resizable(false)
            .collapsible(false)
            .anchor(egui::Align2::CENTER_TOP, [0.0, 80.0])
            .default_width(480.0)
            .frame(
                egui::Frame::popup(&ctx.style())
                    .fill(c.toolbar_bg)
                    .rounding(8.0)
                    .inner_margin(egui::Margin::same(10.0)),
            )
            .show(ctx, |ui| {
                ui.set_width(460.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("🔎").size(14.0));
                    let resp = ui.add(
                        TextEdit::singleline(&mut self.quick_switcher.query)
                            .hint_text("ノート名を検索... (Ctrl+P)")
                            .desired_width(f32::INFINITY)
                            .font(FontId::new(14.0, FontFamily::Proportional))
                            .frame(false),
                    );
                    if self.quick_switcher.focus_query {
                        resp.request_focus();
                        self.quick_switcher.focus_query = false;
                    }
                });
                ui.add_space(6.0);
                ui.add(egui::Separator::default().spacing(0.0).grow(0.0));
                ui.add_space(4.0);

                if candidates.is_empty() {
                    if query.is_empty() {
                        ui.label(
                            RichText::new("ノート名を入力してください")
                                .color(c.text_dim)
                                .italics(),
                        );
                    } else {
                        ui.label(
                            RichText::new(format!(
                                "Enterで「{}」という新規ノートを作成",
                                query
                            ))
                            .color(c.accent),
                        );
                    }
                } else {
                    for (rank, (_, idx)) in candidates.iter().enumerate() {
                        let is_active = rank == self.quick_switcher.selected;
                        let note = &self.notes[*idx];
                        let bg = if is_active {
                            c.selected_item_bg
                        } else {
                            egui::Color32::TRANSPARENT
                        };
                        let resp = egui::Frame::none()
                            .fill(bg)
                            .rounding(4.0)
                            .inner_margin(egui::Margin::symmetric(8.0, 4.0))
                            .show(ui, |ui| {
                                ui.set_width(ui.available_width());
                                ui.horizontal(|ui| {
                                    ui.label(
                                        RichText::new(if note.starred { "★ " } else { "  " })
                                            .color(egui::Color32::from_rgb(255, 200, 60)),
                                    );
                                    ui.label(
                                        RichText::new(&note.title)
                                            .color(if is_active {
                                                egui::Color32::WHITE
                                            } else {
                                                c.text_normal
                                            })
                                            .size(13.0),
                                    );
                                });
                            })
                            .response
                            .interact(egui::Sense::click());
                        if resp.clicked() {
                            navigate_to = Some(*idx);
                            close = true;
                        }
                        if resp.hovered() {
                            self.quick_switcher.selected = rank;
                        }
                    }
                }
                ui.add_space(4.0);
                ui.add(egui::Separator::default().spacing(0.0).grow(0.0));
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("↑↓ 移動  /  Enter 選択  /  Esc 閉じる")
                            .color(c.text_dim)
                            .size(10.0),
                    );
                });
            });

        if let Some(idx) = navigate_to {
            self.selected = idx;
        }
        if close {
            self.quick_switcher.close();
        }
    }

    fn try_paste_image(&mut self, force_message: bool) {
        let result = attachments::paste_clipboard_image();
        match result {
            Ok(Some(path)) => {
                let md = attachments::markdown_link_for(&path);
                let leaked: &'static str = Box::leak(md.into_boxed_str());
                self.pending_action = Some(EditorAction::Insert(leaked));
                self.status_override = Some(format!(
                    "画像を貼り付けました: {}",
                    path.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("image.png")
                ));
                self.status_override_at = Some(Instant::now());
            }
            Ok(None) => {
                if force_message {
                    self.status_override =
                        Some("クリップボードに画像はありません".to_string());
                    self.status_override_at = Some(Instant::now());
                }
            }
            Err(msg) => {
                self.status_override = Some(format!("画像の貼り付け失敗: {}", msg));
                self.status_override_at = Some(Instant::now());
            }
        }
    }

    fn export_html(&self) {
        let note = &self.notes[self.selected];
        let Some(path) = rfd::FileDialog::new()
            .add_filter("HTML", &["html", "htm"])
            .set_file_name(format!("{}.html", note.title))
            .save_file()
        else {
            return;
        };
        let html = export::markdown_to_html(&note.content, &note.title);
        let _ = std::fs::write(path, html);
    }

    fn draw_toc(&mut self, ui: &mut Ui) {
        let c = self.colors();
        egui::Frame::none()
            .fill(c.header_bg)
            .inner_margin(egui::Margin::symmetric(12.0, 6.0))
            .show(ui, |ui| {
                ui.label(RichText::new("📑  目次").color(c.text_dim).size(12.0));
            });
        ui.add(egui::Separator::default().spacing(0.0).grow(0.0));

        let content = self.notes[self.selected].content.clone();
        let headings = toc::extract(&content);

        ScrollArea::vertical()
            .id_salt("toc_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.add_space(4.0);
                if headings.is_empty() {
                    ui.label(
                        RichText::new("  見出しがありません")
                            .color(c.text_dim)
                            .size(11.0)
                            .italics(),
                    );
                    return;
                }
                let mut jump_to: Option<usize> = None;
                for h in &headings {
                    let indent = (h.level.saturating_sub(1) as f32) * 12.0;
                    let resp = ui.horizontal(|ui| {
                        ui.add_space(8.0 + indent);
                        let color = if h.level == 1 { c.text_strong } else { c.text_normal };
                        ui.add(egui::Label::new(
                            RichText::new(&h.text).size(12.0).color(color),
                        ).sense(egui::Sense::click()))
                    });
                    if resp.inner.clicked() {
                        jump_to = Some(h.char_offset);
                    }
                }
                if let Some(offset) = jump_to {
                    let editor_id = egui::Id::new(EDITOR_ID);
                    if let Some(mut state) = egui::TextEdit::load_state(ui.ctx(), editor_id) {
                        let range = egui::text::CCursorRange::two(
                            egui::text::CCursor::new(offset),
                            egui::text::CCursor::new(offset),
                        );
                        state.cursor.set_char_range(Some(range));
                        state.store(ui.ctx(), editor_id);
                    }
                    ui.ctx().memory_mut(|m| m.request_focus(editor_id));
                }
            });
    }

    fn auto_save_if_needed(&mut self) {
        if self.settings.auto_save
            && self.notes_dirty
            && self.last_save_at.elapsed().as_secs() >= AUTOSAVE_INTERVAL_SECS
        {
            self.save_notes();
        }
    }
}

enum NavTarget {
    Wiki(String),
    Index(usize),
}

struct ListMarkerInfo {
    indent: String,
    next_marker: String,
    content_empty: bool,
}

fn detect_list_marker(line: &str) -> Option<ListMarkerInfo> {
    let indent: String = line.chars().take_while(|c| *c == ' ' || *c == '\t').collect();
    let after = &line[indent.len()..];

    if let Some(rest) = after.strip_prefix("- [ ] ").or_else(|| after.strip_prefix("- [x] ")).or_else(|| after.strip_prefix("- [X] ")) {
        return Some(ListMarkerInfo {
            indent,
            next_marker: "- [ ] ".to_string(),
            content_empty: rest.trim().is_empty(),
        });
    }
    if let Some(rest) = after.strip_prefix("- ") {
        return Some(ListMarkerInfo {
            indent,
            next_marker: "- ".to_string(),
            content_empty: rest.trim().is_empty(),
        });
    }
    if let Some(rest) = after.strip_prefix("* ") {
        return Some(ListMarkerInfo {
            indent,
            next_marker: "* ".to_string(),
            content_empty: rest.trim().is_empty(),
        });
    }
    if let Some(rest) = after.strip_prefix("+ ") {
        return Some(ListMarkerInfo {
            indent,
            next_marker: "+ ".to_string(),
            content_empty: rest.trim().is_empty(),
        });
    }
    if let Some(rest) = after.strip_prefix("> ") {
        return Some(ListMarkerInfo {
            indent,
            next_marker: "> ".to_string(),
            content_empty: rest.trim().is_empty(),
        });
    }
    // Numbered: "N. "
    let digit_end = after.find(|c: char| !c.is_ascii_digit()).unwrap_or(0);
    if digit_end > 0 {
        let after_digits = &after[digit_end..];
        if let Some(rest) = after_digits.strip_prefix(". ") {
            let n: u32 = after[..digit_end].parse().unwrap_or(0);
            return Some(ListMarkerInfo {
                indent,
                next_marker: format!("{}. ", n + 1),
                content_empty: rest.trim().is_empty(),
            });
        }
    }
    None
}

fn default_notes() -> Vec<Note> {
    vec![
        Note::new(
            "Markdownとは？",
            "# Markdownとは？\n\n**Markdown(マークダウン)**とは、`# 見出し` `* リスト` などシンプルな書き方で**文書構造**を明示でき、装飾されたHTML文書に変換できる**軽量マークアップ言語**です。\n\n## 箇条書き\n\n- リスト１\n- リスト２\n- リスト３\n\n## コードブロック\n\n```rust\nfn main() {\n    println!(\"Hello, World!\");\n}\n```\n\n## テーブル\n\n| タイトル１ | タイトル２ | タイトル３ |\n|-----------|-----------|----------|\n| みかん    | りんご    | ぶどう    |\n| いちご    | もも      | なし     |\n\n## テキスト装飾\n\n**太字テキスト** と *斜体テキスト* と ~~打ち消し線~~ です。\n\n> ブロッククォート: 重要な引用文はこのように表示されます。\n\n---\n\nインラインコード: `let x = 42;`\n",
        ),
        Note::new(
            "Welcome to the editor!",
            "# Welcome!\n\nThis is a Markdown editor built with Rust and egui.\n\n## Getting Started\n\n1. Click a note in the sidebar\n2. Edit in the left pane\n3. See the preview on the right\n\n## Shortcuts\n\n- **Ctrl+S** — Save to file\n- **Ctrl+O** — Open file\n- **Ctrl+N** — New note\n- **Ctrl+B/I/E/K** — Format\n",
        ),
    ]
}

fn load_japanese_font(ctx: &egui::Context, font_choice: FontChoice) {
    let mut fonts = egui::FontDefinitions::default();

    let fallbacks: &[&str] = &[
        r"C:\Windows\Fonts\YuGothM.ttc",
        r"C:\Windows\Fonts\YuGothR.ttc",
        r"C:\Windows\Fonts\meiryo.ttc",
        r"C:\Windows\Fonts\msgothic.ttc",
    ];

    let selected = font_choice.font_candidates();
    let candidates = selected
        .iter()
        .chain(fallbacks.iter().filter(|f| !selected.contains(f)));

    for &path in candidates {
        if let Ok(bytes) = std::fs::read(path) {
            fonts.font_data.insert(
                "japanese".to_owned(),
                egui::FontData::from_owned(bytes).into(),
            );
            fonts
                .families
                .entry(egui::FontFamily::Proportional)
                .or_default()
                .push("japanese".to_owned());
            fonts
                .families
                .entry(egui::FontFamily::Monospace)
                .or_default()
                .push("japanese".to_owned());
            break;
        }
    }
    ctx.set_fonts(fonts);
}

impl eframe::App for MarkdownApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.handle_shortcuts(ctx);
        self.ensure_valid_selection();
        self.auto_save_if_needed();
        let c = self.colors();

        // Menu bar
        egui::TopBottomPanel::top("menu_bar")
            .frame(
                egui::Frame::none()
                    .fill(c.menu_bg)
                    .inner_margin(egui::Margin::symmetric(8.0, 4.0)),
            )
            .show(ctx, |ui| {
                egui::menu::bar(ui, |ui| {
                    ui.spacing_mut().item_spacing.x = 14.0;
                    ui.spacing_mut().button_padding = egui::vec2(8.0, 4.0);
                    ui.menu_button("ファイル", |ui| {
                        if ui.button("新規ノート  Ctrl+N").clicked() {
                            self.new_note();
                            ui.close_menu();
                        }
                        if ui.button("開く...  Ctrl+O").clicked() {
                            self.open_file();
                            ui.close_menu();
                        }
                        if ui.button("ファイルに保存...  Ctrl+S").clicked() {
                            self.save_current_to_file();
                            ui.close_menu();
                        }
                        if ui.button("すべて保存").clicked() {
                            self.save_notes();
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button("HTMLにエクスポート...").clicked() {
                            self.export_html();
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button("終了").clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    });
                    ui.menu_button("表示", |ui| {
                        ui.checkbox(&mut self.settings.show_sidebar, "サイドバー");
                        ui.checkbox(&mut self.settings.show_toc, "目次 (TOC)");
                        ui.checkbox(&mut self.show_backlinks, "バックリンクパネル");
                        ui.separator();
                        ui.label(RichText::new("表示モード").small().color(c.text_dim));
                        ui.radio_value(&mut self.settings.view_mode, ViewMode::EditorOnly, "📝 編集のみ");
                        ui.radio_value(&mut self.settings.view_mode, ViewMode::Split, "⫼ 分割表示");
                        ui.radio_value(&mut self.settings.view_mode, ViewMode::PreviewOnly, "👁 プレビューのみ");
                        ui.separator();
                        ui.checkbox(&mut self.settings.show_line_numbers, "行番号");
                        ui.checkbox(&mut self.settings.word_wrap, "折り返し");
                        ui.checkbox(&mut self.settings.sync_scroll, "同期スクロール");
                    });
                    ui.menu_button("編集", |ui| {
                        if ui.button("クイックスイッチャー  Ctrl+P").clicked() {
                            self.quick_switcher.open();
                            ui.close_menu();
                        }
                        if ui.button("検索...  Ctrl+F").clicked() {
                            self.find.open_find();
                            ui.close_menu();
                        }
                        if ui.button("置換...  Ctrl+H").clicked() {
                            self.find.open_replace();
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button("📋 クリップボードから画像を貼り付け").clicked() {
                            self.try_paste_image(true);
                            ui.close_menu();
                        }
                        if ui.button("Wikiリンクを挿入  [[]]").clicked() {
                            self.pending_action = Some(EditorAction::Insert("[[]]"));
                            ui.close_menu();
                        }
                    });
                    ui.menu_button("テーマ", |ui| {
                        if ui.radio_value(&mut self.settings.theme, ThemeMode::Dark, "🌙 ダーク").clicked() {
                            theme::apply(ctx, self.settings.theme, self.settings.editor_font_size);
                            self.save_settings();
                            ui.close_menu();
                        }
                        if ui.radio_value(&mut self.settings.theme, ThemeMode::Light, "☀ ライト").clicked() {
                            theme::apply(ctx, self.settings.theme, self.settings.editor_font_size);
                            self.save_settings();
                            ui.close_menu();
                        }
                    });
                    ui.menu_button("ツール", |ui| {
                        if ui.button("⚙ 設定...").clicked() {
                            self.show_settings = true;
                            ui.close_menu();
                        }
                    });
                });
            });

        // Status bar
        let dirty = self.notes_dirty;
        let auto_save = self.settings.auto_save;
        // Clear status override after 6 seconds
        if let Some(at) = self.status_override_at {
            if at.elapsed().as_secs() >= 6 {
                self.status_override = None;
                self.status_override_at = None;
            }
        }
        let status_override = self.status_override.clone();
        egui::TopBottomPanel::bottom("status_bar")
            .frame(egui::Frame::none().fill(c.menu_bg).inner_margin(egui::Margin::symmetric(12.0, 6.0)))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let status = if let Some(s) = status_override.as_deref() {
                        s
                    } else if dirty {
                        if auto_save { "● 自動保存中…" } else { "● 未保存の変更" }
                    } else {
                        "✓ 保存済み"
                    };
                    ui.label(RichText::new(status).color(c.text_dim).size(11.0));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            RichText::new(format!("{} 件のノート", self.notes.iter().filter(|n| !n.trashed).count()))
                                .color(c.text_dim)
                                .size(11.0),
                        );
                    });
                });
            });

        // Sidebar
        if self.settings.show_sidebar {
            egui::SidePanel::left("sidebar")
                .resizable(true)
                .min_width(180.0)
                .default_width(220.0)
                .frame(egui::Frame::none().fill(c.sidebar_bg))
                .show(ctx, |ui| {
                    self.draw_sidebar(ui);
                });
        }

        // Backlinks bottom panel
        if self.show_backlinks {
            egui::TopBottomPanel::bottom("backlinks_panel")
                .resizable(true)
                .min_height(80.0)
                .default_height(160.0)
                .frame(egui::Frame::none().fill(c.sidebar_bg))
                .show(ctx, |ui| {
                    self.draw_backlinks_panel(ui);
                });
        }

        // TOC panel (right side, before central)
        if self.settings.show_toc {
            egui::SidePanel::right("toc")
                .resizable(true)
                .min_width(160.0)
                .default_width(220.0)
                .frame(egui::Frame::none().fill(c.sidebar_bg))
                .show(ctx, |ui| {
                    self.draw_toc(ui);
                });
        }

        // Main area
        egui::CentralPanel::default()
            .frame(egui::Frame::none())
            .show(ctx, |ui| match self.settings.view_mode {
                ViewMode::Split => {
                    ui.columns(2, |cols| {
                        egui::Frame::none()
                            .fill(c.editor_bg)
                            .inner_margin(egui::Margin::ZERO)
                            .show(&mut cols[0], |ui| {
                                ui.set_height(ui.available_height());
                                self.draw_editor(ui);
                            });
                        egui::Frame::none()
                            .fill(c.preview_bg)
                            .inner_margin(egui::Margin::ZERO)
                            .show(&mut cols[1], |ui| {
                                ui.set_height(ui.available_height());
                                self.draw_preview(ui);
                            });
                    });
                }
                ViewMode::EditorOnly => {
                    egui::Frame::none().fill(c.editor_bg).show(ui, |ui| {
                        ui.set_height(ui.available_height());
                        self.draw_editor(ui);
                    });
                }
                ViewMode::PreviewOnly => {
                    egui::Frame::none().fill(c.preview_bg).show(ui, |ui| {
                        ui.set_height(ui.available_height());
                        self.draw_preview(ui);
                    });
                }
            });

        self.draw_find_bar(ctx);
        self.draw_settings_window(ctx);
        self.draw_quick_switcher(ctx);

        // Save on close
        if ctx.input(|i| i.viewport().close_requested()) {
            self.save_notes();
            self.save_settings();
        }
    }
}
