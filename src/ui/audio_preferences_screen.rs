use super::screen_trait::ScreenTrait;
use super::{App, Screen};
use crate::audio_engine::AudioEngine;
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
            .constraints([
                Constraint::Percentage(45),
                Constraint::Percentage(45),
                Constraint::Min(3),
            ])
            .split(area);

        let ctx = AudioEngine::global();
        let ctx = ctx.lock().unwrap();

        let input_items: Vec<ListItem> = ctx
            .input_devices()
            .iter()
            .enumerate()
            .map(|(i, name)| {
                let is_selected = app.selected == 0 && i == app.audio_prefs_input_selected;
                let is_active = ctx.selected_input() == Some(name.as_str());

                let style = if is_selected {
                    Style::default().fg(Color::Black).bg(Color::Green)
                } else if is_active {
                    Style::default().fg(Color::Green)
                } else {
                    Style::default().fg(Color::White)
                };

                let display = if is_active {
                    format!("● {}", name)
                } else {
                    format!("  {}", name)
                };

                ListItem::new(display).style(style)
            })
            .collect();

        let output_items: Vec<ListItem> = ctx
            .output_devices()
            .iter()
            .enumerate()
            .map(|(i, name)| {
                let is_selected = app.selected == 1 && i == app.audio_prefs_output_selected;
                let is_active = ctx.selected_output() == Some(name.as_str());

                let style = if is_selected {
                    Style::default().fg(Color::Black).bg(Color::Green)
                } else if is_active {
                    Style::default().fg(Color::Green)
                } else {
                    Style::default().fg(Color::White)
                };

                let display = if is_active {
                    format!("● {}", name)
                } else {
                    format!("  {}", name)
                };

                ListItem::new(display).style(style)
            })
            .collect();

        let input_list = List::new(input_items).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Input Devices (Tab to switch, Enter to select)"),
        );

        let output_list = List::new(output_items).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Output Devices (Enter to select)"),
        );

        f.render_widget(input_list, chunks[0]);
        f.render_widget(output_list, chunks[1]);

        let refresh_style = if app.selected == 2 {
            Style::default().fg(Color::Black).bg(Color::Green)
        } else {
            Style::default().fg(Color::White)
        };

        let refresh_button = Block::default()
            .borders(Borders::ALL)
            .title("Press 'r' to Refresh Devices | Esc to go back")
            .style(refresh_style);

        f.render_widget(refresh_button, chunks[2]);
    }

    fn handle_input(
        &self,
        app: &mut App,
        key: KeyCode,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let engine = AudioEngine::global();
        let mut engine = engine.lock().unwrap();

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
                    let max = engine.input_devices().len();
                    if app.audio_prefs_input_selected < max.saturating_sub(1) {
                        app.audio_prefs_input_selected += 1;
                    }
                } else if app.selected == 1 {
                    let max = engine.output_devices().len();
                    if app.audio_prefs_output_selected < max.saturating_sub(1) {
                        app.audio_prefs_output_selected += 1;
                    }
                }
            }
            KeyCode::Enter => {
                if app.selected == 0 {
                    if let Some(device) = engine.input_devices().get(app.audio_prefs_input_selected).cloned()
                    {
                        engine.set_input_device(device);
                    }
                } else if app.selected == 1 {
                    if let Some(device) = engine.output_devices().get(app.audio_prefs_output_selected).cloned()
                    {
                        engine.set_output_device(device);
                    }
                }
            }
            KeyCode::Tab => {
                app.selected = if app.selected == 0 { 1 } else { 0 };
            }
            KeyCode::Char('r') | KeyCode::Char('R') => {
                engine.refresh_devices();
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
