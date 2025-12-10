use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::Style,
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use super::audio_preferences_screen::AudioPreferencesScreen;
use super::daw_screen::DawScreen;
use super::effects_screen::EffectsScreen;
use super::main_menu_screen::MainMenuScreen;
use super::screen_trait::ScreenTrait;
use super::{App, Screen};

mod layout_config {
    use ratatui::style::Color;

    pub const TITLE: &str = "Rust Audio Processor";
    pub const TITLE_COLOR: Color = Color::Cyan;
    pub const STATUS_TITLE: &str = "Status";
    pub const STATUS_COLOR: Color = Color::Yellow;
    pub const DEBUG_TITLE: &str = "Debug Log";
    pub const DEBUG_COLOR: Color = Color::Red;

    pub const HEADER_HEIGHT: u16 = 3;
    pub const FOOTER_HEIGHT: u16 = 3;
    pub const MIN_CONTENT_HEIGHT: u16 = 5;
    pub const MARGIN: u16 = 2;
    pub const DEBUG_PANEL_WIDTH: u16 = 50;
    pub const DEBUG_PANEL_HEIGHT: u16 = 6;
}

pub struct AppView;

impl AppView {
    pub fn render(f: &mut Frame, app: &App) {
        let chunks = Self::create_layout(f.area());

        Self::render_title(f, app, chunks[0]);
        Self::render_content(f, app, chunks[1]);
        Self::render_status(f, app, chunks[2]);

        Self::render_debug_panel(f, app, Self::get_debug_panel_area(f.area()));
    }

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

    fn get_debug_panel_area(area: Rect) -> Rect {
        Rect {
            x: area
                .width
                .saturating_sub(layout_config::DEBUG_PANEL_WIDTH + layout_config::MARGIN),
            y: area
                .height
                .saturating_sub(layout_config::DEBUG_PANEL_HEIGHT + layout_config::MARGIN),
            width: layout_config::DEBUG_PANEL_WIDTH
                .min(area.width.saturating_sub(layout_config::MARGIN * 2)),
            height: layout_config::DEBUG_PANEL_HEIGHT
                .min(area.height.saturating_sub(layout_config::MARGIN * 2)),
        }
    }

    fn render_title(f: &mut Frame, app: &App, area: Rect) {
        let title_text = format!("{} - {}", layout_config::TITLE, app.session.name);
        let title = Paragraph::new(title_text)
            .style(Style::default().fg(layout_config::TITLE_COLOR))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(title, area);
    }

    fn render_content(f: &mut Frame, app: &App, area: Rect) {
        match app.screen {
            Screen::MainMenu => MainMenuScreen.render(f, app, area),
            Screen::Effects => EffectsScreen.render(f, app, area),
            Screen::Daw => DawScreen.render(f, app, area),
            Screen::AudioPreferences => AudioPreferencesScreen.render(f, app, area),
        }
    }

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

    fn render_debug_panel(f: &mut Frame, app: &App, area: Rect) {
        if !app.debug_logger.is_enabled() {
            return;
        };

        let logs = app.debug_logger.get_logs();

        let debug_text = if logs.is_empty() {
            "No debug messages".to_string()
        } else {
            logs.join("\n")
        };

        let debug_panel = Paragraph::new(debug_text)
            .style(Style::default().fg(layout_config::DEBUG_COLOR))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(layout_config::DEBUG_TITLE),
            );
        f.render_widget(debug_panel, area);
    }
}
