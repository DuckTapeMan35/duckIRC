use anyhow::Result;
use futures_util::StreamExt;
use irc::client::prelude::*;
use irc::proto::Command;
use tokio::sync::mpsc;
use std::fs;
use std::path::PathBuf;
use dirs::home_dir;

use crate::servers::ServerConfig;

#[derive(Debug)]
pub enum UiEvent {
    Connected { nick: String, server_name: String },
    Disconnected {server_name: String},
    Message(String),
    Error(String),
    ChannelUpdate {
        server_name: String,
        channel_name: String,
        topic: Option<String>,
        client_count: usize,
        clients: Vec<String>,
        is_joined: bool,
        is_dm: bool,
    },
}

#[derive(Debug)]
pub enum IrcCommand {
    Connect(String),      // Connect to server (name or address:port)
    Join(String),         // Join a channel
    PrivMsg(String),      // Send a message
    Nick(String),         // Change nickname
    ListServers,          // List saved servers
    AddServer { name: String, address: String, port: u16, use_tls: bool },
    RemoveServer(String), // Remove server by name
    Disconnect,          // Disconnect from server
    SetCurrentChannel(String), // Update the channel we are viewing
}

pub async fn run_irc(
    ui_tx: mpsc::UnboundedSender<UiEvent>,
    mut irc_rx: mpsc::UnboundedReceiver<IrcCommand>,
) -> Result<()> {
    let mut client: Option<Client> = None;
    let mut stream: Option<irc::client::ClientStream> = None;
    let mut current_channel = String::new();
    let mut current_server_name = String::new();
    let mut accumulated_channels: Vec<String> = Vec::new();
    let config_dir = ensure_config_dir()?;
    let server_config_path = config_dir.join("servers.toml");
    if !server_config_path.exists() {
        create_default_servers_config(&server_config_path)?;
    }
    let mut server_config = ServerConfig::load(server_config_path.to_str().expect("Invalid path"))
        .unwrap_or_else(|_| ServerConfig::default_config());

    loop {
        tokio::select! {
            Some(cmd) = irc_rx.recv() => {
                match cmd {
                    IrcCommand::Connect(server_str) => {
                        if client.is_some() {
                            client = None;
                            ui_tx.send(UiEvent::Disconnected { server_name: current_server_name.clone() }).ok();
                        }
                        
                        // Try to find server by name first
                        let (host, port, use_tls, server_name) = if let Some(server) = server_config.get_server(&server_str) {
                            (server.address.clone(), server.port, server.use_tls, server.name.clone())
                        } else {
                            // Parse as address:port
                            let (h, p, t) = parse_server_address(&server_str);
                            (h, p, t, server_str.clone())
                        };

                        current_server_name = server_name.clone();
                        accumulated_channels.clear();

                        let config = Config {
                            nickname: Some(get_user_nick()?),
                            server: Some(host.clone()),
                            port: Some(port),
                            use_tls: Some(use_tls),
                            ..Default::default()
                        };

                        match Client::from_config(config).await {
                            Ok(mut c) => {
                                if let Err(e) = c.identify() {
                                    ui_tx.send(UiEvent::Error(format!("Failed to identify: {}", e))).ok();
                                    continue;
                                }

                                let nick = c.current_nickname().to_string();
                                ui_tx.send(UiEvent::Connected { 
                                    nick: nick.clone(),
                                    server_name: server_name.clone(),
                                }).ok();

                                stream = Some(c.stream()?);
                                client = Some(c);
                            }
                            Err(e) => {
                                ui_tx.send(UiEvent::Error(format!("Failed to connect: {}", e))).ok();
                            }
                        }
                    }

                    IrcCommand::Join(channel) => {
                        if let Some(c) = &client {
                            c.send_join(&channel)?;
                            current_channel = channel;
                            c.send(Command::NAMES(Some(current_channel.clone()), None))?;
                        } else {
                            ui_tx.send(UiEvent::Error("Not connected yet".to_string())).ok();
                        }
                    }

                    IrcCommand::PrivMsg(msg) => {
                        if let Some(c) = &client {
                            if current_channel.is_empty() {
                                ui_tx.send(UiEvent::Error("No channel joined".to_string())).ok();
                            } else {
                                c.send_privmsg(&current_channel, &msg)?;
                            }
                        } else {
                            ui_tx.send(UiEvent::Error("Not connected yet".to_string())).ok();
                        }
                    }

                    IrcCommand::Nick(nick) => {
                        if let Some(c) = &client {
                            c.send(Command::NICK(nick.clone()))?;
                            set_user_nick(&nick).ok();
                        } else {
                            ui_tx.send(UiEvent::Error("Not connected yet".to_string())).ok();
                        }
                    }
                    
                    IrcCommand::ListServers => {
                        let servers = server_config.list_servers();
                        if servers.is_empty() {
                            ui_tx.send(UiEvent::Message("No servers configured".to_string())).ok();
                        } else {
                            ui_tx.send(UiEvent::Message("Configured servers:".to_string())).ok();
                            for server in servers {
                                ui_tx.send(UiEvent::Message(format!("  {}", server))).ok();
                            }
                        }
                    }
                    
                    IrcCommand::AddServer { name, address, port, use_tls } => {
                        let added = server_config.add_server(name.clone(), address, port, use_tls);
                        if let Err(e) = server_config.save(server_config_path.to_str().expect("invalid path")) {
                            ui_tx.send(UiEvent::Error(format!("Failed to save config: {}", e))).ok();
                        } else if added {
                            ui_tx.send(UiEvent::Message(format!("Added server: {}", name))).ok();
                        } else {
                            ui_tx.send(UiEvent::Error(format!("Server with name '{}' already exists", name))).ok();
                        }
                    }
                    
                    IrcCommand::RemoveServer(name) => {
                        if server_config.remove_server(&name) {
                            if let Err(e) = server_config.save(server_config_path.to_str().expect("invalid path")) {
                                ui_tx.send(UiEvent::Error(format!("Failed to save config: {}", e))).ok();
                            } else {
                                ui_tx.send(UiEvent::Message(format!("Removed server: {}", name))).ok();
                            }
                        } else {
                            ui_tx.send(UiEvent::Error(format!("Server not found: {}", name))).ok();
                        }
                    }
                    
                    IrcCommand::Disconnect => {
                        if let Some(client) = client.take() {
                            drop(client);
                        }

                        ui_tx
                            .send(UiEvent::Disconnected {
                                server_name: current_server_name.clone(),
                            })
                            .ok();
                    }
                    IrcCommand::SetCurrentChannel(channel) => {
                        current_channel = channel;
                    }
                }
            }

            // Handle incoming IRC messages
            Some(irc_msg) = async {
                if let Some(s) = &mut stream { s.next().await } else { None }
            } => {
                let msg = irc_msg?;
                match &msg.command {
                    Command::Response(Response::RPL_NAMREPLY, params) => {
                        if params.len() >= 4 {
                            let channel = params[2].clone();
                            let names = parse_names(&params[3]);

                            ui_tx.send(UiEvent::ChannelUpdate {
                                server_name: current_server_name.clone(),
                                channel_name: channel,
                                topic: None,
                                client_count: names.len(),
                                clients: names,
                                is_joined: true,
                                is_dm: false,
                            }).ok();
                        }
                    }
                    Command::PRIVMSG(target, text) => {
                        let nick = msg.source_nickname().unwrap_or("?");
                        let is_dm = target == client
                            .as_ref()
                            .map(|c| c.current_nickname())
                            .unwrap_or("");

                        ui_tx.send(UiEvent::Message(format!(
                            "<{}> {}",
                            nick,
                            text
                        ))).ok();
                        if is_dm {
                            ui_tx.send(UiEvent::ChannelUpdate {
                                server_name: current_server_name.clone(),
                                channel_name: nick.to_string(),
                                topic: None,
                                client_count: 1,
                                clients: vec![nick.to_string()],
                                is_joined: true,
                                is_dm: true,
                            }).ok();
                        }
                    }

                    Command::JOIN(channel, _, _) => {
                        if let Some(nick) = msg.source_nickname() {
                            ui_tx.send(UiEvent::Message(format!("{} joined {}", nick, channel))).ok();
                            if channel == &current_channel && let Some(c) = &client {
                                c.send(Command::NAMES(Some(channel.clone()), None)).ok();
                            }
                        }
                    }

                    Command::PART(channel, _) => {
                        if let Some(nick) = msg.source_nickname() {
                            ui_tx.send(UiEvent::Message(format!("{} left {}", nick, channel))).ok();
                            if channel == &current_channel && let Some(c) = &client {
                                c.send(Command::NAMES(Some(channel.clone()), None)).ok();
                            }
                        }
                    }

                    Command::QUIT(_) => {
                        if let Some(nick) = msg.source_nickname() {
                            ui_tx.send(UiEvent::Message(format!("{} quit", nick))).ok();
                            if !current_channel.is_empty() && let Some(c) = &client{
                                c.send(Command::NAMES(Some(current_channel.clone()), None)).ok();
                            }
                        }
                    }

                    Command::NAMES(_, Some(names_str)) => {
                        let clients = parse_names(names_str);
                        // Send ChannelUpdate with actual count
                        ui_tx.send(UiEvent::ChannelUpdate {
                            server_name: current_server_name.clone(),
                            channel_name: current_channel.clone(),
                            topic: None,
                            client_count: clients.len(),
                            clients,
                            is_joined: true,
                            is_dm: false,
                        }).ok();
                    }

                    _ => {}
                }
            }
        }
    }
}


pub fn get_config_dir() -> PathBuf {
    if let Some(home) = home_dir() {
        home.join(".config").join("duckIRC")
    } else {
        PathBuf::from("./duckIRC")
    }
}

fn ensure_config_dir() -> Result<PathBuf> {
    let config_dir = get_config_dir();
    fs::create_dir_all(&config_dir)?;
    Ok(config_dir)
}

pub fn parse_server_address(input: &str) -> (String, u16, bool) {
    let input = input.trim();
    let (server_part, is_tls) = input.strip_prefix("tls ")
        .map(|stripped| (stripped, true))
        .unwrap_or((input, false));

    // Split server:port
    let parts: Vec<&str> = server_part.split(':').collect();
    
    if parts.len() != 2 {
        panic!("Invalid server address format. Expected <server:port> or tls <server:port>");
    }

    let server = parts[0].to_string();
    let port = parts[1].parse::<u16>()
        .expect("Port must be a valid u16 number");

    (server, port, is_tls)
}

pub fn get_user_nick() -> Result<String> {
    let config_dir = ensure_config_dir()?;
    let config_path = config_dir.join("runtime_config.toml");
    
    // Create default config if it doesn't exist
    if !config_path.exists() {
        create_default_runtime_config(&config_path)?;
    }
    
    let config = Config::load(&config_path)?;
    Ok(config.nickname.unwrap_or("unknown".to_string()))
}

pub fn set_user_nick(nick: &str) -> Result<()> {
    let config_dir = ensure_config_dir()?;
    let config_path = config_dir.join("runtime_config.toml");
    
    // Create default config if it doesn't exist
    if !config_path.exists() {
        create_default_runtime_config(&config_path)?;
    }
    
    let mut config = Config::load(&config_path)?;
    config.nickname = Some(nick.to_string());
    config.save(&config_path)?;
    Ok(())
}

fn parse_names(names_str: &str) -> Vec<String> {
    names_str
        .split_whitespace()
        .map(|s| s.trim_start_matches('@').trim_start_matches('+').to_string())
        .collect()
}

fn create_default_runtime_config(path: &PathBuf) -> Result<()> {
    let default_config = r##"nickname = "duck"
nick_password = "duck"
username = "duck"
realname = "duck"
server = "thepiratesplunder.org"
port = 6697
password = ""
use_tls = true
encoding = "UTF-8"
channels = ["#TPP"]
umodes = "+RB-x"
user_info = "test user"
ping_time = 180
ping_timeout = 20
burst_window_length = 8
max_messages_in_burst = 15
ghost_sequence = []"##;
    
    fs::write(path, default_config)?;
    Ok(())
}

pub fn create_default_servers_config(path: &PathBuf) -> Result<()> {
    let default_config = r##"[[servers]]
name = "Libera"
address = "irc.libera.chat"
port = 6697
use_tls = true

[[servers]]
name = "OFTC"
address = "irc.oftc.net"
port = 6697
use_tls = true

[[servers]]
name = "tpp"
address = "thepiratesplunder.org"
port = 6697
channels = ["#TPP"]"##;
    
    fs::write(path, default_config)?;
    Ok(())
}
