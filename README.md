# DuckIRC

A terminal-based IRC client with Vim-inspired keybindings, built with Rust and Ratatui.

```
    ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⣠⣤⣤⣤⣄⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀
    ⠀⠀⠀⠀⠀⠀⠀⠀⣠⡿⠋⢁⡀⠉⠙⣿⡄⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀
    ⠀⠀⠀⠀⢴⣿⣿⣿⣿⡇⠀⠘⠋⠀⠀⢸⡇⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀
    ⠀⠀⠀⠀⠀⠉⠉⠉⠙⠻⣷⡄⠀⠀⢠⣿⠃⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀
    ⠀⠀⠀⠀⠀⠀⠀⠀⠀⣠⣿⣷⣶⣶⣿⠃⢀⡀⠀⠀⠀⢰⡿⢷⣄⠀⠀⠀⠀⠀
    ⠀⠀⠀⠀⠀⠀⠀⢀⣾⠟⠀⠀⠀⣴⣿⣿⣿⣿⣿⣿⣿⣾⡿⠀⢻⣇⠀⠀⠀⠀
    ⠀⠀⠀⠀⠀⠀⠀⣸⡏⠀⠀⠀⢸⣿⣿⣿⣿⣿⣿⣿⣿⡉⠀⠀⢸⣿⠀⠀⠀⠀
    ⠀⠀⠀⠀⠀⠀⠀⢹⣇⠀⠀⠀⠀⠻⣿⣿⣿⣿⣿⣿⡿⠁⠀⢀⣾⠇⠀⠀⠀⠀
    ⠀⠀⠀⠀⠀⠀⠀⠈⢿⣦⡀⠀⠀⠀⠀⠉⠉⠉⠉⠁⠀⢀⣤⡾⠋⠀⠀⠀⠀⠀
    ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠉⠻⠷⣶⣦⣤⣤⣤⣤⣶⡶⠾⠛⠋⠀⠀⠀⠀⠀⠀⠀
```

## Features

-  **Vim-inspired keybindings** - Multiple modes (Normal, Insert, Visual, Command)
-  **TUI Interface** - Clean terminal UI with mouse support
-  **Server Tree** - Visual server and channel navigation
-  **Multiple Channels** - Support for channels and direct messages
-  **Colored Nicks** - Unique colors for each user
-  **Clipboard Integration** - Wayland clipboard support
-  **Server Management** - Add, remove, and manage multiple IRC servers
-  **TLS Support** - Secure connections with TLS/SSL

## Installation

### Prerequisites

- Rust 1.70 or higher
- Wayland (for clipboard support)

### Build from source

```bash
git clone https://github.com/yourusername/duckirc
cd duckirc
cargo build --release
```

The binary will be available at `target/release/duckirc`
(there is a release build already included if you simply clone the repo, but if you don't trust me you can build it yourself).

## Quick Start

1. **Launch DuckIRC**:
   ```bash
   ./duckirc
   ```

2. **Connect to a server**:
   ```
   :connect Libera
   ```

3. **Join a channel**:
   ```
   :join #rust
   ```

4. **Start chatting**:
   - Press `i` to enter Insert mode
   - Type your message
   - Press `Enter` to send

## Modes

DuckIRC operates in different modes, similar to Vim:

### Normal Mode
Default mode for navigation and commands
- `i` - Enter Insert mode
- `v` - Enter Visual mode
- `:` - Enter Command mode
- `s` - Enter Server mode
- `m` - Enter Messages mode
- `c` - Enter Clients mode
- `q` - Quit

### Insert Mode
For typing messages
- `Esc` - Return to Normal mode
- `Enter` - Send message
- `Tab` - Switch to Server mode

### Visual Mode
For selecting text
- `h/l` - Move cursor left/right
- `y` - Yank (copy) selection
- `d/x` - Delete selection
- `Esc` - Return to Normal mode

### Command Mode
For executing commands
- `:connect <server>` - Connect to a server
- `:join <#channel>` - Join a channel
- `:msg <user> <message>` - Send a direct message
- `:nick <nickname>` - Change your nickname
- `:quit` or `:q` - Quit the application
- `:clear` or `:c` - Clear messages
- `:add_server <name> <address> <port> [tls]` - Add a server
- `:remove_server <name>` - Remove a server

### Server Mode
Navigate and manage servers/channels
- `j/k` or `↑/↓` - Navigate
- `Enter` - Connect/disconnect server or join channel
- Double-click - Same as Enter
- `Esc` - Return to Normal mode

### Messages Mode
Navigate through chat history
- `j/k` or `↑/↓` - Scroll messages
- `y` - Yank (copy) selected message
- `gg` - Jump to top
- `G` - Jump to bottom
- `Esc` - Return to Normal mode

### Clients Mode
View and interact with users in current channel
- `j/k` or `↑/↓` - Navigate users
- `Enter` - Start direct message
- `y` - Copy username
- `Esc` - Return to Normal mode

### Vimless Mode
A simplified mode without Vim keybindings
- Type normally and press `Enter` to send
- `/quit` or `/q` - Quit
- `/vim` or `/v` - Return to Normal mode
- Commands use `/` prefix instead of `:`

## Keybindings

### Navigation (Normal/Insert/Visual modes)
- `h/l` or `←/→` - Move cursor
- `w` - Move forward by word
- `b` - Move backward by word
- `e` - Move to end of word
- `W/B/E` - Word movements (WORD-based)
- `gg` - Jump to start
- `G` - Jump to end

### Editing (Normal mode)
- `a` - Append (enter Insert mode after cursor)
- `A` - Append at end of line
- `dd` - Delete entire line
- `diw` - Delete inner word
- `p` - Paste from yank buffer

### Mouse Support
- Click to position cursor
- Double-click to select in Server mode
- Scroll to navigate in Messages/Clients/Server modes

## Configuration

DuckIRC stores configuration in `~/.config/duckIRC/`:

### servers.toml

Manage your IRC servers:
```toml
[[servers]]
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
channels = ["#TPP"]
```

Beware of rapidly changing between servers as it may lead to unexpected behavior. I am investigating why it happens.

### runtime_config.toml
You can change these manually but I kind of don't recommend it yet.

User settings:

```toml
nickname = "duck"
username = "duck"
realname = "duck"
```

## Project Structure

```
duckirc/
├── src/
│   ├── main.rs          # Main application loop and event handling
│   ├── app.rs           # Application state and logic
│   ├── irc.rs           # IRC protocol handling
│   ├── ui.rs            # TUI rendering
│   └── servers.rs       # Server configuration management
└── Cargo.toml
```

## Dependencies

- `ratatui` - Terminal UI framework
- `crossterm` - Terminal manipulation
- `tokio` - Async runtime
- `irc` - IRC protocol implementation
- `gapbuf` - Gap buffer for efficient text editing
- `wl-clipboard-rs` - Wayland clipboard integration
- `serde` & `toml` - Configuration serialization

## Acknowledgments

- Inspired by Vim's modal editing
- Built with Ratatui and the Rust IRC library

## TODO

- [ ] Refactor this godawfull codebase into something a human can read
- [ ] Fix rapid server switching issues
- [ ] Add inline image support
- [ ] Add mode vim commands
- [ ] Add more IRC commands
- [ ] Fix bug where the initial greeting message is not shown
- [ ] Implement a plugin system (embdded Lua?)
- [ ] Enhance clipboard support for X11
- [ ] Add theming support for UI customization

