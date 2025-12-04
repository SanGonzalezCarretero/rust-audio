use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
    Terminal,
};
use std::{io, thread::JoinHandle};

mod main_menu_screen;
mod record_mic_screen;
mod effects_screen;
mod daw_screen;

// Add new screens here
pub enum Screen {
    MainMenu,
    RecordMic,
    Effects,
    Daw
}

use crate::wav::WavFile;
use crate::effects::Effect;
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
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(2)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Min(5),
                    Constraint::Length(3),
                ])
                .split(f.area());

            // Title bar
            let title = Paragraph::new("Rust Audio Processor")
                .style(Style::default().fg(Color::Cyan))
                .alignment(Alignment::Center)
                .block(Block::default().borders(Borders::ALL));
            f.render_widget(title, chunks[0]);

            // Main content area - route to screen modules
            match app.screen {
                Screen::MainMenu => main_menu_screen::render(f, &app, chunks[1]),
                Screen::RecordMic => record_mic_screen::render(f, &app, chunks[1]),
                Screen::Effects => effects_screen::render(f, &app, chunks[1]),
                Screen::Daw => daw_screen::render(f, &app, chunks[1]),
                // Add new screen rendering here:
                // Screen::YourNewScreen => your_screen::render(f, &app, chunks[1]),
            }

            // Status bar
            let status_widget = Paragraph::new(app.status.clone())
                .style(Style::default().fg(Color::Yellow))
                .alignment(Alignment::Center)
                .block(Block::default().borders(Borders::ALL).title("Status"));
            f.render_widget(status_widget, chunks[2]);
        })?;

        if let Some(ref handle) = app.handle {
            if handle.is_finished() {
                app.handle = None;
                app.status = format!("Recorded");
            }
        }
        
        // Update playback position from audio thread
        let mut should_stop = false;
        if let Some(ref position_arc) = app.playback_position_arc {
            if let Ok(pos) = position_arc.lock() {
                app.daw_lanes[0].playback_position = *pos;
                
                if *pos >= 1.0 {
                    app.daw_lanes[0].is_playing = false;
                    should_stop = true;
                }
            }
        }
        if should_stop {
            app.playback_position_arc = None;
        }

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => break,
                    KeyCode::Esc => {
                        app.screen = Screen::MainMenu;
                        app.selected = 0;
                    }
                    _ => {
                        // Route input to screen modules
                        let should_quit = match app.screen {
                            Screen::MainMenu => main_menu_screen::handle_input(&mut app, key.code)?,
                            Screen::RecordMic => record_mic_screen::handle_input(&mut app, key.code)?,
                            Screen::Effects => effects_screen::handle_input(&mut app, key.code)?,
                            Screen::Daw => daw_screen::handle_input(&mut app, key.code)?
                        };
                        if should_quit {
                            break;
                        }
                    }
                }
            }
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    Ok(())
}
