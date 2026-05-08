pub mod commands;
pub mod git;

use anstyle::{AnsiColor, Style};

pub const GREEN: Style = Style::new()
    .fg_color(Some(anstyle::Color::Ansi(AnsiColor::Green)))
    .bold();
pub const RED: Style = Style::new()
    .fg_color(Some(anstyle::Color::Ansi(AnsiColor::Red)))
    .bold();
pub const YELLOW: Style = Style::new()
    .fg_color(Some(anstyle::Color::Ansi(AnsiColor::Yellow)))
    .bold();
pub const CYAN: Style = Style::new().fg_color(Some(anstyle::Color::Ansi(AnsiColor::Cyan)));
pub const DIM: Style = Style::new().dimmed();
pub const BOLD: Style = Style::new().bold();
