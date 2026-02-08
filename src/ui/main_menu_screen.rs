use super::screen_trait::ScreenTrait;
use super::{App, Screen};
use crossterm::event::KeyCode;
use ratatui::{
    layout::Rect,
    style::Style,
    widgets::{Block, Borders, List, ListItem},
    Frame,
};

mod layout_config {
    use ratatui::style::Color;

    pub const MENU_TITLE: &str = "Main Menu";
    pub const SELECTED_FG: Color = Color::Black;
    pub const SELECTED_BG: Color = Color::Green;
    pub const DEFAULT_FG: Color = Color::White;
    pub const MENU_ITEMS: &[&str] = &["Daw", "Audio Preferences", "Quit"];
}

pub struct MainMenuScreen;

impl ScreenTrait for MainMenuScreen {
    fn render(&self, f: &mut Frame, app: &App, area: Rect) {
        let selected = match app.screen {
            Screen::MainMenu { selected } => selected,
            _ => 0,
        };

        let list_items: Vec<ListItem> = layout_config::MENU_ITEMS
            .iter()
            .enumerate()
            .map(|(i, item)| {
                let style = if i == selected {
                    Style::default()
                        .fg(layout_config::SELECTED_FG)
                        .bg(layout_config::SELECTED_BG)
                } else {
                    Style::default().fg(layout_config::DEFAULT_FG)
                };
                ListItem::new(*item).style(style)
            })
            .collect();

        let list = List::new(list_items).block(
            Block::default()
                .borders(Borders::ALL)
                .title(layout_config::MENU_TITLE),
        );
        f.render_widget(list, area);
    }

    fn handle_input(
        &self,
        app: &mut App,
        key: KeyCode,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let selected = match app.screen {
            Screen::MainMenu { selected } => selected,
            _ => 0,
        };

        match key {
            KeyCode::Up => {
                if selected > 0 {
                    app.screen = Screen::MainMenu { selected: selected - 1 };
                }
            }
            KeyCode::Down => {
                if selected < layout_config::MENU_ITEMS.len() - 1 {
                    app.screen = Screen::MainMenu { selected: selected + 1 };
                }
            }
            KeyCode::Enter => {
                match selected {
                    0 => {
                        app.screen = Screen::Daw { selected_track: 0 };
                    }
                    1 => {
                        app.screen = Screen::AudioPreferences { selected_panel: 0, input_selected: 0, output_selected: 0 };
                    }
                    2 => return Ok(true), // Quit
                    _ => {}
                }
            }
            _ => {}
        }
        Ok(false) // Don't quit
    }
}
