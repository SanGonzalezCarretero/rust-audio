use super::layout_config;
use crate::ui::{App, Screen};
use crossterm::event::KeyCode;

fn selected_track(app: &App) -> usize {
    match app.screen {
        Screen::Daw { selected_track } => selected_track,
        _ => 0,
    }
}

fn set_selected_track(app: &mut App, value: usize) {
    if let Screen::Daw {
        ref mut selected_track,
    } = app.screen
    {
        *selected_track = value;
    }
}

pub fn handle_input(app: &mut App, key: KeyCode) -> Result<bool, Box<dyn std::error::Error>> {
    let track_count = app.session.tracks.len();
    let max_selected = track_count.saturating_sub(1);

    let sel = selected_track(app);
    if track_count > 0 && sel >= track_count {
        set_selected_track(app, max_selected);
    }
    let sel = selected_track(app);

    match key {
        KeyCode::Up => {
            if sel > 0 {
                set_selected_track(app, sel - 1);
            }
        }
        KeyCode::Down => {
            if sel < max_selected {
                set_selected_track(app, sel + 1);
            }
        }
        KeyCode::Left => {
            if !app.session.transport.is_playing() {
                let delta = -(app.session.sample_rate as f64
                    * layout_config::PLAYHEAD_DELTA_SECONDS) as i64;
                app.session.transport.move_playhead(delta);
                let secs = app
                    .session
                    .transport
                    .playhead_seconds(app.session.sample_rate);
                app.status = format!("Playhead: {:.1}s", secs);
            }
        }
        KeyCode::Right => {
            if !app.session.transport.is_playing() {
                let delta =
                    (app.session.sample_rate as f64 * layout_config::PLAYHEAD_DELTA_SECONDS) as i64;
                app.session.transport.move_playhead(delta);
                let secs = app
                    .session
                    .transport
                    .playhead_seconds(app.session.sample_rate);
                app.status = format!("Playhead: {:.1}s", secs);
            }
        }
        KeyCode::Char('h') => {
            if !app.session.transport.is_playing() {
                app.session.transport.reset_playhead();
                app.status = "Playhead reset to start".to_string();
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
            let track = &mut app.session.tracks[sel];
            if track.is_armed() {
                track.disarm();
                app.session.stop_monitoring();
                app.status = format!("Track {} disarmed", sel + 1);
            } else {
                track.arm();
                if let Err(e) = app.session.start_monitoring() {
                    app.status = format!("Error starting monitoring: {}", e);
                } else {
                    app.status = format!("Track {} armed", sel + 1);
                }
            }
        }

        KeyCode::Char('r') => {
            // Check if any track is currently recording
            let any_recording = app
                .session
                .tracks
                .iter()
                .any(|t| t.state == crate::track::TrackState::Recording);

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
            let track = &mut app.session.tracks[sel];
            if track.is_armed() {
                if track.monitoring {
                    track.stop_monitoring();
                    app.session.stop_monitoring();
                    app.status = format!("Track {} monitoring off", sel + 1);
                    // Restart monitoring for remaining tracks
                    let _ = app.session.start_monitoring();
                } else {
                    track.start_monitoring();
                    match app.session.start_monitoring() {
                        Ok(_) => app.status = format!("Track {} monitoring on", sel + 1),
                        Err(e) => app.status = format!("Error starting monitoring: {}", e),
                    }
                }
            } else {
                app.status = "Track must be armed to monitor (press 'a')".to_string();
            }
        }

        KeyCode::Char('c') => {
            app.session.tracks[sel].clips.clear();
            app.status = format!("Track {} cleared", sel + 1);
        }

        // Volume and mute
        KeyCode::Char('m') => {
            app.session.tracks[sel].muted = !app.session.tracks[sel].muted;
            let status = if app.session.tracks[sel].muted {
                "muted"
            } else {
                "unmuted"
            };
            app.status = format!("Track {} {}", sel + 1, status);
        }

        KeyCode::Char('+') | KeyCode::Char('=') => {
            let track = &mut app.session.tracks[sel];
            track.volume = (track.volume + 0.1).min(2.0);
            app.status = format!("Track {} volume: {:.0}%", sel + 1, track.volume * 100.0);
        }

        KeyCode::Char('-') => {
            let track = &mut app.session.tracks[sel];
            track.volume = (track.volume - 0.1).max(0.0);
            app.status = format!("Track {} volume: {:.0}%", sel + 1, track.volume * 100.0);
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
                    set_selected_track(app, track_count);
                }
                Err(e) => app.status = format!("Cannot add track: {}", e),
            }
        }

        KeyCode::Char('d') => {
            // Delete selected track
            if track_count <= 1 {
                app.status = "Cannot remove the last track".to_string();
            } else {
                match app.session.remove_track(sel) {
                    Ok(_) => {
                        app.status = format!("Track {} removed", sel + 1);
                        // Adjust selected index if needed
                        if sel >= app.session.tracks.len() {
                            set_selected_track(app, app.session.tracks.len().saturating_sub(1));
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
