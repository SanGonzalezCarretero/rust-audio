use super::App;
use crossterm::event::KeyCode;
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, List, ListItem},
    Frame,
};
use std::{fs, thread};

const MAX_DURATION: u64 = 20;

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let items = vec![
        format!("Duration: {} seconds", app.record_duration),
        "Start Recording".to_string(),
    ];

    let list_items: Vec<ListItem> = items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let style = if i == app.selected {
                Style::default().fg(Color::Black).bg(Color::Yellow)
            } else {
                Style::default().fg(Color::White)
            };
            ListItem::new(item.as_str()).style(style)
        })
        .collect();

    let list = List::new(list_items).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Record Microphone"),
    );
    f.render_widget(list, area);
}

pub fn handle_input(app: &mut App, key: KeyCode) -> Result<bool, Box<dyn std::error::Error>> {
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
                    if duration > MAX_DURATION {
                        app.record_duration.pop();
                    }
                }
            }
        }
        KeyCode::Backspace if app.selected == 0 => {
            app.record_duration.pop();
        }
        KeyCode::Enter if app.selected == 1 => {
            let duration: u64 = app.record_duration.parse().unwrap_or(10);
            app.status = format!("Recording {} seconds...", duration);
            let selected_effects = app.selected_effects.clone();

            app.handle = Some(thread::spawn(move || {
                match crate::input::record_and_save_input_device(duration) {
                    Ok(mut wav_file) => {
                        // Apply FX 
                        if !selected_effects.is_empty() {
                            let _ = wav_file.apply_effects(selected_effects);
                        }
                        fs::write("recorded.wav", wav_file.export_to_bytes()).unwrap();
                        // app.status = format!("Recorded succesfully to recorded.wav");
                    }
                    Err(e) => {
                        // app.status = format!("✗ Error: {}", e);
                    }
                }
            }));
        }
        _ => {}
    }
    Ok(false)
}
