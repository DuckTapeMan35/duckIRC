use super::{App, VimMode};

impl App {
    pub fn move_msg_to_index(&mut self, index: usize) {
        if let Some(msgs) = self.get_current_messages_mut() && index < msgs.messages.len() {
            msgs.msg_index = index;
            
            if msgs.msg_index < msgs.msg_scroll {
                msgs.msg_scroll = msgs.msg_index;
            } else if msgs.msg_index >= msgs.msg_scroll + msgs.viewport_height {
                msgs.msg_scroll = msgs.msg_index.saturating_sub(msgs.viewport_height - 1);
            }
        }
    }

    pub fn yank_msg_at_index(&mut self, index: usize) {
        if let Some(msgs) = self.get_current_messages() && let Some(message) = msgs.messages.get(index) {
            self.set_yank(message.text.clone());
        }
    }
    pub fn move_msg_up(&mut self) {
        if let Some(msgs) = self.get_current_messages_mut() {
            if msgs.msg_index > 0 {
                msgs.msg_index -= 1;
            }
            
            if msgs.msg_index < msgs.msg_scroll {
                msgs.msg_scroll = msgs.msg_index;
            }
        }
    }

    pub fn move_msg_down(&mut self) {
        if let Some(msgs) = self.get_current_messages_mut() {
            if msgs.msg_index + 1 < msgs.messages.len() {
                msgs.msg_index += 1;
            }
            
            if msgs.msg_index >= msgs.msg_scroll + msgs.viewport_height {
                msgs.msg_scroll = msgs.msg_index.saturating_sub(msgs.viewport_height - 1);
            }
        }
    }

    pub fn msg_jump_top(&mut self) {
        if let Some(msgs) = self.get_current_messages_mut() {
            msgs.msg_index = 0;
            msgs.msg_scroll = 0;
        }
    }

    pub fn msg_jump_bottom(&mut self) {
        if let Some(msgs) = self.get_current_messages_mut() {
            if msgs.messages.is_empty() {
                return;
            }
            msgs.msg_index = msgs.messages.len() - 1;
            msgs.msg_scroll = msgs.messages.len().saturating_sub(msgs.viewport_height);
        }
    }


    pub fn yank_msg(&mut self) {
        if let Some(msgs) = self.get_current_messages() && let Some(message) = msgs.messages.get(msgs.msg_index) {
            self.set_yank( message.text.clone());
        }
    }

    pub fn push_char_to_messages_cmd(&mut self, c: char) {
        self.messages_cmd.push(c);
    }

    pub fn clear_messages_cmd(&mut self) {
        self.messages_cmd.clear();
    }

    pub fn execute_messages_cmd(&mut self) {
        let cmd = self.messages_cmd.as_str();
        match cmd {
            "q" => {
                self.vim_mode = VimMode::Normal;
                self.prev_mode = Some(VimMode::Messages);
                self.clear_messages_cmd();
            }
            "gg" => {
                self.msg_jump_top();
                self.clear_messages_cmd();
            }
            "G" => {
                self.msg_jump_bottom();
                self.clear_messages_cmd();
            }
            "y" => {
                self.yank_msg();
                self.vim_mode = VimMode::Normal;
                self.prev_mode = Some(VimMode::Messages);
                self.clear_messages_cmd();
            }
            ":" => {
                self.vim_mode = VimMode::Command;
                self.prev_mode = Some(VimMode::Messages);
                self.clear_messages_cmd();
            }
            "s" => {
                self.vim_mode = VimMode::Server;
                self.prev_mode = Some(VimMode::Messages);
                self.rebuild_server_tree();
                self.server_tree_index = 0;
                self.clear_messages_cmd();
            }
            "j" => {
                self.move_msg_down();
                self.clear_messages_cmd();
            }
            "k" => {
                self.move_msg_up();
                self.clear_messages_cmd();
            }
            "c" => {
                self.vim_mode = VimMode::Clients;
                self.prev_mode = Some(VimMode::Messages);
                self.clear_messages_cmd();
            }
            _ => {
            }
        }
    }
}
