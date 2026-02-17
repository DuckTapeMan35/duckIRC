use gapbuf::GapBuffer;
use std::collections::HashMap;
use wl_clipboard_rs::copy::{MimeType, Options, Source};

use super::*;
use crate::servers::ServerConfig;
use crate::ui::color_for_user;
use crate::irc::{get_config_dir, create_default_servers_config};

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

    pub fn make_system_channel_if_missing(&mut self, server_name: &str, channel_name: &str) {
        let key = (server_name.to_string(), channel_name.to_string());
        self.channel_messages.entry(key).or_insert_with(|| ChannelMessages {
            messages: Vec::new(),
            msg_index: 0,
            msg_scroll: 0,
            viewport_height: 0,
        });
    }

    pub fn set_current_channel(&mut self, server_name: &str, channel_name: &str) {
        self.current_channel = Some(ChannelContext {
            server_name: server_name.to_string(),
            channel_name: channel_name.to_string(),
        });
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
}
