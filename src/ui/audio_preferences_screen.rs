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

fn get_prefs(app: &App) -> (usize, usize, usize) {
    match app.screen {
        Screen::AudioPreferences { selected_panel, input_selected, output_selected } => {
            (selected_panel, input_selected, output_selected)
        }
        _ => (0, 0, 0),
    }
}

fn set_prefs(app: &mut App, panel: usize, input: usize, output: usize) {
    app.screen = Screen::AudioPreferences {
        selected_panel: panel,
        input_selected: input,
        output_selected: output,
    };
}

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

        let (selected_panel, input_selected, output_selected) = get_prefs(app);

        let ctx = AudioEngine::global();
        let ctx = ctx.lock().unwrap();

        let input_items: Vec<ListItem> = ctx
            .input_devices()
            .iter()
            .enumerate()
            .map(|(i, name)| {
                let is_selected = selected_panel == 0 && i == input_selected;
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
                let is_selected = selected_panel == 1 && i == output_selected;
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

        let refresh_style = if selected_panel == 2 {
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

        let (panel, input_sel, output_sel) = get_prefs(app);

        match key {
            KeyCode::Up => {
                if panel == 0 && input_sel > 0 {
                    set_prefs(app, panel, input_sel - 1, output_sel);
                } else if panel == 1 && output_sel > 0 {
                    set_prefs(app, panel, input_sel, output_sel - 1);
                }
            }
            KeyCode::Down => {
                if panel == 0 {
                    let max = engine.input_devices().len();
                    if input_sel < max.saturating_sub(1) {
                        set_prefs(app, panel, input_sel + 1, output_sel);
                    }
                } else if panel == 1 {
                    let max = engine.output_devices().len();
                    if output_sel < max.saturating_sub(1) {
                        set_prefs(app, panel, input_sel, output_sel + 1);
                    }
                }
            }
            KeyCode::Enter => {
                if panel == 0 {
                    if let Some(device) = engine
                        .input_devices()
                        .get(input_sel)
                        .cloned()
                    {
                        engine.set_input_device(device);
                    }
                } else if panel == 1 {
                    if let Some(device) = engine
                        .output_devices()
                        .get(output_sel)
                        .cloned()
                    {
                        engine.set_output_device(device);
                    }
                }
            }
            KeyCode::Tab => {
                let new_panel = if panel == 0 { 1 } else { 0 };
                set_prefs(app, new_panel, input_sel, output_sel);
            }
            KeyCode::Char('r') | KeyCode::Char('R') => {
                engine.refresh_devices();
            }
            KeyCode::Esc => {
                app.screen = Screen::MainMenu { selected: 0 };
            }
            _ => {}
        }
        Ok(false)
    }
}
