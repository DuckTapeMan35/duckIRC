use ratatui::{
    widgets::{Clear, Wrap, Paragraph, List, Borders, Block, BorderType, ListItem},
    Frame,
    prelude::*,
    symbols::line,
    text::{Span, Line},
};
use crate::app::{App, VimMode, ColoredMessage};
use crate::app::ServerTreeItem;
use crossterm::cursor::SetCursorStyle;
use crossterm::execute;
use std::io::stdout;

pub fn render(frame: &mut Frame, app: &mut App) {
    // ── Snapshot immutable app state ────────────────────────────
    let vim_mode = app.vim_mode.clone();
    let prev_mode = app.prev_mode.clone();

    let channel_name = app.channel.clone();
    let clients = app.clients.clone();
    let client_index = app.client_index;

    let server_tree = app.server_tree.clone();
    let server_tree_index = app.server_tree_index;

    let mode_name = app.get_mode_name().to_string();
    let msg_chars: Vec<char> = app.get_msg_iter().collect();
    let selection = app.msg_selection_range();
    let msg_cursor_pos = app.msg_cursor_position();

    // ── Cursor style ─────────────────────────────────────────────
    let cursor_style = match vim_mode {
        VimMode::Insert | VimMode::Command => SetCursorStyle::BlinkingBar,
        VimMode::Server => SetCursorStyle::SteadyBlock,
        _ => SetCursorStyle::BlinkingBlock,
    };
    let _ = execute!(stdout(), cursor_style);

    // ── Outer border ─────────────────────────────────────────────
    let [border_area] =
        Layout::vertical([Constraint::Fill(1)]).areas(frame.area());

    Block::bordered()
        .border_type(BorderType::Rounded)
        .render(border_area, frame.buffer_mut());

    // ── Main vertical layout ─────────────────────────────────────
    let layout = Layout::vertical([
        Constraint::Min(1),
        Constraint::Length(3),
    ])
    .split(frame.area());

    // ── Horizontal main split ────────────────────────────────────
    let tree_width = app
        .servers
        .iter()
        .flat_map(|s| {
            std::iter::once(s.name.len())
                .chain(s.channels.iter().map(|c| c.name.len()))
        })
        .max()
        .unwrap_or(0) as u16
        + 10;

    let servers_tab = vim_mode ==  VimMode::Server 
        || (vim_mode == VimMode::Command && prev_mode == Some(VimMode::Server))
        || vim_mode == VimMode::Vimless;

    let clients_tab = vim_mode ==  VimMode::Clients 
        || (vim_mode == VimMode::Command && prev_mode == Some(VimMode::Clients))
        || vim_mode == VimMode::Vimless;

    let main_chunks = Layout::horizontal([
        Constraint::Length(if servers_tab { tree_width } else { 0 }),
        Constraint::Min(1),
        Constraint::Length(if clients_tab { 15 } else { 0 }),
    ])
    .split(layout[0]);

    // ── Servers tree ─────────────────────────────────────────────
    if servers_tab {
        let items = create_tree_view(app);

        let widget = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .title("Servers"),
            )
            .highlight_style(
                Style::default()
                    .bg(Color::Rgb(45, 63, 118))
                    .bold(),
            );

        frame.render_widget(
            widget,
            main_chunks[0],
        );
    }

    // ── Messages  ───────────────────────────────────────────
    let mut message_lines = Vec::new();
    let mut msg_index = 0usize;
    let mut msg_scroll = 0usize;

    if let Some(msgs) = app.get_current_messages_mut() {
    let viewport_height = main_chunks[1].height.saturating_sub(3) as usize;
    let max_width = (main_chunks[1].width.saturating_sub(2)) as usize;
    
    msgs.viewport_height = viewport_height;
    msg_index = msgs.msg_index;
    
    // Calculate total visual lines up to and including current message
    let mut visual_lines_to_current = 0;
    for i in 0..=msg_index.min(msgs.messages.len().saturating_sub(1)) {
        if let Some(msg) = msgs.messages.get(i) {
            let positions = get_wrapped_line_positions(msg, max_width);
            visual_lines_to_current += positions.len();
        }
    }
    
    // Auto-scroll: ensure current message's last visual line is visible
    let current_visual_line = visual_lines_to_current.saturating_sub(1);
    
    if current_visual_line >= msgs.msg_scroll + viewport_height {
        // Scroll down so current line is at bottom of viewport
        msgs.msg_scroll = current_visual_line.saturating_sub(viewport_height - 1);
    } else if current_visual_line < msgs.msg_scroll {
        // Scroll up so current line is at top of viewport
        msgs.msg_scroll = current_visual_line;
    }
    
    msg_scroll = msgs.msg_scroll;
    
    // Now build visible lines based on visual line range
    let mut current_visual_line = 0;
    
    for (msg_idx, msg) in msgs.messages.iter().enumerate() {
        let positions = get_wrapped_line_positions(msg, max_width);
        
        for (line_in_msg, _) in positions.iter().enumerate() {
            // Check if this visual line is in the viewport
            if current_visual_line >= msg_scroll && current_visual_line < msg_scroll + viewport_height {
                // Extract the portion of text for this wrapped line
                let line_text = get_wrapped_line_text(msg, max_width, line_in_msg);
                
                let mut line = Line::from(line_text);
                
                // Highlight if this is the selected message
                if vim_mode == VimMode::Messages && msg_idx == msg_index {
                    line.spans = line.spans.into_iter()
                        .map(|s| Span::styled(
                            s.content,
                            s.style.bg(Color::Rgb(45, 63, 118)).bold(),
                        ))
                        .collect();
                }
                
                message_lines.push(line);
            }
            
            current_visual_line += 1;
            
            // Stop if we've filled the viewport
            if message_lines.len() >= viewport_height {
                break;
            }
        }
        
        if message_lines.len() >= viewport_height {
            break;
        }
    }
}

    let messages_widget = Paragraph::new(message_lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(channel_name),
        )
        .wrap(Wrap { trim: true });

    frame.render_widget(messages_widget, main_chunks[1]);

    // ── Clients panel ─────────────────────────────────────────────
    if clients_tab {
        let items: Vec<ListItem> = clients
            .iter()
            .map(|c| {
                ListItem::new(Span::styled(
                    &c.name,
                    Style::default()
                        .fg(color_for_user(&c.name))
                        .bold(),
                ))
            })
            .collect();

        let widget = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .title("Clients"),
            )
            .highlight_style(
                Style::default()
                    .bg(Color::Rgb(45, 63, 118))
                    .bold(),
            );

        frame.render_widget(
            widget,
            main_chunks[2],
        );
    }

    // ── Input bar ────────────────────────────────────────────────
    let input_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title("Input");

    frame.render_widget(input_block.clone(), layout[1]);
    let inner = input_block.inner(layout[1]);

    let input_chunks = Layout::horizontal([
        Constraint::Length((mode_name.len() + 2) as u16),
        Constraint::Min(1),
    ])
    .split(inner);

    let bg = match vim_mode {
        VimMode::Normal => Color::Blue,
        VimMode::Insert => Color::LightGreen,
        VimMode::Visual => Color::LightMagenta,
        VimMode::Command => Color::Yellow,
        VimMode::Server => Color::Cyan,
        VimMode::Messages => Color::LightBlue,
        VimMode::Clients => Color::LightCyan,
        VimMode::Vimless => Color::Gray,
    };

    frame.render_widget(
        Paragraph::new(mode_name)
            .alignment(Alignment::Center)
            .style(Style::default().bg(bg).fg(Color::Black).bold()),
        input_chunks[0],
    );

    let mut spans = vec![Span::raw(" ")];
    for (i, c) in msg_chars.iter().enumerate() {
        let mut style = Style::default().bold();
        if let Some((s, e)) = selection && i >= s && i < e {
            style = style.bg(Color::DarkGray).fg(Color::Black);
        }
        spans.push(Span::styled(c.to_string(), style));
    }

    frame.render_widget(
        Paragraph::new(Line::from(spans)),
        input_chunks[1],
    );

    // ── Custom vertical separator (RESTORED) ─────────────────────
    let sep_x = input_chunks[1].x;
    let top = layout[1].y;
    let bottom = layout[1].y + layout[1].height - 1;
    let buf = frame.buffer_mut();

    buf[(sep_x, top)].set_symbol(line::HORIZONTAL_DOWN);
    buf[(sep_x, top + 1)].set_symbol(line::VERTICAL);
    buf[(sep_x, bottom)].set_symbol(line::HORIZONTAL_UP);

    // ── Cursor positioning ───────────────────────────────────────
    match vim_mode {
        VimMode::Insert | VimMode::Normal | VimMode::Visual | VimMode::Vimless => {
            frame.set_cursor_position((
                input_chunks[1].x + 1 + msg_cursor_pos as u16,
                input_chunks[1].y,
            ));
        }
        VimMode::Messages => {
            if let Some(msgs) = app.channel_messages.get(&(app.current_channel.as_ref().unwrap().server_name.clone(), app.current_channel.as_ref().unwrap().channel_name.clone())) && msg_index < msgs.messages.len() {
                let max_width = (main_chunks[1].width.saturating_sub(2)) as usize;
                // Calculate total visual lines before current message
                let mut visual_line_index = 0;
                // Sum up all wrapped lines from messages before the current one
                for i in 0..msg_index {
                    if let Some(prev_msg) = msgs.messages.get(i) {
                        let positions = get_wrapped_line_positions(prev_msg, max_width);
                        visual_line_index += positions.len();
                    }
                }
                
                // Add current visual line within message
                let visual_line_in_msg = 0;
                
                visual_line_index += visual_line_in_msg;
                
                // Apply scroll offset
                let y = visual_line_index.saturating_sub(msg_scroll) as u16;
                
                frame.set_cursor_position((
                    main_chunks[1].x + 1,
                    main_chunks[1].y + 1 + y,
                ));
            }
        }
        VimMode::Server => {
            if let Some(item) = server_tree.get(server_tree_index) {
                let x = match item {
                    ServerTreeItem::Server { .. } => 1,
                    ServerTreeItem::Channel { .. } => 4,
                };
                frame.set_cursor_position((
                    main_chunks[0].x + x,
                    main_chunks[0].y + 1 + server_tree_index as u16,
                ));
            }
        }
        VimMode::Clients => {
            frame.set_cursor_position((
                main_chunks[2].x + 1,
                main_chunks[2].y + 1 + client_index as u16,
            ));
        }
        _ => {}
    }

    // ── Command popup ────────────────────────────────────────────
    if vim_mode == VimMode::Command {
        let area = centered_rect(50, 75, frame.area());
        frame.render_widget(Clear, area);

        frame.render_widget(
            Paragraph::new(format!(":{}", app.get_cmd_text()))
                .style(Style::default().bold())
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .title("Command"),
                ),
            area,
        );

        frame.set_cursor_position((
            area.x + 2 + app.cmd_cursor_position() as u16,
            area.y + 1,
        ));
    }

    // ── Normal-mode hint ─────────────────────────────────────────
    if vim_mode == VimMode::Normal && !app.norm.is_empty() {
        let hint_area = right_rect(20, 20, frame.area());
        frame.render_widget(Clear, hint_area);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title(match app.get_norm_text().as_str() {
                "d" => "Delete",
                "g" => "Goto",
                _ => "",
            });

        frame.render_widget(block.clone(), hint_area);

        let inner = block.inner(hint_area);
        frame.render_widget(
            List::new(
                app.get_avaiable_normal_commands()
                    .iter()
                    .map(|c| Line::from(*c))
                    .collect::<Vec<_>>(),
            ),
            inner,
        );
    }
}

// ────────────────────────────────────────────────────────────────
// Helpers
// ────────────────────────────────────────────────────────────────

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let v = Layout::vertical([
        Constraint::Percentage((100 - percent_y) / 2),
        Constraint::Length(3),
        Constraint::Percentage((100 - percent_y) / 2),
    ]).split(area);

    Layout::horizontal([
        Constraint::Percentage((100 - percent_x) / 2),
        Constraint::Percentage(percent_x),
        Constraint::Percentage((100 - percent_x) / 2),
    ]).split(v[1])[1]
}

fn right_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let v = Layout::vertical([
        Constraint::Percentage(100 - percent_y),
        Constraint::Percentage(100 - percent_y),
        Constraint::Percentage(100 - percent_y),
    ]).split(area);

    Layout::horizontal([
        Constraint::Percentage(100 - percent_x - 1),
        Constraint::Percentage(percent_x),
    ]).split(v[1])[1]
}

fn create_tree_view(app: &App) -> Vec<ListItem<'_>> {
    let mut items = Vec::new();

    for row in &app.server_tree {
        match *row {
            ServerTreeItem::Server { server_idx } => {
                let server = &app.servers[server_idx];
                let status = if server.is_connected { "✓" } else { "✗" };
                let style = Style::default().fg(Color::White).bold();

                items.push(ListItem::new(Line::from(vec![
                    Span::styled(&server.name, style),
                    Span::styled(format!(" [{}]", status), if server.is_connected { Color::Green } else { Color::Red }),
                ])));
            }
            ServerTreeItem::Channel { server_idx, channel_idx } => {
                let server = &app.servers[server_idx];
                let channel = &server.channels[channel_idx];

                // Prefix ─ like ├── or ╰──
                let prefix = if channel_idx + 1 == server.channels.len() { "╰──" } else { "├──" };
                let style = if channel.is_joined {
                    Style::default().fg(Color::LightBlue)
                } else {
                    Style::default().fg(Color::DarkGray)
                };

                let channel_name = if channel.is_dm {
                    format!("@{}", channel.name)
                } else {
                    channel.name.clone()
                };
                let mut spans = vec![
                    Span::styled(prefix, Style::default().fg(Color::DarkGray)),
                    Span::styled(channel_name, style),
                ];

                // Show user count if available
                if let Some(count) = channel.client_count {
                    spans.push(Span::styled(
                        format!(" ({})", count),
                        Style::default().fg(Color::Yellow),
                    ));
                }

                items.push(ListItem::new(Line::from(spans)));
            }
        }
    }

    items
}

pub fn color_for_user(nick: &str) -> Color {
    let colors = [
        Color::Red,
        Color::Green,
        Color::Yellow,
        Color::Blue,
        Color::Magenta,
        Color::Cyan,
        Color::LightRed,
        Color::LightGreen,
        Color::LightYellow,
        Color::LightBlue,
        Color::LightMagenta,
        Color::LightCyan,
    ];

    // Hash the nick to pick a color
    let mut hash = 0u64;
    for b in nick.bytes() {
        hash = hash.wrapping_mul(31).wrapping_add(b as u64);
    }
    colors[(hash as usize) % colors.len()]
}

fn get_wrapped_line_positions(msg: &ColoredMessage, max_width: usize) -> Vec<usize> {
    let mut positions = Vec::new();
    
    // Calculate the full display text including nick if present
    let display_text = if let Some(nick) = &msg.nick {
        format!("<{}> {}", nick, msg.text)
    } else {
        msg.text.clone()
    };
    
    let mut current_pos = 0;
    let mut remaining = display_text.as_str();
    
    while !remaining.is_empty() {
        positions.push(current_pos);
        
        if remaining.len() <= max_width {
            break;
        }
        
        // Try to find a good break point (space)
        let mut break_at = max_width;
        if let Some(last_space) = remaining[..max_width].rfind(' ') {
            break_at = last_space + 1; // Include the space in current line
        }
        
        current_pos += break_at;
        remaining = &remaining[break_at..];
    }
    
    positions
}

fn get_wrapped_line_text(msg: &ColoredMessage, max_width: usize, line_index: usize) -> Vec<Span<'_>> {
    // Build full display text
    let display_text = if let Some(nick) = &msg.nick {
        format!("<{}> {}", nick, msg.text)
    } else {
        msg.text.clone()
    };
    
    let mut current_line = 0;
    let mut remaining = display_text.as_str();
    
    while !remaining.is_empty() {
        if current_line == line_index {
            // This is the line we want
            let line_end = if remaining.len() <= max_width {
                remaining.len()
            } else {
                // Find break point
                remaining[..max_width]
                    .rfind(' ')
                    .map(|i| i + 1)
                    .unwrap_or(max_width)
            };
            
            let line_text = &remaining[..line_end];
            
            // Apply coloring for nick if this is the first line
            if line_index == 0 && msg.nick.is_some() {
                let nick = msg.nick.as_ref().unwrap();
                let nick_part = format!("<{}>", nick.clone());
                return vec![
                    Span::styled(
                        nick_part.clone(),
                        Style::default().fg(msg.color.unwrap_or(Color::White)),
                    ),
                    Span::raw(line_text[nick_part.len()..].to_string()),
                ];
            } else {
                return vec![Span::raw(line_text.to_string())];
            }
        }
        
        // Move to next line
        let break_at = if remaining.len() <= max_width {
            remaining.len()
        } else {
            remaining[..max_width]
                .rfind(' ')
                .map(|i| i + 1)
                .unwrap_or(max_width)
        };
        
        remaining = &remaining[break_at..];
        current_line += 1;
    }
    
    vec![Span::raw("")]
}
