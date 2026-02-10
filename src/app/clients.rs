use super::{App, ClientInfo, ChannelContext, VimMode, ChannelInfo};
use crate::irc::IrcCommand;

impl App {
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
}
