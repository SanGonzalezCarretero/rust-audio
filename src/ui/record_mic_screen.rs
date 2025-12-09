use super::screen_trait::ScreenTrait;
use super::App;
use crossterm::event::KeyCode;
use ratatui::{
    layout::Rect,
    style::Style,
    widgets::{Block, Borders, List, ListItem},
    Frame,
};
use std::sync::Arc;
use std::thread;

mod layout_config {
    use ratatui::style::Color;

    pub const TITLE: &str = "Record Microphone";
    pub const SELECTED_FG: Color = Color::Black;
    pub const SELECTED_BG: Color = Color::Yellow;
    pub const DEFAULT_FG: Color = Color::White;
    pub const MAX_DURATION: u64 = 20;
}

pub struct RecordMicScreen;

impl ScreenTrait for RecordMicScreen {
    fn render(&self, f: &mut Frame, app: &App, area: Rect) {
        let items = [
            format!("Duration: {} seconds", app.record_duration),
            "Start Recording".to_string(),
        ];

        let list_items: Vec<ListItem> = items
            .iter()
            .enumerate()
            .map(|(i, item)| {
                let style = if i == app.selected {
                    Style::default()
                        .fg(layout_config::SELECTED_FG)
                        .bg(layout_config::SELECTED_BG)
                } else {
                    Style::default().fg(layout_config::DEFAULT_FG)
                };
                ListItem::new(item.as_str()).style(style)
            })
            .collect();

        let list = List::new(list_items).block(
            Block::default()
                .borders(Borders::ALL)
                .title(layout_config::TITLE),
        );
        f.render_widget(list, area);
    }

    fn handle_input(
        &self,
        app: &mut App,
        key: KeyCode,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        match key {
            KeyCode::Up => {
                if app.selected > 0 {
                    app.selected -= 1;
                }
            }
            KeyCode::Down => {
                if app.selected < 1 {
                    app.selected += 1;
                }
            }
            KeyCode::Char(c) if app.selected == 0 && c.is_ascii_digit() => {
                if app.record_duration.len() < 2 {
                    app.record_duration.push(c);
                    if let Ok(duration) = app.record_duration.parse::<u64>() {
                        if duration > layout_config::MAX_DURATION {
                            app.record_duration.pop();
                        }
                    }
                }
            }
            KeyCode::Backspace if app.selected == 0 => {
                app.record_duration.pop();
            }
            KeyCode::Enter if app.selected == 1 => {
                // Check if there are partially configured effects
                if !app.configuring_effects.is_empty() {
                    app.status = format!(
                        "Warning: {} effect(s) are partially configured and will not be applied",
                        app.configuring_effects.len()
                    );
                    return Ok(false);
                }

                let duration: u64 = app.record_duration.parse().unwrap_or(10);
                app.status = format!("Recording {} seconds...", duration);
                let device_index = app.audio_prefs_input_selected;
                let debug_logger = Arc::new(app.debug_logger.clone());

                app.handle = Some(thread::spawn(move || {
                    let _ = crate::input::record_input_device(duration, device_index, debug_logger);
                }));
            }
            _ => {}
        }
        Ok(false)
    }
}
