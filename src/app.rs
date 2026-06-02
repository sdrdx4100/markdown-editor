use crate::editor_actions::{self, EditorAction};
use crate::note::Note;
use crate::theme;
use egui::{Color32, FontFamily, FontId, RichText, ScrollArea, TextEdit, Ui};
use egui_commonmark::{CommonMarkCache, CommonMarkViewer};
use std::fs;

pub struct MarkdownApp {
    notes: Vec<Note>,
    selected: usize,
    cache: CommonMarkCache,
    show_sidebar: bool,
    search_query: String,
    pending_action: Option<EditorAction>,
}

const EDITOR_ID: &str = "main_editor";

impl MarkdownApp {
    pub fn new(cc: &eframe::CreationContext) -> Self {
        load_japanese_font(&cc.egui_ctx);
        theme::apply(&cc.egui_ctx);

        let notes = vec![
            Note::new(
                "Markdownとは？",
                "# Markdownとは？\n\n**Markdown(マークダウン)**とは、`# 見出し` `* リスト` などシンプルな書き方で**文書構造**を明示でき、装飾されたHTML文書に変換できる**軽量マークアップ言語**です。\n\n## 箇条書き\n\n- リスト１\n- リスト２\n- リスト３\n\n## コードブロック\n\n```rust\nfn main() {\n    println!(\"Hello, World!\");\n}\n```\n\n## テーブル\n\n| タイトル１ | タイトル２ | タイトル３ |\n|-----------|-----------|----------|\n| みかん    | りんご    | ぶどう    |\n| いちご    | もも      | なし     |\n\n## テキスト装飾\n\n**太字テキスト** と *斜体テキスト* と ~~打ち消し線~~ です。\n\n> ブロッククォート: 重要な引用文はこのように表示されます。\n\n---\n\nインラインコード: `let x = 42;`\n",
            ),
            Note::new(
                "Welcome to the editor!",
                "# Welcome!\n\nThis is a Markdown editor built with Rust and egui.\n\n## Getting Started\n\n1. Click a note in the sidebar\n2. Edit in the left pane\n3. See the preview on the right\n\n## Shortcuts\n\n- **Ctrl+S** — Save file\n- **Ctrl+O** — Open file\n- **Ctrl+N** — New note\n",
            ),
            Note::new(
                "Snippet example",
                "# Code Snippets\n\n## Rust\n\n```rust\nstruct Editor {\n    content: String,\n    cursor: usize,\n}\n\nimpl Editor {\n    fn new() -> Self {\n        Self {\n            content: String::new(),\n            cursor: 0,\n        }\n    }\n}\n```\n\n## Python\n\n```python\ndef hello(name: str) -> str:\n    return f\"Hello, {name}!\"\n\nprint(hello(\"World\"))\n```\n",
            ),
        ];

        Self {
            notes,
            selected: 0,
            cache: CommonMarkCache::default(),
            show_sidebar: true,
            search_query: String::new(),
            pending_action: None,
        }
    }

    fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        let (ctrl, shift, key_s, key_n, key_o, key_b, key_i, key_k, key_e, key_q, key_l, key_t) =
            ctx.input(|i| {
                (
                    i.modifiers.ctrl,
                    i.modifiers.shift,
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
                )
            });

        if ctrl && key_s && !shift {
            self.save_current();
        }
        if ctrl && key_n && !shift {
            self.new_note();
        }
        if ctrl && key_o && !shift {
            self.open_file();
        }
        // Formatting shortcuts
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
    }

    fn toolbar_button(&mut self, ui: &mut Ui, label: &str, tooltip: &str, action: EditorAction) {
        let resp = ui.add(
            egui::Button::new(RichText::new(label).size(13.0))
                .min_size(egui::vec2(28.0, 24.0))
                .fill(Color32::from_rgb(40, 42, 50)),
        );
        if resp.on_hover_text(tooltip).clicked() {
            self.pending_action = Some(action);
        }
    }

    fn draw_toolbar(&mut self, ui: &mut Ui) {
        egui::Frame::none()
            .fill(Color32::from_rgb(32, 34, 40))
            .inner_margin(egui::Margin::symmetric(8.0, 6.0))
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing.x = 4.0;
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

    fn save_current(&mut self) {
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
        }
    }

    fn new_note(&mut self) {
        self.notes.push(Note::default());
        self.selected = self.notes.len() - 1;
    }

    fn draw_sidebar(&mut self, ui: &mut Ui) {
        // Header
        egui::Frame::none()
            .fill(theme::SIDEBAR_BG)
            .inner_margin(egui::Margin::symmetric(12.0, 10.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("☰  Markdown Editor")
                            .color(Color32::WHITE)
                            .size(15.0)
                            .strong(),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .button(RichText::new("+").size(18.0).color(theme::ACCENT))
                            .on_hover_text("New note (Ctrl+N)")
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
            .fill(theme::SIDEBAR_BG)
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

        // Note list
        egui::Frame::none()
            .fill(theme::SIDEBAR_BG)
            .show(ui, |ui| {
                ui.label(
                    RichText::new("  すべてのノート")
                        .color(theme::TEXT_DIM)
                        .size(11.0),
                );
                ui.add_space(2.0);

                ScrollArea::vertical().show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    let query = self.search_query.to_lowercase();
                    let indices: Vec<usize> = (0..self.notes.len())
                        .filter(|&i| {
                            query.is_empty()
                                || self.notes[i].title.to_lowercase().contains(&query)
                                || self.notes[i].content.to_lowercase().contains(&query)
                        })
                        .collect();

                    let mut new_selected = self.selected;

                    for i in indices {
                        let is_selected = i == self.selected;
                        let title = self.notes[i].display_title();
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

                        // Allocate the full row first, then sense clicks on that rect
                        let desired_height = if preview.is_empty() { 36.0 } else { 52.0 };
                        let (rect, response) = ui.allocate_exact_size(
                            egui::vec2(ui.available_width(), desired_height),
                            egui::Sense::click(),
                        );

                        if response.clicked() {
                            new_selected = i;
                        }

                        // Paint background (hover effect for unselected)
                        let bg = if is_selected {
                            theme::SELECTED_ITEM_BG
                        } else if response.hovered() {
                            Color32::from_rgb(42, 45, 54)
                        } else {
                            theme::SIDEBAR_BG
                        };
                        ui.painter().rect_filled(rect, 4.0, bg);

                        // Paint title text
                        let title_color = if is_selected { Color32::WHITE } else { theme::TEXT_NORMAL };
                        let title_pos = rect.min + egui::vec2(12.0, 10.0);
                        ui.painter().text(
                            title_pos,
                            egui::Align2::LEFT_TOP,
                            &title,
                            egui::FontId::new(13.0, FontFamily::Proportional),
                            title_color,
                        );

                        // Paint preview text
                        if !preview.is_empty() {
                            let preview_color = if is_selected {
                                Color32::from_rgb(200, 240, 230)
                            } else {
                                theme::TEXT_DIM
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

                        ui.add_space(2.0);
                    }

                    self.selected = new_selected;
                });
            });
    }

    fn draw_editor(&mut self, ui: &mut Ui) {
        // Header
        let path_label = self.notes[self.selected]
            .path
            .as_ref()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "[未保存]".to_string());

        egui::Frame::none()
            .fill(Color32::from_rgb(26, 28, 34))
            .inner_margin(egui::Margin::symmetric(12.0, 6.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("✏  Editor").color(theme::TEXT_DIM).size(12.0));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(RichText::new(path_label).color(theme::TEXT_DIM).size(11.0));
                    });
                });
            });

        // Markdown formatting toolbar
        self.draw_toolbar(ui);

        ui.add(egui::Separator::default().spacing(0.0).grow(0.0));

        // Editor area with line numbers
        ScrollArea::both()
            .id_salt("editor_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.horizontal_top(|ui| {
                    // Line numbers
                    let note = &self.notes[self.selected];
                    let line_count = note.content.lines().count().max(1);
                    let line_nums: String = (1..=line_count).map(|n| format!("{}\n", n)).collect();
                    ui.add(
                        TextEdit::multiline(&mut line_nums.as_str())
                            .desired_width(36.0)
                            .frame(false)
                            .interactive(false)
                            .font(FontId::new(13.0, FontFamily::Monospace))
                            .text_color(theme::LINE_NUMBER_COLOR),
                    );

                    ui.add(egui::Separator::default().vertical().spacing(0.0).grow(0.0));

                    // Text editor
                    let note = &mut self.notes[self.selected];
                    let prev_content = note.content.clone();
                    let editor_id = egui::Id::new(EDITOR_ID);
                    let output = TextEdit::multiline(&mut note.content)
                        .id(editor_id)
                        .desired_width(f32::INFINITY)
                        .desired_rows(40)
                        .frame(false)
                        .font(FontId::new(13.0, FontFamily::Monospace))
                        .text_color(theme::TEXT_NORMAL)
                        .lock_focus(true)
                        .show(ui);

                    if note.content != prev_content {
                        note.modified = true;
                    }

                    // Apply pending markdown action (from toolbar or shortcut)
                    if let Some(action) = self.pending_action.take() {
                        let (sel_start, sel_end) =
                            if let Some(range) = output.cursor_range {
                                (range.primary.ccursor.index, range.secondary.ccursor.index)
                            } else {
                                let end = note.content.chars().count();
                                (end, end)
                            };

                        let result = editor_actions::apply(action, &note.content, sel_start, sel_end);
                        note.content = result.new_content;
                        note.modified = true;

                        // Restore cursor to where the action placed it
                        let mut state = output.state.clone();
                        let new_range = egui::text::CCursorRange::two(
                            egui::text::CCursor::new(result.new_cursor_start),
                            egui::text::CCursor::new(result.new_cursor_end),
                        );
                        state.cursor.set_char_range(Some(new_range));
                        state.store(ui.ctx(), editor_id);
                        ui.ctx().memory_mut(|m| m.request_focus(editor_id));
                    }
                });
            });
    }

    fn draw_preview(&mut self, ui: &mut Ui) {
        let content = self.notes[self.selected].content.clone();

        egui::Frame::none()
            .fill(theme::PREVIEW_BG)
            .show(ui, |ui| {
                // Toolbar
                egui::Frame::none()
                    .fill(Color32::from_rgb(30, 32, 38))
                    .inner_margin(egui::Margin::symmetric(12.0, 8.0))
                    .show(ui, |ui| {
                        ui.label(
                            RichText::new("👁  Preview")
                                .color(theme::TEXT_DIM)
                                .size(12.0),
                        );
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
}

fn load_japanese_font(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    // Try Windows system fonts in order of preference
    let font_candidates = [
        r"C:\Windows\Fonts\YuGothM.ttc",
        r"C:\Windows\Fonts\YuGothR.ttc",
        r"C:\Windows\Fonts\meiryo.ttc",
        r"C:\Windows\Fonts\msgothic.ttc",
    ];

    for path in &font_candidates {
        if let Ok(bytes) = std::fs::read(path) {
            fonts.font_data.insert(
                "japanese".to_owned(),
                egui::FontData::from_owned(bytes).into(),
            );
            // Add as fallback after the built-in fonts for both Proportional and Monospace
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

        // Menu bar must be first so it reserves space before other panels
        egui::TopBottomPanel::top("menu_bar")
            .frame(egui::Frame::none().fill(Color32::from_rgb(20, 22, 26)))
            .show(ctx, |ui| {
                egui::menu::bar(ui, |ui| {
                    ui.menu_button("ファイル", |ui| {
                        if ui.button("新規ノート  Ctrl+N").clicked() {
                            self.new_note();
                            ui.close_menu();
                        }
                        if ui.button("開く...  Ctrl+O").clicked() {
                            self.open_file();
                            ui.close_menu();
                        }
                        if ui.button("保存...  Ctrl+S").clicked() {
                            self.save_current();
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button("終了").clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    });
                    ui.menu_button("表示", |ui| {
                        if ui
                            .checkbox(&mut self.show_sidebar, "サイドバーを表示")
                            .clicked()
                        {
                            ui.close_menu();
                        }
                    });
                });
            });

        // Sidebar
        if self.show_sidebar {
            egui::SidePanel::left("sidebar")
                .resizable(true)
                .min_width(180.0)
                .default_width(220.0)
                .frame(egui::Frame::none().fill(theme::SIDEBAR_BG))
                .show(ctx, |ui| {
                    self.draw_sidebar(ui);
                });
        }

        // Main area: editor + preview
        egui::CentralPanel::default()
            .frame(egui::Frame::none())
            .show(ctx, |ui| {
                ui.columns(2, |cols| {
                    let left_frame = egui::Frame::none()
                        .fill(theme::EDITOR_BG)
                        .inner_margin(egui::Margin::ZERO);
                    left_frame.show(&mut cols[0], |ui| {
                        ui.set_height(ui.available_height());
                        self.draw_editor(ui);
                    });

                    let right_frame = egui::Frame::none()
                        .fill(theme::PREVIEW_BG)
                        .inner_margin(egui::Margin::ZERO);
                    right_frame.show(&mut cols[1], |ui| {
                        ui.set_height(ui.available_height());
                        self.draw_preview(ui);
                    });
                });
            });

    }
}
