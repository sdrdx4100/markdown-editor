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
}

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
        }
    }

    fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        let input = ctx.input(|i| i.clone());

        if input.key_pressed(egui::Key::S) && input.modifiers.ctrl {
            self.save_current();
        }
        if input.key_pressed(egui::Key::N) && input.modifiers.ctrl {
            self.new_note();
        }
        if input.key_pressed(egui::Key::O) && input.modifiers.ctrl {
            self.open_file();
        }
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

                    for i in indices {
                        let is_selected = i == self.selected;
                        let note = &self.notes[i];
                        let title = note.display_title();
                        let preview: String = note
                            .content
                            .lines()
                            .find(|l| !l.trim_start_matches('#').trim().is_empty())
                            .unwrap_or("")
                            .trim_start_matches('#')
                            .trim()
                            .chars()
                            .take(40)
                            .collect();

                        let bg = if is_selected {
                            theme::SELECTED_ITEM_BG
                        } else {
                            theme::SIDEBAR_BG
                        };

                        egui::Frame::none()
                            .fill(bg)
                            .rounding(4.0)
                            .inner_margin(egui::Margin::symmetric(12.0, 8.0))
                            .show(ui, |ui| {
                                ui.set_width(ui.available_width());
                                let resp = ui.vertical(|ui| {
                                    ui.label(
                                        RichText::new(&title)
                                            .color(if is_selected {
                                                Color32::WHITE
                                            } else {
                                                theme::TEXT_NORMAL
                                            })
                                            .size(13.0)
                                            .strong(),
                                    );
                                    if !preview.is_empty() {
                                        ui.label(
                                            RichText::new(&preview)
                                                .color(if is_selected {
                                                    Color32::from_rgb(200, 240, 230)
                                                } else {
                                                    theme::TEXT_DIM
                                                })
                                                .size(11.0),
                                        );
                                    }
                                });
                                if resp.response.interact(egui::Sense::click()).clicked() {
                                    self.selected = i;
                                }
                            });

                        ui.add_space(1.0);
                    }
                });
            });
    }

    fn draw_editor(&mut self, ui: &mut Ui) {
        let note = &mut self.notes[self.selected];

        egui::Frame::none()
            .fill(theme::EDITOR_BG)
            .inner_margin(egui::Margin::symmetric(0.0, 0.0))
            .show(ui, |ui| {
                // Toolbar
                egui::Frame::none()
                    .fill(Color32::from_rgb(26, 28, 34))
                    .inner_margin(egui::Margin::symmetric(12.0, 8.0))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new("✏  Editor")
                                    .color(theme::TEXT_DIM)
                                    .size(12.0),
                            );
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    let path_label = note
                                        .path
                                        .as_ref()
                                        .and_then(|p| p.file_name())
                                        .and_then(|n| n.to_str())
                                        .unwrap_or("[未保存]");
                                    ui.label(
                                        RichText::new(path_label)
                                            .color(theme::TEXT_DIM)
                                            .size(11.0),
                                    );
                                },
                            );
                        });
                    });

                ui.add(egui::Separator::default().spacing(0.0).grow(0.0));

                // Editor area with line numbers
                ScrollArea::both()
                    .id_salt("editor_scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.horizontal_top(|ui| {
                            // Line numbers
                            let line_count = note.content.lines().count().max(1);
                            let line_nums: String = (1..=line_count)
                                .map(|n| format!("{}\n", n))
                                .collect();
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
                            let prev_content = note.content.clone();
                            let editor = TextEdit::multiline(&mut note.content)
                                .desired_width(f32::INFINITY)
                                .desired_rows(40)
                                .frame(false)
                                .font(FontId::new(13.0, FontFamily::Monospace))
                                .text_color(theme::TEXT_NORMAL)
                                .lock_focus(true);

                            ui.add(editor);

                            if note.content != prev_content {
                                note.modified = true;
                            }
                        });
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

                ScrollArea::both()
                    .id_salt("preview_scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        egui::Frame::none()
                            .inner_margin(egui::Margin::symmetric(20.0, 16.0))
                            .show(ui, |ui| {
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

        // Top menu bar
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
    }
}
