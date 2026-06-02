use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub struct Editor {
    lines: Vec<String>,
    cursor_row: usize,
    cursor_col: usize,
    pub scroll_offset: usize,
}

impl Editor {
    pub fn new() -> Self {
        Self {
            lines: vec![String::new()],
            cursor_row: 0,
            cursor_col: 0,
            scroll_offset: 0,
        }
    }

    pub fn insert_text(&mut self, text: &str) {
        self.lines = vec![String::new()];
        self.cursor_row = 0;
        self.cursor_col = 0;
        for ch in text.chars() {
            if ch == '\n' {
                self.insert_newline();
            } else {
                self.insert_char(ch);
            }
        }
        self.cursor_row = 0;
        self.cursor_col = 0;
    }

    pub fn content(&self) -> String {
        self.lines.join("\n")
    }

    pub fn lines(&self) -> &[String] {
        &self.lines
    }

    pub fn cursor_row(&self) -> usize {
        self.cursor_row
    }

    pub fn cursor_line(&self) -> usize {
        self.cursor_row
    }

    pub fn cursor_col(&self) -> usize {
        self.cursor_col
    }

    pub fn handle_key(&mut self, key: KeyEvent) {
        match (key.modifiers, key.code) {
            (KeyModifiers::NONE | KeyModifiers::SHIFT, KeyCode::Char(c)) => self.insert_char(c),
            (KeyModifiers::NONE, KeyCode::Enter) => self.insert_newline(),
            (KeyModifiers::NONE, KeyCode::Backspace) => self.delete_backward(),
            (KeyModifiers::NONE, KeyCode::Delete) => self.delete_forward(),
            (KeyModifiers::NONE, KeyCode::Left) => self.move_left(),
            (KeyModifiers::NONE, KeyCode::Right) => self.move_right(),
            (KeyModifiers::NONE, KeyCode::Up) => self.move_up(),
            (KeyModifiers::NONE, KeyCode::Down) => self.move_down(),
            (KeyModifiers::NONE, KeyCode::Home) => self.cursor_col = 0,
            (KeyModifiers::NONE, KeyCode::End) => {
                self.cursor_col = self.lines[self.cursor_row].len();
            }
            (KeyModifiers::CONTROL, KeyCode::Home) => {
                self.cursor_row = 0;
                self.cursor_col = 0;
                self.scroll_offset = 0;
            }
            (KeyModifiers::CONTROL, KeyCode::End) => {
                self.cursor_row = self.lines.len().saturating_sub(1);
                self.cursor_col = self.lines[self.cursor_row].len();
            }
            (KeyModifiers::NONE, KeyCode::Tab) => {
                for _ in 0..4 {
                    self.insert_char(' ');
                }
            }
            _ => {}
        }
    }

    fn insert_char(&mut self, c: char) {
        let col = self.cursor_col;
        self.lines[self.cursor_row].insert(col, c);
        self.cursor_col += 1;
    }

    fn insert_newline(&mut self) {
        let col = self.cursor_col;
        let rest = self.lines[self.cursor_row].split_off(col);
        self.cursor_row += 1;
        self.lines.insert(self.cursor_row, rest);
        self.cursor_col = 0;
    }

    fn delete_backward(&mut self) {
        if self.cursor_col > 0 {
            self.cursor_col -= 1;
            let col = self.cursor_col;
            self.lines[self.cursor_row].remove(col);
        } else if self.cursor_row > 0 {
            let current = self.lines.remove(self.cursor_row);
            self.cursor_row -= 1;
            self.cursor_col = self.lines[self.cursor_row].len();
            self.lines[self.cursor_row].push_str(&current);
        }
    }

    fn delete_forward(&mut self) {
        let col = self.cursor_col;
        let line_len = self.lines[self.cursor_row].len();
        if col < line_len {
            self.lines[self.cursor_row].remove(col);
        } else if self.cursor_row + 1 < self.lines.len() {
            let next = self.lines.remove(self.cursor_row + 1);
            self.lines[self.cursor_row].push_str(&next);
        }
    }

    fn move_left(&mut self) {
        if self.cursor_col > 0 {
            self.cursor_col -= 1;
        } else if self.cursor_row > 0 {
            self.cursor_row -= 1;
            self.cursor_col = self.lines[self.cursor_row].len();
        }
    }

    fn move_right(&mut self) {
        let line_len = self.lines[self.cursor_row].len();
        if self.cursor_col < line_len {
            self.cursor_col += 1;
        } else if self.cursor_row + 1 < self.lines.len() {
            self.cursor_row += 1;
            self.cursor_col = 0;
        }
    }

    fn move_up(&mut self) {
        if self.cursor_row > 0 {
            self.cursor_row -= 1;
            self.cursor_col = self.cursor_col.min(self.lines[self.cursor_row].len());
            if self.cursor_row < self.scroll_offset {
                self.scroll_offset = self.cursor_row;
            }
        }
    }

    fn move_down(&mut self) {
        if self.cursor_row + 1 < self.lines.len() {
            self.cursor_row += 1;
            self.cursor_col = self.cursor_col.min(self.lines[self.cursor_row].len());
        }
    }

    pub fn adjust_scroll(&mut self, visible_height: usize) {
        if self.cursor_row < self.scroll_offset {
            self.scroll_offset = self.cursor_row;
        } else if self.cursor_row >= self.scroll_offset + visible_height {
            self.scroll_offset = self.cursor_row + 1 - visible_height;
        }
    }
}
