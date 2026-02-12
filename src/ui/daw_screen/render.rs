use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{
        canvas::{Canvas, Line},
        Block, BorderType, Borders, Gauge, Paragraph,
    },
    Frame,
};

use super::layout_config;
use crate::ui::{App, Screen};

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let (selected_track_idx, scroll_offset, selected_clip_idx) = match app.screen {
        Screen::Daw {
            selected_track,
            scroll_offset,
            selected_clip,
        } => (selected_track, scroll_offset, selected_clip),
        _ => (0, 0, None),
    };
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Progress bar
            Constraint::Length(1), // Instructions bar
            Constraint::Min(10),   // Tracks
        ])
        .split(area);

    let is_playing = app.session.transport.is_playing();
    let playhead_secs = app
        .session
        .transport
        .playhead_seconds(app.session.sample_rate);
    let minutes = (playhead_secs / 60.0) as u32;
    let secs = playhead_secs % 60.0;

    let label = if is_playing {
        format!("\u{25b6} Playing  {:02}:{:05.2}", minutes, secs)
    } else {
        format!("\u{23f9} Stopped  {:02}:{:05.2}", minutes, secs)
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
        let is_selected = selected_track_idx == i;

        let border_color =
            if app.session.transport.is_playing() && !track.muted && !track.clips.is_empty() {
                layout_config::SELECTED_BORDER // Show yellow when actively playing
            } else if track.state == crate::track::TrackState::Recording && is_selected {
                layout_config::RECORDING_SELECTED_BORDER
            } else if track.state == crate::track::TrackState::Recording {
                layout_config::RECORDING_BORDER
            } else if track.is_armed() && is_selected {
                layout_config::ARMED_SELECTED_BORDER
            } else if track.is_armed() {
                layout_config::ARMED_BORDER
            } else if is_selected {
                layout_config::SELECTED_BORDER
            } else {
                layout_config::DEFAULT_BORDER
            };

        let status = if track.state == crate::track::TrackState::Recording {
            layout_config::LANE_STATUS_RECORDING
        } else if track.is_armed() {
            layout_config::LANE_STATUS_ARMED
        } else if track.muted {
            layout_config::LANE_STATUS_MUTED
        } else {
            layout_config::LANE_STATUS_ACTIVE
        };

        let title = layout_config::format_lane_title(&track.name, track.volume, status);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(if is_selected {
                BorderType::Thick
            } else {
                BorderType::Plain
            })
            .border_style(Style::default().fg(border_color))
            .title(title);

        // Every track shows a timeline canvas with playhead
        let sample_rate = app.session.sample_rate;
        let playhead_pos = app.session.transport.playhead_position;
        let is_recording = track.state == crate::track::TrackState::Recording;
        let rec_start_pos = track.recording_start_position;

        let clips_waveform: Option<Vec<(f32, f32)>> = track.clips_waveform().map(|w| w.to_vec());
        let clips_end = track.clips_end();
        let clips_waveform_len = clips_waveform.as_ref().map(|w| w.len()).unwrap_or(0);

        let rec_waveform = if is_recording {
            track.recording_waveform()
        } else {
            None
        };

        // Clip bounds with selection state: (start, end, is_selected_clip)
        let clip_bounds: Vec<(f64, f64, bool)> = track
            .clips
            .iter()
            .enumerate()
            .map(|(ci, c)| {
                let start = c.starts_at as f64;
                let end = (c.starts_at + c.wav_data.frame_count() as u64) as f64;
                let sel = is_selected && selected_clip_idx == Some(ci);
                (start, end, sel)
            })
            .collect();

        let timeline_samples = sample_rate as u64 * layout_config::TIMELINE_SECONDS;
        let view_start = scroll_offset as f64;
        let view_end = (scroll_offset + timeline_samples) as f64;

        let canvas = Canvas::default()
            .block(block)
            .x_bounds([view_start, view_end])
            .y_bounds([-1.0, 1.0])
            .paint(move |ctx| {
                // Center line (timeline axis)
                ctx.draw(&Line {
                    x1: view_start,
                    y1: 0.0,
                    x2: view_end,
                    y2: 0.0,
                    color: Color::DarkGray,
                });

                // Draw clip boundaries (box: left, right, top, bottom)
                for &(start, end, is_clip_selected) in &clip_bounds {
                    let clip_color = if is_clip_selected {
                        Color::Green
                    } else {
                        Color::DarkGray
                    };
                    // Top and bottom
                    ctx.draw(&Line { x1: start, y1: 0.95, x2: end, y2: 0.95, color: clip_color });
                    ctx.draw(&Line { x1: start, y1: -0.95, x2: end, y2: -0.95, color: clip_color });
                    // Left and right
                    ctx.draw(&Line { x1: start, y1: -0.95, x2: start, y2: 0.95, color: clip_color });
                    ctx.draw(&Line { x1: end, y1: -0.95, x2: end, y2: 0.95, color: clip_color });
                }

                // Draw existing clips waveform
                if let Some(ref waveform) = clips_waveform {
                    let spp = if clips_waveform_len > 0 {
                        clips_end as f64 / clips_waveform_len as f64
                    } else {
                        1.0
                    };
                    for (j, &(min, max)) in waveform.iter().enumerate() {
                        let x = j as f64 * spp;
                        if !clip_bounds.iter().any(|&(s, e, _)| x >= s && x <= e) {
                            continue;
                        }
                        let y_min = (min * layout_config::WAVEFORM_SENSITIVITY).clamp(-1.0, 1.0) as f64;
                        let y_max = (max * layout_config::WAVEFORM_SENSITIVITY).clamp(-1.0, 1.0) as f64;
                        ctx.draw(&Line {
                            x1: x, y1: y_min, x2: x, y2: y_max,
                            color: Color::Green,
                        });
                    }
                }

                // Draw live recording waveform
                if let Some(ref waveform) = rec_waveform {
                    let chunk = crate::track::RECORDING_WAVEFORM_CHUNK_SIZE as f64;
                    for (j, &(min, max)) in waveform.iter().enumerate() {
                        let x = rec_start_pos as f64 + j as f64 * chunk;
                        let y_min = (min * layout_config::WAVEFORM_SENSITIVITY).clamp(-1.0, 1.0) as f64;
                        let y_max = (max * layout_config::WAVEFORM_SENSITIVITY).clamp(-1.0, 1.0) as f64;
                        ctx.draw(&Line {
                            x1: x, y1: y_min, x2: x, y2: y_max,
                            color: Color::LightRed,
                        });
                    }
                }

                // Draw playhead line (always visible, cyan)
                ctx.draw(&Line {
                    x1: playhead_pos as f64,
                    y1: -1.0,
                    x2: playhead_pos as f64,
                    y2: 1.0,
                    color: Color::Cyan,
                });
            });
        f.render_widget(canvas, *chunk);
    }
}
