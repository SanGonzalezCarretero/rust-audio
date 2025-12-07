use crossterm::event::KeyCode;
use ratatui::{layout::Rect, Frame};

use super::App;

pub trait ScreenTrait {
    fn render(&self, f: &mut Frame, app: &App, area: Rect);

    fn handle_input(&self, app: &mut App, key: KeyCode)
        -> Result<bool, Box<dyn std::error::Error>>;
}
