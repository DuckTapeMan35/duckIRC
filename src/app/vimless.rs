use super::{App, VimMode, ChannelContext, ChannelInfo};
use crate::irc::IrcCommand;

impl App {
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
