use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{io, thread::JoinHandle};

mod daw_screen;
mod effects_screen;
mod event_handler;
mod main_menu_screen;
mod record_mic_screen;
mod view;

// Add new screens here
pub enum Screen {
    MainMenu,
    RecordMic,
    Effects,
    Daw,
}

use crate::effects::Effect;
use crate::wav::WavFile;
use std::sync::{Arc, Mutex};

pub struct DawLane {
    pub file_path: String,
    pub wav_data: Option<WavFile>,
    pub volume: f64,
    pub muted: bool,
    pub playback_position: f64,
    pub is_playing: bool,
}

impl DawLane {
    fn new() -> Self {
        DawLane {
            file_path: String::new(),
            wav_data: None,
            volume: 1.0,
            muted: false,
            playback_position: 0.0,
            is_playing: false,
        }
    }
}

pub struct App {
    pub screen: Screen,
    pub selected: usize,
    pub status: String,
    pub record_duration: String,
    pub wav_file: Option<WavFile>,
    pub selected_effects: Vec<Effect>,
    pub handle: Option<JoinHandle<()>>,
    pub daw_lanes: [DawLane; 3],
    pub playback_position_arc: Option<Arc<Mutex<f64>>>,
    pub input_mode: bool,
    pub input_buffer: String,
    pub active_parameter_edit: Option<(usize, String)>, // (effect_index, parameter_name)
    pub configuring_effects: Vec<(usize, Vec<(String, String)>)>, // (effect_index, parameters_with_empty_values)
}

impl App {
    fn new() -> Self {
        App {
            screen: Screen::MainMenu,
            selected: 0,
            status: String::from("Ready"),
            record_duration: String::from("10"),
            wav_file: None,
            selected_effects: vec![],
            handle: None,
            daw_lanes: [DawLane::new(), DawLane::new(), DawLane::new()],
            playback_position_arc: None,
            input_mode: false,
            input_buffer: String::new(),
            active_parameter_edit: None,
            configuring_effects: vec![],
        }
    }
}

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();

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
