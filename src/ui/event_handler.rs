use crossterm::event::{self, Event, KeyCode};
use std::time::Duration;

use super::audio_preferences_screen::AudioPreferencesScreen;
use super::daw_screen::DawScreen;
use super::main_menu_screen::MainMenuScreen;
use super::screen_trait::ScreenTrait;
use super::{App, Screen};

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
    }

    fn poll_for_event() -> Result<bool, Box<dyn std::error::Error>> {
        event::poll(Duration::from_millis(event_config::POLL_TIMEOUT_MS))
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
    }

    fn handle_user_input(app: &mut App) -> Result<bool, Box<dyn std::error::Error>> {
        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Char(c) if c == event_config::QUIT_KEY => return Ok(true),
                code if code == event_config::BACK_KEY => {
                    Self::handle_back_key(app);
                }
                _ => {
                    return Self::route_to_screen_handler(app, key.code);
                }
            }
        }
        Ok(false)
    }

    fn handle_back_key(app: &mut App) {
        app.screen = Screen::MainMenu { selected: 0 };
    }

    fn route_to_screen_handler(
        app: &mut App,
        key: KeyCode,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        match app.screen {
            Screen::MainMenu { .. } => MainMenuScreen.handle_input(app, key),
            Screen::Daw { .. } => DawScreen.handle_input(app, key),
            Screen::AudioPreferences { .. } => AudioPreferencesScreen.handle_input(app, key),
        }
    }
}
