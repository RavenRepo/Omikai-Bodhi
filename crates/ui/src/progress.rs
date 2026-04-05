const SPINNER_FRAMES: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

#[derive(Debug, Clone, Default)]
pub struct ProgressIndicator {
    pub message: String,
    pub spinner_idx: usize,
    pub active: bool,
}

impl ProgressIndicator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn start(&mut self, message: impl Into<String>) {
        self.message = message.into();
        self.spinner_idx = 0;
        self.active = true;
    }

    pub fn stop(&mut self) {
        self.active = false;
        self.message.clear();
    }

    pub fn tick(&mut self) {
        if self.active {
            self.spinner_idx = (self.spinner_idx + 1) % SPINNER_FRAMES.len();
        }
    }

    pub fn current_frame(&self) -> &'static str {
        SPINNER_FRAMES[self.spinner_idx]
    }

    pub fn render(&self) -> String {
        if self.active {
            format!("{} {}", self.current_frame(), self.message)
        } else {
            String::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_progress_indicator_new() {
        let indicator = ProgressIndicator::new();
        assert!(!indicator.active);
        assert!(indicator.message.is_empty());
        assert_eq!(indicator.spinner_idx, 0);
    }

    #[test]
    fn test_progress_indicator_start_stop() {
        let mut indicator = ProgressIndicator::new();
        indicator.start("Loading...");
        assert!(indicator.active);
        assert_eq!(indicator.message, "Loading...");

        indicator.stop();
        assert!(!indicator.active);
        assert!(indicator.message.is_empty());
    }

    #[test]
    fn test_progress_indicator_tick() {
        let mut indicator = ProgressIndicator::new();
        indicator.start("Processing");
        assert_eq!(indicator.spinner_idx, 0);

        indicator.tick();
        assert_eq!(indicator.spinner_idx, 1);

        for _ in 0..9 {
            indicator.tick();
        }
        assert_eq!(indicator.spinner_idx, 0);
    }

    #[test]
    fn test_progress_indicator_render() {
        let mut indicator = ProgressIndicator::new();
        assert!(indicator.render().is_empty());

        indicator.start("Working");
        let rendered = indicator.render();
        assert!(rendered.contains("Working"));
        assert!(rendered.starts_with(SPINNER_FRAMES[0]));
    }
}
