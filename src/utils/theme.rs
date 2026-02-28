pub mod colors {
    pub const RESET: &str = "\x1b[0m";
    pub const BOLD: &str = "\x1b[1m";
    pub const DIM: &str = "\x1b[2m";
    pub const ITALIC: &str = "\x1b[3m";
    pub const UNDERLINE: &str = "\x1b[4m";
    pub const BLINK: &str = "\x1b[5m";

    pub const ACCENT: &str = "\x1b[38;5;51m";
    pub const ACCENT_BOLD: &str = "\x1b[1;38;5;51m";
    pub const ACCENT_DIM: &str = "\x1b[2;38;5;51m";
    pub const ACCENT_BG: &str = "\x1b[48;5;51m";
    
    pub const MINT: &str = "\x1b[38;5;85m";
    pub const MINT_BOLD: &str = "\x1b[1;38;5;85m";
    
    pub const SUCCESS: &str = "\x1b[38;5;46m";
    pub const SUCCESS_BOLD: &str = "\x1b[1;38;5;46m";
    pub const SUCCESS_BG: &str = "\x1b[48;5;46m";
    
    pub const WARN: &str = "\x1b[38;5;214m";
    pub const WARN_BOLD: &str = "\x1b[1;38;5;214m";
    pub const WARN_BG: &str = "\x1b[48;5;214m";
    
    pub const ERROR: &str = "\x1b[38;5;196m";
    pub const ERROR_BOLD: &str = "\x1b[1;38;5;196m";
    pub const ERROR_BG: &str = "\x1b[48;5;196m";
    
    pub const MUTED: &str = "\x1b[38;5;245m";
    pub const MUTED_DIM: &str = "\x1b[2;38;5;245m";
    
    pub const HIGHLIGHT: &str = "\x1b[38;5;213m";
    pub const HIGHLIGHT_BOLD: &str = "\x1b[1;38;5;213m";
    
    pub const GOLD: &str = "\x1b[38;5;220m";
    pub const GOLD_BOLD: &str = "\x1b[1;38;5;220m";
    
    pub const BOX: &str = "\x1b[38;5;33m";
    pub const BOX_DIM: &str = "\x1b[2;38;5;33m";
    
    pub const CYAN: &str = "\x1b[38;5;87m";
    pub const BLUE: &str = "\x1b[38;5;39m";
    pub const PURPLE: &str = "\x1b[38;5;129m";
    pub const GREEN: &str = "\x1b[38;5;82m";
    pub const YELLOW: &str = "\x1b[38;5;226m";
    pub const ORANGE: &str = "\x1b[38;5;208m";
    pub const RED: &str = "\x1b[38;5;203m";
    pub const PINK: &str = "\x1b[38;5;211m";
    
    pub fn gradient(text: &str, start_color: u8, end_color: u8) -> String {
        let chars: Vec<char> = text.chars().collect();
        let len = chars.len();
        if len == 0 {
            return String::new();
        }
        chars.iter().enumerate().map(|(i, c)| {
            let ratio = i as f64 / len.max(1) as f64;
            let color = start_color + ((end_color as f64 - start_color as f64) * ratio) as u8;
            format!("\x1b[38;5;{}m{}\x1b[0m", color, c)
        }).collect()
    }
}

pub mod icons {
    pub const INFO: &str = "ℹ";
    pub const OK: &str = "✓";
    pub const WARN: &str = "⚠";
    pub const ERR: &str = "✗";
    pub const ARROW: &str = "▶";
    pub const ARROW_RIGHT: &str = "→";
    pub const ARROW_LEFT: &str = "←";
    pub const DOT: &str = "•";
    pub const TRADE: &str = "◆";
    pub const VAULT: &str = "◇";
    pub const STAR: &str = "★";
    pub const DIAMOND: &str = "♦";
    pub const CIRCLE: &str = "●";
    pub const SQUARE: &str = "■";
    pub const TRIANGLE: &str = "▲";
    pub const CHECK: &str = "✔";
    pub const CROSS: &str = "✘";
    pub const PLUS: &str = "＋";
    pub const MINUS: &str = "－";
    pub const MONEY: &str = "💰";
    pub const CHART: &str = "📊";
    pub const ROCKET: &str = "🚀";
    pub const FIRE: &str = "🔥";
    pub const SPARKLES: &str = "✨";
    pub const LIGHTNING: &str = "⚡";
    pub const SHIELD: &str = "🛡";
    pub const TARGET: &str = "🎯";
}

pub fn panel_top(width: usize) -> String {
    format!(
        "{}╭{}╮{}",
        colors::BOX,
        "─".repeat(width.saturating_sub(2)),
        colors::RESET
    )
}

pub fn panel_bottom(width: usize) -> String {
    format!(
        "{}╰{}╯{}",
        colors::BOX,
        "─".repeat(width.saturating_sub(2)),
        colors::RESET
    )
}

pub fn panel_side() -> String {
    format!("{}│{}", colors::BOX, colors::RESET)
}

pub fn separator_line(width: usize, style: &str) -> String {
    let char = match style {
        "double" => "═",
        "thick" => "━",
        "dotted" => "┄",
        "dashed" => "┅",
        _ => "─",
    };
    format!("{}{}{}", colors::BOX_DIM, char.repeat(width), colors::RESET)
}

pub fn box_drawing(corner: &str, horizontal: &str, vertical: &str) -> (String, String, String) {
    let top = format!("{}{}{}", colors::BOX, corner, colors::RESET);
    let h = format!("{}{}{}", colors::BOX, horizontal, colors::RESET);
    let v = format!("{}{}{}", colors::BOX, vertical, colors::RESET);
    (top, h, v)
}

#[rustfmt::skip]
pub const BANNER: &[&str] = &[
    "  ██████╗  ██████╗ ██╗  ██╗   ██╗███╗   ███╗ █████╗ ██████╗ ██╗  ██╗███████╗████████╗",
    "  ██╔══██╗██╔═══██╗██║  ╚██╗ ██╔╝████╗ ████║██╔══██╗██╔══██╗██║ ██╔╝██╔════╝╚══██╔══╝",
    "  ██████╔╝██║   ██║██║   ╚████╔╝ ██╔████╔██║███████║██████╔╝█████╔╝ █████╗     ██║   ",
    "  ██╔═══╝ ██║   ██║██║    ╚██╔╝  ██║╚██╔╝██║██╔══██║██╔══██╗██╔═██╗ ██╔══╝     ██║   ",
    "  ██║     ╚██████╔╝███████╗██║   ██║ ╚═╝ ██║██║  ██║██║  ██║██║  ██╗███████╗   ██║   ",
    "  ╚═╝      ╚═════╝ ╚══════╝╚═╝   ╚═╝     ╚═╝╚═╝  ╚═╝╚═╝  ╚═╝╚═╝  ╚═╝╚══════╝   ╚═╝   ",
];

pub fn banner_gradient() -> Vec<String> {
    BANNER.iter().enumerate().map(|(i, line)| {
        let color = if i < 2 {
            colors::ACCENT
        } else if i < 4 {
            colors::CYAN
        } else {
            colors::HIGHLIGHT
        };
        format!("{}{}{}", color, line, colors::RESET)
    }).collect()
}
