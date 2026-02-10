use super::{App, ServerTreeItem};

impl App {
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
}
