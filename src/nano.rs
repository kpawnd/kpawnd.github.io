use wasm_bindgen::prelude::*;

/// Nano text editor state - managed in Rust
#[wasm_bindgen]
pub struct NanoEditor {
    filename: String,
    lines: Vec<String>,
    cursor_row: usize,
    cursor_col: usize,
    modified: bool,
    clipboard: Vec<String>,
}

#[wasm_bindgen]
impl NanoEditor {
    #[wasm_bindgen(constructor)]
    pub fn new(filename: &str, content: &str) -> NanoEditor {
        let lines = if content.is_empty() {
            vec![String::new()]
        } else {
            content.lines().map(|s| s.to_string()).collect()
        };

        NanoEditor {
            filename: filename.to_string(),
            lines,
            cursor_row: 0,
            cursor_col: 0,
            modified: false,
            clipboard: Vec::new(),
        }
    }

    /// Get the filename
    pub fn get_filename(&self) -> String {
        self.filename.clone()
    }

    /// Set the filename
    pub fn set_filename(&mut self, name: &str) {
        self.filename = name.to_string();
    }

    /// Get current cursor row
    pub fn get_cursor_row(&self) -> usize {
        self.cursor_row
    }

    /// Get current cursor column
    pub fn get_cursor_col(&self) -> usize {
        self.cursor_col
    }

    /// Get total line count
    pub fn line_count(&self) -> usize {
        self.lines.len()
    }

    /// Get a specific line
    pub fn get_line(&self, row: usize) -> String {
        self.lines.get(row).cloned().unwrap_or_default()
    }

    /// Check if file is modified
    pub fn is_modified(&self) -> bool {
        self.modified
    }

    /// Get full content as string
    pub fn get_content(&self) -> String {
        self.lines.join("\n")
    }

    /// Move cursor up
    pub fn cursor_up(&mut self) {
        if self.cursor_row > 0 {
            self.cursor_row -= 1;
            let line_len = self
                .lines
                .get(self.cursor_row)
                .map(|l| l.len())
                .unwrap_or(0);
            self.cursor_col = self.cursor_col.min(line_len);
        }
    }

    /// Move cursor down
    pub fn cursor_down(&mut self) {
        if self.cursor_row < self.lines.len().saturating_sub(1) {
            self.cursor_row += 1;
            let line_len = self
                .lines
                .get(self.cursor_row)
                .map(|l| l.len())
                .unwrap_or(0);
            self.cursor_col = self.cursor_col.min(line_len);
        }
    }

    /// Move cursor left
    pub fn cursor_left(&mut self) {
        if self.cursor_col > 0 {
            self.cursor_col -= 1;
        } else if self.cursor_row > 0 {
            self.cursor_row -= 1;
            self.cursor_col = self
                .lines
                .get(self.cursor_row)
                .map(|l| l.len())
                .unwrap_or(0);
        }
    }

    /// Move cursor right
    pub fn cursor_right(&mut self) {
        let line_len = self
            .lines
            .get(self.cursor_row)
            .map(|l| l.len())
            .unwrap_or(0);
        if self.cursor_col < line_len {
            self.cursor_col += 1;
        } else if self.cursor_row < self.lines.len().saturating_sub(1) {
            self.cursor_row += 1;
            self.cursor_col = 0;
        }
    }

    /// Move cursor to start of line
    pub fn cursor_home(&mut self) {
        self.cursor_col = 0;
    }

    /// Move cursor to end of line
    pub fn cursor_end(&mut self) {
        self.cursor_col = self
            .lines
            .get(self.cursor_row)
            .map(|l| l.len())
            .unwrap_or(0);
    }

    /// Page up
    pub fn page_up(&mut self, page_size: usize) {
        self.cursor_row = self.cursor_row.saturating_sub(page_size);
        let line_len = self
            .lines
            .get(self.cursor_row)
            .map(|l| l.len())
            .unwrap_or(0);
        self.cursor_col = self.cursor_col.min(line_len);
    }

    /// Page down
    pub fn page_down(&mut self, page_size: usize) {
        self.cursor_row = (self.cursor_row + page_size).min(self.lines.len().saturating_sub(1));
        let line_len = self
            .lines
            .get(self.cursor_row)
            .map(|l| l.len())
            .unwrap_or(0);
        self.cursor_col = self.cursor_col.min(line_len);
    }

    /// Insert a character at cursor position (takes a string, uses first char)
    pub fn insert_char(&mut self, s: &str) {
        let c = match s.chars().next() {
            Some(c) => c,
            None => return,
        };

        if let Some(line) = self.lines.get_mut(self.cursor_row) {
            // Handle UTF-8 properly
            let mut new_line = String::with_capacity(line.len() + 1);
            let chars: Vec<char> = line.chars().collect();
            let col = self.cursor_col.min(chars.len());

            for (i, ch) in chars.iter().enumerate() {
                if i == col {
                    new_line.push(c);
                }
                new_line.push(*ch);
            }
            if col >= chars.len() {
                new_line.push(c);
            }

            *line = new_line;
            self.cursor_col += 1;
            self.modified = true;
        }
    }

    /// Internal method to insert actual char
    fn insert_char_internal(&mut self, c: char) {
        if let Some(line) = self.lines.get_mut(self.cursor_row) {
            let mut new_line = String::with_capacity(line.len() + 1);
            let chars: Vec<char> = line.chars().collect();
            let col = self.cursor_col.min(chars.len());

            for (i, ch) in chars.iter().enumerate() {
                if i == col {
                    new_line.push(c);
                }
                new_line.push(*ch);
            }
            if col >= chars.len() {
                new_line.push(c);
            }

            *line = new_line;
            self.cursor_col += 1;
            self.modified = true;
        }
    }

    /// Insert a string at cursor position
    pub fn insert_string(&mut self, s: &str) {
        for c in s.chars() {
            if c == '\n' {
                self.insert_newline();
            } else {
                self.insert_char_internal(c);
            }
        }
    }

    /// Insert newline at cursor
    pub fn insert_newline(&mut self) {
        if let Some(line) = self.lines.get(self.cursor_row).cloned() {
            let chars: Vec<char> = line.chars().collect();
            let col = self.cursor_col.min(chars.len());

            let before: String = chars[..col].iter().collect();
            let after: String = chars[col..].iter().collect();

            self.lines[self.cursor_row] = before;
            self.lines.insert(self.cursor_row + 1, after);
            self.cursor_row += 1;
            self.cursor_col = 0;
            self.modified = true;
        }
    }

    /// Delete character before cursor (backspace)
    pub fn backspace(&mut self) {
        if self.cursor_col > 0 {
            if let Some(line) = self.lines.get_mut(self.cursor_row) {
                let chars: Vec<char> = line.chars().collect();
                let col = self.cursor_col.min(chars.len());

                let mut new_line: String = chars[..col - 1].iter().collect();
                new_line.extend(chars[col..].iter());

                *line = new_line;
                self.cursor_col -= 1;
                self.modified = true;
            }
        } else if self.cursor_row > 0 {
            // Merge with previous line
            let current = self.lines.remove(self.cursor_row);
            self.cursor_row -= 1;
            let prev_len = self.lines[self.cursor_row].len();
            self.lines[self.cursor_row].push_str(&current);
            self.cursor_col = prev_len;
            self.modified = true;
        }
    }

    /// Delete character at cursor (delete key)
    pub fn delete(&mut self) {
        if let Some(line) = self.lines.get(self.cursor_row).cloned() {
            let chars: Vec<char> = line.chars().collect();
            let col = self.cursor_col.min(chars.len());

            if col < chars.len() {
                let mut new_line: String = chars[..col].iter().collect();
                new_line.extend(chars[col + 1..].iter());
                self.lines[self.cursor_row] = new_line;
                self.modified = true;
            } else if self.cursor_row < self.lines.len().saturating_sub(1) {
                // Merge with next line
                let next = self.lines.remove(self.cursor_row + 1);
                self.lines[self.cursor_row].push_str(&next);
                self.modified = true;
            }
        }
    }

    /// Cut current line (Ctrl+K)
    pub fn cut_line(&mut self) {
        if self.lines.len() > 1 {
            let cut = self.lines.remove(self.cursor_row);
            self.clipboard = vec![cut];
            if self.cursor_row >= self.lines.len() {
                self.cursor_row = self.lines.len().saturating_sub(1);
            }
            let line_len = self
                .lines
                .get(self.cursor_row)
                .map(|l| l.len())
                .unwrap_or(0);
            self.cursor_col = self.cursor_col.min(line_len);
            self.modified = true;
        } else if !self.lines.is_empty() {
            self.clipboard = vec![self.lines[0].clone()];
            self.lines[0].clear();
            self.cursor_col = 0;
            self.modified = true;
        }
    }

    /// Paste clipboard (Ctrl+U)
    pub fn paste(&mut self) {
        for line in self.clipboard.clone() {
            self.insert_string(&line);
            self.insert_newline();
        }
    }

    /// Mark as saved
    pub fn mark_saved(&mut self) {
        self.modified = false;
    }

    /// Get visible lines for rendering (returns JSON array)
    pub fn get_visible_lines(&self, start: usize, count: usize) -> String {
        let end = (start + count).min(self.lines.len());
        let visible: Vec<&String> = self.lines[start..end].iter().collect();
        serde_json::to_string(&visible).unwrap_or_else(|_| "[]".to_string())
    }

    /// Calculate the viewport start for keeping cursor in view
    pub fn calculate_viewport_start(&self, visible_lines: usize) -> usize {
        let half = visible_lines / 2;
        if self.cursor_row < half {
            0
        } else if self.cursor_row > self.lines.len().saturating_sub(half) {
            self.lines.len().saturating_sub(visible_lines)
        } else {
            self.cursor_row.saturating_sub(half)
        }
    }

    /// Find text (returns row, col or -1 if not found)
    pub fn find(&self, needle: &str) -> String {
        for (row, line) in self.lines.iter().enumerate() {
            if let Some(col) = line.find(needle) {
                return format!("{}:{}", row, col);
            }
        }
        "-1".to_string()
    }

    /// Find and goto
    pub fn find_goto(&mut self, needle: &str) -> bool {
        for (row, line) in self.lines.iter().enumerate() {
            if let Some(col) = line.find(needle) {
                self.cursor_row = row;
                self.cursor_col = col;
                return true;
            }
        }
        false
    }

    /// Replace text at current position
    pub fn replace(&mut self, old: &str, new: &str) -> bool {
        if let Some(line) = self.lines.get_mut(self.cursor_row) {
            if let Some(pos) = line.find(old) {
                let new_line = line[..pos].to_string() + new + &line[pos + old.len()..];
                *line = new_line;
                self.modified = true;
                return true;
            }
        }
        false
    }

    /// Replace all occurrences
    pub fn replace_all(&mut self, old: &str, new: &str) -> usize {
        let mut count = 0;
        for line in &mut self.lines {
            while line.contains(old) {
                *line = line.replacen(old, new, 1);
                count += 1;
            }
        }
        if count > 0 {
            self.modified = true;
        }
        count
    }

    /// Goto specific line
    pub fn goto_line(&mut self, line_num: usize) {
        self.cursor_row = line_num
            .saturating_sub(1)
            .min(self.lines.len().saturating_sub(1));
        self.cursor_col = 0;
    }

    /// Render the editor as terminal output (for Rust-side rendering)
    pub fn render(&self, visible_lines: usize) -> String {
        let mut output = String::new();

        // Header
        let modified_marker = if self.modified { " Modified" } else { "" };
        output.push_str(&format!(
            "\x1b[COLOR:white]\x1b[BG:blue]  GNU nano 7.2                    {}{}\x1b[COLOR:reset]\x1b[BG:reset]\n",
            self.filename,
            modified_marker
        ));

        // Calculate viewport
        let start = self.calculate_viewport_start(visible_lines);
        let end = (start + visible_lines).min(self.lines.len());

        // Content lines
        for row in start..end {
            let line = self.lines.get(row).map(|s| s.as_str()).unwrap_or("");

            if row == self.cursor_row {
                // Show cursor on this line
                let chars: Vec<char> = line.chars().collect();
                let col = self.cursor_col.min(chars.len());

                let before: String = chars[..col].iter().collect();
                let cursor_char = chars.get(col).copied().unwrap_or(' ');
                let after: String = if col < chars.len() {
                    chars[col + 1..].iter().collect()
                } else {
                    String::new()
                };

                output.push_str(&format!(
                    "{}\x1b[COLOR:black]\x1b[BG:white]{}\x1b[COLOR:reset]\x1b[BG:reset]{}\n",
                    before, cursor_char, after
                ));
            } else {
                output.push_str(line);
                output.push('\n');
            }
        }

        // Pad empty lines
        for _ in (end - start)..visible_lines {
            output.push('\n');
        }

        // Status line
        output.push_str(&format!(
            "\x1b[COLOR:gray][ line {}/{}, col {} ]\x1b[COLOR:reset]\n",
            self.cursor_row + 1,
            self.lines.len(),
            self.cursor_col + 1
        ));

        // Help bar
        output.push_str("\x1b[BG:gray]\x1b[COLOR:white]^G\x1b[COLOR:black] Help  \x1b[COLOR:white]^O\x1b[COLOR:black] Write Out  \x1b[COLOR:white]^W\x1b[COLOR:black] Where Is  \x1b[COLOR:white]^K\x1b[COLOR:black] Cut  \x1b[COLOR:white]^C\x1b[COLOR:black] Location\x1b[BG:reset]\n");
        output.push_str("\x1b[BG:gray]\x1b[COLOR:white]^X\x1b[COLOR:black] Exit  \x1b[COLOR:white]^R\x1b[COLOR:black] Read File  \x1b[COLOR:white]^\\\x1b[COLOR:black] Replace  \x1b[COLOR:white]^U\x1b[COLOR:black] Paste  \x1b[COLOR:white]^T\x1b[COLOR:black] Execute\x1b[BG:reset]");

        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_empty() {
        let editor = NanoEditor::new("test.txt", "");
        assert_eq!(editor.line_count(), 1);
        assert_eq!(editor.get_line(0), "");
    }

    #[test]
    fn test_new_with_content() {
        let editor = NanoEditor::new("test.txt", "hello\nworld");
        assert_eq!(editor.line_count(), 2);
        assert_eq!(editor.get_line(0), "hello");
        assert_eq!(editor.get_line(1), "world");
    }

    #[test]
    fn test_insert_char() {
        let mut editor = NanoEditor::new("test.txt", "hello");
        editor.cursor_col = 5;
        editor.insert_char("!");
        assert_eq!(editor.get_line(0), "hello!");
    }

    #[test]
    fn test_backspace() {
        let mut editor = NanoEditor::new("test.txt", "hello");
        editor.cursor_col = 5;
        editor.backspace();
        assert_eq!(editor.get_line(0), "hell");
    }
}
