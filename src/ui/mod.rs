use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;

mod audio_preferences_screen;
mod daw_screen;
mod debug_logger;
mod event_handler;
mod main_menu_screen;
mod screen_trait;
mod view;

pub use debug_logger::DebugLogger;

pub enum Screen {
    MainMenu { selected: usize },
    Daw { selected_track: usize },
    AudioPreferences { selected_panel: usize, input_selected: usize, output_selected: usize },
}

use crate::audio_engine::AudioEngine;
use crate::session::Session;
pub struct App {
    pub screen: Screen,
    pub status: String,
    pub session: Session,
    pub debug_logger: DebugLogger,
}

impl App {
    fn new(debug_mode: bool) -> Self {
        // Use input device sample rate — recordings are made at this rate,
        // so playback and monitoring must match it for correct pitch.
        let sample_rate = AudioEngine::get_input_device()
            .map(|d| d.sample_rate)
            .unwrap_or(48000);

        let mut session = Session::new("Untitled Project".to_string(), sample_rate);
        let _ = session.add_track("Track 1".to_string());

        App {
            screen: Screen::MainMenu { selected: 0 },
            status: String::from("Ready"),
            session,
            debug_logger: DebugLogger::new(debug_mode),
        }
    }

    pub fn debug_log(&self, message: String) {
        self.debug_logger.log(message);
    }
}

pub fn run(debug_mode: bool) -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(debug_mode);

    loop {
        terminal.draw(|f| {
            view::AppView::render(f, &app);
        })?;

        if event_handler::AppEventHandler::process_events(&mut app)? {
            break;
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    Ok(())
}
