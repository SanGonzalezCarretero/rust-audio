use crate::audio_engine::AudioEngine;
use crate::device::{AudioDevice, DeviceProvider};
use crate::effects::EffectInstance;
use crate::wav::WavFile;
use cpal::traits::{DeviceTrait, StreamTrait};
use cpal::{BufferSize, Stream};
use ringbuf::{
    traits::{Consumer, Producer, Split},
    HeapRb,
};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex, RwLock,
};

// Audio configuration constants
const AUDIO_BUFFER_SIZE: u32 = 32; // Frames per callback (~0.67ms @ 48kHz)
const RING_BUFFER_MULTIPLIER: usize = 2; // Ring size = buffer_size * multiplier
const PREFILL_BUFFER_COUNT: usize = 0; // Number of buffers to pre-fill with silence
const MONITOR_BUFFER_SAMPLES: usize = 4800; // ~100ms @ 48kHz for UI visualization

pub const LATENCY_MS: f32 = 10.0;

// Fixed chunk size for recording waveform: each peak point = this many raw samples.
// ~20ms at 48kHz → stable left-to-right waveform growth during recording.
pub const RECORDING_WAVEFORM_CHUNK_SIZE: usize = 960;

pub struct Clip {
    pub wav_data: WavFile,
    pub starts_at: u64, // sample position on the timeline
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrackState {
    Idle,
    Armed,
    Recording,
}

pub struct ThreadHandles {
    pub waveform: Option<std::thread::JoinHandle<()>>,
    waveform_stop: Arc<AtomicBool>,
}

impl Default for ThreadHandles {
    fn default() -> Self {
        Self {
            waveform: None,
            waveform_stop: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl ThreadHandles {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn stop_waveform(&self) {
        self.waveform_stop.store(true, Ordering::Relaxed);
    }

    pub fn reset_waveform_stop(&self) {
        self.waveform_stop.store(false, Ordering::Relaxed);
    }

    pub fn waveform_stop(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.waveform_stop)
    }
}

pub struct Track {
    pub name: String,
    pub armed: bool,
    pub monitoring: bool,
    pub state: TrackState,

    // FX chain
    pub fx_chain: Vec<EffectInstance>,

    // Monitoring buffer - for live display visualization
    pub monitor_buffer: Arc<Mutex<Vec<f32>>>,

    // Playback data (recorded or loaded)
    pub clips: Vec<Clip>,
    pub file_path: String,
    pub recording_start_position: u64,

    // Playback state
    pub volume: f64,
    pub muted: bool,
    pub is_playing: Arc<AtomicBool>,

    // Audio streams
    input_stream: Option<Stream>,
    output_stream: Option<Stream>,
    playback_thread: Option<std::thread::JoinHandle<()>>,

    // Recording buffer (RwLock allows concurrent reads while recording)
    recording_buffer: Option<Arc<RwLock<Vec<f32>>>>,
    recording_channels: Option<u16>,
    recording_sample_rate: Option<u32>,

    // Thread handles for background processing
    thread_handles: ThreadHandles,

    // Waveform result (written by background thread, read by UI)
    waveform: Arc<RwLock<Vec<(f64, f64)>>>,
}

impl Default for Track {
    fn default() -> Self {
        Track {
            name: "Untitled".to_string(),
            armed: false,
            monitoring: false,
            state: TrackState::Idle,
            fx_chain: vec![],
            monitor_buffer: Arc::new(Mutex::new(vec![0.0; MONITOR_BUFFER_SAMPLES])),
            clips: Vec::new(),
            file_path: String::new(),
            recording_start_position: 0,
            volume: 1.0,
            muted: false,
            is_playing: Arc::new(AtomicBool::new(false)),
            input_stream: None,
            output_stream: None,
            playback_thread: None,
            recording_buffer: None,
            recording_channels: None,
            recording_sample_rate: None,
            thread_handles: ThreadHandles::new(),
            waveform: Arc::new(RwLock::new(Vec::new())),
        }
    }
}

impl Track {
    pub fn new(name: String) -> Self {
        Track {
            name,
            ..Self::default()
        }
    }

    pub fn arm(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.armed = true;
        self.state = TrackState::Armed;

        self.start_monitoring()?;

        Ok(())
    }

    pub fn disarm(&mut self) {
        self.armed = false;
        self.state = TrackState::Idle;
        self.stop_monitoring();
    }

    pub fn start_monitoring(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if !self.armed {
            return Err("Track must be armed to monitor".into());
        }

        let input_device = AudioEngine::get_input_device()?;

        let output_device = AudioDevice::OUTPUT.default()?;

        let mut config = input_device.config.clone();
        config.buffer_size = BufferSize::Fixed(AUDIO_BUFFER_SIZE);

        let buffer_samples = AUDIO_BUFFER_SIZE as usize * config.channels as usize;

        let ring_size = buffer_samples * RING_BUFFER_MULTIPLIER;
        let ring = HeapRb::<f32>::new(ring_size);
        let (mut producer, mut consumer) = ring.split();

        for _ in 0..(buffer_samples * PREFILL_BUFFER_COUNT) {
            let _ = producer.try_push(0.0);
        }

        let monitor_buffer = Arc::clone(&self.monitor_buffer);

        let input_data_fn = move |data: &[f32], _: &cpal::InputCallbackInfo| {
            if let Ok(mut buffer) = monitor_buffer.try_lock() {
                buffer.truncate(0);
                buffer.extend_from_slice(data);
            }

            let _ = producer.push_slice(data);
        };

        let output_data_fn = move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
            let popped = consumer.pop_slice(data);

            for sample in &mut data[popped..] {
                *sample = 0.0;
            }

            if config.channels == 2 && popped > 0 {
                for i in (0..popped).step_by(2) {
                    if i + 1 < data.len() {
                        data[i + 1] = data[i];
                    }
                }
            }
        };

        let input_stream =
            input_device
                .device
                .build_input_stream(&config, input_data_fn, err_fn, None)?;

        let output_stream =
            output_device
                .device
                .build_output_stream(&config, output_data_fn, err_fn, None)?;

        input_stream.play()?;
        output_stream.play()?;

        self.input_stream = Some(input_stream);
        self.output_stream = Some(output_stream);
        self.monitoring = true;

        Ok(())
    }

    pub fn stop_monitoring(&mut self) {
        self.input_stream = None;
        self.output_stream = None;
        self.monitoring = false;
    }

    /// Prepare this track for recording: set up buffers and waveform thread.
    /// Does NOT open any audio device or stream — the Session owns the shared input stream.
    pub fn prepare_recording(&mut self, playhead_position: u64, sample_rate: u32, channels: u16) {
        self.stop_monitoring();

        self.recording_start_position = playhead_position;

        let recorded_samples = Arc::new(RwLock::new(Vec::new()));

        self.recording_buffer = Some(Arc::clone(&recorded_samples));
        self.recording_channels = Some(channels);
        self.recording_sample_rate = Some(sample_rate);

        // Reset waveform result for new recording
        if let Ok(mut waveform) = self.waveform.write() {
            waveform.clear();
        }

        self.state = TrackState::Recording;

        // Reset stop flag and prepare for thread spawn
        self.thread_handles.reset_waveform_stop();

        // Clone shared state for background thread
        let recording_buffer_clone = Arc::clone(&recorded_samples);
        let waveform_clone = Arc::clone(&self.waveform);
        let should_stop = self.thread_handles.waveform_stop();

        let handle = std::thread::spawn(move || {
            const UPDATE_INTERVAL_MS: u64 = 50; // Update ~20 times per second

            // Track how many raw samples have been fully processed into waveform points.
            // Only complete chunks of RECORDING_WAVEFORM_CHUNK_SIZE are consumed.
            let mut processed_samples: usize = 0;

            loop {
                if should_stop.load(Ordering::Relaxed) {
                    break;
                }

                if let Ok(samples) = recording_buffer_clone.try_read() {
                    let total = samples.len();
                    let unprocessed = total - processed_samples;
                    let complete_chunks = unprocessed / RECORDING_WAVEFORM_CHUNK_SIZE;

                    if complete_chunks > 0 {
                        let process_up_to =
                            processed_samples + complete_chunks * RECORDING_WAVEFORM_CHUNK_SIZE;
                        let new_samples = &samples[processed_samples..process_up_to];
                        let samples_f64: Vec<f64> =
                            new_samples.iter().map(|&s| s as f64).collect();

                        let new_peaks = downsample_bipolar_fixed(
                            &samples_f64,
                            RECORDING_WAVEFORM_CHUNK_SIZE,
                        );
                        processed_samples = process_up_to;

                        if let Ok(mut waveform) = waveform_clone.write() {
                            waveform.extend(new_peaks);
                        }
                    }
                }

                std::thread::sleep(std::time::Duration::from_millis(UPDATE_INTERVAL_MS));
            }
        });

        self.thread_handles.waveform = Some(handle);
    }

    /// Get a clone of the recording buffer Arc, if recording is prepared.
    pub fn recording_buffer_handle(&self) -> Option<Arc<RwLock<Vec<f32>>>> {
        self.recording_buffer.as_ref().map(Arc::clone)
    }

    /// Get a clone of the monitor buffer Arc.
    pub fn monitor_buffer_handle(&self) -> Arc<Mutex<Vec<f32>>> {
        Arc::clone(&self.monitor_buffer)
    }

    pub fn stop_recording(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.state != TrackState::Recording {
            return Err("Not currently recording".into());
        }

        // Signal waveform thread to stop
        self.thread_handles.stop_waveform();

        // Wait for waveform thread to finish (allows final processing)
        if let Some(handle) = self.thread_handles.waveform.take() {
            let _ = handle.join();
        }

        self.output_stream = None;

        if let Some(buffer) = self.recording_buffer.take() {
            if let Ok(samples) = buffer.read() {
                if !samples.is_empty() {
                    let channels = self.recording_channels.unwrap_or(1);
                    let sample_rate = self.recording_sample_rate.unwrap_or(48000);

                    let samples_f64: Vec<f64> = samples.iter().map(|&s| s as f64).collect();
                    let mut wav = WavFile::new(sample_rate, channels);
                    wav.from_f64_samples(&samples_f64);

                    self.clips.push(Clip {
                        wav_data: wav,
                        starts_at: self.recording_start_position,
                    });
                }
            }
        }

        self.recording_channels = None;
        self.recording_sample_rate = None;
        self.state = TrackState::Armed;
        Ok(())
    }

    pub fn waveform(&self) -> Option<Vec<(f64, f64)>> {
        // During recording, read from background thread's result
        if self.state == TrackState::Recording {
            if let Ok(waveform) = self.waveform.read() {
                if waveform.is_empty() {
                    return None;
                }
                return Some(waveform.clone());
            }
            return None;
        }

        if self.clips.is_empty() {
            return None;
        }

        // Composite all clips into a single sample buffer
        let end_sample = self.clips.iter().map(|clip| {
            clip.starts_at + clip.wav_data.to_f64_samples().len() as u64
        }).max().unwrap_or(0);

        if end_sample == 0 {
            return None;
        }

        let mut mixed = vec![0.0f64; end_sample as usize];

        for clip in &self.clips {
            let clip_samples = clip.wav_data.to_f64_samples();
            for (j, &sample) in clip_samples.iter().enumerate() {
                let pos = clip.starts_at as usize + j;
                if pos < mixed.len() {
                    mixed[pos] += sample;
                }
            }
        }

        Some(downsample_bipolar(&mixed))
    }

    pub fn play_from(&mut self, playhead_position: u64, _session_sample_rate: u32) -> Result<(), Box<dyn std::error::Error>> {
        // Stop any existing playback first
        self.stop_playback();

        if self.clips.is_empty() {
            return Ok(()); // Nothing to play — not an error
        }

        // Use the clip's own sample rate for the audio stream to preserve correct pitch
        let sample_rate = self.clips[0].wav_data.header.sample_rate;
        let channels = self.clips[0].wav_data.header.num_channels;

        // Find the end of the furthest clip
        let end_sample = self.clips.iter().map(|clip| {
            clip.starts_at + clip.wav_data.to_f64_samples().len() as u64
        }).max().unwrap_or(0);

        if playhead_position >= end_sample {
            return Ok(()); // Playhead is past all clips — nothing to play
        }

        // Build mixed buffer from playhead_position to end
        let buffer_len = (end_sample - playhead_position) as usize;
        let mut mixed = vec![0.0f64; buffer_len];

        for clip in &self.clips {
            let clip_samples = clip.wav_data.to_f64_samples();
            for (j, &sample) in clip_samples.iter().enumerate() {
                let absolute_pos = clip.starts_at + j as u64;
                if absolute_pos >= playhead_position && absolute_pos < end_sample {
                    let buf_idx = (absolute_pos - playhead_position) as usize;
                    mixed[buf_idx] += sample;
                }
            }
        }

        // Apply volume
        let volume = self.volume;
        let samples: Vec<f64> = mixed.iter().map(|&s| s * volume).collect();

        let output_device = {
            let engine = AudioEngine::global();
            let engine = engine.lock().unwrap();
            engine.selected_output().map(|s| s.to_string())
        };

        // Reset flag
        self.is_playing.store(true, Ordering::Relaxed);

        let is_playing_flag = Arc::clone(&self.is_playing);

        // Spawn thread for non-blocking playback
        let handle = std::thread::spawn(move || {
            use cpal::{SampleRate, StreamConfig};

            let device = if let Some(name) = output_device {
                if let Ok(audio_device) = AudioDevice::OUTPUT.by_name(&name) {
                    audio_device.device
                } else if let Ok(audio_device) = AudioDevice::OUTPUT.default() {
                    audio_device.device
                } else {
                    is_playing_flag.store(false, Ordering::Relaxed);
                    return;
                }
            } else if let Ok(audio_device) = AudioDevice::OUTPUT.default() {
                audio_device.device
            } else {
                is_playing_flag.store(false, Ordering::Relaxed);
                return;
            };

            let config = StreamConfig {
                channels,
                sample_rate: SampleRate(sample_rate),
                buffer_size: cpal::BufferSize::Default,
            };

            let samples = Arc::new(samples);
            let samples_clone = Arc::clone(&samples);
            let total_samples = samples.len();
            let mut sample_idx = 0;
            let is_playing_for_closure = Arc::clone(&is_playing_flag);
            let is_playing_for_loop = Arc::clone(&is_playing_flag);

            if let Ok(stream) = device.build_output_stream(
                &config,
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    if !is_playing_for_closure.load(Ordering::Relaxed) {
                        for frame in data.iter_mut() {
                            *frame = 0.0;
                        }
                        return;
                    }

                    for frame in data.iter_mut() {
                        *frame = if sample_idx < samples_clone.len() {
                            samples_clone[sample_idx] as f32
                        } else {
                            0.0
                        };
                        sample_idx += 1;
                    }
                },
                |err| eprintln!("Stream error: {}", err),
                None,
            ) {
                if let Err(e) = stream.play() {
                    eprintln!("Failed to play stream: {}", e);
                    is_playing_flag.store(false, Ordering::Relaxed);
                    return;
                }

                let duration = total_samples as f64 / sample_rate as f64;
                let start = std::time::Instant::now();
                while start.elapsed().as_secs_f64() < duration {
                    if !is_playing_for_loop.load(Ordering::Relaxed) {
                        break;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
            }

            is_playing_flag.store(false, Ordering::Relaxed);
        });

        self.playback_thread = Some(handle);
        Ok(())
    }

    pub fn stop_playback(&mut self) {
        // Set flag to stop playback
        self.is_playing.store(false, Ordering::Relaxed);

        // Wait for thread to finish (with timeout to avoid blocking forever)
        if let Some(handle) = self.playback_thread.take() {
            // Don't wait indefinitely - just drop the handle
            // The is_playing flag will stop audio production
            drop(handle);
        }
    }

    pub fn is_playback_finished(&self) -> bool {
        !self.is_playing.load(Ordering::Relaxed)
    }

    pub fn is_playing_track(&self) -> bool {
        self.is_playing.load(Ordering::Relaxed)
    }

    pub fn cleanup(&mut self) {
        // Stop playback
        self.stop_playback();

        // Stop recording if active
        if self.state == TrackState::Recording {
            let _ = self.stop_recording();
        }

        // Disarm if armed
        if self.armed {
            self.disarm();
        }

        // Stop monitoring (disarm already does this, but be explicit)
        self.stop_monitoring();
    }
}

/// Returns (min_peak, max_peak) tuples for drawing waveform from center axis.
fn downsample_bipolar(samples: &[f64]) -> Vec<(f64, f64)> {
    const MAX_POINTS: usize = 500;
    let chunk_size = (samples.len() / MAX_POINTS).max(1);

    samples
        .chunks(chunk_size)
        .map(|chunk| {
            let min = chunk.iter().cloned().fold(f64::INFINITY, f64::min);
            let max = chunk.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            (min, max)
        })
        .collect()
}

/// Fixed chunk_size version — each point always represents the same number of samples.
/// Only processes complete chunks; leftover samples are ignored (caller should track offset).
fn downsample_bipolar_fixed(samples: &[f64], chunk_size: usize) -> Vec<(f64, f64)> {
    samples
        .chunks_exact(chunk_size)
        .map(|chunk| {
            let min = chunk.iter().cloned().fold(f64::INFINITY, f64::min);
            let max = chunk.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            (min, max)
        })
        .collect()
}

fn err_fn(err: cpal::StreamError) {
    eprintln!("Track stream error: {err}");
}
