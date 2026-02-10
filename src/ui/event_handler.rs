use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use std::time::Duration;

use super::audio_preferences_screen::AudioPreferencesScreen;
use super::daw_screen::DawScreen;
use super::main_menu_screen::MainMenuScreen;
use super::screen_trait::ScreenTrait;
use super::{App, Screen};
use crate::project;

mod event_config {
    use crossterm::event::KeyCode;

    pub const POLL_TIMEOUT_MS: u64 = 100;
    pub const QUIT_KEY: char = 'q';
    pub const BACK_KEY: KeyCode = KeyCode::Esc;
}

pub struct AppEventHandler;

impl AppEventHandler {
    pub fn process_events(app: &mut App) -> Result<bool, Box<dyn std::error::Error>> {
        Self::update_background_tasks(app);

        if Self::poll_for_event()? {
            return Self::handle_user_input(app);
        }

        Ok(false)
    }

    fn update_background_tasks(app: &mut App) {
        // Check if playback has finished and reset transport state
        app.session.check_playback_status();

        // Auto-scroll timeline to follow playhead during playback/recording
        if app.session.transport.is_playing() {
            if let Screen::Daw {
                ref mut scroll_offset,
                ..
            } = app.screen
            {
                let timeline_samples = app.session.sample_rate as u64
                    * super::daw_screen::layout_config::TIMELINE_SECONDS;
                let playhead = app.session.transport.playhead_position;

                if playhead >= *scroll_offset + timeline_samples {
                    *scroll_offset = playhead;
                }
            }
        }
    }

    fn poll_for_event() -> Result<bool, Box<dyn std::error::Error>> {
        event::poll(Duration::from_millis(event_config::POLL_TIMEOUT_MS))
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
    }

    fn handle_user_input(app: &mut App) -> Result<bool, Box<dyn std::error::Error>> {
        if let Event::Key(key) = event::read()? {
            // Ctrl+S: save project (only in DAW screen)
            if key.modifiers.contains(KeyModifiers::CONTROL)
                && key.code == KeyCode::Char('s')
            {
                if matches!(app.screen, Screen::Daw { .. }) {
                    Self::save_project(app);
                }
                return Ok(false);
            }

            // On text-input screens (NewProject), don't intercept 'q' or Esc globally;
            // let the screen handler deal with them.
            let is_text_input = matches!(app.screen, Screen::NewProject { .. });

            if !is_text_input {
                match key.code {
                    KeyCode::Char(c) if c == event_config::QUIT_KEY => return Ok(true),
                    code if code == event_config::BACK_KEY => {
                        // If a clip is selected in DAW, route Esc to the screen handler
                        // to deselect it instead of going back to the main menu.
                        if matches!(
                            app.screen,
                            Screen::Daw {
                                selected_clip: Some(_),
                                ..
                            }
                        ) {
                            return Self::route_to_screen_handler(app, key.code);
                        }
                        Self::handle_back_key(app);
                        return Ok(false);
                    }
                    _ => {}
                }
            }

            return Self::route_to_screen_handler(app, key.code);
        }
        Ok(false)
    }

    fn handle_back_key(app: &mut App) {
        if matches!(app.screen, Screen::AudioPreferences { .. }) {
            let engine = crate::audio_engine::AudioEngine::global();
            let engine = engine.lock().unwrap();
            engine.save_config();
        }
        app.screen = Screen::MainMenu { selected: 0 };
    }

    fn save_project(app: &mut App) {
        if let Some(ref dir) = app.project_dir {
            match project::save_project(&app.session, dir) {
                Ok(()) => app.status = "Project saved".to_string(),
                Err(e) => app.status = format!("Save error: {}", e),
            }
        } else {
            app.status = "No project directory set".to_string();
        }
    }

    fn route_to_screen_handler(
        app: &mut App,
        key: KeyCode,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        match app.screen {
            Screen::MainMenu { .. } => MainMenuScreen.handle_input(app, key),
            Screen::NewProject { .. } => MainMenuScreen.handle_input(app, key),
            Screen::OpenProject { .. } => MainMenuScreen.handle_input(app, key),
            Screen::Daw { .. } => DawScreen.handle_input(app, key),
            Screen::AudioPreferences { .. } => AudioPreferencesScreen.handle_input(app, key),
        }
    }
}
