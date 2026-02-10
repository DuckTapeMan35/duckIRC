use super::{App, VimMode};

impl App {
    pub fn push_vis_char(&mut self, c: char) {
        self.vis.push(c);
    }
    pub fn clear_vis(&mut self) {
        self.vis.clear();
    }
    pub fn msg_selection_range(&self) -> Option<(usize, usize)> {
        if self.vim_mode != VimMode::Visual {
            return None;
        }

        let sel_start = self.sel_start?;  // This should be pinned when entering visual mode
        let cursor = self.msg_cursor;     // This moves as you navigate
        
        // Return (start, end) where start is always <= end
        // The +1 makes it exclusive for rendering (i < end)
        if cursor >= sel_start {
            Some((sel_start, cursor + 1))
        } else {
            Some((cursor, sel_start + 1))
        }
    }

    pub fn execute_vis(&mut self) {
        let vis = self.vis.clone();
        match vis.as_str() {
            "h" => {
                self.move_msg_cursor_left();
                self.clear_vis();
            }
            "l" => {
                self.move_msg_cursor_right();
                self.clear_vis();
            }
            "y" => {
                // Use msg_selection_range to get the correct range
                if let Some((start, end)) = self.msg_selection_range() {
                    let text = self.msg.iter()
                        .skip(start)
                        .take(end - start)
                        .collect();
                    self.set_yank(text);
                }
                self.clear_vis();
                self.vim_mode = VimMode::Normal;
                self.prev_mode = Some(VimMode::Visual);
            }
            "b" => {
                self.move_msg_cursor_back_word();
                self.clear_vis();
            }
            "B" => {
                self.move_msg_cursor_back_word_uppercase();
                self.clear_vis();
            }
            "w" => {
                self.move_msg_cursor_forward_word();
                self.clear_vis();
            }
            "W" => {
                self.move_msg_cursor_forward_word_uppercase();
                self.clear_vis();
            }
            "e" => {
                self.move_msg_cursor_end_of_word();
                self.clear_vis();
            }
            "E" => {
                self.move_msg_cursor_end_of_word_uppercase();
                self.clear_vis();
            }
            "x" | "d" => {
                // Use msg_selection_range to get the correct range
                if let Some((start, end)) = self.msg_selection_range() {
                    // Move cursor to start position for take_msg_from_cursor_to_x
                    let old_cursor = self.msg_cursor;
                    self.msg_cursor = start;
                    let text = self.take_msg_from_cursor_to_x(end);
                    self.set_yank(text);
                    // Adjust cursor if needed
                    if old_cursor < start {
                        self.msg_cursor = start; // Cursor stays at start after deletion
                    }
                    if self.msg_cursor > self.msg.len().saturating_sub(1) {
                        self.move_msg_cursor_left();
                    }
                }
                self.clear_vis();
                self.vim_mode = VimMode::Normal;
                self.prev_mode = Some(VimMode::Visual);
            }
            _ => {
            }
        }
    }
}
