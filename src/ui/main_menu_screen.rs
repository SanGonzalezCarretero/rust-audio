use ratatui::{
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, List, ListItem},
    Frame,
};
use crossterm::event::KeyCode;
use super::{App, Screen};

// Define menu items here
const MENU_ITEMS: &[&str] = &[
    "Record Microphone",
    "Select Effects",
    "Daw",
    "Quit",
];

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let list_items: Vec<ListItem> = MENU_ITEMS
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let style = if i == app.selected {
                Style::default().fg(Color::Black).bg(Color::Green)
            } else {
                Style::default().fg(Color::White)
            };
            ListItem::new(*item).style(style)
        })
        .collect();
    
    let list = List::new(list_items)
        .block(Block::default().borders(Borders::ALL).title("Main Menu"));
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
            if app.selected < MENU_ITEMS.len() - 1 {
                app.selected += 1;
            }
        }
        KeyCode::Enter => {
            match app.selected {
                0 => {
                    app.screen = Screen::RecordMic;
                    app.selected = 0;
                }
                1 => {
                    app.screen = Screen::Effects;
                    app.selected = 0;
                },
                2 => {
                    app.screen = Screen::Daw;
                    app.selected = 0;
                },
                2 => return Ok(true), // Quit
                _ => {}
            }
        }
        _ => {}
    }
    Ok(false) // Don't quit
}
