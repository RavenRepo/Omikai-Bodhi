use crossterm::{
    execute,
    terminal::{Clear, ClearType},
};
use std::sync::Arc;
use theasus_terminal::{DynTerminal, KeyEvent, Result, Terminal, TerminalError, TerminalSize};

pub struct CrosstermTerminal {
    size: TerminalSize,
}

impl CrosstermTerminal {
    pub fn new() -> std::io::Result<Self> {
        let size = Self::get_size()?;
        Ok(Self { size })
    }

    fn get_size() -> std::io::Result<TerminalSize> {
        let size = crossterm::terminal::size()?;
        Ok(TerminalSize {
            rows: size.1,
            cols: size.0,
        })
    }

    pub fn refresh_size(&mut self) -> std::io::Result<()> {
        self.size = Self::get_size()?;
        Ok(())
    }
}

impl Terminal for CrosstermTerminal {
    fn size(&self) -> TerminalSize {
        self.size
    }

    fn clear_screen(&self) -> std::io::Result<()> {
        execute!(std::io::stdout(), Clear(ClearType::All))?;
        Ok(())
    }

    fn clear_line(&self) -> std::io::Result<()> {
        execute!(std::io::stdout(), Clear(ClearType::CurrentLine))?;
        Ok(())
    }

    fn move_cursor(&self, row: i16, col: i16) -> std::io::Result<()> {
        use crossterm::cursor::MoveTo;
        execute!(std::io::stdout(), MoveTo(col as u16, row as u16))?;
        Ok(())
    }

    fn save_cursor_position(&self) -> std::io::Result<()> {
        execute!(std::io::stdout(), crossterm::cursor::SavePosition)?;
        Ok(())
    }

    fn restore_cursor_position(&self) -> std::io::Result<()> {
        execute!(std::io::stdout(), crossterm::cursor::RestorePosition)?;
        Ok(())
    }

    fn enable_raw_mode(&self) -> std::io::Result<()> {
        crossterm::terminal::enable_raw_mode()?;
        Ok(())
    }

    fn disable_raw_mode(&self) -> std::io::Result<()> {
        crossterm::terminal::disable_raw_mode()?;
        Ok(())
    }

    fn hide_cursor(&self) -> std::io::Result<()> {
        execute!(std::io::stdout(), crossterm::cursor::Hide)?;
        Ok(())
    }

    fn show_cursor(&self) -> std::io::Result<()> {
        execute!(std::io::stdout(), crossterm::cursor::Show)?;
        Ok(())
    }

    fn set_fg_color(&self, color: u8) -> std::io::Result<()> {
        use crossterm::style::SetForegroundColor;
        let color = color_to_crossterm(color);
        execute!(std::io::stdout(), SetForegroundColor(color))?;
        Ok(())
    }

    fn set_bg_color(&self, color: u8) -> std::io::Result<()> {
        use crossterm::style::SetBackgroundColor;
        let color = color_to_crossterm(color);
        execute!(std::io::stdout(), SetBackgroundColor(color))?;
        Ok(())
    }

    fn reset_colors(&self) -> std::io::Result<()> {
        execute!(std::io::stdout(), crossterm::style::ResetColor)?;
        Ok(())
    }
}

fn color_to_crossterm(color: u8) -> crossterm::style::Color {
    match color {
        0 => crossterm::style::Color::Black,
        1 => crossterm::style::Color::DarkRed,
        2 => crossterm::style::Color::DarkGreen,
        3 => crossterm::style::Color::DarkYellow,
        4 => crossterm::style::Color::DarkBlue,
        5 => crossterm::style::Color::DarkMagenta,
        6 => crossterm::style::Color::DarkCyan,
        7 => crossterm::style::Color::Grey,
        8 => crossterm::style::Color::DarkGrey,
        9 => crossterm::style::Color::Red,
        10 => crossterm::style::Color::Green,
        11 => crossterm::style::Color::Yellow,
        12 => crossterm::style::Color::Blue,
        13 => crossterm::style::Color::Magenta,
        14 => crossterm::style::Color::Cyan,
        15 => crossterm::style::Color::White,
        _ => crossterm::style::Color::White,
    }
}

impl Default for CrosstermTerminal {
    fn default() -> Self {
        Self::new().expect("Failed to create terminal")
    }
}
