use super::{App, VimMode};

impl App {
    pub fn push_norm_char(&mut self, c: char) {
        self.norm.push(c);
    }

    pub fn clear_norm(&mut self) {
        self.norm.clear();
    }

    pub fn get_norm_text(&self) -> String {
        self.norm.clone()
    }
    pub fn execute_normal(&mut self) {
        let norm = self.get_norm_text();
        match norm.as_str() {
            "dd" => {
                self.clear_msg();
                self.clear_norm();
            }
            "gg" => {
                self.move_msg_cursor_to_start();
                self.clear_norm();
            }
            "diw" => {
                self.delete_inner_word_msg();
                self.clear_norm();
            }
            "G" => {
                self.move_msg_cursor_to_end();
                self.clear_norm();
            }
            "C" => {
                self.clear_messages();
                self.clear_norm();
            }
            "a" => {
                self.vim_mode = VimMode::Insert;
                self.prev_mode = Some(VimMode::Normal);
                self.clear_norm();
            }
            "b" => {
                self.move_msg_cursor_back_word();
                self.clear_norm();
            }
            "B" => {
                self.move_msg_cursor_back_word_uppercase();
                self.clear_norm();
            }
            "w" => {
                self.move_msg_cursor_forward_word();
                self.clear_norm();
            }
            "W" => {
                self.move_msg_cursor_forward_word_uppercase();
                self.clear_norm();
            }
            "e" => {
                self.move_msg_cursor_end_of_word();
                self.clear_norm();
            }
            "E" => {
                self.move_msg_cursor_end_of_word_uppercase();
                self.clear_norm();
            }
            "A" => {
                self.move_msg_cursor_to_end();
                self.vim_mode = VimMode::Insert;
                self.prev_mode = Some(VimMode::Normal);
                self.clear_norm();
            }
            "q" => {
                self.should_quit = true;
            }
            "h" => {
                self.move_msg_cursor_left();
                self.clear_norm();
            }
            "l" => {
                self.move_msg_cursor_right();
                self.clear_norm();
            }
            "p" => {
                self.insert_msg_str(self.yank.clone().as_str());
                self.clear_norm();
            }
            "s" => {
                self.vim_mode = VimMode::Server;
                self.prev_mode = Some(VimMode::Normal);
                self.rebuild_server_tree();
                self.server_tree_index = 0;
                self.clear_norm();
            }
            "v" => {
                self.vim_mode = VimMode::Visual;
                self.prev_mode = Some(VimMode::Normal);
                self.sel_start = Some(self.msg_cursor);
                self.clear_norm();
            }
            "i" => {
                self.vim_mode = VimMode::Insert;
                self.prev_mode = Some(VimMode::Normal);
                self.clear_norm();
            }
            "m" => {
                self.vim_mode = VimMode::Messages;
                self.prev_mode = Some(VimMode::Normal);
                self.clear_norm();
            }
            "c" => {
                self.vim_mode = VimMode::Clients;
                self.prev_mode = Some(VimMode::Normal);
                self.clear_norm();
            }
            _ => {
            }
        }
    }

    pub fn get_avaiable_normal_commands(&self) -> Vec<&'static str> {
        match self.get_norm_text().as_str() {
            "d" => vec!["d -> delete msg", "i -> delete inner"],
            "di" => vec!["w -> delete inner word"],
            "g" => vec!["gg -> go to start of msg"],
            _ => vec![],
        }
    }
}
