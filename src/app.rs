use crate::editor::Editor;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::{fs, path::PathBuf};

pub struct App {
    pub editor: Editor,
    pub file_path: Option<PathBuf>,
    pub should_quit: bool,
    pub status_message: String,
    pub show_help: bool,
}

impl App {
    pub fn new() -> Self {
        let mut editor = Editor::new();
        editor.insert_text("# Hello, Markdown Editor!\n\nStart typing here...\n\n## Features\n\n- Split view: editor + preview\n- **Bold** and *italic* text\n- `inline code`\n- Lists and headings\n\n```rust\nfn main() {\n    println!(\"Hello!\");\n}\n```\n");
        Self {
            editor,
            file_path: None,
            should_quit: false,
            status_message: String::from("Ctrl+Q: Quit | Ctrl+S: Save | Ctrl+O: Open | Ctrl+N: New | Ctrl+H: Help"),
            show_help: false,
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) {
        if self.show_help {
            self.show_help = false;
            return;
        }

        match (key.modifiers, key.code) {
            (KeyModifiers::CONTROL, KeyCode::Char('h')) => {
                self.show_help = true;
            }
            _ => {
                self.editor.handle_key(key);
                self.status_message = format!(
                    "Ln {}, Col {} | Ctrl+Q: Quit | Ctrl+S: Save",
                    self.editor.cursor_line() + 1,
                    self.editor.cursor_col() + 1,
                );
            }
        }
    }

    pub fn save_file(&mut self) {
        let path = if let Some(p) = &self.file_path {
            p.clone()
        } else {
            PathBuf::from("output.md")
        };

        match fs::write(&path, self.editor.content()) {
            Ok(_) => {
                self.file_path = Some(path.clone());
                self.status_message = format!("Saved: {}", path.display());
            }
            Err(e) => {
                self.status_message = format!("Save error: {}", e);
            }
        }
    }

    pub fn open_file(&mut self) {
        let path = PathBuf::from("output.md");
        match fs::read_to_string(&path) {
            Ok(content) => {
                self.editor = Editor::new();
                self.editor.insert_text(&content);
                self.file_path = Some(path.clone());
                self.status_message = format!("Opened: {}", path.display());
            }
            Err(e) => {
                self.status_message = format!("Open error: {}", e);
            }
        }
    }

    pub fn new_file(&mut self) {
        self.editor = Editor::new();
        self.file_path = None;
        self.status_message = String::from("New file created");
    }
}
