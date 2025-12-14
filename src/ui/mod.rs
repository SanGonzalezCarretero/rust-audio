use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use rand::random;
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{io, thread::JoinHandle};

mod audio_preferences_screen;
mod daw_screen;
mod debug_logger;
mod effects_screen;
mod event_handler;
mod main_menu_screen;
mod screen_trait;
mod view;

pub use debug_logger::DebugLogger;

// Add new screens here
pub enum Screen {
    MainMenu,
    Effects,
    Daw,
    AudioPreferences,
}

use crate::audio_engine::AudioEngine;
use crate::effects::EffectInstance;
use crate::session::Session;
use crate::wav::WavFile;

pub struct App {
    pub screen: Screen,
    pub selected: usize,
    pub status: String,
    pub record_duration: String,
    pub wav_file: Option<WavFile>,
    pub selected_effects: Vec<EffectInstance>,
    pub handle: Option<JoinHandle<()>>,
    pub session: Session,
    pub input_mode: bool,
    pub input_buffer: String,
    pub active_parameter_edit: Option<(usize, String)>, // (effect_index, parameter_name)
    pub configuring_effects: Vec<(usize, Vec<(String, String)>)>, // (effect_index, parameters_with_empty_values)
    pub audio_prefs_input_selected: usize,
    pub audio_prefs_output_selected: usize,
    pub debug_logger: DebugLogger,
}

impl App {
    fn new(debug_mode: bool) -> Self {
        // Get device sample rate to avoid pitch issues
        let sample_rate = AudioEngine::get_output_device()
            .map(|d| d.sample_rate)
            .unwrap_or(48000);

        let mut session = Session::new("Untitled Project".to_string(), sample_rate);
        session.add_track("Track 1".to_string());
        session.add_track("Track 2".to_string());
        session.add_track("Track 3".to_string());

        App {
            screen: Screen::MainMenu,
            selected: 0,
            status: String::from("Ready"),
            record_duration: String::from("10"),
            wav_file: None,
            selected_effects: vec![],
            handle: None,
            session,
            input_mode: false,
            input_buffer: String::new(),
            active_parameter_edit: None,
            configuring_effects: vec![],
            audio_prefs_input_selected: 0,
            audio_prefs_output_selected: 0,
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

        app.debug_log(format!("{:?}", random::<f64>()));
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    Ok(())
}
