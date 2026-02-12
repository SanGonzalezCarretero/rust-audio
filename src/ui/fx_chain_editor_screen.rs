use super::screen_trait::ScreenTrait;
use super::{App, Screen};
use crate::effects::EffectType;
use crossterm::event::KeyCode;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};
use strum::IntoEnumIterator;

pub struct FxChainEditorScreen;

// Helper to extract state from Screen::FxChainEditor
fn get_state(app: &App) -> (usize, usize, Option<usize>, bool, usize) {
    match app.screen {
        Screen::FxChainEditor {
            track_index,
            selected_effect,
            editing_param,
            add_mode,
            add_mode_selected,
        } => (
            track_index,
            selected_effect,
            editing_param,
            add_mode,
            add_mode_selected,
        ),
        _ => (0, 0, None, false, 0),
    }
}

fn set_state(
    app: &mut App,
    track_index: usize,
    selected_effect: usize,
    editing_param: Option<usize>,
    add_mode: bool,
    add_mode_selected: usize,
) {
    app.screen = Screen::FxChainEditor {
        track_index,
        selected_effect,
        editing_param,
        add_mode,
        add_mode_selected,
    };
}

impl ScreenTrait for FxChainEditorScreen {
    fn render(&self, f: &mut Frame, app: &App, area: Rect) {
        let (track_index, selected_effect, editing_param, add_mode, add_mode_selected) =
            get_state(app);

        // Get track name for title
        let track_name = app
            .session
            .get_track(track_index)
            .map(|t| t.name.clone())
            .unwrap_or_else(|| "Unknown".to_string());

        // Main layout: content area + instructions bar
        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(5), Constraint::Length(3)])
            .split(area);

        // Content area: left panel (effects list) + right panel (parameters)
        let content_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(main_chunks[0]);

        if add_mode {
            // Render effect type picker
            self.render_add_mode(f, content_chunks[0], add_mode_selected);
            // Right panel shows description
            self.render_add_mode_info(f, content_chunks[1], add_mode_selected);
        } else {
            // Render effects chain list
            self.render_effects_list(
                f,
                app,
                content_chunks[0],
                track_index,
                selected_effect,
                &track_name,
            );

            // Render parameters panel
            self.render_parameters(
                f,
                app,
                content_chunks[1],
                track_index,
                selected_effect,
                editing_param,
            );
        }

        // Render instructions bar
        self.render_instructions(f, main_chunks[1], add_mode, editing_param.is_some());
    }

    fn handle_input(
        &self,
        app: &mut App,
        key: KeyCode,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let (track_index, selected_effect, editing_param, add_mode, add_mode_selected) =
            get_state(app);

        if add_mode {
            return self.handle_add_mode_input(
                app,
                key,
                track_index,
                selected_effect,
                add_mode_selected,
            );
        }

        if editing_param.is_some() {
            return self.handle_param_edit_input(
                app,
                key,
                track_index,
                selected_effect,
                editing_param.unwrap(),
            );
        }

        // Normal mode: navigating effects list
        let fx_chain_len = app
            .session
            .get_track(track_index)
            .map(|t| t.fx_chain.len())
            .unwrap_or(0);

        // Total items: "Add Effect" + existing effects
        let total_items = 1 + fx_chain_len;

        match key {
            KeyCode::Up => {
                if selected_effect > 0 {
                    set_state(
                        app,
                        track_index,
                        selected_effect - 1,
                        None,
                        false,
                        add_mode_selected,
                    );
                }
            }
            KeyCode::Down => {
                if selected_effect < total_items.saturating_sub(1) {
                    set_state(
                        app,
                        track_index,
                        selected_effect + 1,
                        None,
                        false,
                        add_mode_selected,
                    );
                }
            }
            KeyCode::Enter => {
                if selected_effect == 0 {
                    // "Add Effect" selected - enter add mode
                    set_state(app, track_index, selected_effect, None, true, 0);
                } else {
                    // Effect selected - enter parameter editing mode if it has params
                    let effect_idx = selected_effect - 1;
                    if let Some(track) = app.session.get_track(track_index) {
                        if let Some(effect) = track.fx_chain.get(effect_idx) {
                            if !effect.parameters().is_empty() {
                                set_state(
                                    app,
                                    track_index,
                                    selected_effect,
                                    Some(0),
                                    false,
                                    add_mode_selected,
                                );
                            }
                        }
                    }
                }
            }
            KeyCode::Delete | KeyCode::Backspace => {
                if selected_effect > 0 {
                    let effect_idx = selected_effect - 1;
                    if app
                        .session
                        .remove_effect_from_track(track_index, effect_idx)
                        .is_ok()
                    {
                        // Adjust selection if we removed the last effect
                        let new_len = app
                            .session
                            .get_track(track_index)
                            .map(|t| t.fx_chain.len())
                            .unwrap_or(0);
                        let new_selected = if selected_effect > new_len {
                            new_len
                        } else {
                            selected_effect
                        };
                        set_state(
                            app,
                            track_index,
                            new_selected,
                            None,
                            false,
                            add_mode_selected,
                        );
                        app.status = "Effect removed".to_string();
                    }
                }
            }
            KeyCode::Char('a') | KeyCode::Char('A') => {
                // Enter add mode
                set_state(app, track_index, selected_effect, None, true, 0);
            }
            KeyCode::Esc => {
                // Return to DAW screen
                app.screen = Screen::Daw {
                    selected_track: track_index,
                    scroll_offset: 0,
                    selected_clip: None,
                };
            }
            _ => {}
        }

        Ok(false)
    }
}

impl FxChainEditorScreen {
    fn render_effects_list(
        &self,
        f: &mut Frame,
        app: &App,
        area: Rect,
        track_index: usize,
        selected_effect: usize,
        track_name: &str,
    ) {
        let mut items: Vec<ListItem> = Vec::new();

        // "Add Effect" option at index 0
        let add_style = if selected_effect == 0 {
            Style::default().fg(Color::Black).bg(Color::Green)
        } else {
            Style::default().fg(Color::Green)
        };
        items.push(ListItem::new(Line::from(vec![Span::styled(
            "[+] Add Effect",
            add_style,
        )])));

        // Existing effects
        if let Some(track) = app.session.get_track(track_index) {
            for (i, effect) in track.fx_chain.iter().enumerate() {
                let effect_index = i + 1; // +1 because "Add Effect" is at 0
                let is_selected = selected_effect == effect_index;

                let style = if is_selected {
                    Style::default().fg(Color::Black).bg(Color::Cyan)
                } else {
                    Style::default().fg(Color::White)
                };

                let display_name = effect.display_name();
                items.push(ListItem::new(Line::from(vec![Span::styled(
                    format!("{}. {}", i + 1, display_name),
                    style,
                )])));
            }
        }

        let list = List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!("FX Chain - {}", track_name)),
        );

        f.render_widget(list, area);
    }

    fn render_parameters(
        &self,
        f: &mut Frame,
        app: &App,
        area: Rect,
        track_index: usize,
        selected_effect: usize,
        editing_param: Option<usize>,
    ) {
        let mut items: Vec<ListItem> = Vec::new();

        if selected_effect == 0 {
            // "Add Effect" selected - show hint
            items.push(ListItem::new(Line::from(Span::styled(
                "Press Enter or 'a' to add an effect",
                Style::default().fg(Color::DarkGray),
            ))));
        } else {
            let effect_idx = selected_effect - 1;
            if let Some(track) = app.session.get_track(track_index) {
                if let Some(effect) = track.fx_chain.get(effect_idx) {
                    let params = effect.parameters();
                    if params.is_empty() {
                        items.push(ListItem::new(Line::from(Span::styled(
                            "No configurable parameters",
                            Style::default().fg(Color::DarkGray),
                        ))));
                    } else {
                        for (i, (name, value)) in params.iter().enumerate() {
                            let is_editing = editing_param == Some(i);
                            let style = if is_editing {
                                Style::default().fg(Color::Black).bg(Color::Yellow)
                            } else {
                                Style::default().fg(Color::White)
                            };

                            let hint = if is_editing { " [+/-]" } else { "" };
                            items.push(ListItem::new(Line::from(vec![Span::styled(
                                format!("{}: {}{}", name, value, hint),
                                style,
                            )])));
                        }
                    }
                }
            }
        }

        let list = List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Parameters"),
        );

        f.render_widget(list, area);
    }

    fn render_add_mode(&self, f: &mut Frame, area: Rect, selected: usize) {
        let effect_types: Vec<EffectType> = EffectType::iter().collect();

        let items: Vec<ListItem> = effect_types
            .iter()
            .enumerate()
            .map(|(i, et)| {
                let style = if i == selected {
                    Style::default().fg(Color::Black).bg(Color::Green)
                } else {
                    Style::default().fg(Color::White)
                };
                ListItem::new(Line::from(Span::styled(et.name(), style)))
            })
            .collect();

        let list = List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Select Effect to Add"),
        );

        f.render_widget(list, area);
    }

    fn render_add_mode_info(&self, f: &mut Frame, area: Rect, selected: usize) {
        let effect_types: Vec<EffectType> = EffectType::iter().collect();

        let info = if let Some(et) = effect_types.get(selected) {
            let default_effect = et.create_default();
            let params = default_effect.parameters();
            if params.is_empty() {
                "No configurable parameters".to_string()
            } else {
                let param_list: Vec<String> = params
                    .iter()
                    .map(|(name, value)| format!("  {}: {}", name, value))
                    .collect();
                format!("Default parameters:\n{}", param_list.join("\n"))
            }
        } else {
            "".to_string()
        };

        let paragraph = Paragraph::new(info).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Effect Info"),
        );

        f.render_widget(paragraph, area);
    }

    fn render_instructions(&self, f: &mut Frame, area: Rect, add_mode: bool, editing_param: bool) {
        let instructions = if add_mode {
            "Up/Down: Navigate | Enter: Add | Esc: Cancel"
        } else if editing_param {
            "Up/Down: Select param | +/-: Adjust value | Esc: Done"
        } else {
            "Up/Down: Navigate | Enter: Edit params | Del: Remove | a: Add | Esc: Back to DAW"
        };

        let paragraph = Paragraph::new(Line::from(Span::styled(
            instructions,
            Style::default().fg(Color::Cyan),
        )))
        .block(Block::default().borders(Borders::ALL));

        f.render_widget(paragraph, area);
    }

    fn handle_add_mode_input(
        &self,
        app: &mut App,
        key: KeyCode,
        track_index: usize,
        selected_effect: usize,
        add_mode_selected: usize,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let effect_types: Vec<EffectType> = EffectType::iter().collect();
        let total_types = effect_types.len();

        match key {
            KeyCode::Up => {
                if add_mode_selected > 0 {
                    set_state(
                        app,
                        track_index,
                        selected_effect,
                        None,
                        true,
                        add_mode_selected - 1,
                    );
                }
            }
            KeyCode::Down => {
                if add_mode_selected < total_types.saturating_sub(1) {
                    set_state(
                        app,
                        track_index,
                        selected_effect,
                        None,
                        true,
                        add_mode_selected + 1,
                    );
                }
            }
            KeyCode::Enter => {
                // Add the selected effect type
                if let Some(et) = effect_types.get(add_mode_selected) {
                    let effect = et.create_default();
                    if app
                        .session
                        .add_effect_to_track(track_index, effect)
                        .is_ok()
                    {
                        // Exit add mode and select the newly added effect
                        let new_len = app
                            .session
                            .get_track(track_index)
                            .map(|t| t.fx_chain.len())
                            .unwrap_or(0);
                        set_state(app, track_index, new_len, None, false, 0);
                        app.status = format!("Added {}", et.name());
                    }
                }
            }
            KeyCode::Esc => {
                // Exit add mode
                set_state(app, track_index, selected_effect, None, false, 0);
            }
            _ => {}
        }

        Ok(false)
    }

    fn handle_param_edit_input(
        &self,
        app: &mut App,
        key: KeyCode,
        track_index: usize,
        selected_effect: usize,
        editing_param: usize,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let effect_idx = selected_effect - 1;

        // Get current parameters
        let params = app
            .session
            .get_track(track_index)
            .and_then(|t| t.fx_chain.get(effect_idx))
            .map(|e| e.parameters())
            .unwrap_or_default();

        let total_params = params.len();

        match key {
            KeyCode::Up => {
                if editing_param > 0 {
                    set_state(
                        app,
                        track_index,
                        selected_effect,
                        Some(editing_param - 1),
                        false,
                        0,
                    );
                }
            }
            KeyCode::Down => {
                if editing_param < total_params.saturating_sub(1) {
                    set_state(
                        app,
                        track_index,
                        selected_effect,
                        Some(editing_param + 1),
                        false,
                        0,
                    );
                }
            }
            KeyCode::Char('+') | KeyCode::Char('=') | KeyCode::Right => {
                // Increment parameter value
                if let Some((param_name, param_value)) = params.get(editing_param) {
                    if let Ok(current) = param_value.parse::<i64>() {
                        let new_value = current + 1;
                        if app
                            .session
                            .update_effect_param(
                                track_index,
                                effect_idx,
                                param_name,
                                &new_value.to_string(),
                            )
                            .is_err()
                        {
                            // Value out of range, ignore
                        }
                    } else if let Ok(current) = param_value.parse::<f64>() {
                        let new_value = current + 0.1;
                        let _ = app.session.update_effect_param(
                            track_index,
                            effect_idx,
                            param_name,
                            &format!("{:.1}", new_value),
                        );
                    }
                }
            }
            KeyCode::Char('-') | KeyCode::Left => {
                // Decrement parameter value
                if let Some((param_name, param_value)) = params.get(editing_param) {
                    if let Ok(current) = param_value.parse::<i64>() {
                        let new_value = current - 1;
                        if new_value >= 0 {
                            let _ = app.session.update_effect_param(
                                track_index,
                                effect_idx,
                                param_name,
                                &new_value.to_string(),
                            );
                        }
                    } else if let Ok(current) = param_value.parse::<f64>() {
                        let new_value = (current - 0.1).max(0.0);
                        let _ = app.session.update_effect_param(
                            track_index,
                            effect_idx,
                            param_name,
                            &format!("{:.1}", new_value),
                        );
                    }
                }
            }
            KeyCode::Esc | KeyCode::Enter => {
                // Exit parameter editing mode
                set_state(app, track_index, selected_effect, None, false, 0);
            }
            _ => {}
        }

        Ok(false)
    }
}
