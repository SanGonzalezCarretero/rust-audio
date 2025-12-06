use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use super::daw_screen;
use super::effects_screen;
use super::main_menu_screen;
use super::record_mic_screen;
use super::{App, Screen};

/// Declarative configuration constants for the application layout
mod layout_config {
    use super::*;

    pub const TITLE: &str = "Rust Audio Processor";
    pub const TITLE_COLOR: Color = Color::Cyan;
    pub const STATUS_TITLE: &str = "Status";
    pub const STATUS_COLOR: Color = Color::Yellow;

    pub const HEADER_HEIGHT: u16 = 3;
    pub const FOOTER_HEIGHT: u16 = 3;
    pub const MIN_CONTENT_HEIGHT: u16 = 5;
    pub const MARGIN: u16 = 2;
}

/// Declarative view definition for the application layout
pub struct AppView;

impl AppView {
    /// Render the complete application view declaratively
    pub fn render(f: &mut Frame, app: &App) {
        let chunks = Self::create_layout(f.area());

        Self::render_title(f, chunks[0]);
        Self::render_content(f, app, chunks[1]);
        Self::render_status(f, app, chunks[2]);
    }

    /// Declare the layout structure with named constraints
    fn create_layout(area: Rect) -> std::rc::Rc<[Rect]> {
        Layout::default()
            .direction(Direction::Vertical)
            .margin(layout_config::MARGIN)
            .constraints([
                Constraint::Length(layout_config::HEADER_HEIGHT),
                Constraint::Min(layout_config::MIN_CONTENT_HEIGHT),
                Constraint::Length(layout_config::FOOTER_HEIGHT),
            ])
            .split(area)
    }

    /// Declarative title bar configuration
    fn render_title(f: &mut Frame, area: Rect) {
        let title = Paragraph::new(layout_config::TITLE)
            .style(Style::default().fg(layout_config::TITLE_COLOR))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(title, area);
    }

    /// Declarative content area routing
    fn render_content(f: &mut Frame, app: &App, area: Rect) {
        match app.screen {
            Screen::MainMenu => main_menu_screen::render(f, app, area),
            Screen::RecordMic => record_mic_screen::render(f, app, area),
            Screen::Effects => effects_screen::render(f, app, area),
            Screen::Daw => daw_screen::render(f, app, area),
        }
    }

    /// Declarative status bar configuration
    fn render_status(f: &mut Frame, app: &App, area: Rect) {
        let status = Paragraph::new(app.status.clone())
            .style(Style::default().fg(layout_config::STATUS_COLOR))
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(layout_config::STATUS_TITLE),
            );
        f.render_widget(status, area);
    }
}
