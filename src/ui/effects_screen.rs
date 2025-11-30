use ratatui::{
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, List, ListItem},
    Frame,
};
use crossterm::event::KeyCode;
use super::App;
use crate::effects::Effect;

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let all_effects = Effect::all_effects();
    
    let list_items: Vec<ListItem> = all_effects
        .iter()
        .enumerate()
        .map(|(i, (name, effect))| {
            let is_selected = app.selected_effects.iter().any(|e| format!("{:?}", e) == format!("{:?}", effect));
            let checkbox = if is_selected { "✓" } else { "☐" };
            let text = format!("{} {}", checkbox, name);
            
            let style = if i == app.selected {
                Style::default().fg(Color::Black).bg(Color::Yellow)
            } else if is_selected {
                Style::default().fg(Color::Green)
            } else {
                Style::default().fg(Color::White)
            };
            
            ListItem::new(text).style(style)
        })
        .collect();
    
    let list = List::new(list_items)
        .block(Block::default().borders(Borders::ALL).title(format!("Select Effects ({})", app.selected_effects.len())));
    f.render_widget(list, area);
}

pub fn handle_input(app: &mut App, key: KeyCode) -> Result<bool, Box<dyn std::error::Error>> {
    let all_effects = Effect::all_effects();
    let effect_count = all_effects.len();
    
    match key {
        KeyCode::Up => {
            if app.selected > 0 {
                app.selected -= 1;
            }
        }
        KeyCode::Down => {
            if app.selected < effect_count - 1 {
                app.selected += 1;
            }
        }
        KeyCode::Char(' ') | KeyCode::Enter => {
            let (_name, effect) = &all_effects[app.selected];
            let effect_debug = format!("{:?}", effect);
            
            if let Some(pos) = app.selected_effects.iter().position(|e| format!("{:?}", e) == effect_debug) {
                app.selected_effects.remove(pos);
            } else {
                app.selected_effects.push(effect.clone());
            }
        }
        _ => {}
    }
    Ok(false)
}
