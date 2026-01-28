use color_eyre::eyre::Result;
use ratatui::{DefaultTerminal, crossterm::{event::{self, Event}}};
use crossterm::event::{EnableMouseCapture, DisableMouseCapture};
use crossterm::execute;
use crossterm::event::MouseEventKind;
use crossterm::event::MouseButton;
use tokio::sync::mpsc;
use tokio::time::{Duration, Instant};
use std::iter::once;
mod app;
use app::{App, VimMode, ClientInfo, ChannelInfo, ChannelContext};
use app::ServerTreeItem;
mod irc;
use irc::*;
mod ui;
use ui::render;
mod servers;

struct ClickState {
    last_click_time: Option<Instant>,
    last_click_pos: Option<(u16, u16)>,
    double_click_threshold: Duration,
}

impl ClickState {
    fn new() -> Self {
        Self {
            last_click_time: None,
            last_click_pos: None,
            double_click_threshold: Duration::from_millis(500),
        }
    }

    fn is_double_click(&mut self, x: u16, y: u16) -> bool {
        let now = Instant::now();
        let is_double = if let Some(last_time) = self.last_click_time {
            if let Some((last_x, last_y)) = self.last_click_pos {
                now.duration_since(last_time) <= self.double_click_threshold && last_x == x && last_y == y
            } else {
                false
            }
        } else {
            false
        };

        self.last_click_time = Some(now);
        self.last_click_pos = Some((x, y));
        is_double
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    let (irc_tx, irc_rx) = mpsc::unbounded_channel::<IrcCommand>();  // UI -> IRC
    let (ui_tx, mut ui_rx) = mpsc::unbounded_channel::<UiEvent>(); // IRC -> UI
    
    // Start the IRC client
    tokio::spawn(run_irc(ui_tx.clone(), irc_rx));
    
    let mut app = App::new();
    app.push_initial_messages();
    
    let initial_nick = get_user_nick().unwrap_or("guest".to_string());
    app.current_nick = initial_nick;
    execute!(std::io::stdout(), EnableMouseCapture)?;
    let terminal = ratatui::init();
    let result = run(terminal, &mut app, irc_tx, &mut ui_rx).await;
    execute!(std::io::stdout(), DisableMouseCapture)?;
    ratatui::restore();
    result
}

async fn run(
    mut terminal: DefaultTerminal, 
    app: &mut App,
    irc_tx: mpsc::UnboundedSender<IrcCommand>,
    ui_rx: &mut mpsc::UnboundedReceiver<UiEvent>,
) -> Result<()> {
    let mut click_state = ClickState::new();
    loop {
        if app.should_quit {
            break;
        }
        
        // Check for IRC messages (non-blocking)
        while let Ok(event) = ui_rx.try_recv() {
            match event {
                UiEvent::Connected { nick , server_name} => {
                    app.is_connected = true;
    
                    // Ensure we have a status channel for this server
                    app.current_channel = Some(ChannelContext {
                        server_name: server_name.clone(),
                        channel_name: "status".to_string(),
                    });
                    
                    // Initialize messages for status channel
                    app.channel_messages
                        .entry((server_name.clone(), "status".to_string()))
                        .or_default();
                    
                    app.push_system_to_current(format!("✔ Connected as {}", nick));
                    app.push_system_to_current("':join #channel' to join a channel".to_string());
                    
                    // Update server connection status
                    for server in &mut app.servers {
                        if server.name == server_name {
                            server.is_connected = true;
                            break;
                        }
                    }
                }
                UiEvent::Disconnected { server_name } => {
                    app.is_connected = false;
                    for server in &mut app.servers {
                        if server.name == server_name {
                            server.is_connected = false;
                            break;
                        }
                    }
                }
                UiEvent::Message(msg) => {
                    // Parse nick from message if you use <nick> format
                    if let Some((nick, text)) = msg.strip_prefix('<').and_then(|s| s.split_once('>')) {
                        app.push_user_msg_to_current(nick, text);
                    } else {
                        app.push_system_to_current(msg); // fallback for system messages
                    }
                }
                UiEvent::Error(err) => {
                    app.push_system_to_current(format!("✖ IRC error: {}", err));
                    if err.contains("connection") || err.contains("connect") {
                        app.is_connected = false;
                    }
                }
                UiEvent::ChannelUpdate {
                    server_name,
                    channel_name,
                    topic,
                    client_count,
                    clients,
                    is_joined,
                    is_dm,
                } => {
                    for server in &mut app.servers {
                        if server.name != server_name {
                            continue;
                        }

                        // Try to find existing channel
                        let mut found = false;

                        for channel in &mut server.channels {
                            if channel.name == channel_name {
                                channel.topic = topic.clone();
                                channel.client_count = Some(client_count);
                                channel.is_joined = is_joined;
                                channel.is_dm = is_dm;
                                found = true;
                                break;
                            }
                        }

                        if !found {
                            server.channels.push(ChannelInfo {
                                name: channel_name.clone(),
                                topic: topic.clone(),
                                client_count: Some(client_count),
                                is_joined,
                                is_dm
                            });
                        }
                    }

                    if let Some(current) = &app.current_channel && (current.server_name == server_name && current.channel_name == channel_name) {
                        app.clients = clients
                            .into_iter()
                            .map(|nick| ClientInfo {
                                name: nick,
                            })
                            .collect();
                    }

                    app.rebuild_server_tree();
                }
            }
        }
        
        terminal.draw(|f| {render(f, app);})?;
        
        // Use a timeout to poll both events and IRC messages
        if event::poll(Duration::from_millis(50))? {
            match event::read()? {
                Event::Key(key) => {
                    match app.vim_mode {
                        VimMode::Normal => {
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
                        VimMode::Insert => {
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
                                        irc_tx.send(irc::IrcCommand::PrivMsg(msg.clone())).ok();
                                        // Echo locally
                                        let nick = get_user_nick().unwrap_or("guest".to_string());
                                        app.push_user_msg_to_current(&nick, &msg);
                                    }
                                    app.msg_cursor = 0;
                                }
                                _ => {}
                            }
                        }
                        VimMode::Visual => {
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
                        VimMode::Command => {
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
                                    app.execute_command(&cmd, &irc_tx);
                                    app.return_to_prev_mode();
                                }
                                _ => {}
                            }
                        }
                        VimMode::Server => {
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
                        VimMode::Messages => {
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
                        VimMode::Clients => {
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
                                    app.join_selected_client_channel(&irc_tx);
                                    app.rebuild_server_tree();
                                }
                                event::KeyCode::Char(c) => {
                                    app.push_char_to_clients_cmd(c);
                                    app.execute_clients_cmd();
                                }
                                _ => {}
                            }
                        }
                        VimMode::Vimless => {
                            match key.code {
                                event::KeyCode::Enter => {
                                    app.execute_vimless(&irc_tx);
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
                    }
                }
                Event::Mouse(mouse) => {
                    match mouse.kind {
                        MouseEventKind::Down(MouseButton::Left) => {
                            match app.vim_mode {
                                VimMode::Server => {
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
                                    
                                    // Server tree is in the leftmost panel
                                    let click_x = mouse.column;
                                    let click_y = mouse.row;

                                    let terminal_heigh = terminal.size()?.height;
                                    let input_area_start_y = terminal_heigh.saturating_sub(4);

                                    let message_area_start_x = tree_width;
                                    let message_area_start_y = 1;
                                    
                                    // Check if click is within server tree bounds
                                    if click_x > 0 && click_x <= tree_width && click_y > 0 {
                                        // Convert click position to tree index
                                        let tree_item_index = (click_y as usize).saturating_sub(1);
                                        
                                        if tree_item_index < app.server_tree.len() {
                                            // Check for double-click
                                            let is_double = click_state.is_double_click(click_x, click_y);
                                            
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
                                    } else if click_x >= message_area_start_x && click_y >= message_area_start_y && click_y < input_area_start_y {
                                        // Click is in message area, switch to Messages mode
                                        let click_y = mouse.row;
                                        let msg_index = click_y.saturating_sub(1) as usize;
                                        app.vim_mode = VimMode::Messages;
                                        app.move_msg_to_index(msg_index);
                                    } else if click_y >= input_area_start_y {
                                        app.vim_mode = VimMode::Normal;
                                    }
                                }
                                VimMode::Messages => {
                                    let click_y = mouse.row;
                                    let terminal_height = terminal.size()?.height;
                                    let message_area_end_y = terminal_height.saturating_sub(4);
                                    if click_y <= message_area_end_y {
                                        let msg_index = click_y.saturating_sub(1) as usize;
                                        app.move_msg_to_index(msg_index);
                                    } else {
                                        app.vim_mode = VimMode::Normal;
                                        app.prev_mode = Some(VimMode::Messages);
                                    }
                                }
                                VimMode::Normal | VimMode::Insert => {
                                    let click_y = mouse.row;
                                    let terminal_height = terminal.size()?.height;
                                    let message_area_end_y = terminal_height.saturating_sub(4);
                                    if click_y <= message_area_end_y {
                                        let msg_index = click_y.saturating_sub(1) as usize;
                                        app.vim_mode = VimMode::Messages;
                                        app.move_msg_to_index(msg_index);
                                    }
                                }
                                VimMode::Clients => {
                                    let click_y = mouse.row;
                                    let click_x = mouse.column;
                                    let terminal_width = terminal.size()?.width;
                                    let terminal_height = terminal.size()?.height;
                                    let message_area_x_end = terminal_width.saturating_sub(16);
                                    let message_area_y_end = terminal_height.saturating_sub(4);
                                    let is_double = click_state.is_double_click(click_x, click_y);
                                    
                                    if click_x <= message_area_x_end && click_y <= message_area_y_end {
                                        let msg_index = click_y.saturating_sub(1) as usize;
                                        app.vim_mode = VimMode::Messages;
                                        app.move_msg_to_index(msg_index);
                                    } else if click_x > message_area_x_end && click_y <= message_area_y_end {
                                        if is_double {
                                            app.join_selected_client_channel(&irc_tx);
                                            app.rebuild_server_tree();
                                        } else {
                                            app.move_client_to_index(click_y.saturating_sub(1) as usize);
                                        }
                                    }
                                    else if click_y > message_area_y_end {
                                        app.vim_mode = VimMode::Normal;
                                        app.prev_mode = Some(VimMode::Clients);
                                    }
                                }
                                VimMode::Vimless => {
                                    let terminal_height = terminal.size()?.height;
                                    let terminal_width = terminal.size()?.width;
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
                                    let click_y = mouse.row;
                                    let click_x = mouse.column;
                                    match (click_x, click_y) {
                                        (x, y) if x >= message_area_end_x && y >= message_area_start_y && y < input_area_start_y => {
                                            app.join_selected_client_channel(&irc_tx);
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
                                _ => {}
                            }
                        }
                        MouseEventKind::ScrollUp => {
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
                        MouseEventKind::ScrollDown => {
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
                        _ => {}
                    }
                }
                _ => {}
            }
        }
    }
    Ok(())
}
