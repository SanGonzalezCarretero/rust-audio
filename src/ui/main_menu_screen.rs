use super::screen_trait::ScreenTrait;
use super::{App, Screen};
use crate::audio_engine::AudioEngine;
use crate::project;
use crate::session::Session;
use crossterm::event::KeyCode;
use ratatui::{
    layout::Rect,
    style::Style,
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

mod layout_config {
    use ratatui::style::Color;

    pub const MENU_TITLE: &str = "Main Menu";
    pub const SELECTED_FG: Color = Color::Black;
    pub const SELECTED_BG: Color = Color::Green;
    pub const DEFAULT_FG: Color = Color::White;
    pub const NEW_PROJECT_TITLE: &str = "New Project";
    pub const OPEN_PROJECT_TITLE: &str = "Open Project";
}

pub struct MainMenuScreen;

impl MainMenuScreen {
    fn menu_items() -> Vec<&'static str> {
        let projects_dir = project::projects_dir();
        let has_projects = !project::list_projects(&projects_dir).is_empty();

        let mut items = vec!["New Project"];
        if has_projects {
            items.push("Open Project");
        }
        items.push("Audio Preferences");
        items.push("Quit");
        items
    }
}

impl ScreenTrait for MainMenuScreen {
    fn render(&self, f: &mut Frame, app: &App, area: Rect) {
        match &app.screen {
            Screen::NewProject { name } => {
                let prompt = format!("Enter project name: {}_", name);
                let paragraph = Paragraph::new(prompt).block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(layout_config::NEW_PROJECT_TITLE),
                );
                f.render_widget(paragraph, area);
            }
            Screen::OpenProject { selected, projects } => {
                let list_items: Vec<ListItem> = projects
                    .iter()
                    .enumerate()
                    .map(|(i, name)| {
                        let style = if i == *selected {
                            Style::default()
                                .fg(layout_config::SELECTED_FG)
                                .bg(layout_config::SELECTED_BG)
                        } else {
                            Style::default().fg(layout_config::DEFAULT_FG)
                        };
                        ListItem::new(name.as_str()).style(style)
                    })
                    .collect();

                let list = List::new(list_items).block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(layout_config::OPEN_PROJECT_TITLE),
                );
                f.render_widget(list, area);
            }
            Screen::MainMenu { selected } => {
                let items = Self::menu_items();
                let list_items: Vec<ListItem> = items
                    .iter()
                    .enumerate()
                    .map(|(i, label)| {
                        let style = if i == *selected {
                            Style::default()
                                .fg(layout_config::SELECTED_FG)
                                .bg(layout_config::SELECTED_BG)
                        } else {
                            Style::default().fg(layout_config::DEFAULT_FG)
                        };
                        ListItem::new(*label).style(style)
                    })
                    .collect();

                let list = List::new(list_items).block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(layout_config::MENU_TITLE),
                );
                f.render_widget(list, area);
            }
            _ => {}
        }
    }

    fn handle_input(
        &self,
        app: &mut App,
        key: KeyCode,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        match &mut app.screen {
            Screen::NewProject { name } => match key {
                KeyCode::Char(c) => {
                    name.push(c);
                }
                KeyCode::Backspace => {
                    name.pop();
                }
                KeyCode::Esc => {
                    app.screen = Screen::MainMenu { selected: 0 };
                }
                KeyCode::Enter => {
                    if name.is_empty() {
                        app.status = "Project name cannot be empty".to_string();
                        return Ok(false);
                    }

                    let projects_dir = project::projects_dir();

                    if project::is_inside_project(&projects_dir) {
                        app.status =
                            "Cannot create a project inside another project folder".to_string();
                        return Ok(false);
                    }

                    let project_dir = projects_dir.join(&*name);

                    if project_dir.join("project.json").exists() {
                        app.status = format!("Project '{}' already exists", name);
                        return Ok(false);
                    }

                    let sample_rate = AudioEngine::get_input_device()
                        .map(|d| d.sample_rate)
                        .unwrap_or(48000);

                    let project_name = name.clone();
                    let mut session = Session::new(project_name.clone(), sample_rate);
                    let _ = session.add_track("Track 1".to_string());

                    project::save_project(&session, &project_dir)?;

                    app.session = session;
                    app.project_dir = Some(project_dir);
                    app.screen = Screen::Daw { selected_track: 0, scroll_offset: 0 };
                    app.status = format!("Project '{}' created", project_name);
                }
                _ => {}
            },
            Screen::OpenProject { selected, projects } => match key {
                KeyCode::Up => {
                    if *selected > 0 {
                        *selected -= 1;
                    }
                }
                KeyCode::Down => {
                    if *selected < projects.len().saturating_sub(1) {
                        *selected += 1;
                    }
                }
                KeyCode::Enter => {
                    if *selected < projects.len() {
                        let project_name = projects[*selected].clone();
                        let projects_dir = project::projects_dir();
                        let project_dir = projects_dir.join(&project_name);

                        match project::load_project(&project_dir) {
                            Ok(session) => {
                                app.session = session;
                                app.project_dir = Some(project_dir);
                                app.screen = Screen::Daw { selected_track: 0, scroll_offset: 0 };
                                app.status = format!("Project '{}' loaded", project_name);
                            }
                            Err(e) => {
                                app.status = format!("Failed to load project: {}", e);
                            }
                        }
                    }
                }
                KeyCode::Esc => {
                    app.screen = Screen::MainMenu { selected: 0 };
                }
                _ => {}
            },
            Screen::MainMenu { selected } => {
                let items = Self::menu_items();
                match key {
                    KeyCode::Up => {
                        if *selected > 0 {
                            *selected -= 1;
                        }
                    }
                    KeyCode::Down => {
                        if *selected < items.len() - 1 {
                            *selected += 1;
                        }
                    }
                    KeyCode::Enter => match items[*selected] {
                        "New Project" => {
                            app.screen = Screen::NewProject {
                                name: String::new(),
                            };
                        }
                        "Open Project" => {
                            let projects_dir = project::projects_dir();
                            let projects = project::list_projects(&projects_dir);
                            if !projects.is_empty() {
                                app.screen = Screen::OpenProject {
                                    selected: 0,
                                    projects,
                                };
                            }
                        }
                        "Audio Preferences" => {
                            app.screen = Screen::AudioPreferences {
                                selected_panel: 0,
                                input_selected: 0,
                                output_selected: 0,
                            };
                        }
                        "Quit" => return Ok(true),
                        _ => {}
                    },
                    _ => {}
                }
            }
            _ => {}
        }
        Ok(false)
    }
}
