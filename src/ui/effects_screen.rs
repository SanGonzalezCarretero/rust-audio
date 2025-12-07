use super::screen_trait::ScreenTrait;
use super::App;
use crate::effects::EffectType;
use crossterm::event::KeyCode;
use ratatui::{
    layout::Rect,
    style::Style,
    widgets::{Block, Borders, List, ListItem},
    Frame,
};
use strum::IntoEnumIterator;

mod layout_config {
    use ratatui::style::Color;

    pub const SELECTED_FG: Color = Color::Black;
    pub const SELECTED_BG: Color = Color::Yellow;
    pub const ENABLED_EFFECT_FG: Color = Color::Green;
    pub const DEFAULT_FG: Color = Color::White;
    pub const PARAMETER_FG: Color = Color::Gray;
    pub const EDITING_FG: Color = Color::Black;
    pub const EDITING_BG: Color = Color::Cyan;
    pub const CHECKBOX_ENABLED: &str = "✓";
    pub const CHECKBOX_DISABLED: &str = "☐";

    pub fn format_title(selected_count: usize) -> String {
        format!(
            "Select Effects ({}) - Space: toggle, Enter: edit param, Esc: back",
            selected_count
        )
    }
}

// Helper enum to track what each row represents
#[derive(Debug, Clone)]
enum ListRow {
    Effect(usize),                    // Effect index
    Parameter(usize, String, String), // Effect index, param name, param value
}

pub struct EffectsScreen;

impl ScreenTrait for EffectsScreen {
    fn render(&self, f: &mut Frame, app: &App, area: Rect) {
        // Get all effect variants from registry
        let all_effect_types: Vec<EffectType> = EffectType::iter().collect();

        let mut list_items: Vec<ListItem> = Vec::new();
        let mut rows: Vec<ListRow> = Vec::new();

        for (idx, effect_type) in all_effect_types.iter().enumerate() {
            // Create a default instance for comparison
            let effect_for_comparison = effect_type.create_default();

            // Check if this effect is enabled
            let enabled_effect = app
                .selected_effects
                .iter()
                .find(|e| e.same_variant(&effect_for_comparison));

            let configuring = app
                .configuring_effects
                .iter()
                .find(|(config_idx, _)| *config_idx == idx);

            let is_enabled = enabled_effect.is_some() || configuring.is_some();
            let checkbox = if is_enabled {
                layout_config::CHECKBOX_ENABLED
            } else {
                layout_config::CHECKBOX_DISABLED
            };

            // Get the effect name
            let effect_name = if let Some(effect) = enabled_effect {
                // Fully configured effect - show full name with parameters
                effect.display_name()
            } else if configuring.is_some() {
                // Configuring effect - show just base name
                effect_type.name()
            } else {
                // Unconfigured effect - show just base name
                effect_type.name()
            };

            // Main effect line
            let is_current_row = rows.len() == app.selected;
            let style = if is_current_row && app.active_parameter_edit.is_none() {
                Style::default()
                    .fg(layout_config::SELECTED_FG)
                    .bg(layout_config::SELECTED_BG)
            } else if is_enabled {
                Style::default().fg(layout_config::ENABLED_EFFECT_FG)
            } else {
                Style::default().fg(layout_config::DEFAULT_FG)
            };

            list_items.push(ListItem::new(format!("{} {}", checkbox, effect_name)).style(style));
            rows.push(ListRow::Effect(idx));

            // Show parameters if effect is enabled and has parameters
            if is_enabled {
                let parameters: Vec<(String, String)> = if let Some(effect) = enabled_effect {
                    // Fully configured effect - get parameters from the effect
                    effect.parameters()
                } else if let Some((_, params)) = configuring {
                    // Configuring effect - get parameters from configuring_effects (may be empty)
                    params.clone()
                } else {
                    vec![]
                };

                if !parameters.is_empty() {
                    for (param_name, param_value) in parameters {
                        let is_current_row = rows.len() == app.selected;
                        let is_editing = app
                            .active_parameter_edit
                            .as_ref()
                            .map(|(edit_idx, edit_param)| {
                                *edit_idx == idx && edit_param == &param_name
                            })
                            .unwrap_or(false);

                        // Check if parameter is empty (from configuring_effects)
                        let is_empty = param_value.is_empty();

                        let param_text = if is_editing {
                            format!(
                                "    {} = {} [editing: {}]",
                                param_name, param_value, app.input_buffer
                            )
                        } else if is_empty {
                            format!("    {} = ", param_name)
                        } else {
                            format!("    {} = {}", param_name, param_value)
                        };

                        let param_style = if is_editing {
                            Style::default()
                                .fg(layout_config::EDITING_FG)
                                .bg(layout_config::EDITING_BG)
                        } else if is_current_row && app.active_parameter_edit.is_none() {
                            Style::default()
                                .fg(layout_config::SELECTED_FG)
                                .bg(layout_config::SELECTED_BG)
                        } else {
                            Style::default().fg(layout_config::PARAMETER_FG)
                        };

                        list_items.push(ListItem::new(param_text).style(param_style));
                        rows.push(ListRow::Parameter(
                            idx,
                            param_name.clone(),
                            param_value.clone(),
                        ));
                    }
                }
            }
        }

        let list = List::new(list_items).block(
            Block::default()
                .borders(Borders::ALL)
                .title(layout_config::format_title(app.selected_effects.len())),
        );
        f.render_widget(list, area);
    }

    fn handle_input(
        &self,
        app: &mut App,
        key: KeyCode,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let all_effect_types: Vec<EffectType> = EffectType::iter().collect();

        // If we're currently editing a parameter
        if let Some((effect_idx, param_name)) = &app.active_parameter_edit {
            match key {
                KeyCode::Enter => {
                    // Confirm the edit
                    let effect_idx = *effect_idx;
                    let param_name = param_name.clone();
                    let input_value = app.input_buffer.trim().to_string();

                    if input_value.is_empty() {
                        app.status = "Error: Parameter value cannot be empty".to_string();
                        return Ok(false);
                    }

                    // Check if effect is in configuring_effects
                    if let Some((_config_idx, params)) = app
                        .configuring_effects
                        .iter_mut()
                        .find(|(idx, _)| *idx == effect_idx)
                    {
                        // Update parameter in configuring_effects
                        if let Some(param) = params.iter_mut().find(|(name, _)| name == &param_name)
                        {
                            param.1 = input_value.clone();
                            app.status = format!("Set {} = {}", param_name, input_value);

                            // Check if all parameters are now filled
                            let all_filled = params.iter().all(|(_, value)| !value.is_empty());

                            if all_filled {
                                // All parameters filled - create the effect and move to selected_effects
                                let effect_type = &all_effect_types[effect_idx];
                                // Create default instance for this effect type
                                let mut new_effect = effect_type.create_default();

                                for (p_name, p_value) in params {
                                    match new_effect.update_parameter(p_name, p_value) {
                                        Ok(updated) => new_effect = updated,
                                        Err(e) => {
                                            app.status = format!("Error: {}", e);
                                            return Ok(false);
                                        }
                                    }
                                }

                                // Remove from configuring and add to selected
                                app.configuring_effects.remove(
                                    app.configuring_effects
                                        .iter()
                                        .position(|(idx, _)| *idx == effect_idx)
                                        .unwrap(),
                                );
                                app.selected_effects.push(new_effect.clone());
                                app.status = format!("Applied {}", new_effect.display_name());
                            }
                        }
                    } else {
                        // Try to find and update effect
                        let effect_type = &all_effect_types[effect_idx];
                        let effect_for_compare = effect_type.create_default();

                        if let Some(effect) = app
                            .selected_effects
                            .iter_mut()
                            .find(|e| e.same_variant(&effect_for_compare))
                        {
                            // Update parameter in selected_effects
                            match effect.update_parameter(&param_name, &input_value) {
                                Ok(new_effect) => {
                                    *effect = new_effect;
                                    app.status =
                                        format!("Updated {} = {}", param_name, input_value);
                                }
                                Err(e) => {
                                    app.status = format!("Error: {}", e);
                                    return Ok(false);
                                }
                            }
                        } else {
                            app.status = "Error: Effect not found".to_string();
                            return Ok(false);
                        }
                    }

                    app.active_parameter_edit = None;
                    app.input_buffer.clear();
                }
                KeyCode::Esc => {
                    // Cancel the edit
                    app.active_parameter_edit = None;
                    app.input_buffer.clear();
                    app.status = "Edit cancelled".to_string();
                }
                KeyCode::Backspace => {
                    app.input_buffer.pop();
                }
                KeyCode::Char(c) => {
                    app.input_buffer.push(c);
                }
                _ => {}
            }
            return Ok(false);
        }

        // Build the flat list of rows to know what the cursor is on
        let mut rows: Vec<ListRow> = Vec::new();
        for (idx, effect_type) in all_effect_types.iter().enumerate() {
            rows.push(ListRow::Effect(idx));

            // Create default instance for comparison
            let effect_for_compare = effect_type.create_default();

            // Check both selected_effects and configuring_effects
            let found_effect = app
                .selected_effects
                .iter()
                .find(|e| e.same_variant(&effect_for_compare));

            if let Some(effect) = found_effect {
                let parameters = effect.parameters();
                for (param_name, param_value) in parameters {
                    rows.push(ListRow::Parameter(idx, param_name, param_value));
                }
            } else if let Some((_, params)) = app
                .configuring_effects
                .iter()
                .find(|(config_idx, _)| *config_idx == idx)
            {
                for (param_name, param_value) in params {
                    rows.push(ListRow::Parameter(
                        idx,
                        param_name.clone(),
                        param_value.clone(),
                    ));
                }
            }
        }

        let row_count = rows.len();

        // Normal navigation mode
        match key {
            KeyCode::Up => {
                if app.selected > 0 {
                    app.selected -= 1;
                }
            }
            KeyCode::Down => {
                if app.selected < row_count.saturating_sub(1) {
                    app.selected += 1;
                }
            }
            KeyCode::Char(' ') => {
                // Toggle effect on/off (only works when on an effect row)
                if let Some(ListRow::Effect(effect_idx)) = rows.get(app.selected) {
                    let effect_type = &all_effect_types[*effect_idx];

                    // Create default instance for comparison
                    let effect_for_compare = effect_type.create_default();

                    // Check if effect is in selected_effects
                    if let Some(pos) = app
                        .selected_effects
                        .iter()
                        .position(|e| e.same_variant(&effect_for_compare))
                    {
                        // Effect is enabled, remove it
                        let effect_name = app.selected_effects[pos].display_name();
                        app.selected_effects.remove(pos);
                        app.status = format!("Disabled {}", effect_name);
                    } else if let Some(pos) = app
                        .configuring_effects
                        .iter()
                        .position(|(config_idx, _)| *config_idx == *effect_idx)
                    {
                        // Effect is configuring, remove it
                        app.configuring_effects.remove(pos);
                        app.status = "Configuration cancelled".to_string();
                    } else {
                        // Effect is not enabled - check if it has parameters
                        // Create default instance to check parameters
                        let temp_effect = effect_type.create_default();
                        let parameters = temp_effect.parameters();

                        if parameters.is_empty() {
                            // No parameters - add directly to selected_effects
                            let effect_name = temp_effect.display_name();
                            app.selected_effects.push(temp_effect);
                            app.status = format!("Enabled {}", effect_name);
                        } else {
                            // Has parameters - add to configuring_effects with empty values
                            let empty_params: Vec<(String, String)> = parameters
                                .into_iter()
                                .map(|(name, _)| (name, String::new()))
                                .collect();
                            app.configuring_effects.push((*effect_idx, empty_params));
                            app.status =
                                format!("Enabled {} - configure parameters", effect_type.name());
                        }
                    }
                }
            }
            KeyCode::Enter => {
                // Enter edit mode - works for both effects and parameters
                match rows.get(app.selected) {
                    Some(ListRow::Effect(effect_idx)) => {
                        // On an effect row - start editing first parameter if available
                        let effect_type = &all_effect_types[*effect_idx];

                        // Check configuring_effects first
                        if let Some((_, params)) = app
                            .configuring_effects
                            .iter()
                            .find(|(idx, _)| *idx == *effect_idx)
                        {
                            if !params.is_empty() {
                                let first_param = params[0].0.clone();
                                let first_value = params[0].1.clone();
                                app.active_parameter_edit =
                                    Some((*effect_idx, first_param.clone()));
                                app.input_buffer = first_value;
                                app.status = format!(
                                    "Editing {} (Enter to confirm, Esc to cancel)",
                                    first_param
                                );
                            }
                        } else {
                            // Create default instance for comparison
                            let effect_for_compare = effect_type.create_default();

                            if let Some(effect) = app
                                .selected_effects
                                .iter()
                                .find(|e| e.same_variant(&effect_for_compare))
                            {
                                let parameters = effect.parameters();
                                if !parameters.is_empty() {
                                    let first_param = parameters[0].0.clone();
                                    let first_value = parameters[0].1.clone();
                                    app.active_parameter_edit =
                                        Some((*effect_idx, first_param.clone()));
                                    app.input_buffer = first_value;
                                    app.status = format!(
                                        "Editing {} (Enter to confirm, Esc to cancel)",
                                        first_param
                                    );
                                } else {
                                    app.status =
                                        "This effect has no configurable parameters".to_string();
                                }
                            } else {
                                app.status =
                                    "Enable the effect first to configure parameters".to_string();
                            }
                        }
                    }
                    Some(ListRow::Parameter(effect_idx, param_name, param_value)) => {
                        // On a parameter row - directly start editing this parameter
                        app.active_parameter_edit = Some((*effect_idx, param_name.clone()));
                        app.input_buffer = param_value.clone();
                        app.status =
                            format!("Editing {} (Enter: confirm, Esc: cancel)", param_name);
                    }
                    None => {}
                }
            }
            _ => {}
        }

        Ok(false)
    }
}
