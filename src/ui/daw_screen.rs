
use crossterm::event::KeyCode;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Gauge, List, ListItem, Sparkline},
    Frame,
};

use super::screen_trait::ScreenTrait;
use super::App;

mod layout_config {
    use ratatui::layout::Constraint;
    use ratatui::style::Color;

    pub const LANE_PERCENTAGES: [u16; 3] = [33, 33, 34];
    pub const SELECTED_BORDER: Color = Color::Yellow;
    pub const DEFAULT_BORDER: Color = Color::White;
    pub const ARMED_BORDER: Color = Color::Red;
    pub const RECORDING_BORDER: Color = Color::Magenta;
    pub const EMPTY_LANE_MESSAGE: &str = "Space: Play All | 'p': Solo | 'a': Arm | 'r': Record";
    pub const LANE_STATUS_EMPTY: &str = "Empty";
    pub const LANE_STATUS_ARMED: &str = "ARMED";
    pub const LANE_STATUS_MUTED: &str = "MUTED";
    pub const LANE_STATUS_ACTIVE: &str = "ACTIVE";
    pub const LANE_STATUS_RECORDING: &str = "🔴 REC";

    pub fn get_lane_constraints() -> [Constraint; 3] {
        [
            Constraint::Percentage(LANE_PERCENTAGES[0]),
            Constraint::Percentage(LANE_PERCENTAGES[1]),
            Constraint::Percentage(LANE_PERCENTAGES[2]),
        ]
    }

    pub fn format_lane_title(
        lane_num: usize,
        volume: f64,
        status: &str,
        file_path: &str,
    ) -> String {
        format!(
            "Lane {} | Vol: {:.0}% | {} | {}",
            lane_num,
            volume * 100.0,
            status,
            if file_path.is_empty() {
                LANE_STATUS_EMPTY
            } else {
                file_path
            }
        )
    }
}

pub struct DawScreen;

impl ScreenTrait for DawScreen {
    fn render(&self, f: &mut Frame, app: &App, area: Rect) {
        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Progress bar
                Constraint::Min(10),   // Tracks
            ])
            .split(area);

        let is_playing = app.session.transport.is_playing();
        
        let label = if is_playing {
            "▶ Playing".to_string()
        } else {
            "⏹ Stopped".to_string()
        };
        
        let gauge = Gauge::default()
            .block(Block::default().borders(Borders::ALL).title("Transport"))
            .gauge_style(Style::default().fg(if is_playing { Color::Green } else { Color::Gray }))
            .label(label);
        
        f.render_widget(gauge, main_chunks[0]);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(layout_config::get_lane_constraints())
            .split(main_chunks[1]);

        for (i, chunk) in chunks.iter().enumerate() {
            let track = &app.session.tracks[i];
            let is_selected = app.selected == i;

            let border_color = if app.session.transport.is_playing() && !track.muted && track.wav_data.is_some() {
                layout_config::SELECTED_BORDER // Show yellow when actively playing
            } else if track.state == crate::track::TrackState::Recording {
                layout_config::RECORDING_BORDER
            } else if track.armed {
                layout_config::ARMED_BORDER
            } else if is_selected {
                layout_config::SELECTED_BORDER
            } else {
                layout_config::DEFAULT_BORDER
            };

            let status = if track.state == crate::track::TrackState::Recording {
                layout_config::LANE_STATUS_RECORDING
            } else if track.armed {
                layout_config::LANE_STATUS_ARMED
            } else if track.muted {
                layout_config::LANE_STATUS_MUTED
            } else {
                layout_config::LANE_STATUS_ACTIVE
            };

            let title =
                layout_config::format_lane_title(i + 1, track.volume, status, &track.file_path);

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(border_color))
                .title(title);

            if let Some(waveform) = track.waveform() {
                let max_val = waveform.iter().cloned().fold(0.0f64, f64::max).max(0.001);
                let waveform_u64: Vec<u64> = waveform
                    .iter()
                    .map(|&v| ((v / max_val) * 100.0) as u64)
                    .collect();
                let sparkline = Sparkline::default()
                    .block(block)
                    .data(&waveform_u64)
                    .style(Style::default().fg(layout_config::DEFAULT_BORDER));
                f.render_widget(sparkline, *chunk);
            } else {
                let list = List::new(vec![ListItem::new(layout_config::EMPTY_LANE_MESSAGE)]).block(block);
                f.render_widget(list, *chunk);
            }
        }
    }

    fn handle_input(
        &self,
        app: &mut App,
        key: KeyCode,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        match key {
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

            // Global transport control
            KeyCode::Char(' ') | KeyCode::Enter => {
                match app.session.toggle_playback() {
                    Ok(_) => {
                        let state = if app.session.transport.is_playing() {
                            "Playing all tracks"
                        } else {
                            "Stopped"
                        };
                        app.status = state.to_string();
                    }
                    Err(e) => app.status = format!("Playback error: {}", e),
                }
            }

            // Individual track playback (solo)
            KeyCode::Char('p') => {
                match app.session.tracks[app.selected].play() {
                    Ok(_) => app.status = format!("Playing Track {}", app.selected + 1),
                    Err(e) => app.status = format!("Error playing: {}", e),
                }
            }
        
            KeyCode::Char('a') => {
                let track = &mut app.session.tracks[app.selected];
                if track.armed {
                    track.disarm();
                    app.status = format!("Track {} disarmed", app.selected + 1);
                } else {
                    match track.arm() {
                        Ok(_) => {
                            app.status = format!("Track {} armed", app.selected + 1);
                        },
                        Err(e) => app.status = format!("Error arming track: {}", e),
                    }
                }
            }
            
            KeyCode::Char('r') => {
                let track = &mut app.session.tracks[app.selected];
                if track.state == crate::track::TrackState::Recording {
                    match track.stop_recording() {
                        Ok(_) => app.status = format!("Track {} stopped recording", app.selected + 1),
                        Err(e) => app.status = format!("Error stopping recording: {}", e),
                    }
                } else if track.armed {
                    match track.start_recording() {
                        Ok(_) => app.status = format!("Track {} recording!", app.selected + 1),
                        Err(e) => app.status = format!("Error starting recording: {}", e),
                    }
                } else {
                    app.status = "Track must be armed to record (press 'a')".to_string();
                }
            }
            
            KeyCode::Char('M') => {
                // Toggle monitoring (capital M)
                let track = &mut app.session.tracks[app.selected];
                if track.armed {
                    if track.monitoring {
                        track.stop_monitoring();
                        app.status = format!("Track {} monitoring off", app.selected + 1);
                    } else {
                        match track.start_monitoring() {
                            Ok(_) => app.status = format!("Track {} monitoring on", app.selected + 1),
                            Err(e) => app.status = format!("Error starting monitoring: {}", e),
                        }
                    }
                } else {
                    app.status = "Track must be armed to monitor (press 'a')".to_string();
                }
            }
            
            // File operations
            KeyCode::Char('c') => {
                app.session.tracks[app.selected].file_path.clear();
                app.session.tracks[app.selected].wav_data = None;
                app.status = format!("Track {} cleared", app.selected + 1);
            }
            
            // Volume and mute
            KeyCode::Char('m') => {
                app.session.tracks[app.selected].muted = !app.session.tracks[app.selected].muted;
                let status = if app.session.tracks[app.selected].muted { "muted" } else { "unmuted" };
                app.status = format!("Track {} {}", app.selected + 1, status);
            }
            
            KeyCode::Char('+') | KeyCode::Char('=') => {
                let track = &mut app.session.tracks[app.selected];
                track.volume = (track.volume + 0.1).min(2.0);
                app.status = format!("Track {} volume: {:.0}%", app.selected + 1, track.volume * 100.0);
            }
            
            KeyCode::Char('-') => {
                let track = &mut app.session.tracks[app.selected];
                track.volume = (track.volume - 0.1).max(0.0);
                app.status = format!("Track {} volume: {:.0}%", app.selected + 1, track.volume * 100.0);
            }
            
            KeyCode::Char('x') => {
                app.status = "Export mixed audio".to_string();
            }
            
            _ => {}
        }
        Ok(false)
    }
}
