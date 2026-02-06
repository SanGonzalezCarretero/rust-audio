use crate::audio_engine::AudioEngine;
use crate::track::{Track, LATENCY_MS};
use cpal::traits::{DeviceTrait, StreamTrait};
use cpal::{BufferSize, Stream};
use ringbuf::{traits::Producer, HeapProd};
use std::sync::{Arc, Mutex};
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportState {
    Stopped,
    Playing,
}

pub struct Transport {
    pub state: TransportState,
    pub playhead_position: u64,
    playback_origin: u64,
    playback_start_time: Option<Instant>,
}

impl Default for Transport {
    fn default() -> Self {
        Transport {
            state: TransportState::Stopped,
            playhead_position: 0,
            playback_origin: 0,
            playback_start_time: None,
        }
    }
}

impl Transport {
    pub fn play(&mut self) {
        self.state = TransportState::Playing;
        self.playback_origin = self.playhead_position;
        self.playback_start_time = Some(Instant::now());
    }

    pub fn stop(&mut self) {
        self.state = TransportState::Stopped;
        self.playback_start_time = None;
    }

    pub fn is_playing(&self) -> bool {
        self.state == TransportState::Playing
    }

    pub fn move_playhead(&mut self, delta_samples: i64) {
        if delta_samples < 0 {
            self.playhead_position = self
                .playhead_position
                .saturating_sub(delta_samples.unsigned_abs());
        } else {
            self.playhead_position = self.playhead_position.saturating_add(delta_samples as u64);
        }
    }

    pub fn playhead_seconds(&self, sample_rate: u32) -> f64 {
        self.playhead_position as f64 / sample_rate as f64
    }

    pub fn advance_playhead(&mut self, sample_rate: u32) {
        if let Some(start_time) = self.playback_start_time {
            let elapsed_secs = start_time.elapsed().as_secs_f64();
            let elapsed_samples = (elapsed_secs * sample_rate as f64) as u64;
            self.playhead_position = self.playback_origin + elapsed_samples;
        }
    }
}

pub struct Session {
    pub name: String,
    pub tracks: Vec<Track>,
    pub sample_rate: u32,
    pub transport: Transport,
    shared_input_stream: Option<Stream>,
}

impl Session {
    pub fn new(name: String, sample_rate: u32) -> Self {
        Session {
            name,
            tracks: Vec::new(),
            sample_rate,
            transport: Transport::default(),
            shared_input_stream: None,
        }
    }

    pub fn add_track(&mut self, name: String) -> Result<(), Box<dyn std::error::Error>> {
        const MAX_TRACKS: usize = 3;
        if self.tracks.len() >= MAX_TRACKS {
            return Err(format!("Maximum of {} tracks allowed", MAX_TRACKS).into());
        }
        self.tracks.push(Track::new(name));
        Ok(())
    }

    pub fn get_track_mut(&mut self, index: usize) -> Option<&mut Track> {
        self.tracks.get_mut(index)
    }

    pub fn get_track(&self, index: usize) -> Option<&Track> {
        self.tracks.get(index)
    }

    pub fn track_count(&self) -> usize {
        self.tracks.len()
    }

    pub fn toggle_playback(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.transport.is_playing() {
            self.stop_playback()
        } else {
            self.start_playback()
        }
    }

    pub fn start_playback(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let playhead_pos = self.transport.playhead_position;
        let sample_rate = self.sample_rate;

        // Start playback on all eligible tracks (don't let one failure stop others)
        let mut any_started = false;
        for track in &mut self.tracks {
            if !track.muted
                && !track.clips.is_empty()
                && track.play_from(playhead_pos, sample_rate).is_ok()
                && track.is_playing_track()
            {
                any_started = true;
            }
        }

        // Only set transport to Playing if at least one track started
        if any_started {
            self.transport.play();
        }

        Ok(())
    }

    pub fn stop_playback(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        for track in &mut self.tracks {
            track.stop_playback();
        }

        self.transport.stop();
        Ok(())
    }

    /// Start recording on all armed tracks using a single shared input stream.
    /// Returns the number of tracks that started recording.
    pub fn start_recording(&mut self) -> Result<usize, Box<dyn std::error::Error>> {
        let armed_count = self.tracks.iter().filter(|t| t.armed).count();
        if armed_count == 0 {
            return Ok(0);
        }

        // Get input device ONCE
        let input_device = AudioEngine::get_input_device()?;
        let mut config = input_device.config.clone();
        let sample_rate = config.sample_rate.0;
        let channels = config.channels;
        let recording_buffer_size = (sample_rate as f32 * LATENCY_MS / 1000.0) as u32;
        config.buffer_size = BufferSize::Fixed(recording_buffer_size);

        let playhead_pos = self.transport.playhead_position;

        // Prepare all armed tracks (sets up buffers, waveform threads, state)
        for track in &mut self.tracks {
            if track.armed {
                track.prepare_recording(playhead_pos, sample_rate, channels);
            }
        }

        // Take ring buffer producers and monitor handles from all recording tracks
        let mut rec_producers: Vec<HeapProd<f32>> = Vec::new();
        let mut mon_buffers: Vec<Arc<Mutex<Vec<f32>>>> = Vec::new();

        for track in &mut self.tracks {
            if let Some(prod) = track.take_recording_producer() {
                rec_producers.push(prod);
                mon_buffers.push(track.monitor_buffer_handle());
            }
        }

        // Build ONE input stream that fans data to ALL recording tracks (lock-free)
        let input_data_fn = move |data: &[f32], _: &cpal::InputCallbackInfo| {
            for mon in &mon_buffers {
                if let Ok(mut buffer) = mon.try_lock() {
                    buffer.truncate(0);
                    buffer.extend_from_slice(data);
                }
            }
            for prod in &mut rec_producers {
                prod.push_slice(data);
            }
        };

        let input_stream = input_device.device.build_input_stream(
            &config,
            input_data_fn,
            |err| eprintln!("Shared input stream error: {err}"),
            None,
        )?;

        input_stream.play()?;
        self.shared_input_stream = Some(input_stream);

        // Start overdub playback on non-recording tracks
        self.start_overdub_playback();

        Ok(armed_count)
    }

    /// Start playback on non-recording tracks so the user hears them while recording (overdub).
    pub fn start_overdub_playback(&mut self) {
        let playhead_pos = self.transport.playhead_position;
        let sample_rate = self.sample_rate;

        let mut any_started = false;
        for track in &mut self.tracks {
            if track.state != crate::track::TrackState::Recording
                && !track.muted
                && !track.clips.is_empty()
                && track.play_from(playhead_pos, sample_rate).is_ok()
                && track.is_playing_track()
            {
                any_started = true;
            }
        }

        if any_started {
            self.transport.play();
        }
    }

    /// Stop recording on all tracks that are recording, and stop playback.
    pub fn stop_all_recording(&mut self) {
        // Drop shared input stream FIRST to stop audio capture
        self.shared_input_stream = None;

        for track in &mut self.tracks {
            if track.state == crate::track::TrackState::Recording {
                let _ = track.stop_recording();
            }
            track.stop_playback();
        }
        self.transport.stop();
    }

    pub fn check_playback_status(&mut self) {
        if self.transport.is_playing() {
            // Advance playhead based on elapsed time
            self.transport.advance_playhead(self.sample_rate);

            let all_finished = self.tracks.iter().all(|track| {
                if track.clips.is_empty() || track.muted {
                    true
                } else {
                    track.is_playback_finished()
                }
            });

            if all_finished {
                self.transport.stop();
            }
        }
    }

    pub fn remove_track(&mut self, index: usize) -> Result<(), Box<dyn std::error::Error>> {
        const MIN_TRACKS: usize = 1;
        if self.tracks.len() <= MIN_TRACKS {
            return Err("Cannot remove the last track".into());
        }

        if index >= self.tracks.len() {
            return Err("Track index out of bounds".into());
        }

        // Cleanup the track before removal
        self.tracks[index].cleanup();

        // Remove the track
        self.tracks.remove(index);

        Ok(())
    }
}
