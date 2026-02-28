use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

static SPINNER_INDEX: AtomicUsize = AtomicUsize::new(0);

pub enum SpinnerStyle {
    Bars,
    Dots,
    Pulse,
    Bounce,
    Clock,
    Moon,
    Arrow,
    Braille,
    Gradient,
}

impl SpinnerStyle {
    pub fn frames(&self) -> &'static [&'static str] {
        match self {
            SpinnerStyle::Bars => &[
                "▰▱▱▱▱▱▱",
                "▰▰▱▱▱▱▱",
                "▰▰▰▱▱▱▱",
                "▰▰▰▰▱▱▱",
                "▰▰▰▰▰▱▱",
                "▰▰▰▰▰▰▱",
                "▰▰▰▰▰▰▰",
                "▱▱▱▱▱▱▱",
            ],
            SpinnerStyle::Dots => &[
                "⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"
            ],
            SpinnerStyle::Pulse => &[
                "●○○", "○●○", "○○●", "○●○"
            ],
            SpinnerStyle::Bounce => &[
                "⠁", "⠂", "⠄", "⠂"
            ],
            SpinnerStyle::Clock => &[
                "🕐", "🕑", "🕒", "🕓", "🕔", "🕕", "🕖", "🕗", "🕘", "🕙", "🕚", "🕛"
            ],
            SpinnerStyle::Moon => &[
                "🌑", "🌒", "🌓", "🌔", "🌕", "🌖", "🌗", "🌘"
            ],
            SpinnerStyle::Arrow => &[
                "←", "↖", "↑", "↗", "→", "↘", "↓", "↙"
            ],
            SpinnerStyle::Braille => &[
                "⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"
            ],
            SpinnerStyle::Gradient => &[
                "▰▱▱▱▱▱▱",
                "▰▰▱▱▱▱▱",
                "▰▰▰▱▱▱▱",
                "▰▰▰▰▱▱▱",
                "▰▰▰▰▰▱▱",
                "▰▰▰▰▰▰▱",
                "▰▰▰▰▰▰▰",
                "▱▱▱▱▱▱▱",
            ],
        }
    }

    pub fn interval(&self) -> Duration {
        match self {
            SpinnerStyle::Bars | SpinnerStyle::Gradient => Duration::from_millis(120),
            SpinnerStyle::Dots | SpinnerStyle::Braille => Duration::from_millis(80),
            SpinnerStyle::Pulse => Duration::from_millis(150),
            SpinnerStyle::Bounce => Duration::from_millis(100),
            SpinnerStyle::Clock => Duration::from_millis(200),
            SpinnerStyle::Moon => Duration::from_millis(150),
            SpinnerStyle::Arrow => Duration::from_millis(100),
        }
    }
}

pub struct Spinner {
    style: SpinnerStyle,
}

impl Default for Spinner {
    fn default() -> Self {
        Self {
            style: SpinnerStyle::Bars,
        }
    }
}

impl Spinner {
    pub fn new(style: SpinnerStyle) -> Self {
        Self { style }
    }

    pub fn frame(&self) -> &'static str {
        let idx = SPINNER_INDEX.fetch_add(1, Ordering::Relaxed);
        let frames = self.style.frames();
        frames[idx % frames.len()]
    }

    pub fn frame_colored(&self) -> String {
        let idx = SPINNER_INDEX.fetch_add(1, Ordering::Relaxed);
        let frames = self.style.frames();
        let frame = frames[idx % frames.len()];
        let current_idx = idx % frames.len();
        
        match self.style {
            SpinnerStyle::Gradient => {
                let colors = [51, 87, 123, 159, 195, 159, 123, 87];
                let color = colors[current_idx % colors.len()];
                format!("\x1b[38;5;{}m{}\x1b[0m", color, frame)
            }
            SpinnerStyle::Bars => {
                let colors = [51, 87, 123, 159, 195, 159, 123, 87];
                let color = colors[current_idx % colors.len()];
                format!("\x1b[38;5;{}m{}\x1b[0m", color, frame)
            }
            SpinnerStyle::Dots | SpinnerStyle::Braille => {
                let colors = [46, 82, 118, 154, 190, 154, 118, 82];
                let color = colors[current_idx % colors.len()];
                format!("\x1b[38;5;{}m{}\x1b[0m", color, frame)
            }
            SpinnerStyle::Pulse => {
                let colors = [213, 219, 225, 219];
                let color = colors[current_idx % colors.len()];
                format!("\x1b[38;5;{}m{}\x1b[0m", color, frame)
            }
            _ => frame.to_string(),
        }
    }

    pub fn interval(&self) -> Duration {
        self.style.interval()
    }

    pub fn reset() {
        SPINNER_INDEX.store(0, Ordering::Relaxed);
    }
}
