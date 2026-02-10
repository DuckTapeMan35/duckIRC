use gapbuf::GapBuffer;
use ratatui::style::Color;
use std::collections::HashMap;

pub mod core;
pub mod input;
pub mod command;
pub mod server;
pub mod normal;
pub mod visual;
pub mod messages;
pub mod clients;
pub mod vimless;

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
    // input buffers
    pub msg: GapBuffer<char>,
    pub cmd: GapBuffer<char>,

    // vim mode trackers
    pub vim_mode: VimMode,
    pub prev_mode: Option<VimMode>,

    // vim motion buffers
    pub norm: String,
    pub vis: String,
    pub messages_cmd: String,
    pub clients_cmd: String,

    // visual mode selection
    pub sel_start: Option<usize>,
    pub yank: String,

    // cursors
    pub msg_cursor: usize,
    pub cmd_cursor: usize,

    // irc state
    pub channel: String,
    pub should_quit: bool,
    pub is_connected: bool,
    pub servers: Vec<ServerInfo>,
    pub server_tree: Vec<ServerTreeItem>,
    pub server_tree_index: usize,
    pub current_nick: String,

    // ui state
    pub client_index: usize,
    pub clients: Vec<ClientInfo>,
    pub current_channel: Option<ChannelContext>,
    pub channel_messages: HashMap<(String,String), ChannelMessages>,
}


