use super::layout_config;
use crate::ui::{App, Screen};
use crossterm::event::KeyCode;

fn selected_track(app: &App) -> usize {
    match app.screen {
        Screen::Daw { selected_track, .. } => selected_track,
        _ => 0,
    }
}

fn set_selected_track(app: &mut App, value: usize) {
    if let Screen::Daw {
        ref mut selected_track,
        ..
    } = app.screen
    {
        *selected_track = value;
    }
}

fn scroll_offset(app: &App) -> u64 {
    match app.screen {
        Screen::Daw { scroll_offset, .. } => scroll_offset,
        _ => 0,
    }
}

fn set_scroll_offset(app: &mut App, value: u64) {
    if let Screen::Daw {
        ref mut scroll_offset,
        ..
    } = app.screen
    {
        *scroll_offset = value;
    }
}

fn selected_clip(app: &App) -> Option<usize> {
    match app.screen {
        Screen::Daw { selected_clip, .. } => selected_clip,
        _ => None,
    }
}

fn set_selected_clip(app: &mut App, value: Option<usize>) {
    if let Screen::Daw {
        ref mut selected_clip,
        ..
    } = app.screen
    {
        *selected_clip = value;
    }
}

/// Auto-scroll the viewport so the playhead stays visible.
fn ensure_playhead_visible(app: &mut App) {
    let timeline_samples = app.session.sample_rate as u64 * layout_config::TIMELINE_SECONDS;
    let playhead = app.session.transport.playhead_position;
    let offset = scroll_offset(app);

    let left_of_viewport = playhead < offset;
    let right_of_viewport = playhead >= offset + timeline_samples;

    if left_of_viewport || right_of_viewport {
        set_scroll_offset(app, playhead);
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

    // Validate selected_clip
    if let Some(clip_idx) = selected_clip(app) {
        if track_count == 0 || clip_idx >= app.session.tracks[sel].clips.len() {
            set_selected_clip(app, None);
        }
    }

    match key {
        KeyCode::Up => {
            if sel > 0 {
                set_selected_track(app, sel - 1);
                set_selected_clip(app, None);
            }
        }
        KeyCode::Down => {
            if sel < max_selected {
                set_selected_track(app, sel + 1);
                set_selected_clip(app, None);
            }
        }
        KeyCode::Left => {
            if let Some(clip_idx) = selected_clip(app) {
                if !app.session.transport.is_playing() {
                    // Move the Clip to the left
                    let delta = (app.session.sample_rate as f64
                        * layout_config::PLAYHEAD_DELTA_SECONDS)
                        as u64;
                    let track = &mut app.session.tracks[sel];
                    track.clips[clip_idx].starts_at =
                        track.clips[clip_idx].starts_at.saturating_sub(delta);
                    track.cache_waveform();
                    let secs =
                        track.clips[clip_idx].starts_at as f64 / app.session.sample_rate as f64;
                    app.status = format!("Clip moved to {:.1}s", secs);
                }
            } else if !app.session.transport.is_playing() {
                let delta = -(app.session.sample_rate as f64
                    * layout_config::PLAYHEAD_DELTA_SECONDS) as i64;
                app.session.transport.move_playhead(delta);
                ensure_playhead_visible(app);
                let secs = app
                    .session
                    .transport
                    .playhead_seconds(app.session.sample_rate);
                app.status = format!("Playhead: {:.1}s", secs);
            }
        }
        KeyCode::Right => {
            if let Some(clip_idx) = selected_clip(app) {
                if !app.session.transport.is_playing() {
                    // Move the Clip to the right
                    let delta = (app.session.sample_rate as f64
                        * layout_config::PLAYHEAD_DELTA_SECONDS)
                        as u64;
                    let track = &mut app.session.tracks[sel];
                    track.clips[clip_idx].starts_at =
                        track.clips[clip_idx].starts_at.saturating_add(delta);
                    track.cache_waveform();
                    let secs =
                        track.clips[clip_idx].starts_at as f64 / app.session.sample_rate as f64;
                    app.status = format!("Clip moved to {:.1}s", secs);
                }
            } else if !app.session.transport.is_playing() {
                let delta =
                    (app.session.sample_rate as f64 * layout_config::PLAYHEAD_DELTA_SECONDS) as i64;
                app.session.transport.move_playhead(delta);
                ensure_playhead_visible(app);
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
                set_scroll_offset(app, 0);
                app.status = "Playhead reset to start".to_string();
            }
        }

        // Manual timeline scrolling
        KeyCode::Char('[') => {
            let scroll_step = app.session.sample_rate as u64 * layout_config::SCROLL_STEP_SECONDS;
            let offset = scroll_offset(app);
            set_scroll_offset(app, offset.saturating_sub(scroll_step));
        }
        KeyCode::Char(']') => {
            let scroll_step = app.session.sample_rate as u64 * layout_config::SCROLL_STEP_SECONDS;
            let offset = scroll_offset(app);
            set_scroll_offset(app, offset + scroll_step);
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
            set_selected_clip(app, None);
            app.session.tracks[sel].clips.clear();
            app.session.tracks[sel].cache_waveform();
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

        KeyCode::Char('i') => {
            let track = &mut app.session.tracks[sel];
            track.input_channel = match track.input_channel {
                None => Some(0),
                Some(0) => Some(1),
                Some(_) => None,
            };
            let label = match track.input_channel {
                None => "All".to_string(),
                Some(ch) => format!("In {}", ch + 1),
            };
            app.status = format!("Track {} input: {}", sel + 1, label);
            // Restart monitoring so the callback picks up the new input channel
            app.session.stop_monitoring();
            let _ = app.session.start_monitoring();
        }

        KeyCode::Char('x') => {
            let samples = app.session.render_full_mix();
            if samples.is_empty() {
                app.status = "Nothing to export".to_string();
            } else {
                let mut wav = crate::wav::WavFile::new(app.session.sample_rate, 1);
                wav.from_f32_samples(&samples);
                let dir = app
                    .project_dir
                    .clone()
                    .unwrap_or_else(|| std::path::PathBuf::from("."));
                let name = &app.session.name;
                let path = {
                    let base = dir.join(format!("{}_mix.wav", name));
                    if !base.exists() {
                        base
                    } else {
                        let mut n = 1u32;
                        loop {
                            let p = dir.join(format!("{}_mix_{}.wav", name, n));
                            if !p.exists() {
                                break p;
                            }
                            n += 1;
                        }
                    }
                };
                match wav.save_to_file(&path) {
                    Ok(()) => app.status = format!("Exported to {}", path.display()),
                    Err(e) => app.status = format!("Export error: {}", e),
                }
            }
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
            set_selected_clip(app, None);
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

        KeyCode::Tab => {
            if !app.session.transport.is_playing() && track_count > 0 {
                let clip_count = app.session.tracks[sel].clips.len();
                if clip_count > 0 {
                    let next = match selected_clip(app) {
                        None => Some(0),
                        Some(idx) if idx + 1 < clip_count => Some(idx + 1),
                        Some(_) => None,
                    };
                    set_selected_clip(app, next);
                    if let Some(idx) = next {
                        let clip = &app.session.tracks[sel].clips[idx];
                        let secs = clip.starts_at as f64 / app.session.sample_rate as f64;
                        app.status = format!("Clip {} selected (at {:.1}s)", idx + 1, secs);
                    } else {
                        app.status = "Clip deselected".to_string();
                    }
                }
            }
        }

        KeyCode::Backspace => {
            if let Some(clip_idx) = selected_clip(app) {
                if !app.session.transport.is_playing() {
                    app.session.tracks[sel].clips.remove(clip_idx);
                    app.session.tracks[sel].cache_waveform();
                    set_selected_clip(app, None);
                    app.status = format!("Clip {} deleted", clip_idx + 1);
                }
            }
        }

        KeyCode::Esc => {
            if selected_clip(app).is_some() {
                set_selected_clip(app, None);
                app.status = "Clip deselected".to_string();
            }
        }

        _ => {}
    }
    Ok(false)
}
