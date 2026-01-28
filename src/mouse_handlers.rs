use crate::app::{App, VimMode};
use crate::click_state::ClickState;
use crate::irc::IrcCommand;
use crate::ChannelContext;
use crate::ServerTreeItem;
use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use std::iter::once;
use tokio::sync::mpsc;
use ratatui::DefaultTerminal;

pub fn handle_mouse_event(
    app: &mut App,
    mouse: MouseEvent,
    click_state: &mut ClickState,
    irc_tx: &mpsc::UnboundedSender<IrcCommand>,
    terminal: &DefaultTerminal
) {
    let x = mouse.column;
    let y = mouse.row;
    let terminal_heigh = terminal.size().unwrap().height;
    let terminal_width = terminal.size().unwrap().width;
    match mouse.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            handle_left_click(app, x, y, click_state, irc_tx, terminal_heigh, terminal_width);
        },
        MouseEventKind::ScrollUp => {
            handle_scroll_up(app);
        }
        MouseEventKind::ScrollDown => {
            handle_scroll_down(app);
        }
        _ => {}
    }
}

// --- left click handler ---
fn handle_left_click(
    app: &mut App,
    x: u16,
    y: u16,
    click_state: &mut ClickState,
    irc_tx: &mpsc::UnboundedSender<IrcCommand>,
    terminal_height: u16,
    terminal_width: u16,
) {
    match app.vim_mode {
        VimMode::Normal | VimMode::Insert => {
            handle_normal_insert_click(app, x, y);
        }
        VimMode::Clients => {
            handle_clients_click(app, x, y, terminal_height, terminal_width, click_state, irc_tx);
        }
        VimMode::Server => {
            handle_server_click(app, x, y, irc_tx, click_state, terminal_height);
        }
        VimMode::Messages => {
            handle_messages_click(app,  y, terminal_height);
        }
        VimMode::Vimless => {
            handle_vimless_click(app, x, y, terminal_height, terminal_width, irc_tx);
        }
        _ => {}
    }
}

fn handle_server_click(
    app: &mut App,
    x: u16,
    y: u16,
    irc_tx: &mpsc::UnboundedSender<IrcCommand>,
    click_state: &mut ClickState,
    terminal_height: u16,
) {
    let tree_width = app
    .servers
    .iter()
    .flat_map(|s| {
        once(s.name.len())
            .chain(s.channels.iter().map(|c| c.name.len()))
    })
    .max()
    .unwrap_or(0) as u16
    + 10;

    let input_area_start_y = terminal_height.saturating_sub(4);

    let message_area_start_x = tree_width;
    let message_area_start_y = 1;

    // Check if click is within server tree bounds
    if x > 0 && x <= tree_width && y > 0 {
    // Convert click position to tree index
    let tree_item_index = (y as usize).saturating_sub(1);

    if tree_item_index < app.server_tree.len() {
        // Check for double-click
        let is_double = click_state.is_double_click(x, y);
        
        // Always update selection
        app.server_tree_index = tree_item_index;
        
        // If double-click, also execute the Enter action
        if is_double && let Some(item) = app.server_tree.get(tree_item_index) {
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
    } else if x >= message_area_start_x && y >= message_area_start_y && y < input_area_start_y {
        // Click is in message area, switch to Messages mode
        let msg_index = y.saturating_sub(1) as usize;
        app.vim_mode = VimMode::Messages;
        app.move_msg_to_index(msg_index);
    } else if y >= input_area_start_y {
        // click in input area, switch to Normal mode
        app.vim_mode = VimMode::Normal;
    }
}

fn handle_normal_insert_click(
    app: &mut App,
    y: u16,
    terminal_height: u16,
) {
    let message_area_end_y = terminal_height.saturating_sub(4);
    if y <= message_area_end_y {
        let msg_index = y.saturating_sub(1) as usize;
        app.vim_mode = VimMode::Messages;
        app.move_msg_to_index(msg_index);
    }
}

fn handle_clients_click(
    app: &mut App,
    x: u16,
    y: u16,
    terminal_height: u16,
    terminal_width: u16,
    click_state: &mut ClickState,
    irc_tx: &mpsc::UnboundedSender<IrcCommand>,
) {
    let message_area_x_end = terminal_width.saturating_sub(16);
    let message_area_y_end = terminal_height.saturating_sub(4);
    let is_double = click_state.is_double_click(x, y);
    
    if x <= message_area_x_end && y <= message_area_y_end {
        let msg_index = y.saturating_sub(1) as usize;
        app.vim_mode = VimMode::Messages;
        app.move_msg_to_index(msg_index);
    } else if x > message_area_x_end && y <= message_area_y_end {
        if is_double {
            app.join_selected_client_channel(irc_tx);
            app.rebuild_server_tree();
        } else {
            app.move_client_to_index(y.saturating_sub(1) as usize);
        }
    }
    else if y > message_area_y_end {
        app.vim_mode = VimMode::Normal;
        app.prev_mode = Some(VimMode::Clients);
    }
}

fn handle_messages_click(
    app: &mut App,
    y: u16,
    terminal_height: u16,
) {
    let message_area_end_y = terminal_height.saturating_sub(4);
    if y <= message_area_end_y {
        let msg_index = y.saturating_sub(1) as usize;
        app.move_msg_to_index(msg_index);
    } else {
        app.vim_mode = VimMode::Normal;
        app.prev_mode = Some(VimMode::Messages);
    }
}

fn handle_vimless_click(
    app: &mut App,
    x: u16,
    y: u16,
    terminal_height: u16,
    terminal_width: u16,
    irc_tx: &mpsc::UnboundedSender<IrcCommand>,
) {
    let server_tree_width = app
        .servers
        .iter()
        .flat_map(|s| {
            once(s.name.len())
                .chain(s.channels.iter().map(|c| c.name.len()))
        })
        .max()
        .unwrap_or(0) as u16
        + 10;
    let message_area_end_x = terminal_width.saturating_sub(16);
    let message_area_start_y = 1;
    let input_area_start_y = terminal_height.saturating_sub(4);
    match (x, y) {
        (x, y) if x >= message_area_end_x && y >= message_area_start_y && y < input_area_start_y => {
            app.join_selected_client_channel(irc_tx);
            app.rebuild_server_tree();
        }
        (x, y) if x > server_tree_width && x < message_area_end_x && y < input_area_start_y => {
            let msg_index = y.saturating_sub(1) as usize;
            app.yank_msg_at_index(msg_index);
        }
        (x, y) if x <= server_tree_width && x < input_area_start_y => {
            let tree_item_index = (y as usize).saturating_sub(1);
            if let Some(item) = app.server_tree.get(tree_item_index) {
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

// --- scroll up handler ---
fn handle_scroll_up(app: &mut App) {
    match app.vim_mode {
        VimMode::Messages => {
            app.move_msg_up();
        }
        VimMode::Clients => {
            app.move_client_selection_up();
        }
        VimMode::Server => {
            app.move_server_selection_up();
        }
        _ => {}
    }
}

// --- scroll down handler ---
fn handle_scroll_down(app: &mut App) {
    match app.vim_mode {
        VimMode::Messages => {
            app.move_msg_down();
        }
        VimMode::Clients => {
            app.move_client_selection_down();
        }
        VimMode::Server => {
            app.move_server_selection_down();
        }
        _ => {}
    }
}
