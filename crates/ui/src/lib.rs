use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::terminal;
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style, Stylize},
    widgets::{Block, Borders, List, ListItem, Paragraph, ScrollbarState},
    Frame, Terminal,
};
use std::io;

pub struct App {
    pub messages: Vec<MessageItem>,
    pub input: String,
    pub history: Vec<String>,
    pub history_index: isize,
    pub scroll_state: ScrollbarState,
    pub scroll_offset: u16,
    pub running: bool,
    pub status: String,
    pub model: String,
}

#[derive(Clone)]
pub struct MessageItem {
    pub role: MessageRole,
    pub content: String,
    pub timestamp: String,
}

#[derive(Clone, Debug, PartialEq)]
pub enum MessageRole {
    User,
    Assistant,
    System,
    Tool,
}

impl MessageItem {
    pub fn user(content: String) -> Self {
        Self {
            role: MessageRole::User,
            content,
            timestamp: chrono::Local::now().format("%H:%M").to_string(),
        }
    }

    pub fn assistant(content: String) -> Self {
        Self {
            role: MessageRole::Assistant,
            content,
            timestamp: chrono::Local::now().format("%H:%M").to_string(),
        }
    }

    pub fn system(content: String) -> Self {
        Self {
            role: MessageRole::System,
            content,
            timestamp: chrono::Local::now().format("%H:%M").to_string(),
        }
    }

    pub fn tool(name: &str, content: String) -> Self {
        Self {
            role: MessageRole::Tool,
            content: format!("[{}] {}", name, content),
            timestamp: chrono::Local::now().format("%H:%M").to_string(),
        }
    }

    fn color(&self) -> Color {
        match self.role {
            MessageRole::User => Color::Green,
            MessageRole::Assistant => Color::Cyan,
            MessageRole::System => Color::Yellow,
            MessageRole::Tool => Color::Magenta,
        }
    }

    fn prefix(&self) -> &str {
        match self.role {
            MessageRole::User => ">",
            MessageRole::Assistant => "🤖",
            MessageRole::System => "!",
            MessageRole::Tool => "🔧",
        }
    }
}

impl App {
    pub fn new(model: &str) -> Self {
        Self {
            messages: vec![MessageItem::system(
                "Welcome to Bodhi! Type /help for available commands.".to_string(),
            )],
            input: String::new(),
            history: Vec::new(),
            history_index: -1,
            scroll_state: ScrollbarState::new(0),
            scroll_offset: 0,
            running: true,
            status: "Ready".to_string(),
            model: model.to_string(),
        }
    }

    pub fn add_message(&mut self, msg: MessageItem) {
        self.messages.push(msg);
    }

    pub fn add_to_history(&mut self, cmd: String) {
        self.history.push(cmd);
        self.history_index = self.history.len() as isize;
    }

    pub fn history_up(&mut self) -> Option<&String> {
        if !self.history.is_empty() {
            if self.history_index > 0 {
                self.history_index -= 1;
            }
            self.history.get(self.history_index as usize).map(|s| {
                self.input = s.clone();
                s
            })
        } else {
            None
        }
    }

    pub fn history_down(&mut self) -> Option<&String> {
        if !self.history.is_empty() {
            if self.history_index < (self.history.len() - 1) as isize {
                self.history_index += 1;
                self.input = self.history[self.history_index as usize].clone();
            } else {
                self.history_index = self.history.len() as isize;
                self.input.clear();
            }
            None
        } else {
            None
        }
    }

    pub fn scroll_down(&mut self) {
        if self.scroll_offset < self.messages.len().saturating_sub(1) as u16 {
            self.scroll_offset += 1;
        }
    }

    pub fn scroll_up(&mut self) {
        if self.scroll_offset > 0 {
            self.scroll_offset -= 1;
        }
    }

    pub fn set_status(&mut self, status: &str) {
        self.status = status.to_string();
    }

    pub fn set_model(&mut self, model: &str) {
        self.model = model.to_string();
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new("gpt-4o")
    }
}

pub fn run_ui(app: &mut App, llm_callback: &mut impl FnMut(String) -> String) -> Result<()> {
    terminal::enable_raw_mode()?;
    let stdout = io::stdout();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Set initial cursor position
    let _ = terminal.set_cursor_position((0, 0));

    loop {
        terminal.draw(|f| ui(f, app))?;

        if !app.running {
            break;
        }

        // Handle input events
        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                match key.code {
                    KeyCode::Char('c') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                        app.running = false;
                    }
                    KeyCode::Char('l') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                        app.messages.clear();
                        app.scroll_offset = 0;
                    }
                    KeyCode::Char(c) => {
                        app.input.push(c);
                    }
                    KeyCode::Backspace => {
                        app.input.pop();
                    }
                    KeyCode::Enter => {
                        if !app.input.is_empty() {
                            let input = app.input.clone();
                            app.add_to_history(input.clone());
                            app.add_message(MessageItem::user(input.clone()));
                            app.input.clear();

                            // Check for commands
                            if input.starts_with('/') {
                                handle_command(app, &input);
                            } else {
                                app.set_status("Processing...");
                                let response = llm_callback(input);
                                app.add_message(MessageItem::assistant(response));
                                app.set_status("Ready");
                            }
                        }
                    }
                    KeyCode::Up => {
                        app.history_up();
                    }
                    KeyCode::Down => {
                        app.history_down();
                    }
                    KeyCode::Left if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                        // Move word left
                        if let Some(pos) = app.input.rfind(' ') {
                            app.input.truncate(pos + 1);
                        } else {
                            app.input.clear();
                        }
                    }
                    KeyCode::Right if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                        // Skip word (not implemented)
                    }
                    KeyCode::Home => {
                        app.input.clear();
                    }
                    KeyCode::End => {
                        // Keep as is
                    }
                    KeyCode::PageUp => {
                        app.scroll_up();
                    }
                    KeyCode::PageDown => {
                        app.scroll_down();
                    }
                    KeyCode::Esc => {
                        app.running = false;
                    }
                    _ => {}
                }
            }
        }
    }

    terminal::disable_raw_mode()?;
    Ok(())
}

fn handle_command(app: &mut App, input: &str) {
    let parts: Vec<&str> = input.splitn(2, ' ').collect();
    let cmd = parts[0].trim_start_matches('/');

    match cmd {
        "help" | "h" => {
            app.add_message(MessageItem::system(
                "Commands: /help, /clear, /exit, /status, /model, /compact, /tools, /agents"
                    .to_string(),
            ));
        }
        "clear" | "c" => {
            app.messages.clear();
            app.scroll_offset = 0;
            app.add_message(MessageItem::system("Screen cleared".to_string()));
        }
        "exit" | "quit" | "q" => {
            app.running = false;
        }
        "status" | "s" => {
            let status = format!(
                "Model: {} | Messages: {} | Status: {}",
                app.model,
                app.messages.len(),
                app.status
            );
            app.add_message(MessageItem::system(status));
        }
        "model" | "m" => {
            if let Some(model) = parts.get(1) {
                app.set_model(model);
                app.add_message(MessageItem::system(format!("Model set to: {}", model)));
            } else {
                app.add_message(MessageItem::system(format!("Current model: {}", app.model)));
            }
        }
        "compact" => {
            if app.messages.len() > 20 {
                let sys_msgs: Vec<MessageItem> = app
                    .messages
                    .iter()
                    .filter(|m| m.role == MessageRole::System)
                    .cloned()
                    .collect();
                let last_msgs: Vec<MessageItem> =
                    app.messages.iter().rev().take(10).cloned().collect();
                app.messages = sys_msgs;
                app.messages.extend(last_msgs.into_iter().rev());
                app.add_message(MessageItem::system("Conversation compacted".to_string()));
            }
        }
        "tools" | "t" => {
            app.add_message(MessageItem::system(
                "Available tools: bash, file_read, file_write, grep, glob".to_string(),
            ));
        }
        "agents" | "a" => {
            app.add_message(MessageItem::system(
                "Available agents: general-purpose, explore, plan".to_string(),
            ));
        }
        _ => {
            app.add_message(MessageItem::system(format!("Unknown command: {}", cmd)));
        }
    }
}

fn ui(frame: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(3),
            Constraint::Length(1),
        ])
        .split(frame.area());

    // Title bar
    let title = Paragraph::new(" Bodhi AI Terminal ")
        .style(Style::default().fg(Color::Cyan).bold())
        .block(Block::default());
    frame.render_widget(title, chunks[0]);

    // Messages area with scrollbar
    let messages_area = chunks[1];
    let message_list: Vec<ListItem> = app
        .messages
        .iter()
        .skip(app.scroll_offset as usize)
        .map(|msg| {
            let content = format!("{} {}", msg.prefix(), msg.content);
            ListItem::new(content).style(Style::default().fg(msg.color()))
        })
        .collect();

    let messages_widget = List::new(message_list)
        .block(Block::default().borders(Borders::ALL).title("Messages"))
        .style(Style::default().fg(Color::White));
    frame.render_widget(messages_widget, messages_area);

    // Input area
    let input_area = Paragraph::new(app.input.as_str())
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Input")
                .border_style(Style::default().fg(Color::Blue)),
        )
        .style(Style::default().fg(Color::Yellow));
    frame.render_widget(input_area, chunks[2]);

    // Move cursor to input position
    let input_row = chunks[2].y + 1;
    let input_col = chunks[2].x + 1 + app.input.len() as u16;
    frame.set_cursor_position((input_col.min(chunks[2].right() - 1), input_row));

    // Status bar
    let status_text = format!(
        " Model: {} | Messages: {} | {} | Press Ctrl+C to quit ",
        app.model,
        app.messages.len(),
        app.status
    );
    let status = Paragraph::new(status_text)
        .style(Style::default().fg(Color::DarkGray))
        .block(Block::default());
    frame.render_widget(status, chunks[3]);
}
