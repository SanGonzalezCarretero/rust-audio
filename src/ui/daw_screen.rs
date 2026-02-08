use crossterm::event::KeyCode;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{
        canvas::{Canvas, Line},
        Block, BorderType, Borders, Gauge, List, ListItem, Paragraph,
    },
    Frame,
};

use super::screen_trait::ScreenTrait;
use super::App;

mod layout_config {
    use ratatui::layout::Constraint;
    use ratatui::style::Color;

    pub const SELECTED_BORDER: Color = Color::Yellow;
    pub const DEFAULT_BORDER: Color = Color::White;
    pub const ARMED_BORDER: Color = Color::Red;
    pub const ARMED_SELECTED_BORDER: Color = Color::LightRed;
    pub const RECORDING_BORDER: Color = Color::Magenta;
    pub const RECORDING_SELECTED_BORDER: Color = Color::LightMagenta;
    pub const EMPTY_LANE_MESSAGE: &str = "'p': Solo | 'a': Arm | 'r': Record | 'f': Record Armed";
    pub const LANE_STATUS_EMPTY: &str = "Empty";
    pub const LANE_STATUS_ARMED: &str = "ARMED";
    pub const LANE_STATUS_MUTED: &str = "MUTED";
    pub const LANE_STATUS_ACTIVE: &str = "ACTIVE";
    pub const LANE_STATUS_RECORDING: &str = "🔴 REC";
    pub const GLOBAL_INSTRUCTIONS: &str =
        "n: Add new track | d: Delete track | Space: Play all tracks";

    pub fn get_lane_constraints(track_count: usize) -> Vec<Constraint> {
        let denominator = track_count.max(3) as u32;
        (0..track_count)
            .map(|_| Constraint::Ratio(1, denominator))
            .collect()
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
                Constraint::Length(1), // Instructions bar
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
            .gauge_style(Style::default().fg(if is_playing {
                Color::Green
            } else {
                Color::Gray
            }))
            .label(label);

        f.render_widget(gauge, main_chunks[0]);

        // Render instructions bar
        let instructions = Paragraph::new(layout_config::GLOBAL_INSTRUCTIONS)
            .block(Block::default().borders(Borders::NONE))
            .style(Style::default().fg(Color::Gray));
        f.render_widget(instructions, main_chunks[1]);

        let track_count = app.session.tracks.len();
        let constraints = layout_config::get_lane_constraints(track_count);
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(main_chunks[2]);

        for (i, chunk) in chunks.iter().enumerate() {
            if i >= track_count {
                break;
            }
            let track = &app.session.tracks[i];
            let is_selected = app.selected == i;

            let border_color =
                if app.session.transport.is_playing() && !track.muted && track.wav_data.is_some() {
                    layout_config::SELECTED_BORDER // Show yellow when actively playing
                } else if track.state == crate::track::TrackState::Recording && is_selected {
                    layout_config::RECORDING_SELECTED_BORDER
                } else if track.state == crate::track::TrackState::Recording {
                    layout_config::RECORDING_BORDER
                } else if track.armed && is_selected {
                    layout_config::ARMED_SELECTED_BORDER
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
                .border_type(if is_selected {
                    BorderType::Thick
                } else {
                    BorderType::Plain
                })
                .border_style(Style::default().fg(border_color))
                .title(title);

            if let Some(waveform) = track.waveform() {
                let canvas = Canvas::default()
                    .block(block)
                    .x_bounds([0.0, waveform.len() as f64])
                    .y_bounds([-1.0, 1.0])
                    .paint(|ctx| {
                        ctx.draw(&Line {
                            x1: 0.0,
                            y1: 0.0,
                            x2: waveform.len() as f64,
                            y2: 0.0,
                            color: Color::DarkGray,
                        });

                        for (i, &(min, max)) in waveform.iter().enumerate() {
                            let x = i as f64;
                            // Amplify the waveform for better visibility
                            const SENSITIVITY: f64 = 8.0;
                            let y_min = (min * SENSITIVITY).clamp(-1.0, 1.0);
                            let y_max = (max * SENSITIVITY).clamp(-1.0, 1.0);

                            // Draw vertical line from min to max
                            ctx.draw(&Line {
                                x1: x,
                                y1: y_min,
                                x2: x,
                                y2: y_max,
                                color: layout_config::DEFAULT_BORDER,
                            });
                        }
                    });
                f.render_widget(canvas, *chunk);
            } else {
                let list =
                    List::new(vec![ListItem::new(layout_config::EMPTY_LANE_MESSAGE)]).block(block);
                f.render_widget(list, *chunk);
            }
        }
    }

    fn handle_input(
        &self,
        app: &mut App,
        key: KeyCode,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let track_count = app.session.tracks.len();
        let max_selected = track_count.saturating_sub(1);

        if track_count > 0 && app.selected >= track_count {
            app.selected = max_selected;
        }

        match key {
            KeyCode::Up => {
                if app.selected > 0 {
                    app.selected -= 1;
                }
            }
            KeyCode::Down => {
                if app.selected < max_selected {
                    app.selected += 1;
                }
            }

            // Global transport control
            KeyCode::Char(' ') | KeyCode::Enter => match app.session.toggle_playback() {
                Ok(_) => {
                    let state = if app.session.transport.is_playing() {
                        "Playing all tracks"
                    } else {
                        "Stopped"
                    };
                    app.status = state.to_string();
                }
                Err(e) => app.status = format!("Playback error: {}", e),
            },

            // Individual track playback (solo) - toggle playback
            KeyCode::Char('p') => {
                if app.session.transport.is_playing() {
                    app.status = "Stop global playback first (press space)".to_string();
                } else {
                    let track = &mut app.session.tracks[app.selected];
                    if track.is_playing_track() {
                        track.stop_playback();
                        app.status = format!("Track {} stopped", app.selected + 1);
                    } else {
                        match track.play() {
                            Ok(_) => app.status = format!("Playing Track {}", app.selected + 1),
                            Err(e) => app.status = format!("Error playing: {}", e),
                        }
                    }
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
                        }
                        Err(e) => app.status = format!("Error arming track: {}", e),
                    }
                }
            }

            KeyCode::Char('r') => {
                let track = &mut app.session.tracks[app.selected];
                if track.state == crate::track::TrackState::Recording {
                    match track.stop_recording() {
                        Ok(_) => {
                            app.status = format!("Track {} stopped recording", app.selected + 1)
                        }
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
                            Ok(_) => {
                                app.status = format!("Track {} monitoring on", app.selected + 1)
                            }
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
                let status = if app.session.tracks[app.selected].muted {
                    "muted"
                } else {
                    "unmuted"
                };
                app.status = format!("Track {} {}", app.selected + 1, status);
            }

            KeyCode::Char('+') | KeyCode::Char('=') => {
                let track = &mut app.session.tracks[app.selected];
                track.volume = (track.volume + 0.1).min(2.0);
                app.status = format!(
                    "Track {} volume: {:.0}%",
                    app.selected + 1,
                    track.volume * 100.0
                );
            }

            KeyCode::Char('-') => {
                let track = &mut app.session.tracks[app.selected];
                track.volume = (track.volume - 0.1).max(0.0);
                app.status = format!(
                    "Track {} volume: {:.0}%",
                    app.selected + 1,
                    track.volume * 100.0
                );
            }

            KeyCode::Char('x') => {
                app.status = "Export mixed audio".to_string();
            }

            KeyCode::Char('f') => {
                // Record all armed tracks simultaneously
                let mut recording_count = 0;
                let mut errors = Vec::new();

                for (i, track) in app.session.tracks.iter_mut().enumerate() {
                    if track.armed {
                        match track.start_recording() {
                            Ok(_) => recording_count += 1,
                            Err(e) => errors.push(format!("Track {}: {}", i + 1, e)),
                        }
                    }
                }

                if recording_count > 0 {
                    app.status = format!("Recording {} armed track(s)", recording_count);
                } else if !errors.is_empty() {
                    app.status = format!("Errors: {}", errors.join(", "));
                } else {
                    app.status = "No armed tracks to record".to_string();
                }
            }

            KeyCode::Char('n') => {
                // Add new track
                let track_num = track_count + 1;
                match app.session.add_track(format!("Track {}", track_num)) {
                    Ok(_) => {
                        app.status = format!("Track {} added", track_num);
                        // Select the newly added track
                        app.selected = track_count;
                    }
                    Err(e) => app.status = format!("Cannot add track: {}", e),
                }
            }

            KeyCode::Char('d') => {
                // Delete selected track
                if track_count <= 1 {
                    app.status = "Cannot remove the last track".to_string();
                } else {
                    let selected_index = app.selected;
                    match app.session.remove_track(selected_index) {
                        Ok(_) => {
                            app.status = format!("Track {} removed", selected_index + 1);
                            // Adjust selected index if needed
                            if app.selected >= app.session.tracks.len() {
                                app.selected = app.session.tracks.len().saturating_sub(1);
                            }
                        }
                        Err(e) => app.status = format!("Cannot remove track: {}", e),
                    }
                }
            }

            _ => {}
        }
        Ok(false)
    }
}
