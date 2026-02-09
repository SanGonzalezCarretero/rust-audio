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
    let selected_track_idx = match app.screen {
        Screen::Daw { selected_track } => selected_track,
        _ => 0,
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

        let title = layout_config::format_lane_title(i + 1, track.volume, status);

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

        let waveform = track.waveform();
        let waveform_len = waveform.as_ref().map(|w| w.len()).unwrap_or(0);

        // Calculate furthest_end based on track state
        let furthest_end = if is_recording {
            // Each waveform point = exactly RECORDING_WAVEFORM_CHUNK_SIZE samples
            rec_start_pos + waveform_len as u64 * crate::track::RECORDING_WAVEFORM_CHUNK_SIZE as u64
        } else {
            track.clips_end()
        };

        let timeline_samples = sample_rate as u64 * layout_config::TIMELINE_SECONDS;

        let canvas = Canvas::default()
            .block(block)
            .x_bounds([0.0, timeline_samples as f64])
            .y_bounds([-1.0, 1.0])
            .paint(move |ctx| {
                // Center line (timeline axis)
                ctx.draw(&Line {
                    x1: 0.0,
                    y1: 0.0,
                    x2: timeline_samples as f64,
                    y2: 0.0,
                    color: Color::DarkGray,
                });

                // Draw waveform if present
                if let Some(ref waveform) = waveform {
                    let (origin, samples_per_point) = if is_recording {
                        let chunk = crate::track::RECORDING_WAVEFORM_CHUNK_SIZE as f64;
                        (rec_start_pos as f64, chunk)
                    } else {
                        let spp = if waveform_len > 0 {
                            furthest_end as f64 / waveform_len as f64
                        } else {
                            1.0
                        };
                        (0.0, spp)
                    };

                    for (j, &(min, max)) in waveform.iter().enumerate() {
                        let x = origin + j as f64 * samples_per_point;
                        let y_min = (min * layout_config::WAVEFORM_SENSITIVITY).clamp(-1.0, 1.0);
                        let y_max = (max * layout_config::WAVEFORM_SENSITIVITY).clamp(-1.0, 1.0);

                        ctx.draw(&Line {
                            x1: x,
                            y1: y_min,
                            x2: x,
                            y2: y_max,
                            color: layout_config::DEFAULT_BORDER,
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
