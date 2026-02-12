use crate::audio_engine::AudioEngine;
use crate::master_bus::{MasterBus, MasterBusConfig};
use crate::track::{Track, TrackState};
use cpal::traits::{DeviceTrait, StreamTrait};
use cpal::{BufferSize, Stream};
use ringbuf::{
    traits::{Producer, Split},
    HeapProd, HeapRb,
};
use std::borrow::Cow;

const INPUT_BUFFER_FRAMES: u32 = 32;
const MONITOR_RING_BUFFER_SIZE: usize = 128;

/// Extract a single channel from interleaved audio data.
/// Returns the original data when no channel is selected (all channels),
/// or an owned mono extraction for a specific channel.
fn extract_channel<'a>(data: &'a [f32], channels: usize, input_channel: Option<u16>) -> Cow<'a, [f32]> {
    match input_channel {
        None => Cow::Borrowed(data),
        Some(sel) => {
            let sel = sel as usize;
            let num_frames = data.len() / channels;
            Cow::Owned((0..num_frames).map(|f| data[f * channels + sel]).collect())
        }
    }
}

/// Compute a mono sample for one frame based on channel selection.
/// Returns the selected channel's sample, or the average of all channels.
fn monitor_frame_sample(data: &[f32], frame: usize, channels: usize, input_channel: Option<u16>) -> f32 {
    match input_channel {
        Some(sel) => data[frame * channels + sel as usize],
        None => {
            let start = frame * channels;
            data[start..start + channels].iter().sum::<f32>() / channels as f32
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportState {
    Stopped,
    Playing,
    Recording,
}

pub struct Transport {
    pub state: TransportState,
    pub playhead_position: u64,
    playback_origin: u64,
}

impl Default for Transport {
    fn default() -> Self {
        Transport {
            state: TransportState::Stopped,
            playhead_position: 0,
            playback_origin: 0,
        }
    }
}

impl Transport {
    pub fn play(&mut self) {
        self.state = TransportState::Playing;
        self.playback_origin = self.playhead_position;
    }

    pub fn record(&mut self) {
        self.state = TransportState::Recording;
        self.playback_origin = self.playhead_position;
    }

    pub fn stop(&mut self) {
        self.state = TransportState::Stopped;
    }

    pub fn is_playing(&self) -> bool {
        matches!(
            self.state,
            TransportState::Playing | TransportState::Recording
        )
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

    pub fn reset_playhead(&mut self) {
        self.playhead_position = 0;
    }

    pub fn playhead_seconds(&self, sample_rate: u32) -> f64 {
        self.playhead_position as f64 / sample_rate as f64
    }

    pub fn advance_playhead_from_master(&mut self, samples_consumed: u64) {
        self.playhead_position = self.playback_origin + samples_consumed;
    }
}

pub struct Session {
    pub name: String,
    pub tracks: Vec<Track>,
    pub sample_rate: u32,
    pub transport: Transport,
    master_bus: MasterBus,
    shared_input_stream: Option<Stream>,
}

impl Session {
    pub fn new(name: String, sample_rate: u32) -> Self {
        Session {
            name,
            tracks: Vec::new(),
            sample_rate,
            transport: Transport::default(),
            master_bus: MasterBus::default(),
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

    // --- Playback ---

    pub fn toggle_playback(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.transport.is_playing() {
            self.stop_playback()
        } else {
            self.start_playback()
        }
    }

    pub fn start_playback(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let playhead_pos = self.transport.playhead_position;

        // Pre-render all tracks from playhead position and mix them into one mono buffer
        // This happens BEFORE playback starts (not real-time)
        let master_buffer = self.render_master_buffer(playhead_pos);
        if master_buffer.is_empty() {
            return Ok(());
        }

        let monitor_consumer = self.build_monitor_consumer();

        self.master_bus.start(MasterBusConfig {
            playback_samples: Some(master_buffer),
            monitor_consumer,
            sample_rate: self.sample_rate,
            low_latency: false,
        })?;

        self.transport.play();
        Ok(())
    }

    pub fn stop_playback(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.master_bus.stop();
        self.transport.stop();

        self.refresh_monitoring();

        Ok(())
    }

    pub fn check_playback_status(&mut self) {
        if self.transport.is_playing() {
            let frames_consumed = self.master_bus.frames_consumed();
            self.transport.advance_playhead_from_master(frames_consumed);

            if self.master_bus.is_finished() {
                self.master_bus.stop();
                self.transport.stop();
                self.refresh_monitoring();
            }
        }
    }

    // --- Recording ---

    pub fn start_recording(&mut self) -> Result<usize, Box<dyn std::error::Error>> {
        let armed_count = self.tracks.iter().filter(|t| t.is_armed()).count();
        if armed_count == 0 {
            return Ok(0);
        }

        self.master_bus.stop();
        self.shared_input_stream = None;

        let input_device = AudioEngine::get_input_device()?;
        let mut config = input_device.config.clone();
        if config.sample_rate.0 != self.sample_rate {
            return Err(format!(
                "Input device sample rate ({}Hz) does not match session sample rate ({}Hz). Change your input device or start a new session.",
                config.sample_rate.0, self.sample_rate
            ).into());
        }
        let channels = config.channels;
        config.buffer_size = BufferSize::Fixed(INPUT_BUFFER_FRAMES);

        let playhead_pos = self.transport.playhead_position;

        for track in &mut self.tracks {
            if track.is_armed() {
                let rec_channels = if track.input_channel.is_some() { 1 } else { channels };
                track.prepare_recording(playhead_pos, self.sample_rate, rec_channels);
            }
        }

        let mut rec_producers: Vec<HeapProd<f32>> = Vec::new();
        let mut input_channels: Vec<Option<u16>> = Vec::new();

        for track in &mut self.tracks {
            if let Some(prod) = track.take_recording_producer() {
                rec_producers.push(prod);
                input_channels.push(track.input_channel);
            }
        }

        let monitor_ring = HeapRb::<f32>::new(MONITOR_RING_BUFFER_SIZE);
        let (mut monitor_producer, monitor_consumer) = monitor_ring.split();

        let input_data_fn = move |data: &[f32], _: &cpal::InputCallbackInfo| {
            let ch = channels as usize;
            let num_frames = data.len() / ch;

            // Route input to each track's recording buffer
            for (i, prod) in rec_producers.iter_mut().enumerate() {
                let samples = extract_channel(data, ch, input_channels[i]);
                prod.push_slice(&samples);
            }

            // Mix selected channels for headphone output
            for frame in 0..num_frames {
                let mix: f32 = input_channels.iter()
                    .map(|&ic| monitor_frame_sample(data, frame, ch, ic))
                    .sum::<f32>() / input_channels.len() as f32;
                let _ = monitor_producer.try_push(mix);
            }
        };

        let playback_buffer = self.render_overdub_buffer(playhead_pos);

        let input_stream = input_device.device.build_input_stream(
            &config,
            input_data_fn,
            |err| eprintln!("Shared input stream error: {err}"),
            None,
        )?;

        self.master_bus.start(MasterBusConfig {
            playback_samples: if playback_buffer.is_empty() {
                None
            } else {
                Some(playback_buffer)
            },
            monitor_consumer: Some(monitor_consumer),
            sample_rate: self.sample_rate,
            low_latency: true,
        })?;

        // Start input AFTER master bus so both streams begin together
        input_stream.play()?;
        self.shared_input_stream = Some(input_stream);

        self.transport.record();

        Ok(armed_count)
    }

    pub fn stop_all_recording(&mut self) {
        // Drop shared input stream FIRST to stop audio capture
        self.shared_input_stream = None;
        self.master_bus.stop();

        // Finalize all recording tracks (save buffers to track data)
        for track in &mut self.tracks {
            if track.state == TrackState::Recording {
                let _ = track.stop_recording();
            }
        }
        self.transport.stop();

        // Restart monitoring if any tracks are still armed
        self.refresh_monitoring();
    }

    // --- Monitoring ---

    pub fn start_monitoring(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let has_armed = self.tracks.iter().any(|t| t.is_armed() && t.monitoring);
        if !has_armed {
            return Ok(());
        }

        if self.transport.is_playing() {
            return Ok(());
        }

        let input_device = AudioEngine::get_input_device()?;
        let mut config = input_device.config.clone();
        if config.sample_rate.0 != self.sample_rate {
            return Err(format!(
                "Input device sample rate ({}Hz) does not match session sample rate ({}Hz). Change your input device or start a new session.",
                config.sample_rate.0, self.sample_rate
            ).into());
        }
        let channels = config.channels;
        config.buffer_size = BufferSize::Fixed(INPUT_BUFFER_FRAMES);

        let input_channels: Vec<Option<u16>> = self.tracks.iter()
            .filter(|t| t.is_armed() && t.monitoring)
            .map(|t| t.input_channel)
            .collect();

        let monitor_ring = HeapRb::<f32>::new(MONITOR_RING_BUFFER_SIZE);
        let (mut monitor_producer, monitor_consumer) = monitor_ring.split();

        let input_data_fn = move |data: &[f32], _: &cpal::InputCallbackInfo| {
            let ch = channels as usize;
            let num_frames = data.len() / ch;

            // Mix selected channels for headphone output
            for frame in 0..num_frames {
                let mix: f32 = input_channels.iter()
                    .map(|&ic| monitor_frame_sample(data, frame, ch, ic))
                    .sum::<f32>() / input_channels.len() as f32;
                let _ = monitor_producer.try_push(mix);
            }
        };

        let input_stream = input_device.device.build_input_stream(
            &config,
            input_data_fn,
            |err| eprintln!("Monitor input stream error: {err}"),
            None,
        )?;

        input_stream.play()?;
        self.shared_input_stream = Some(input_stream);

        self.master_bus.start(MasterBusConfig {
            playback_samples: None,
            monitor_consumer: Some(monitor_consumer),
            sample_rate: self.sample_rate,
            low_latency: true,
        })?;

        Ok(())
    }

    pub fn stop_monitoring(&mut self) {
        if !self.transport.is_playing() {
            self.shared_input_stream = None;
            self.master_bus.stop();
        }
    }

    /// Restart monitoring if any tracks are armed and monitoring is enabled.
    fn refresh_monitoring(&mut self) {
        let has_monitoring = self.tracks.iter().any(|t| t.is_armed() && t.monitoring);
        if has_monitoring {
            let _ = self.start_monitoring();
        }
    }

    // --- Track management ---

    pub fn remove_track(&mut self, index: usize) -> Result<(), Box<dyn std::error::Error>> {
        const MIN_TRACKS: usize = 1;
        if self.tracks.len() <= MIN_TRACKS {
            return Err("Cannot remove the last track".into());
        }

        if index >= self.tracks.len() {
            return Err("Track index out of bounds".into());
        }

        self.tracks[index].cleanup();
        self.tracks.remove(index);

        Ok(())
    }

    // --- FX Chain management ---

    pub fn add_effect_to_track(
        &mut self,
        track_idx: usize,
        effect: crate::effects::EffectInstance,
    ) -> Result<(), String> {
        let track = self
            .tracks
            .get_mut(track_idx)
            .ok_or_else(|| "Track index out of bounds".to_string())?;
        track.fx_chain.push(effect);
        Ok(())
    }

    pub fn remove_effect_from_track(
        &mut self,
        track_idx: usize,
        effect_idx: usize,
    ) -> Result<(), String> {
        let track = self
            .tracks
            .get_mut(track_idx)
            .ok_or_else(|| "Track index out of bounds".to_string())?;
        if effect_idx >= track.fx_chain.len() {
            return Err("Effect index out of bounds".to_string());
        }
        track.fx_chain.remove(effect_idx);
        Ok(())
    }

    pub fn update_effect_param(
        &mut self,
        track_idx: usize,
        effect_idx: usize,
        param: &str,
        value: &str,
    ) -> Result<(), String> {
        let track = self
            .tracks
            .get_mut(track_idx)
            .ok_or_else(|| "Track index out of bounds".to_string())?;
        if effect_idx >= track.fx_chain.len() {
            return Err("Effect index out of bounds".to_string());
        }
        let updated = track.fx_chain[effect_idx].update_parameter(param, value)?;
        track.fx_chain[effect_idx] = updated;
        Ok(())
    }

    /// Render the entire master mix from the start as f32 samples.
    pub fn render_full_mix(&self) -> Vec<f32> {
        self.render_master_buffer(0)
    }

    // --- Internal helpers ---

    /// Pre-render all non-muted tracks and sum into a mono buffer.
    fn render_master_buffer(&self, playhead_pos: u64) -> Vec<f32> {
        self.mix_tracks(playhead_pos, |_| true)
    }

    fn render_overdub_buffer(&self, playhead_pos: u64) -> Vec<f32> {
        self.mix_tracks(playhead_pos, |_| true)
    }

    /// Sum rendered samples from tracks matching the predicate.
    /// This is where the actual mixing happens - each track renders its audio
    /// from the playhead position, then we sum them all together.
    fn mix_tracks(&self, playhead_pos: u64, include: impl Fn(&Track) -> bool) -> Vec<f32> {
        let mut master: Vec<f32> = Vec::new();

        for track in &self.tracks {
            if !include(track) {
                continue;
            }
            // Ask the track to render its audio from this position
            let rendered = track.render(playhead_pos, self.sample_rate);
            if rendered.is_empty() {
                continue;
            }
            // Mix by summing samples
            if master.is_empty() {
                master = rendered;
            } else {
                // Extend master buffer if this track is longer
                if rendered.len() > master.len() {
                    master.resize(rendered.len(), 0.0);
                }
                // Add this track's samples to the master mix
                for (i, &s) in rendered.iter().enumerate() {
                    master[i] += s;
                }
            }
        }

        master
    }

    /// Build a monitor consumer if any armed tracks have monitoring enabled.
    /// Returns None if no monitoring is active or during recording.
    fn build_monitor_consumer(&self) -> Option<ringbuf::HeapCons<f32>> {
        // During playback, we don't set up live monitoring from scratch.
        // Monitoring is handled by start_monitoring() / recording flow.
        None
    }
}
