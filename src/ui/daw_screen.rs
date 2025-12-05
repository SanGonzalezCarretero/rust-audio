use std::{fs, sync::{Arc, Mutex}};

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, List, ListItem, Sparkline},
    Frame,
};
use crossterm::event::KeyCode;
use crate::{device::AudioDevice, wav::WavFile};

use super::App;

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(33),
            Constraint::Percentage(33),
            Constraint::Percentage(34),
        ])
        .split(area);

    for (i, chunk) in chunks.iter().enumerate() {
        let lane = &app.daw_lanes[i];
        let is_selected = app.selected == i;
        
        let border_color = if is_selected { Color::Yellow } else { Color::White };
        let title = format!("Lane {} | Vol: {:.0}% | {} | {}", 
            i + 1, 
            lane.volume * 100.0,
            if lane.muted { "MUTED" } else { "Active" },
            if lane.file_path.is_empty() { "Empty" } else { &lane.file_path }
        );
        
        if let Some(ref wav) = lane.wav_data {
            let samples = wav.to_f64_samples();
            let width = chunk.width.saturating_sub(2) as usize;
            let samples_per_pixel = samples.len() / width;
            
            let waveform: Vec<u64> = (0..width)
                .map(|i| {
                    let start = i * samples_per_pixel;
                    let end = ((i + 1) * samples_per_pixel).min(samples.len());
                    
                    if start < samples.len() {
                        let chunk_samples = &samples[start..end];
                        let max_amplitude = chunk_samples.iter()
                            .map(|s| s.abs())
                            .fold(0.0f64, |a, b| a.max(b));
                        
                        ((max_amplitude * 100.0) as u64).min(100)
                    } else {
                        0
                    }
                })
                .collect();
            
            let sparkline = Sparkline::default()
                .block(Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(border_color))
                    .title(title))
                .data(&waveform)
                .style(Style::default().fg(Color::Cyan));
            
            f.render_widget(sparkline, *chunk);
            
            if lane.is_playing {
                let cursor_x = ((lane.playback_position * width as f64) as u16).min(width as u16 - 1);
                let cursor_area = Rect {
                    x: chunk.x + 1 + cursor_x,
                    y: chunk.y + 1,
                    width: 1,
                    height: chunk.height.saturating_sub(2),
                };
                let cursor = Block::default().style(Style::default().bg(Color::Red));
                f.render_widget(cursor, cursor_area);
            }
        } else {
            let list = List::new(vec![ListItem::new("Press 'l' to load WAV file")])
                .block(Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(border_color))
                    .title(title));
            
            f.render_widget(list, *chunk);
        }
    }
}

pub fn handle_input(app: &mut App, key: KeyCode) -> Result<bool, Box<dyn std::error::Error>> {
    if app.input_mode {
        match key {
            KeyCode::Enter => {
                let file_path = app.input_buffer.clone();
                
                if let Some(lane_idx) = app.daw_lanes.iter().position(|lane| lane.wav_data.is_none()) {
                    match fs::read(&file_path) {
                        Ok(bytes) => {
                            match WavFile::from_bytes(bytes) {
                                Ok(wav) => {
                                    app.daw_lanes[lane_idx].file_path = file_path.clone();
                                    app.daw_lanes[lane_idx].wav_data = Some(wav);
                                    app.status = format!("Loaded {} into Lane {}", file_path, lane_idx + 1);
                                }
                                Err(e) => app.status = format!("Error parsing WAV: {}", e),
                            }
                        }
                        Err(e) => app.status = format!("Error reading file: {}", e),
                    }
                } else {
                    app.status = "All lanes are full".to_string();
                }
                
                app.input_mode = false;
                app.input_buffer.clear();
            }
            KeyCode::Esc => {
                app.input_mode = false;
                app.input_buffer.clear();
                app.status = "Cancelled".to_string();
            }
            KeyCode::Backspace => {
                app.input_buffer.pop();
                app.status = format!("Enter file path: {}", app.input_buffer);
            }
            KeyCode::Char(c) => {
                app.input_buffer.push(c);
                app.status = format!("Enter file path: {}", app.input_buffer);
            }
            _ => {}
        }
        return Ok(false);
    }
    
    match key {
        KeyCode::Char('1') => {
            let lane_idx = 0;
            if let Some(ref wav) = app.daw_lanes[lane_idx].wav_data {
                use std::sync::{Arc, Mutex};
                
                let playback_position = Arc::new(Mutex::new(0.0f64));
                app.playback_position_arc = Some(Arc::clone(&playback_position));
                
                app.daw_lanes[lane_idx].is_playing = true;
                app.daw_lanes[lane_idx].playback_position = 0.0;
                
                AudioDevice::play_audio(
                    wav.to_f64_samples(),
                    wav.header.sample_rate,
                    wav.header.num_channels,
                    playback_position
                );
                
                app.status = "Playing Lane 1".to_string();
            }
        }
        KeyCode::Char('2') => {
            let lane_idx = 1;
            if let Some(ref wav) = app.daw_lanes[lane_idx].wav_data {
                use std::sync::{Arc, Mutex};
                
                let playback_position = Arc::new(Mutex::new(0.0f64));
                app.playback_position_arc = Some(Arc::clone(&playback_position));
                
                app.daw_lanes[lane_idx].is_playing = true;
                app.daw_lanes[lane_idx].playback_position = 0.0;
                
                AudioDevice::play_audio(
                    wav.to_f64_samples(),
                    wav.header.sample_rate,
                    wav.header.num_channels,
                    playback_position
                );
                
                app.status = "Playing Lane 2".to_string();
            }
        }
        KeyCode::Up => {
            if app.selected > 0 {
                app.selected -= 1;
            }
        }
        KeyCode::Down => {
            if app.selected < 2 {
                app.selected += 1;
            }
        }
        KeyCode::Char('l') => {
            app.input_mode = true;
            app.input_buffer.clear();
            app.status = "Enter file path: ".to_string();
        }
        KeyCode::Char('c') => {
            app.daw_lanes[app.selected].file_path.clear();
            app.daw_lanes[app.selected].wav_data = None;
            app.status = format!("Lane {} cleared", app.selected + 1);
        }
        KeyCode::Char('m') => {
            app.daw_lanes[app.selected].muted = !app.daw_lanes[app.selected].muted;
        }
        KeyCode::Char('+') | KeyCode::Char('=') => {
            let lane = &mut app.daw_lanes[app.selected];
            lane.volume = (lane.volume + 0.1).min(2.0);
        }
        KeyCode::Char('-') => {
            let lane = &mut app.daw_lanes[app.selected];
            lane.volume = (lane.volume - 0.1).max(0.0);
        }
        KeyCode::Char('x') => {
            app.status = "Export mixed audio".to_string();
        }
        _ => {}
    }
    Ok(false)
}
