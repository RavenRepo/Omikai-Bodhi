use anyhow::Result;
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Borders, List, Paragraph},
    Frame, Terminal,
};
use std::io;
use std::sync::Arc;
use theasus_terminal::Terminal as TerminalTrait;

pub struct App {
    pub messages: Vec<String>,
    pub input: String,
    pub history: Vec<String>,
    pub history_index: isize,
}

impl App {
    pub fn new() -> Self {
        Self {
            messages: vec!["Bodhi AI Terminal v0.1.0".to_string()],
            input: String::new(),
            history: Vec::new(),
            history_index: -1,
        }
    }

    pub fn add_message(&mut self, msg: String) {
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
            self.history.get(self.history_index as usize)
        } else {
            None
        }
    }

    pub fn history_down(&mut self) -> Option<&String> {
        if !self.history.is_empty() {
            if self.history_index < (self.history.len() - 1) as isize {
                self.history_index += 1;
            } else {
                self.history_index = self.history.len() as isize;
                self.input.clear();
                return None;
            }
            self.history.get(self.history_index as usize)
        } else {
            None
        }
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

pub fn run_ui(app: &mut App) -> Result<()> {
    let stdout = io::stdout();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    loop {
        terminal.draw(|f| ui(f, app))?;

        // Input handling would go here
        // For now, just render the basic UI
        break;
    }

    Ok(())
}

fn ui(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(frame.area());

    let title = Paragraph::new(" Bodhi AI Terminal ")
        .style(Style::default().fg(Color::Cyan))
        .block(Block::default().borders(Borders::ALL).title("Chat"));
    frame.render_widget(title, chunks[0]);

    let messages: Vec<&str> = app.messages.iter().map(|s| s.as_str()).collect();
    let messages_widget = List::new(messages)
        .block(Block::default().borders(Borders::ALL).title("Messages"))
        .style(Style::default().fg(Color::White));
    frame.render_widget(messages_widget, chunks[1]);

    let input = Paragraph::new(app.input.as_str())
        .block(Block::default().borders(Borders::ALL).title("Input"))
        .style(Style::default().fg(Color::Yellow));
    frame.render_widget(input, chunks[2]);
}
