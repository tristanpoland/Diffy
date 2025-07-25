use crate::core::{DiffyCore, types::{DiffResult, FileEntry, DiffStatus, FileDiff}};
use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Frame, Terminal,
};
use std::io;
use std::path::PathBuf;

#[derive(Clone, Copy, PartialEq)]
pub enum DiffViewMode {
    Unified,
    SideBySide,
}

pub struct TuiApp {
    core: DiffyCore,
    diff_result: Option<DiffResult>,
    tree_state: ListState,
    tree_items: Vec<TreeDisplayItem>,
    selected_file: Option<PathBuf>,
    current_diff: Option<FileDiff>,
    diff_view_mode: DiffViewMode,
    scroll_offset: u16,
    should_quit: bool,
}

#[derive(Clone)]
struct TreeDisplayItem {
    path: PathBuf,
    display_name: String,
    status: DiffStatus,
    is_directory: bool,
    indent_level: usize,
}

impl TuiApp {
    pub fn new(core: DiffyCore) -> Self {
        Self {
            core,
            diff_result: None,
            tree_state: ListState::default(),
            tree_items: Vec::new(),
            selected_file: None,
            current_diff: None,
            diff_view_mode: DiffViewMode::Unified,
            scroll_offset: 0,
            should_quit: false,
        }
    }

    pub fn run(&mut self) -> Result<()> {
        // Setup terminal
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        // Load initial data
        self.load_diff_result()?;

        // Main loop
        let result = self.run_app(&mut terminal);

        // Restore terminal
        disable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        terminal.show_cursor()?;

        result
    }

    fn load_diff_result(&mut self) -> Result<()> {
        let diff_result = self.core.analyze()?;
        self.tree_items = Self::flatten_tree(&diff_result.tree, 0);
        if !self.tree_items.is_empty() {
            self.tree_state.select(Some(0));
        }
        self.diff_result = Some(diff_result);
        Ok(())
    }

    fn flatten_tree(entry: &FileEntry, indent_level: usize) -> Vec<TreeDisplayItem> {
        let mut items = Vec::new();
        
        if !entry.relative_path.as_os_str().is_empty() {
            let display_name = entry.relative_path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            
            items.push(TreeDisplayItem {
                path: entry.relative_path.clone(),
                display_name,
                status: entry.status.clone(),
                is_directory: entry.is_directory,
                indent_level,
            });
        }

        // Sort children: directories first, then files
        let mut sorted_children = entry.children.clone();
        sorted_children.sort_by(|a, b| {
            match (a.is_directory, b.is_directory) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.relative_path.cmp(&b.relative_path),
            }
        });

        for child in &sorted_children {
            let child_indent = if entry.relative_path.as_os_str().is_empty() {
                indent_level
            } else {
                indent_level + 1
            };
            items.extend(Self::flatten_tree(child, child_indent));
        }

        items
    }

    fn run_app<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> Result<()> {
        loop {
            terminal.draw(|f| self.ui(f))?;

            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') => {
                            self.should_quit = true;
                        }
                        KeyCode::Down => {
                            self.next_item();
                        }
                        KeyCode::Up => {
                            self.previous_item();
                        }
                        KeyCode::Enter => {
                            self.select_current_item()?;
                        }
                        KeyCode::Char('u') => {
                            self.diff_view_mode = DiffViewMode::Unified;
                        }
                        KeyCode::Char('s') => {
                            self.diff_view_mode = DiffViewMode::SideBySide;
                        }
                        KeyCode::PageDown | KeyCode::Char('j') => {
                            self.scroll_down();
                        }
                        KeyCode::PageUp | KeyCode::Char('k') => {
                            self.scroll_up();
                        }
                        KeyCode::Home => {
                            self.scroll_offset = 0;
                        }
                        _ => {}
                    }
                }
            }

            if self.should_quit {
                break;
            }
        }
        Ok(())
    }

    fn next_item(&mut self) {
        let i = match self.tree_state.selected() {
            Some(i) => {
                if i >= self.tree_items.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.tree_state.select(Some(i));
    }

    fn previous_item(&mut self) {
        let i = match self.tree_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.tree_items.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.tree_state.select(Some(i));
    }

    fn select_current_item(&mut self) -> Result<()> {
        if let Some(i) = self.tree_state.selected() {
            if let Some(item) = self.tree_items.get(i) {
                if !item.is_directory {
                    self.selected_file = Some(item.path.clone());
                    self.current_diff = Some(self.core.get_file_diff(&item.path)?);
                    self.scroll_offset = 0; // Reset scroll when selecting new file
                }
            }
        }
        Ok(())
    }

    fn scroll_down(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_add(3);
    }

    fn scroll_up(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_sub(3);
    }

    fn ui(&mut self, f: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
            .split(f.size());

        // File tree panel
        self.render_file_tree(f, chunks[0]);

        // Diff panel
        self.render_diff_panel(f, chunks[1]);
    }

    fn render_file_tree(&mut self, f: &mut Frame, area: Rect) {
        let items: Vec<ListItem> = self
            .tree_items
            .iter()
            .map(|item| {
                let indent = "  ".repeat(item.indent_level);
                let tree_connector = if item.indent_level > 0 { "├─ " } else { "" };
                let icon = if item.is_directory { "📁" } else { "📄" };
                let status_icon = item.status.icon();
                let color = match item.status {
                    DiffStatus::Added => Color::Green,
                    DiffStatus::Removed => Color::Red,
                    DiffStatus::Modified => Color::Yellow,
                    DiffStatus::Unchanged => Color::White,
                    DiffStatus::Conflicted => Color::Magenta,
                };

                ListItem::new(Line::from(vec![
                    Span::raw(indent),
                    Span::styled(status_icon, Style::default().fg(color)),
                    Span::raw(" "),
                    Span::styled(tree_connector, Style::default().fg(Color::DarkGray)),
                    Span::raw(icon),
                    Span::raw(" "),
                    Span::styled(&item.display_name, Style::default().fg(color)),
                ]))
            })
            .collect();

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title("Files"))
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
            .highlight_symbol("▶ ");

        f.render_stateful_widget(list, area, &mut self.tree_state);
    }

    fn render_diff_panel(&self, f: &mut Frame, area: Rect) {
        if let Some(diff) = &self.current_diff {
            match self.diff_view_mode {
                DiffViewMode::Unified => self.render_unified_diff(f, area, diff),
                DiffViewMode::SideBySide => self.render_side_by_side_diff(f, area, diff),
            }
        } else {
            let mode_text = match self.diff_view_mode {
                DiffViewMode::Unified => "Unified",
                DiffViewMode::SideBySide => "Side-by-Side",
            };
            
            let help_text = vec![
                Line::from("File Navigation:"),
                Line::from("  ↑/↓ arrows - Navigate file tree"),
                Line::from("  Enter - View file diff"),
                Line::from(""),
                Line::from("Diff Controls:"),
                Line::from("  u - Unified diff mode"),
                Line::from("  s - Side-by-side mode"),
                Line::from("  j/PageDown - Scroll down"),
                Line::from("  k/PageUp - Scroll up"),
                Line::from("  Home - Scroll to top"),
                Line::from(""),
                Line::from("  q - Quit"),
                Line::from(""),
                Line::from(format!("Current mode: {}", mode_text)),
                Line::from(""),
                Line::from("Legend:"),
                Line::from(vec![
                    Span::styled("+ ", Style::default().fg(Color::Green)),
                    Span::raw("Added lines")
                ]),
                Line::from(vec![
                    Span::styled("- ", Style::default().fg(Color::Red)),
                    Span::raw("Removed lines")
                ]),
                Line::from(vec![
                    Span::styled("  ", Style::default().fg(Color::White)),
                    Span::raw("Context lines")
                ]),
            ];
            let help = Paragraph::new(help_text)
                .block(Block::default().borders(Borders::ALL).title("Help"))
                .wrap(Wrap { trim: true });
            f.render_widget(help, area);
        }
    }

    fn render_unified_diff(&self, f: &mut Frame, area: Rect, diff: &FileDiff) {
        if diff.hunks.is_empty() {
            let content = diff.left_content.as_deref()
                .or(diff.right_content.as_deref())
                .unwrap_or("File not found");
            let paragraph = Paragraph::new(content)
                .block(Block::default().borders(Borders::ALL).title("No Changes"))
                .wrap(Wrap { trim: true });
            f.render_widget(paragraph, area);
            return;
        }

        let mut diff_lines = Vec::new();
        
        for hunk in &diff.hunks {
            // Add hunk header with background highlight
            diff_lines.push(Line::from(vec![
                Span::styled(
                    format!("@@ -{},{} +{},{} @@", 
                        hunk.old_start, hunk.old_lines, 
                        hunk.new_start, hunk.new_lines),
                    Style::default().fg(Color::Cyan).bg(Color::DarkGray)
                )
            ]));

            // Add diff lines with appropriate colors and backgrounds
            for line in &hunk.lines {
                let (fg_color, bg_color, prefix) = match line.kind {
                    crate::core::types::DiffLineKind::Addition => (Color::Green, Color::Rgb(0, 64, 0), "+"),
                    crate::core::types::DiffLineKind::Deletion => (Color::Red, Color::Rgb(64, 0, 0), "-"),
                    crate::core::types::DiffLineKind::Context => (Color::White, Color::Reset, " "),
                };

                diff_lines.push(Line::from(vec![
                    Span::styled(prefix.to_string(), Style::default().fg(fg_color).bg(bg_color)),
                    Span::styled(line.content.clone(), Style::default().fg(fg_color).bg(bg_color))
                ]));
            }
        }

        let diff_paragraph = Paragraph::new(diff_lines)
            .block(Block::default().borders(Borders::ALL).title("Unified Diff"))
            .wrap(Wrap { trim: false })
            .scroll((self.scroll_offset, 0));
        f.render_widget(diff_paragraph, area);
    }

    fn render_side_by_side_diff(&self, f: &mut Frame, area: Rect, diff: &FileDiff) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);

        // Render left side with highlighting
        let left_lines = self.create_side_by_side_lines(diff, true);
        let left_paragraph = Paragraph::new(left_lines)
            .block(Block::default().borders(Borders::ALL).title("Left (Original)"))
            .wrap(Wrap { trim: false })
            .scroll((self.scroll_offset, 0));
        f.render_widget(left_paragraph, chunks[0]);

        // Render right side with highlighting
        let right_lines = self.create_side_by_side_lines(diff, false);
        let right_paragraph = Paragraph::new(right_lines)
            .block(Block::default().borders(Borders::ALL).title("Right (Modified)"))
            .wrap(Wrap { trim: false })
            .scroll((self.scroll_offset, 0));
        f.render_widget(right_paragraph, chunks[1]);
    }

    fn create_side_by_side_lines(&self, diff: &FileDiff, is_left_side: bool) -> Vec<Line<'static>> {
        let mut lines = Vec::new();
        
        if diff.hunks.is_empty() {
            let content = if is_left_side {
                diff.left_content.as_deref().unwrap_or("File not found")
            } else {
                diff.right_content.as_deref().unwrap_or("File not found")
            };
            
            for line in content.lines() {
                lines.push(Line::from(line.to_string()));
            }
            return lines;
        }

        for hunk in &diff.hunks {
            // Add hunk header
            lines.push(Line::from(vec![
                Span::styled(
                    format!("@@ -{},{} +{},{} @@", 
                        hunk.old_start, hunk.old_lines, 
                        hunk.new_start, hunk.new_lines),
                    Style::default().fg(Color::Cyan).bg(Color::DarkGray)
                )
            ]));

            for line in &hunk.lines {
                match line.kind {
                    crate::core::types::DiffLineKind::Context => {
                        lines.push(Line::from(vec![
                            Span::styled(line.content.clone(), Style::default().fg(Color::White))
                        ]));
                    }
                    crate::core::types::DiffLineKind::Addition => {
                        if is_left_side {
                            lines.push(Line::from(""));
                        } else {
                            lines.push(Line::from(vec![
                                Span::styled(line.content.clone(), Style::default().fg(Color::Green).bg(Color::Rgb(0, 64, 0)))
                            ]));
                        }
                    }
                    crate::core::types::DiffLineKind::Deletion => {
                        if is_left_side {
                            lines.push(Line::from(vec![
                                Span::styled(line.content.clone(), Style::default().fg(Color::Red).bg(Color::Rgb(64, 0, 0)))
                            ]));
                        } else {
                            lines.push(Line::from(""));
                        }
                    }
                }
            }
        }

        lines
    }
}