# Diffy - Directory & File Diff Tool

A modular, well-architected Rust CLI and web directory/file diff program with VSCode-inspired interface and AMOLED dark theme.

## Features

- **CLI TUI Interface**: Terminal-based file tree navigation with colored status indicators
- **Web Interface**: Browser-based UI with Monaco editor for side-by-side diffs
- **Modular Architecture**: Separate core, CLI, and web modules for maintainability
- **AMOLED Dark Theme**: Pure black background optimized for OLED displays
- **VSCode-Inspired**: Familiar interface patterns and styling
- **File Tree Navigation**: Color-coded status indicators (Added/Removed/Modified/Unchanged)
- **Side-by-Side Diffs**: Split-pane view showing left vs right comparisons

## Installation

```bash
cargo install --path .
```

## Usage

### CLI Mode (Terminal UI)

```bash
# Compare two directories with TUI
diffy --left ./old_project --right ./new_project

# Compare two files with TUI  
diffy --left ./file1.txt --right ./file2.txt
```

### Web Mode

```bash
# Start web server on default port 3000
diffy --left ./old_project --right ./new_project --web

# Start on custom port
diffy --left ./old_project --right ./new_project --web --port 8080

# Auto-open browser
diffy --left ./old_project --right ./new_project --web --open
```

### Options

- `--left, -l <PATH>`: Left directory or file path
- `--right, -r <PATH>`: Right directory or file path  
- `--web`: Start web server instead of TUI
- `--port <PORT>`: Port for web server (default: 3000)
- `--open`: Open browser automatically when using --web
- `--verbose, -v`: Enable verbose logging

## Architecture

```
diffy/
├── src/
│   ├── core/           # Core diff engine and file tree logic
│   │   ├── diff.rs     # Diff algorithms using similar crate
│   │   ├── tree.rs     # File tree builder with status detection
│   │   └── types.rs    # Common data structures
│   ├── cli/            # Terminal UI using ratatui
│   │   └── tui.rs      # Terminal user interface
│   └── web/            # Web server using axum
│       └── server.rs   # Web server with Monaco editor
└── static/             # Web assets (served via CDN)
```

## Status Colors

- 🟢 **Green (+)**: Added files/directories
- 🔴 **Red (-)**: Removed files/directories  
- 🟡 **Yellow (~)**: Modified files
- ⚪ **White**: Unchanged files
- 🟣 **Purple (!)**: Conflicted files

## Controls

### CLI Mode
- `↑/↓`: Navigate file tree
- `Enter`: View file diff  
- `q`: Quit

### Web Mode
- Click files in tree to view diffs
- Monaco editor provides syntax highlighting and scrolling
- Responsive design works on desktop and mobile

## Technology Stack

- **Core**: Rust with ignore, similar, walkdir crates
- **CLI**: ratatui + crossterm for terminal UI
- **Web**: axum web server + Monaco editor
- **Diff Engine**: similar crate for text diffing
- **File Discovery**: ignore crate respects .gitignore

## License

MIT License - see LICENSE file for details.