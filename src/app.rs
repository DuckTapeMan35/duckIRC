use gapbuf::GapBuffer;
use ratatui::style::Color;
use std::collections::HashMap;
use wl_clipboard_rs::copy::{MimeType, Options, Source};
use crate::irc::IrcCommand;
use crate::servers::ServerConfig;
use crate::ui::color_for_user;
use crate::irc::{get_config_dir, create_default_servers_config};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServerTreeItem {
    Server { server_idx: usize },
    Channel { server_idx: usize, channel_idx: usize },
}

#[derive(Debug, Clone, Default)]
pub struct ChannelMessages {
    pub messages: Vec<ColoredMessage>,
    pub msg_index: usize,
    pub msg_scroll: usize,
    pub viewport_height: usize,
}

#[derive(Debug, Clone)]
pub struct ChannelContext {
    pub server_name: String,
    pub channel_name: String,
}

#[derive(Debug, Clone)]
pub struct ClientInfo {
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct ChannelInfo {
    pub name: String,
    pub topic: Option<String>,
    pub client_count: Option<usize>,
    pub is_joined: bool,
    pub is_dm: bool,
}

#[derive(Debug, Clone)]
pub struct ServerInfo {
    pub name: String,
    pub is_connected: bool,
    pub channels: Vec<ChannelInfo>,
    pub is_expanded: bool,
}

#[derive(Debug, Clone)]
pub struct ColoredMessage {
    pub nick: Option<String>,
    pub text: String,
    pub color: Option<Color>,
}

#[derive(Default, Debug, PartialEq, Clone)]
pub enum VimMode {
    #[default] Normal,
    Insert,
    Visual,
    Command,
    Server,
    Messages,
    Clients,
    Vimless,
}

#[derive(Default)]
pub struct App {
    pub msg: GapBuffer<char>,
    pub cmd: GapBuffer<char>,
    pub norm: String,
    pub vis: String,
    pub messages_cmd: String,
    pub clients_cmd: String,
    pub sel_start: Option<usize>,
    pub yank: String,
    pub msg_cursor: usize,
    pub cmd_cursor: usize,
    pub channel: String,
    pub should_quit: bool,
    pub vim_mode: VimMode,
    pub is_connected: bool,
    pub servers: Vec<ServerInfo>,
    pub server_tree: Vec<ServerTreeItem>,
    pub server_tree_index: usize,
    pub prev_mode: Option<VimMode>,
    pub client_index: usize,
    pub clients: Vec<ClientInfo>,
    pub current_nick: String,
    pub current_channel: Option<ChannelContext>,
    pub channel_messages: HashMap<(String,String), ChannelMessages>,
}

impl App {
    pub fn new() -> Self {
        let config_dir = get_config_dir();
        let server_config_path = config_dir.join("servers.toml");
        if !server_config_path.exists() {
            create_default_servers_config(&server_config_path).ok();
        }
        let server_config = ServerConfig::load(server_config_path.to_str().expect("Invalid path"))
            .unwrap_or_else(|_| ServerConfig::default_config());
        let servers = server_config.servers
            .iter()
            .map(|s| ServerInfo {
                name: s.name.clone(),
                is_connected: false,
                channels: Vec::new(),
                is_expanded: false,
            })
            .collect();
        Self {
            msg: GapBuffer::new(),
            cmd: GapBuffer::new(),
            norm: String::new(),
            vis: String::new(),
            messages_cmd: String::new(),
            clients_cmd: String::new(),
            msg_cursor: 0,
            cmd_cursor: 0,
            channel: String::new(),
            should_quit: false,
            vim_mode: VimMode::Normal,
            sel_start: None,
            yank: String::new(),
            is_connected: false,
            servers,
            server_tree: Vec::new(),
            server_tree_index: 0,
            prev_mode: None,
            client_index: 0,
            clients: Vec::new(),
            current_nick: String::new(),
            channel_messages: HashMap::new(),
            current_channel: None,
        }
    }

    pub fn get_mode_name(&self) -> &str {
        match self.vim_mode {
            VimMode::Normal => "NORMAL",
            VimMode::Insert => "INSERT",
            VimMode::Visual => "VISUAL",
            VimMode::Command => "COMMAND",
            VimMode::Server => "SERVER",
            VimMode::Messages => "MESSAGES",
            VimMode::Clients => "CLIENTS",
            VimMode::Vimless => "VIMLESS",
        }
    }

    pub fn set_yank(&mut self, text: String) {
        // 1. Store in the internal buffer (for pasting within the app with 'p')
        self.yank = text.clone();

        // 2. Copy to the system's Wayland clipboard
        let opts = Options::new();
        opts.copy(
            Source::Bytes(text.into_bytes().into()),
            MimeType::Autodetect,
        ).ok();
    }

    pub fn get_current_messages(&self) -> Option<&ChannelMessages> {
        let (server_name, channel_name) = self.get_current_channel_key()?;
        self.channel_messages.get(&(server_name, channel_name))
    }

    pub fn get_current_messages_mut(&mut self) -> Option<&mut ChannelMessages> {
        let (server_name, channel_name) = self.get_current_channel_key()?;
        self.channel_messages.get_mut(&(server_name, channel_name))
    }

    fn get_current_channel_key(&self) -> Option<(String, String)> {
        self.current_channel.as_ref().map(|ctx| (ctx.server_name.clone(), ctx.channel_name.clone()))
    }

    pub fn push_without_updating_scroll(&mut self, text: String) {
        if let Some(msgs) = self.get_current_messages_mut() {
            msgs.messages.push(ColoredMessage {
                nick: None,
                text,
                color: None,
            });
        }
    }

    pub fn cycle_mode(&mut self) {
        self.vim_mode = match self.vim_mode {
            VimMode::Normal => VimMode::Server,
            VimMode::Insert => VimMode::Server,
            VimMode::Visual => VimMode::Server,
            VimMode::Command => VimMode::Normal,
            VimMode::Server => VimMode::Messages,
            VimMode::Messages => VimMode::Clients,
            VimMode::Clients => VimMode::Normal,
            VimMode::Vimless => VimMode::Vimless,
        };
    }


    pub fn push_initial_messages(&mut self) {

        let ascii_art = "⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀
    ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⣠⣤⣤⣤⣄⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀
    ⠀⠀⠀⠀⠀⠀⠀⠀⣠⡿⠋⢁⡀⠉⠙⣿⡄⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀
    ⠀⠀⠀⠀⢴⣿⣿⣿⣿⡇⠀⠘⠋⠀⠀⢸⡇⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀
    ⠀⠀⠀⠀⠀⠉⠉⠉⠙⠻⣷⡄⠀⠀⢠⣿⠃⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀
    ⠀⠀⠀⠀⠀⠀⠀⠀⠀⣠⣿⣷⣶⣶⣿⠃⢀⡀⠀⠀⠀⢰⡿⢷⣄⠀⠀⠀⠀⠀
    ⠀⠀⠀⠀⠀⠀⠀⢀⣾⠟⠀⠀⠀⣴⣿⣿⣿⣿⣿⣿⣿⣾⡿⠀⢻⣇⠀⠀⠀⠀
    ⠀⠀⠀⠀⠀⠀⠀⣸⡏⠀⠀⠀⢸⣿⣿⣿⣿⣿⣿⣿⣿⡉⠀⠀⢸⣿⠀⠀⠀⠀
    ⠀⠀⠀⠀⠀⠀⠀⢹⣇⠀⠀⠀⠀⠻⣿⣿⣿⣿⣿⣿⡿⠁⠀⢀⣾⠇⠀⠀⠀⠀
    ⠀⠀⠀⠀⠀⠀⠀⠈⢿⣦⡀⠀⠀⠀⠀⠉⠉⠉⠉⠁⠀⢀⣤⡾⠋⠀⠀⠀⠀⠀
    ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠉⠻⠷⣶⣦⣤⣤⣤⣤⣶⡶⠾⠛⠋⠀⠀⠀⠀⠀⠀⠀
    ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⢀⣠⣿⡇⠀⣿⡇⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀
    ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠘⠿⡿⠟⣡⣾⣿⡇⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀
    ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠙⠛⠛⠁⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀
    ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀";
        self.push_without_updating_scroll("Welcome to DuckIRC!".to_string());
        for line in ascii_art.lines() {
            self.push_without_updating_scroll(line.to_string());
        }

    }

    pub fn clear_messages(&mut self) {
        if let Some(msgs) = self.get_current_messages_mut() {
            msgs.messages.clear();
            msgs.msg_index = 0;
            msgs.msg_scroll = 0;
        }
    }

    pub fn return_to_prev_mode(&mut self) {
        let temp = self.prev_mode.clone();
        self.vim_mode = self.prev_mode.clone().unwrap_or(VimMode::Normal);
        self.prev_mode = temp;
    }

    // Push a normal system message
    pub fn push_system_to_current(&mut self, text: String) {
        if let Some(msgs) = self.get_current_messages_mut() {
            let msg_len_before = msgs.messages.len();
            
            msgs.messages.push(ColoredMessage {
                nick: None,
                text,
                color: None,
            });
            
            // Check if we were at bottom before adding
            let was_at_bottom = if msg_len_before > 0 {
                msgs.msg_index == msg_len_before - 1
            } else {
                true // Empty list means we're "at bottom"
            };
            
            if was_at_bottom {
                msgs.msg_index = msgs.messages.len().saturating_sub(1);
                if msgs.viewport_height > 0 {
                    msgs.msg_scroll = msgs.messages
                        .len()
                        .saturating_sub(msgs.viewport_height);
                }
            }
        }
    }

    // Push a user message with optional colored nick
    pub fn push_user_msg_to_current(&mut self, nick: &str, text: &str) {
        if let Some(msgs) = self.get_current_messages_mut() {
            let msg_len_before = msgs.messages.len();
            
            msgs.messages.push(ColoredMessage {
                nick: Some(nick.to_string()),
                text: text.to_string(),
                color: Some(color_for_user(nick)),
            });
            
            // Check if we were at bottom before adding
            let was_at_bottom = if msg_len_before > 0 {
                msgs.msg_index == msg_len_before - 1
            } else {
                true // Empty list means we're "at bottom"
            };
            
            if was_at_bottom {
                msgs.msg_index = msgs.messages.len().saturating_sub(1);
                if msgs.viewport_height > 0 {
                    msgs.msg_scroll = msgs.messages
                        .len()
                        .saturating_sub(msgs.viewport_height);
                }
            }
        }
    }

    // ----------------- Input Buffer Methods ----------------
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

    // ----------------- Command Buffer Methods ----------------
    pub fn insert_cmd_char(&mut self, c: char) {
        if self.cmd_cursor > self.cmd.len() {
            return;
        }
        self.cmd.insert(self.cmd_cursor, c);
        self.cmd_cursor += 1;
    }
    pub fn delete_cmd_char(&mut self) {
        if self.cmd_cursor == 0 {
            return;
        }
        self.cmd.remove(self.cmd_cursor.saturating_sub(1));
        self.cmd_cursor = self.cmd_cursor.saturating_sub(1);
    }
    pub fn move_cmd_cursor_left(&mut self) {
        if self.cmd_cursor == 0 {
            return;
        }
        self.cmd_cursor = self.cmd_cursor.saturating_sub(1);
    }

    pub fn move_cmd_cursor_right(&mut self) {
        if self.cmd_cursor >= self.cmd.len() {
            return;
        }
        self.cmd_cursor += 1;
    }

    pub fn cmd_cursor_position(&self) -> usize {
        self.cmd_cursor
    }

    pub fn clear_cmd(&mut self) {
        self.cmd_cursor = 0;
        self.cmd.clear();
    }
    pub fn take_cmd_text(&mut self) -> String {
        self.cmd_cursor = 0;
        self.cmd.drain(..).collect()
    }
    pub fn get_cmd_text(&self) -> String {
        self.cmd.iter().collect()
    }
    pub fn execute_command(
        &mut self,
        cmd: &str,
        irc_tx: &tokio::sync::mpsc::UnboundedSender<IrcCommand>,
    ) {
        match cmd {
            "quit" | "q" => {
                self.should_quit = true;
            }
            "clear" | "c" => {
                self.clear_messages();
            }
            "Vimless" | "vimless" => {
                self.vim_mode = VimMode::Vimless;
                self.prev_mode = Some(VimMode::Vimless);
                self.rebuild_server_tree();
            }
            s if s.starts_with("set_nick") || s.starts_with("nick") => {
                let parts: Vec<&str> = s.splitn(2, ' ').collect();
                if parts.len() < 2 {
                    self.push_system_to_current("Usage: nick <nickname>".to_string());
                    return;
                }
                let nick = parts[1].trim();
                irc_tx.send(IrcCommand::Nick(nick.to_string())).ok();
                self.current_nick = nick.to_string();
            }
            s if s.starts_with("connect") => {
                if self.is_connected {
                    self.push_system_to_current("Already connected.".to_string());
                } else {
                    let parts: Vec<&str> = s.splitn(2, ' ').collect();
                    if parts.len() < 2 {
                        self.push_system_to_current("Usage: connect <server_name|server:port>".to_string());
                        self.push_system_to_current("Example: connect Libera".to_string());
                        self.push_system_to_current("Example: connect irc.example.org:6667".to_string());
                        return;
                    }
                    
                    let server = parts[1].trim();
                    if server.is_empty() {
                        self.push_system_to_current("Please specify a server".to_string());
                        return;
                    }
                    
                    irc_tx.send(IrcCommand::Connect(server.to_string())).ok();
                    self.push_system_to_current(format!("Connecting to {}...", server));
                }
            }
            s if s.starts_with("disconnect") => {
                if !self.is_connected {
                    self.push_system_to_current("Not connected.".to_string());
                } else {
                    irc_tx.send(IrcCommand::Disconnect).ok();
                    self.is_connected = false;
                    self.push_system_to_current("Disconnected from server.".to_string());
                }
            }
            s if s.starts_with("join") => {
                if !self.is_connected {
                    self.push_system_to_current("Not connected to server yet. Use 'connect <server>' first.".to_string());
                    return;
                }
                
                let parts: Vec<&str> = s.splitn(2, ' ').collect();
                if parts.len() < 2 {
                    self.push_system_to_current("Usage: join <#channel>".to_string());
                    self.push_system_to_current("Example: join #rust".to_string());
                    return;
                }
                
                let channel = parts[1].trim();
                if channel.is_empty() || !channel.starts_with('#') {
                    self.push_system_to_current("Channel must start with #".to_string());
                    return;
                }
                
                let current_server_name = if let Some(current_server) = self.servers.iter().find(|s| s.is_connected) {
                    current_server.name.clone()
                } else {
                    self.push_system_to_current("Error: No server connected".to_string());
                    return;
                };
                
                self.current_channel = Some(ChannelContext {
                    server_name: current_server_name.clone(),
                    channel_name: channel.to_string(),
                });
                
                self.channel_messages
                    .entry((current_server_name.clone(), channel.to_string()))
                    .or_default();
                
                self.channel = channel.to_string();
                
                
                irc_tx.send(IrcCommand::Join(channel.to_string())).ok();
                irc_tx.send(IrcCommand::SetCurrentChannel(channel.to_string())).ok();
                
            }
            s if s.starts_with("msg") => {
                if !self.is_connected {
                    self.push_system_to_current("Not connected to server yet. Use 'connect <server>' first.".to_string());
                    return;
                }

                let parts: Vec<&str> = s.splitn(3, ' ').collect();
                if parts.len() < 3 {
                    self.push_system_to_current("Usage: msg <user> <message>".to_string());
                    self.push_system_to_current("Example: msg Alice Hello!".to_string());
                    return;
                }

                let target_user = parts[1].trim();
                let message = parts[2..].join(" ");
                if message.is_empty() {
                    self.push_system_to_current("Message cannot be empty".to_string());
                    return;
                }

                // Find connected server
                if let Some(pos) = self.servers.iter().position(|s| s.is_connected) {
                    let server_name = self.servers[pos].name.clone();

                    let server = &mut self.servers[pos];

                    // Ensure DM channel exists
                    if !server.channels.iter().any(|c| c.name == target_user) {
                        server.channels.push(ChannelInfo {
                            name: target_user.to_string(),
                            topic: None,
                            client_count: Some(1),
                            is_joined: true,
                            is_dm: true,
                        });
                    }

                    // Ensure message buffer exists BEFORE pushing message
                    self.channel_messages
                        .entry((server_name.clone(), target_user.to_string()))
                        .or_default();

                    // Switch current buffer
                    self.current_channel = Some(ChannelContext {
                        server_name: server_name.clone(),
                        channel_name: target_user.to_string(),
                    });
                    self.channel = target_user.to_string();

                    // Now push message
                    let nick = self.current_nick.clone();
                    self.push_user_msg_to_current(nick.as_str(), message.as_str());
                }

                // Send the message
                irc_tx.send(IrcCommand::Join(target_user.to_string())).ok();
                irc_tx.send(IrcCommand::PrivMsg(message.clone())).ok();
                irc_tx.send(IrcCommand::SetCurrentChannel(target_user.to_string())).ok();
                self.rebuild_server_tree();
            }
            "servers" | "list_servers" => {
                irc_tx.send(IrcCommand::ListServers).ok();
            }
            s if s.starts_with("add_server") || s.starts_with("add") => {
                // Format: add_server <name> <address> <port> [tls]
                let parts: Vec<&str> = s.split_whitespace().collect();
                if parts.len() < 4 {
                    self.push_system_to_current("Usage: add_server <name> <address> <port> [tls]".to_string());
                    self.push_system_to_current("Example: add_server MyServer irc.example.org 6697 true".to_string());
                    return;
                }
                
                let name = parts[1].to_string();
                let address = parts[2].to_string();
                let port = match parts[3].parse::<u16>() {
                    Ok(p) => p,
                    Err(_) => {
                        self.push_system_to_current("Invalid port number".to_string());
                        return;
                    }
                };
                let use_tls = parts.get(4)
                    .map(|s| s.parse::<bool>().unwrap_or(true))
                    .unwrap_or(true);
                
                irc_tx.send(IrcCommand::AddServer {
                    name,
                    address,
                    port,
                    use_tls,
                }).ok();
            }
            s if s.starts_with("remove_server") || s.starts_with("rm_server") => {
                let parts: Vec<&str> = s.splitn(2, ' ').collect();
                if parts.len() < 2 {
                    self.push_system_to_current("Usage: remove_server <name>".to_string());
                    return;
                }
                
                let name = parts[1].trim().to_string();
                irc_tx.send(IrcCommand::RemoveServer(name)).ok();
            }
            "status" => {
                let status = if self.is_connected {
                    "Connected"
                } else {
                    "Disconnected"
                };
                let channel_status = if self.channel.is_empty() {
                    "No channel joined"
                } else {
                    &self.channel.clone()
                };
                self.push_system_to_current(format!("Status: {}", status));
                self.push_system_to_current(format!("Channel: {}", channel_status));
            }
            "" => {
                // Empty command, do nothing
            }
            _ => {
                self.push_system_to_current(format!("Unknown command: {}. Type 'help' for available commands.", cmd));
            }
        }
    }

    // ----------------- Normal Buffer Methods ----------------
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

    // ----------------- sel Buffer Methods ----------------
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

    // ----------------- Server Buffer Methods ----------------
    pub fn move_server_selection_up(&mut self) {
        if self.server_tree_index > 0 {
            self.server_tree_index -= 1;
        }
    }

    pub fn move_server_selection_down(&mut self) {
        if self.server_tree_index + 1 < self.server_tree.len() {
            self.server_tree_index += 1;
        }
    }

    pub fn is_server_connected(&self, server_index: usize) -> bool {
        if let Some(server) = self.servers.get(server_index) {
            server.is_connected
        } else {
            false
        }
    }

    pub fn toggle_server_expansion(&mut self, server_index: usize) {
        for server in &mut self.servers {
            server.is_expanded = false;
        }
        if let Some(server) = self.servers.get_mut(server_index) {
            server.is_expanded = !server.is_expanded;
        }
    }
    pub fn rebuild_server_tree(&mut self) {
        self.server_tree.clear();

        for (s_idx, server) in self.servers.iter().enumerate() {
            self.server_tree.push(ServerTreeItem::Server {
                server_idx: s_idx,
            });

            if server.is_expanded {
                for (c_idx, _) in server.channels.iter().enumerate() {
                    self.server_tree.push(ServerTreeItem::Channel {
                        server_idx: s_idx,
                        channel_idx: c_idx,
                    });
                }
            }
        }

        // Clamp selection
        if self.server_tree_index >= self.server_tree.len() {
            self.server_tree_index = self.server_tree.len().saturating_sub(1);
        }
    }

    // ----------------- Message Buffer Methods ----------------
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

    // ----------------- Client Buffer Methods ----------------
    pub fn move_client_selection_up(&mut self) {
        if self.client_index > 0 {
            self.client_index -= 1;
        }
    }

    pub fn move_client_selection_down(&mut self) {
        if self.client_index + 1 < self.clients.len() {
            self.client_index += 1;
        }
    }

    pub fn get_selected_client(&self) -> Option<&ClientInfo> {
        self.clients.get(self.client_index)
    }

    pub fn client_jump_top(&mut self) {
        self.client_index = 0;
    }

    pub fn client_jump_bottom(&mut self) {
        if self.clients.is_empty() {
            return;
        }
        self.client_index = self.clients.len() - 1;
    }

    pub fn yank_client(&mut self) {
        if let Some(client) = self.clients.get(self.client_index) {
            self.set_yank(client.name.clone());
        }
    }

    pub fn join_selected_client_channel(&mut self, irc_tx: &tokio::sync::mpsc::UnboundedSender<IrcCommand>) {
        if let Some(client) = self.get_selected_client() {
            if !self.is_connected {
                self.push_system_to_current("Not connected to server yet. Use 'connect <server>' first.".to_string());
                return;
            }

            let channel_name = client.name.clone();

            // Find and update the connected server
            let current_server_name = if let Some(server) = self.servers.iter_mut().find(|s| s.is_connected) {
                let server_name = server.name.clone();
                
                // Add channel to server's channel list if not already there
                if !server.channels.iter().any(|c| c.name == channel_name) {
                    server.channels.push(ChannelInfo {
                        name: channel_name.clone(),
                        topic: None,
                        client_count: None,
                        is_joined: true,
                        is_dm: true,
                    });
                }
                
                server_name
            } else {
                self.push_system_to_current("Error: No server connected".to_string());
                return;
            };

            self.current_channel = Some(ChannelContext {
                server_name: current_server_name.clone(),
                channel_name: channel_name.clone(),
            });

            self.channel_messages
                .entry((current_server_name.clone(), channel_name.clone()))
                .or_default();

            self.channel = channel_name.clone();

            irc_tx.send(IrcCommand::Join(channel_name.clone())).ok();
            irc_tx.send(IrcCommand::SetCurrentChannel(channel_name)).ok();
        }
    }

    pub fn move_client_to_index(&mut self, index: usize) {
        if index < self.clients.len() {
            self.client_index = index;
        }
    }

    pub fn clear_clients_cmd(&mut self) {
        self.clients_cmd.clear();
    }

    pub fn push_char_to_clients_cmd(&mut self, c: char) {
        self.clients_cmd.push(c);
    }

    pub fn execute_clients_cmd(&mut self) {
        let cmd = self.clients_cmd.as_str();
        match cmd {
            "q" => {
                self.vim_mode = VimMode::Normal;
                self.prev_mode = Some(VimMode::Clients);
                self.clear_clients_cmd();
            }
            ":" => {
                self.vim_mode = VimMode::Command;
                self.prev_mode = Some(VimMode::Clients);
            }
            "gg" => {
                self.client_jump_top();
                self.clear_clients_cmd();
            }
            "G" => {
                self.client_jump_bottom();
                self.clear_clients_cmd();
            }
            "y" => {
                self.yank_client();
                self.vim_mode = VimMode::Normal;
                self.prev_mode = Some(VimMode::Clients);
                self.clear_clients_cmd();
            }
            "j" => {
                self.move_client_selection_down();
                self.clear_clients_cmd();
            }
            "k" => {
                self.move_client_selection_up();
                self.clear_clients_cmd();
            }
            "m" => {
                self.vim_mode = VimMode::Messages;
                self.prev_mode = Some(VimMode::Clients);
                self.clear_clients_cmd();
            }
            "s" => {
                self.vim_mode = VimMode::Server;
                self.prev_mode = Some(VimMode::Clients);
                self.rebuild_server_tree();
                self.server_tree_index = 0;
                self.clear_clients_cmd();
            }
            "i" => {
                self.vim_mode = VimMode::Insert;
                self.prev_mode = Some(VimMode::Clients);
                self.clear_clients_cmd();
            }
            _ => {
            }
        }
    }

    // ----------------- Vimless Mode Methods ----------------
    pub fn execute_vimless(&mut self, irc_tx: &tokio::sync::mpsc::UnboundedSender<IrcCommand>) {
        let cmd = self.take_msg_text();
        match cmd.as_str() {
            "/quit" | "/q" => {
                self.should_quit = true;
            }
            "/vim" | "/v" => {
                self.vim_mode = VimMode::Normal;
                self.prev_mode = None;
            }
            s if s.starts_with("/nick") => {
                let parts: Vec<&str> = s.splitn(2, ' ').collect();
                if parts.len() < 2 {
                    self.push_system_to_current("Usage: /nick <nickname>".to_string());
                    return;
                }
                let nick = parts[1].trim();
                irc_tx.send(IrcCommand::Nick(nick.to_string())).ok();
                self.current_nick = nick.to_string();
            }
            s if s.starts_with("/connect") => {
                if self.is_connected {
                    self.push_system_to_current("Already connected.".to_string());
                } else {
                    let parts: Vec<&str> = s.splitn(2, ' ').collect();
                    if parts.len() < 2 {
                        self.push_system_to_current("Usage: /connect <server_name|server:port>".to_string());
                        return;
                    }
                    
                    let server = parts[1].trim();
                    if server.is_empty() {
                        self.push_system_to_current("Please specify a server".to_string());
                        return;
                    }
                    
                    irc_tx.send(IrcCommand::Connect(server.to_string())).ok();
                    self.push_system_to_current(format!("Connecting to {}...", server));
                }
            }
            s if s.starts_with("/disconnect") => {
                if !self.is_connected {
                    self.push_system_to_current("Not connected.".to_string());
                } else {
                    irc_tx.send(IrcCommand::Disconnect).ok();
                    self.is_connected = false;
                    self.push_system_to_current("Disconnected from server.".to_string());
                }
            }
            s if s.starts_with("/join") => {
                if !self.is_connected {
                    self.push_system_to_current("Not connected to server yet. Use '/connect <server>' first.".to_string());
                    return;
                }
                
                let parts: Vec<&str> = s.splitn(2, ' ').collect();
                if parts.len() < 2 {
                    self.push_system_to_current("Usage: /join <#channel>".to_string());
                    return;
                }
                
                let channel = parts[1].trim();
                if channel.is_empty() || !channel.starts_with('#') {
                    self.push_system_to_current("Channel must start with #".to_string());
                    return;
                }
                
                let current_server_name = if let Some(current_server) = self.servers.iter().find(|s| s.is_connected) {
                    current_server.name.clone()
                } else {
                    self.push_system_to_current("Error: No server connected".to_string());
                    return;
                };
                
                self.current_channel = Some(ChannelContext {
                    server_name: current_server_name.clone(),
                    channel_name: channel.to_string(),
                });
                
                self.channel_messages
                    .entry((current_server_name.clone(), channel.to_string()))
                    .or_default();
                
                self.channel = channel.to_string();
                
                
                irc_tx.send(IrcCommand::Join(channel.to_string())).ok();
                irc_tx.send(IrcCommand::SetCurrentChannel(channel.to_string())).ok();
                self.rebuild_server_tree();
                
            }
            s if s.starts_with("/msg") => {
                if !self.is_connected {
                    self.push_system_to_current("Not connected to server yet. Use '/connect <server>' first.".to_string());
                    return;
                }

                let parts: Vec<&str> = s.splitn(3, ' ').collect();
                if parts.len() < 3 {
                    self.push_system_to_current("Usage: /msg <user> <message>".to_string());
                    return;
                }

                let target_user = parts[1].trim();
                let message = parts[2..].join(" ");
                if message.is_empty() {
                    self.push_system_to_current("Message cannot be empty".to_string());
                    return;
                }

                // Find connected server
                if let Some(pos) = self.servers.iter().position(|s| s.is_connected) {
                    let server_name = self.servers[pos].name.clone();

                    let server = &mut self.servers[pos];

                    // Ensure DM channel exists
                    if !server.channels.iter().any(|c| c.name == target_user) {
                        server.channels.push(ChannelInfo {
                            name: target_user.to_string(),
                            topic: None,
                            client_count: Some(1),
                            is_joined: true,
                            is_dm: true,
                        });
                    }

                    // Ensure message buffer exists BEFORE pushing message
                    self.channel_messages
                        .entry((server_name.clone(), target_user.to_string()))
                        .or_default();

                    // Switch current buffer
                    self.current_channel = Some(ChannelContext {
                        server_name: server_name.clone(),
                        channel_name: target_user.to_string(),
                    });
                    self.channel = target_user.to_string();

                    // Now push message
                    let nick = self.current_nick.clone();
                    self.push_user_msg_to_current(nick.as_str(), message.as_str());
                }

                // Send the message
                irc_tx.send(IrcCommand::Join(target_user.to_string())).ok();
                irc_tx.send(IrcCommand::PrivMsg(message.clone())).ok();
                irc_tx.send(IrcCommand::SetCurrentChannel(target_user.to_string())).ok();
                self.rebuild_server_tree();
            }
            _ => {
                self.push_user_msg_to_current(self.current_nick.clone().as_str(), cmd.as_str());
                irc_tx.send(IrcCommand::PrivMsg(cmd)).ok();
            }
        }
    }
}
