use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerminalSize {
    pub rows: u16,
    pub cols: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyEvent {
    Enter,
    Escape,
    Backspace,
    Tab,
    Up,
    Down,
    Left,
    Right,
    CtrlC,
    CtrlD,
    Char(char),
    Unknown,
}

pub trait Terminal: Send + Sync {
    fn size(&self) -> TerminalSize;

    fn clear_screen(&self) -> std::io::Result<()>;
    fn clear_line(&self) -> std::io::Result<()>;

    fn move_cursor(&self, row: i16, col: i16) -> std::io::Result<()>;
    fn save_cursor_position(&self) -> std::io::Result<()>;
    fn restore_cursor_position(&self) -> std::io::Result<()>;

    fn enable_raw_mode(&self) -> std::io::Result<()>;
    fn disable_raw_mode(&self) -> std::io::Result<()>;

    fn hide_cursor(&self) -> std::io::Result<()>;
    fn show_cursor(&self) -> std::io::Result<()>;

    fn set_fg_color(&self, color: u8) -> std::io::Result<()>;
    fn set_bg_color(&self, color: u8) -> std::io::Result<()>;
    fn reset_colors(&self) -> std::io::Result<()>;
}

pub type DynTerminal = Arc<dyn Terminal>;

#[derive(Debug, thiserror::Error)]
pub enum TerminalError {
    #[error("Failed to get terminal size")]
    SizeError,

    #[error("Failed to enable raw mode: {0}")]
    RawModeError(std::io::Error),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, TerminalError>;
