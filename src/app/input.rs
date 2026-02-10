use crate::app::{App, VimMode};

impl App {
    pub fn move_msg_cursor_back_word(&mut self) {
        if self.msg_cursor == 0 {
            return;
        }

        let mut pos = self.msg_cursor;

        // Move left over any whitespace
        while pos > 0 && self.msg[pos - 1].is_whitespace() {
            pos -= 1;
        }

        // Move left over the word characters
        while pos > 0 && !self.msg[pos - 1].is_whitespace() {
            pos -= 1;
        }

        self.msg_cursor = pos;
    }

    pub fn move_msg_cursor_back_word_uppercase(&mut self) {
        if self.msg_cursor == 0 {
            return;
        }

        let mut pos = self.msg_cursor;
        
        // Skip whitespace at current position
        while pos > 0 && self.msg[pos - 1].is_whitespace() {
            pos -= 1;
        }
        
        // Skip non-whitespace (the WORD)
        while pos > 0 && !self.msg[pos - 1].is_whitespace() {
            pos -= 1;
        }
        
        self.msg_cursor = pos;
    }

    pub fn move_msg_cursor_forward_word(&mut self) {
        let len = self.msg.len();
        if self.msg_cursor >= len {
            return;
        }

        let mut pos = self.msg_cursor;

        // Skip any current word we're on
        while pos < len && !self.msg[pos].is_whitespace() {
            pos += 1;
        }

        // Skip whitespace between words
        while pos < len && self.msg[pos].is_whitespace() {
            pos += 1;
        }

        // Now we're at the beginning of next word
        self.msg_cursor = pos;
    }

    pub fn move_msg_cursor_forward_word_uppercase(&mut self) {
        let len = self.msg.len();
        if self.msg_cursor >= len {
            return;
        }

        let mut pos = self.msg_cursor;
        
        // If inside a WORD, move to its end
        while pos < len && !self.msg[pos].is_whitespace() {
            pos += 1;
        }
        
        // Skip whitespace to next WORD
        while pos < len && self.msg[pos].is_whitespace() {
            pos += 1;
        }
        
        self.msg_cursor = pos;
    }

    pub fn move_msg_cursor_end_of_word(&mut self) {
        let len = self.msg.len();
        if self.msg_cursor >= len {
            return;
        }

        let mut pos = self.msg_cursor;
        
        // Skip to end of current word
        while pos < len && !self.msg[pos].is_whitespace() {
            pos += 1;
        }
        
        // We're now at whitespace or end of line
        // If not at end and there's another word, skip whitespace and go to end of next word
        if pos < len && self.msg[pos].is_whitespace() {
            // Skip whitespace
            while pos < len && self.msg[pos].is_whitespace() {
                pos += 1;
            }
            // Go to end of next word
            while pos < len && !self.msg[pos].is_whitespace() {
                pos += 1;
            }
        }
        
        // Move back to last character of word (not the whitespace after it)
        if pos > 0 && pos <= len {
            pos -= 1;
        }
        
        self.msg_cursor = pos;
    }

    // uppercase E
    pub fn move_msg_cursor_end_of_word_uppercase(&mut self) {
        let len = self.msg.len();
        if self.msg_cursor >= len {
            return;
        }

        let mut pos = self.msg_cursor;
        
        // Move to end of current non-whitespace sequence (WORD)
        while pos < len && !self.msg[pos].is_whitespace() {
            pos += 1;
        }
        
        // If we hit whitespace and there's more content
        if pos < len && self.msg[pos].is_whitespace() {
            // Skip all whitespace
            while pos < len && self.msg[pos].is_whitespace() {
                pos += 1;
            }
            // Move to end of next WORD
            while pos < len && !self.msg[pos].is_whitespace() {
                pos += 1;
            }
        }
        
        // Position at last character of WORD
        let _ = pos.saturating_sub(1);
        
        self.msg_cursor = pos;
    }

    pub fn get_msg_iter(&self) -> impl Iterator<Item = char> + '_ {
        self.msg.iter().cloned()
    }

    pub fn insert_msg_char(&mut self, c: char) {
        self.msg.insert(self.msg_cursor, c);
        self.msg_cursor += 1;
    }
    pub fn delete_msg_char(&mut self) {
        if self.msg_cursor == 0 {
            return;
        }
        self.msg.remove(self.msg_cursor.saturating_sub(1));
        self.msg_cursor = self.msg_cursor.saturating_sub(1);
    }
    pub fn delete_inner_word_msg(&mut self) {
        if self.msg.is_empty() {
            return;
        }

        let cursor = self.msg_cursor;
        let len = self.msg.len();

        // If cursor is at the end of buffer, there's nothing to delete
        if cursor >= len {
            return;
        }

        // Find word boundaries
        let (word_start, word_end) = self.find_word_boundaries(cursor);

        // If no word found at cursor position (cursor is on whitespace)
        if word_start == word_end {
            return;
        }

        // Delete the word and store it in yank buffer
        self.msg_cursor = word_start;
        let text = self.take_msg_from_cursor_to_x(word_end);
        self.set_yank(text);
        if self.msg_cursor > self.msg.len().saturating_sub(1) {
            self.move_msg_cursor_left();
        }
    }


    fn find_word_boundaries(&self, cursor: usize) -> (usize, usize) {
        let len = self.msg.len();
        let cursor_in_word = is_word_char(self.msg[cursor]);
        
        // If buffer is empty
        if len == 0 {
            return (0, 0);
        }

        // Helper function to check if a character is word character
        fn is_word_char(c: char) -> bool {
            c.is_alphanumeric() || c == '_'
        }

        // Find start of word
        let mut start = cursor;
        
        // If cursor is on a word character, search backward to find word start
        if cursor < len && cursor_in_word {
            // Move backward until we hit non-word char or start of buffer
            while start > 0 && is_word_char(self.msg[start - 1]) {
                start -= 1;
            }
        } else if !cursor_in_word {
            // if it is not in a word character, move backward to find previous word
            while start > 0 && !is_word_char(self.msg[start - 1]) {
                start -= 1;
            }
        }

        // Find end of word
        let mut end = start;
        if cursor_in_word {
            while end < len && is_word_char(self.msg[end]) {
                end += 1;
            }
        } else {
            while end < len && !is_word_char(self.msg[end]) {
                end += 1;
            }
        }
        (start, end)
    }
    pub fn move_msg_cursor_left(&mut self) {
        self.msg_cursor = self.msg_cursor.saturating_sub(1);
    }

    pub fn move_msg_cursor_right(&mut self) {
        if self.msg_cursor >= self.msg.len() {
            return;
        }
        if (self.vim_mode == VimMode::Visual || self.vim_mode == VimMode::Normal) && self.msg_cursor >= self.msg.len() - 1 {
            return;
        }
        self.msg_cursor += 1;
    }

    pub fn insert_msg_str(&mut self, s: &str) {
        for c in s.chars() {
            self.insert_msg_char(c);
        }
    }

    pub fn move_msg_cursor_to_start(&mut self) {
        self.msg_cursor = 0;
    }

    pub fn move_msg_cursor_to_end(&mut self) {
        self.msg_cursor = self.msg.len();
    }

    pub fn take_msg_from_cursor_to_x(&mut self, x: usize) -> String {
        let start = self.msg_cursor.min(self.msg.len());
        let end = x.min(self.msg.len());
        if start >= end {
            return String::new();
        }
        let mut result = String::new();
        for _ in start..end {
            let c = self.msg.remove(self.msg_cursor);
            result.push(c);
        }
        result
    }

    pub fn msg_cursor_position(&self) -> usize {
        self.msg_cursor
    }

    pub fn clear_msg(&mut self) {
        self.msg.clear();
        self.msg_cursor = 0;
    }

    pub fn take_msg_text(&mut self) -> String {
        self.msg_cursor = 0;
        self.msg.drain(..).collect()
    }
}
