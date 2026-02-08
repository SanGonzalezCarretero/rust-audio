use crossterm::event::KeyCode;
use crate::ui::App;

pub fn handle_input(app: &mut App, key: KeyCode) -> Result<bool, Box<dyn std::error::Error>> {
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
        KeyCode::Left => {
            if !app.session.transport.is_playing() {
                let delta = -(app.session.sample_rate as i64 / 2); // -0.5 seconds
                app.session.transport.move_playhead(delta);
                let secs = app.session.transport.playhead_seconds(app.session.sample_rate);
                app.status = format!("Playhead: {:.1}s", secs);
            }
        }
        KeyCode::Right => {
            if !app.session.transport.is_playing() {
                let delta = app.session.sample_rate as i64 / 2; // +0.5 seconds
                app.session.transport.move_playhead(delta);
                let secs = app.session.transport.playhead_seconds(app.session.sample_rate);
                app.status = format!("Playhead: {:.1}s", secs);
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
            // Check if any track is currently recording
            let any_recording = app.session.tracks.iter().any(|t| {
                t.state == crate::track::TrackState::Recording
            });

            if any_recording {
                // Stop all recording and overdub playback
                app.session.stop_all_recording();
                app.status = "Recording stopped".to_string();
            } else {
                // Start recording on all armed tracks via shared input stream
                match app.session.start_recording() {
                    Ok(0) => {
                        app.status = "No armed tracks to record (press 'a')".to_string();
                    }
                    Ok(count) => {
                        app.status = format!("Recording {} armed track(s)", count);
                    }
                    Err(e) => {
                        app.status = format!("Recording error: {}", e);
                    }
                }
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
            app.session.tracks[app.selected].clips.clear();
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
