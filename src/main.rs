use color_eyre::eyre::Result;
use ratatui::{DefaultTerminal, crossterm::{event::{self, Event}}};
use crossterm::event::{EnableMouseCapture, DisableMouseCapture};
use crossterm::execute;
use tokio::sync::mpsc;
use tokio::time::Duration;
mod app;
use app::{App, ClientInfo, ChannelInfo, ChannelContext};
use app::ServerTreeItem;
mod irc;
use irc::*;
mod ui;
use ui::render;
mod servers;
mod click_state;
use click_state::ClickState;
mod mouse_handlers;
use mouse_handlers::handle_mouse_event;
mod keyboard_handlers;
use keyboard_handlers::*;

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
                    handle_keyboard_event(key, app, &irc_tx);
                }
                Event::Mouse(mouse) => {
                    handle_mouse_event(app, mouse, &mut click_state, &irc_tx, &terminal);
                }
                _ => {}
            }
        }
    }
    Ok(())
}
