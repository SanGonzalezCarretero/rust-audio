use super::screen_trait::ScreenTrait;
use super::{App, Screen};
use crate::device::AudioDevice;
use crossterm::event::KeyCode;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, List, ListItem},
    Frame,
};

pub struct AudioPreferencesScreen;

impl ScreenTrait for AudioPreferencesScreen {
    fn render(&self, f: &mut Frame, app: &App, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);

        let input_devices = AudioDevice::list_input_devices().unwrap_or_default();
        let output_devices = AudioDevice::list_output_devices().unwrap_or_default();

        let input_items: Vec<ListItem> = input_devices
            .iter()
            .enumerate()
            .map(|(i, name)| {
                let style = if app.selected == 0 && i == app.audio_prefs_input_selected {
                    Style::default().fg(Color::Black).bg(Color::Green)
                } else {
                    Style::default().fg(Color::White)
                };
                ListItem::new(name.as_str()).style(style)
            })
            .collect();

        let output_items: Vec<ListItem> = output_devices
            .iter()
            .enumerate()
            .map(|(i, name)| {
                let style = if app.selected == 1 && i == app.audio_prefs_output_selected {
                    Style::default().fg(Color::Black).bg(Color::Green)
                } else {
                    Style::default().fg(Color::White)
                };
                ListItem::new(name.as_str()).style(style)
            })
            .collect();

        let input_list = List::new(input_items).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Input Devices (Tab to switch)"),
        );

        let output_list = List::new(output_items).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Output Devices"),
        );

        f.render_widget(input_list, chunks[0]);
        f.render_widget(output_list, chunks[1]);
    }

    fn handle_input(
        &self,
        app: &mut App,
        key: KeyCode,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        match key {
            KeyCode::Up => {
                if app.selected == 0 && app.audio_prefs_input_selected > 0 {
                    app.audio_prefs_input_selected -= 1;
                } else if app.selected == 1 && app.audio_prefs_output_selected > 0 {
                    app.audio_prefs_output_selected -= 1;
                }
            }
            KeyCode::Down => {
                if app.selected == 0 {
                    let max = AudioDevice::list_input_devices().unwrap_or_default().len();
                    if app.audio_prefs_input_selected < max.saturating_sub(1) {
                        app.audio_prefs_input_selected += 1;
                    }
                } else if app.selected == 1 {
                    let max = AudioDevice::list_output_devices().unwrap_or_default().len();
                    if app.audio_prefs_output_selected < max.saturating_sub(1) {
                        app.audio_prefs_output_selected += 1;
                    }
                }
            }
            KeyCode::Tab => {
                app.selected = if app.selected == 0 { 1 } else { 0 };
            }
            KeyCode::Esc => {
                app.screen = Screen::MainMenu;
                app.selected = 0;
            }
            _ => {}
        }
        Ok(false)
    }
}
