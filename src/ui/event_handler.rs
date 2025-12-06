use crossterm::event::{self, Event, KeyCode};
use std::time::Duration;

use super::daw_screen;
use super::effects_screen;
use super::main_menu_screen;
use super::record_mic_screen;
use super::{App, Screen};

/// Declarative configuration constants for event handling
mod event_config {
    use crossterm::event::KeyCode;

    pub const POLL_TIMEOUT_MS: u64 = 100;
    pub const QUIT_KEY: char = 'q';
    pub const BACK_KEY: KeyCode = KeyCode::Esc;
    pub const RECORDED_STATUS: &str = "Recorded";
    pub const PLAYBACK_COMPLETE_THRESHOLD: f64 = 1.0;
}

/// Declarative event handler for the application
pub struct AppEventHandler;

impl AppEventHandler {
    /// Process all application events (background tasks, playback updates, user input)
    /// Returns true if the application should quit
    pub fn process_events(app: &mut App) -> Result<bool, Box<dyn std::error::Error>> {
        Self::update_background_tasks(app);
        Self::update_playback_position(app);

        if Self::poll_for_event()? {
            return Self::handle_user_input(app);
        }

        Ok(false)
    }

    /// Update background tasks (e.g., recording completion)
    fn update_background_tasks(app: &mut App) {
        if let Some(ref handle) = app.handle {
            if handle.is_finished() {
                app.handle = None;
                app.status = event_config::RECORDED_STATUS.to_string();
            }
        }
    }

    /// Update playback position from audio thread
    fn update_playback_position(app: &mut App) {
        let mut should_stop = false;
        if let Some(ref position_arc) = app.playback_position_arc {
            if let Ok(pos) = position_arc.lock() {
                app.daw_lanes[0].playback_position = *pos;

                if *pos >= event_config::PLAYBACK_COMPLETE_THRESHOLD {
                    app.daw_lanes[0].is_playing = false;
                    should_stop = true;
                }
            }
        }
        if should_stop {
            app.playback_position_arc = None;
        }
    }

    /// Poll for events with configured timeout
    fn poll_for_event() -> Result<bool, Box<dyn std::error::Error>> {
        event::poll(Duration::from_millis(event_config::POLL_TIMEOUT_MS))
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
    }

    /// Handle user keyboard input
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

    /// Handle the back/escape key
    fn handle_back_key(app: &mut App) {
        app.screen = Screen::MainMenu;
        app.selected = 0;
    }

    /// Route input to the appropriate screen handler
    fn route_to_screen_handler(
        app: &mut App,
        key: KeyCode,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let should_quit = match app.screen {
            Screen::MainMenu => main_menu_screen::handle_input(app, key)?,
            Screen::RecordMic => record_mic_screen::handle_input(app, key)?,
            Screen::Effects => effects_screen::handle_input(app, key)?,
            Screen::Daw => daw_screen::handle_input(app, key)?,
        };
        Ok(should_quit)
    }
}
