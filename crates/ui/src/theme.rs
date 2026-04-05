use ratatui::style::Color;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Theme {
    pub user_color: Color,
    pub assistant_color: Color,
    pub system_color: Color,
    pub error_color: Color,
    pub border_color: Color,
    pub tool_color: Color,
    pub title_color: Color,
    pub input_color: Color,
    pub status_color: Color,
    pub code_block_color: Color,
}

impl Theme {
    pub fn dark() -> Self {
        Self {
            user_color: Color::Green,
            assistant_color: Color::Cyan,
            system_color: Color::Yellow,
            error_color: Color::Red,
            border_color: Color::Blue,
            tool_color: Color::Magenta,
            title_color: Color::Cyan,
            input_color: Color::Yellow,
            status_color: Color::DarkGray,
            code_block_color: Color::DarkGray,
        }
    }

    pub fn light() -> Self {
        Self {
            user_color: Color::Green,
            assistant_color: Color::Cyan,
            system_color: Color::Yellow,
            error_color: Color::Red,
            border_color: Color::Blue,
            tool_color: Color::Magenta,
            title_color: Color::Blue,
            input_color: Color::Black,
            status_color: Color::Gray,
            code_block_color: Color::Gray,
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::dark()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dark_theme() {
        let theme = Theme::dark();
        assert_eq!(theme.user_color, Color::Green);
        assert_eq!(theme.assistant_color, Color::Cyan);
        assert_eq!(theme.code_block_color, Color::DarkGray);
    }

    #[test]
    fn test_light_theme() {
        let theme = Theme::light();
        assert_eq!(theme.user_color, Color::Green);
        assert_eq!(theme.assistant_color, Color::Cyan);
        assert_eq!(theme.code_block_color, Color::Gray);
    }

    #[test]
    fn test_default_is_dark() {
        let theme = Theme::default();
        assert_eq!(theme, Theme::dark());
    }
}
