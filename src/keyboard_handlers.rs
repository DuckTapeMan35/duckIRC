use crate::irc::{IrcCommand, get_user_nick};
use crate::app::{App, VimMode, ChannelContext};
use crate::ServerTreeItem;
use tokio::sync::mpsc;
use ratatui::crossterm::event;
use crossterm::event::KeyEvent;

pub fn handle_keyboard_event(key: KeyEvent, app: &mut App, irc_tx: &mpsc::UnboundedSender<IrcCommand>,) {
    match app.vim_mode {
        VimMode::Normal => {handle_normal(key, app);},
        VimMode::Insert => {handle_insert(key, app, irc_tx);},
        VimMode::Visual => {handle_visual(key, app);},
        VimMode::Command => {handle_command(key, app, irc_tx);},
        VimMode::Server => {handle_server(key, app, irc_tx);},
        VimMode::Messages => {handle_messages(key, app);},
        VimMode::Clients => {handle_clients(key, app, irc_tx);},
        VimMode::Vimless => {handle_vimless(key, app, irc_tx);}
    }
}

fn handle_normal(key: KeyEvent, app: &mut App, ) {
    match key.code {
        event::KeyCode::Tab => {
            app.rebuild_server_tree();
            app.cycle_mode();
        }
        event::KeyCode::Char(':') => {
            app.vim_mode = VimMode::Command;
            app.prev_mode = Some(VimMode::Normal);
            app.clear_norm();
        }
        event::KeyCode::Esc => {
            app.clear_norm();
        }
        event::KeyCode::Left => {
            app.push_norm_char('h');
            app.execute_normal();
        }
        event::KeyCode::Right => {
            app.push_norm_char('l');
            app.execute_normal();
        }
        event::KeyCode::Char(c) => {
            app.push_norm_char(c);
            app.execute_normal();
        }
        _ => {}
    }
}

fn handle_insert(key: KeyEvent, app: &mut App, irc_tx: &mpsc::UnboundedSender<IrcCommand>,) {
    match key.code {
        event::KeyCode::Tab => {
            app.rebuild_server_tree();
            app.cycle_mode();
        }
        event::KeyCode::Esc => {
            app.vim_mode = VimMode::Normal;
            app.prev_mode = Some(VimMode::Insert);
            //move cursor backwards if necessary
            if app.msg_cursor == app.msg.len() {
                app.move_msg_cursor_left();
            }
        }
        event::KeyCode::Char(c) => {
            app.insert_msg_char(c);
        }
        event::KeyCode::Backspace => {
            app.delete_msg_char();
        }
        event::KeyCode::Left => {
            app.move_msg_cursor_left();
        }
        event::KeyCode::Right => {
            app.move_msg_cursor_right();
        }
        event::KeyCode::Enter => {
            let msg = app.take_msg_text();
            if !msg.is_empty() {
                // Send to IRC
                irc_tx.send(IrcCommand::PrivMsg(msg.clone())).ok();
                // Echo locally
                let nick = get_user_nick().unwrap_or("guest".to_string());
                app.push_user_msg_to_current(&nick, &msg);
            }
            app.msg_cursor = 0;
        }
        _ => {}
    }
}

fn handle_visual(key: KeyEvent, app: &mut App, ) {
    match key.code {
        event::KeyCode::Tab => {
            app.rebuild_server_tree();
            app.cycle_mode();
        }
        event::KeyCode::Esc => {
            app.vim_mode = VimMode::Normal;
            app.prev_mode = Some(VimMode::Visual);
            app.vis.clear();
            app.sel_start = None;
        }
        event::KeyCode::Left => {
            app.push_vis_char('h');
            app.execute_vis();
        }
        event::KeyCode::Right => {
            app.push_vis_char('l');
            app.execute_vis();
        }
        event::KeyCode::Char(c) => {
            app.push_vis_char(c);
            app.execute_vis();
        }
        _ => {}
    }
}

fn handle_command(key: KeyEvent, app: &mut App, irc_tx: &mpsc::UnboundedSender<IrcCommand>,) {
    match key.code {
        event::KeyCode::Esc => {
            app.clear_cmd();
            app.return_to_prev_mode();
        }
        event::KeyCode::Char(c) => {
            app.insert_cmd_char(c);
        }
        event::KeyCode::Backspace => {
            app.delete_cmd_char();
        }
        event::KeyCode::Left => {
            app.move_cmd_cursor_left();
        }
        event::KeyCode::Right => {
            app.move_cmd_cursor_right();
        }
        event::KeyCode::Enter => {
            let cmd = app.take_cmd_text();
            app.execute_command(&cmd, irc_tx);
            app.return_to_prev_mode();
        }
        _ => {}
    }
}

fn handle_server(key: KeyEvent, app: &mut App, irc_tx: &mpsc::UnboundedSender<IrcCommand>) {
    match key.code {
        event::KeyCode::Tab => {
            app.cycle_mode();
        }
        event::KeyCode::Esc => {
            app.vim_mode = VimMode::Normal;
            app.prev_mode = Some(VimMode::Server);
        }
        event::KeyCode::Char('c') => {
            app.vim_mode = VimMode::Clients;
            app.prev_mode = Some(VimMode::Server);
        }
        event::KeyCode::Char('q') => {
            app.vim_mode = VimMode::Normal;
        }
        event::KeyCode::Char('m') => {
            app.vim_mode = VimMode::Messages;
            app.prev_mode = Some(VimMode::Server);
        }
        event::KeyCode::Char('i') => {
            app.vim_mode = VimMode::Insert;
            app.prev_mode = Some(VimMode::Server);
        }
        event::KeyCode::Down => {
            app.move_server_selection_down();
        }
        event::KeyCode::Up => {
            app.move_server_selection_up();
        }
        event::KeyCode::Char(':') => {
            app.vim_mode = VimMode::Command;
            app.prev_mode = Some(VimMode::Server);
        }
        event::KeyCode::Enter => {
            if let Some(tree_index) = Some(app.server_tree_index) && let Some(item) = app.server_tree.get(tree_index) {
                match item {
                    ServerTreeItem::Server { server_idx } => {
                        let server_idx_copy = *server_idx;
                        let server = &app.servers[server_idx_copy];
                        let server_name = server.name.clone();

                        if app.is_server_connected(server_idx_copy) {
                            irc_tx.send(IrcCommand::Disconnect).ok();
                            app.push_system_to_current(format!("Disconnecting from {}...", server_name));
                            
                            app.current_channel = None;
                            app.channel.clear();
                        } else {
                            // Disconnect any currently connected server
                            irc_tx.send(IrcCommand::Disconnect).ok();

                            // Connect to this server
                            irc_tx.send(IrcCommand::Connect(server_name.clone())).ok();
                            app.push_system_to_current(format!("Connecting to {}...", server_name));
                            
                            app.current_channel = Some(ChannelContext {
                                server_name: server_name.clone(),
                                channel_name: "status".to_string(),
                            });

                            app.channel_messages
                                .entry((server_name.clone(), "status".to_string()))
                                .or_default();
                        }

                        app.toggle_server_expansion(server_idx_copy);
                    }
                    ServerTreeItem::Channel { server_idx, channel_idx } => {
                        let server = &app.servers[*server_idx];
                        let channel = &server.channels[*channel_idx];
                        let channel_name = channel.name.clone();

                        // Auto-join the channel if connected to server
                        if app.is_server_connected(*server_idx) {
                            irc_tx.send(IrcCommand::Join(channel_name.clone())).ok();
                            
                            app.current_channel = Some(ChannelContext {
                                server_name: server.name.clone(),
                                channel_name: channel_name.clone(),
                            });

                            irc_tx.send(IrcCommand::SetCurrentChannel(channel_name.clone())).ok();
                            
                            // Initialize messages for this channel if needed
                            app.channel_messages
                                .entry((server.name.clone(), channel_name.clone()))
                                .or_default();
                            
                            app.channel = channel_name.clone();
                        } else {
                            app.push_system_to_current(format!(
                                "Not connected to server {}. Connect first.",
                                server.name
                            ));
                        }
                    }
                }
            }
        }
        _ => {}
    }
}

fn handle_messages(key: KeyEvent, app: &mut App, ) {
    match key.code {
        event::KeyCode::Tab => {
            app.cycle_mode();
        }
        event::KeyCode::Esc => {
            app.vim_mode = VimMode::Normal;
            app.prev_mode = Some(VimMode::Messages);
        }
        event::KeyCode::Down => {
            app.move_msg_down();
        }
        event::KeyCode::Up => {
            app.move_msg_up();
        }
        event::KeyCode::Char(c) => {
            app.push_char_to_messages_cmd(c);
            app.execute_messages_cmd();
        }
        _ => {}
    }
}

fn handle_clients(key: KeyEvent, app: &mut App, irc_tx: &mpsc::UnboundedSender<IrcCommand>,) {
    match key.code {
        event::KeyCode::Tab => {
            app.cycle_mode();
        }
        event::KeyCode::Esc => {
            app.vim_mode = VimMode::Normal;
            app.prev_mode = Some(VimMode::Clients);
        }
        event::KeyCode::Down => {
            app.move_client_selection_down();
        }
        event::KeyCode::Up => {
            app.move_client_selection_up();
        }
        event::KeyCode::Enter => {
            app.join_selected_client_channel(irc_tx);
            app.rebuild_server_tree();
        }
        event::KeyCode::Char(c) => {
            app.push_char_to_clients_cmd(c);
            app.execute_clients_cmd();
        }
        _ => {}
    }
}

fn handle_vimless(key: KeyEvent, app: &mut App, irc_tx: &mpsc::UnboundedSender<IrcCommand>,) {
    match key.code {
        event::KeyCode::Enter => {
            app.execute_vimless(irc_tx);
        }
        event::KeyCode::Char(c) => {
            app.insert_msg_char(c);
        }
        event::KeyCode::Left => {
            app.move_msg_cursor_left();
        }
        event::KeyCode::Right => {
            app.move_msg_cursor_right();
        }
        event::KeyCode::Backspace => {
            app.delete_msg_char();
        }
        event::KeyCode::Up => {
            app.move_msg_cursor_to_start();
        }
        event::KeyCode::Down => {
            app.move_msg_cursor_to_end();
        }
        _ => {}
    }
}
