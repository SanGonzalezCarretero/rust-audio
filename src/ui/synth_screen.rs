use super::App;
use crossterm::event::KeyCode;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use std::thread;
use std::time::Duration;

pub struct SynthParams {
    pub waveform_idx: usize,
    pub frequency: f32,
    pub duration: f32,
    pub amplitude: f32,
}

impl Default for SynthParams {
    fn default() -> Self {
        Self {
            waveform_idx: 0,
            frequency: 440.0,
            duration: 2.0,
            amplitude: 0.5,
        }
    }
}

const WAVEFORMS: &[&str] = &["Sine", "Square", "Saw", "Triangle"];

fn play_tone(params: &SynthParams) -> Result<(), Box<dyn std::error::Error>> {
    use crate::tones::stream_setup_for;
    use cpal::traits::StreamTrait;
    
    let stream = stream_setup_for(params.frequency, params.waveform_idx, params.amplitude)?;
    stream.play()?;
    thread::sleep(Duration::from_secs_f32(params.duration));
    drop(stream);
    Ok(())
}

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let default_params = SynthParams::default();
    let params = app.synth_params.as_ref().unwrap_or(&default_params);
    
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(10), Constraint::Min(5)])
        .split(area);

    let params_block = Block::default()
        .borders(Borders::ALL)
        .title("Tone Generator");
    
    f.render_widget(params_block, chunks[0]);

    let inner = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(chunks[0]);

    let styles = [
        if app.selected == 0 { Style::default().fg(Color::Yellow) } else { Style::default() },
        if app.selected == 1 { Style::default().fg(Color::Yellow) } else { Style::default() },
        if app.selected == 2 { Style::default().fg(Color::Yellow) } else { Style::default() },
        if app.selected == 3 { Style::default().fg(Color::Yellow) } else { Style::default() },
        if app.selected == 4 { Style::default().fg(Color::Black).bg(Color::Green) } else { Style::default() },
    ];

    f.render_widget(Paragraph::new(format!("Waveform: {}", WAVEFORMS[params.waveform_idx])).style(styles[0]), inner[0]);
    f.render_widget(Paragraph::new(format!("Frequency: {:.1} Hz", params.frequency)).style(styles[1]), inner[1]);
    f.render_widget(Paragraph::new(format!("Duration: {:.1} s", params.duration)).style(styles[2]), inner[2]);
    f.render_widget(Paragraph::new(format!("Amplitude: {:.2}", params.amplitude)).style(styles[3]), inner[3]);
    f.render_widget(Paragraph::new("[ Play Tone ]").style(styles[4]), inner[4]);

    let instructions = Paragraph::new("↑/↓: Navigate | ←/→: Adjust | Enter: Play | Esc: Back")
        .block(Block::default().borders(Borders::ALL).title("Controls"));
    f.render_widget(instructions, chunks[1]);
}

pub fn handle_input(app: &mut App, key: KeyCode) -> Result<bool, Box<dyn std::error::Error>> {
    let params = app.synth_params.get_or_insert_with(SynthParams::default);
    
    match key {
        KeyCode::Up => if app.selected > 0 { app.selected -= 1; },
        KeyCode::Down => if app.selected < 4 { app.selected += 1; },
        KeyCode::Left => match app.selected {
            0 => params.waveform_idx = params.waveform_idx.saturating_sub(1),
            1 => params.frequency = (params.frequency - 10.0).max(20.0),
            2 => params.duration = (params.duration - 0.1).max(0.1),
            3 => params.amplitude = (params.amplitude - 0.05).max(0.0),
            _ => {}
        },
        KeyCode::Right => match app.selected {
            0 => params.waveform_idx = (params.waveform_idx + 1).min(WAVEFORMS.len() - 1),
            1 => params.frequency = (params.frequency + 10.0).min(20000.0),
            2 => params.duration = (params.duration + 0.1).min(10.0),
            3 => params.amplitude = (params.amplitude + 0.05).min(1.0),
            _ => {}
        },
        KeyCode::Enter if app.selected == 4 => {
            match play_tone(params) {
                Ok(_) => app.status = format!("Played {} wave at {:.1}Hz", WAVEFORMS[params.waveform_idx], params.frequency),
                Err(e) => app.status = format!("Error: {}", e),
            }
        },
        _ => {}
    }
    Ok(false)
}
