use crate::app::{App, ChannelContext, VimMode};

use crate::irc::IrcCommand;
use super::ChannelInfo;

impl App {
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
            "list" | "ls" => {
                if !self.is_connected {
                    self.push_system_to_current("Not connected to server yet. Use 'connect <server>' first.".to_string());
                    return;
                }
                
                irc_tx.send(IrcCommand::ListChannels).ok();
                self.push_system_to_current("Requesting channel list...".to_string());
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
}
