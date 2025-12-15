use crate::audio_engine::AudioEngine;
use crate::device::{AudioDevice, DeviceProvider};
use crate::effects::EffectInstance;
use crate::ui::DebugLogger;
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

const LATENCY_MS: f32 = 10.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrackState {
    Idle,
    Armed,
    Recording,
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
    pub wav_data: Option<WavFile>,
    pub file_path: String,

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
}

impl Track {
    pub fn new(name: String) -> Self {
        Track {
            name,
            armed: false,
            monitoring: false,
            state: TrackState::Idle,
            fx_chain: vec![],
            monitor_buffer: Arc::new(Mutex::new(vec![0.0; MONITOR_BUFFER_SAMPLES])),
            wav_data: None,
            file_path: String::new(),
            volume: 1.0,
            muted: false,
            is_playing: Arc::new(AtomicBool::new(false)),
            input_stream: None,
            output_stream: None,
            playback_thread: None,
            recording_buffer: None,
            recording_channels: None,
            recording_sample_rate: None,
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

    pub fn start_recording(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if !self.armed {
            return Err("Track must be armed to record".into());
        }

        let input_device = AudioEngine::get_input_device()?;

        let mut config = input_device.config.clone();
        let sample_rate = config.sample_rate.0;
        let recording_buffer_size = (sample_rate as f32 * LATENCY_MS / 1000.0) as u32;

        config.buffer_size = BufferSize::Fixed(recording_buffer_size);

        let recorded_samples = Arc::new(RwLock::new(Vec::new()));
        let recorded_samples_clone = Arc::clone(&recorded_samples);

        let monitor_buffer = Arc::clone(&self.monitor_buffer);

        let input_data_fn = move |data: &[f32], _: &cpal::InputCallbackInfo| {
            if let Ok(mut buffer) = monitor_buffer.try_lock() {
                buffer.truncate(0);
                buffer.extend_from_slice(data);
            }

            // Write lock for adding new samples
            if let Ok(mut samples) = recorded_samples_clone.write() {
                samples.extend_from_slice(data);
            }
        };

        let input_stream =
            input_device
                .device
                .build_input_stream(&config, input_data_fn, err_fn, None)?;

        input_stream.play()?;

        self.input_stream = Some(input_stream);
        self.state = TrackState::Recording;

        self.recording_buffer = Some(recorded_samples);
        self.recording_channels = Some(config.channels);
        self.recording_sample_rate = Some(sample_rate);

        Ok(())
    }

    pub fn stop_recording(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.state != TrackState::Recording {
            return Err("Not currently recording".into());
        }

        self.input_stream = None;
        self.output_stream = None;

        if let Some(buffer) = self.recording_buffer.take() {
            if let Ok(samples) = buffer.read() {
                if !samples.is_empty() {
                    let channels = self.recording_channels.unwrap_or(1);
                    let sample_rate = self.recording_sample_rate.unwrap_or(48000);

                    let samples_f64: Vec<f64> = samples.iter().map(|&s| s as f64).collect();
                    let mut wav = WavFile::new(sample_rate, channels);
                    wav.from_f64_samples(&samples_f64);

                    self.wav_data = Some(wav);
                }
            }
        }

        self.recording_channels = None;
        self.recording_sample_rate = None;
        self.state = TrackState::Armed;
        Ok(())
    }

    /// Returns waveform data for UI visualization as (min, max) pairs.
    /// If actively recording, returns data from the growing recording buffer.
    /// Otherwise, returns data from the completed wav file.
    /// Each element is (min_peak, max_peak) for that chunk - bipolar waveform data.
    pub fn waveform(&self, debug_logger: Option<&DebugLogger>) -> Option<Vec<(f64, f64)>> {
        // Priority 1: If recording, show live buffer
        if self.state == TrackState::Recording {
            if let Some(ref buffer) = self.recording_buffer {
                if let Ok(samples) = buffer.try_read() {
                    if samples.is_empty() {
                        return None;
                    }

                    let sample_count = samples.len();
                    let sample_rate = self.recording_sample_rate.unwrap_or(48000);
                    let duration_secs = sample_count as f64 / sample_rate as f64;

                    if let Some(logger) = debug_logger {
                        let samples_f64: Vec<f64> = samples.iter().map(|&s| s as f64).collect();
                        let downsampled = downsample_bipolar(&samples_f64);

                        logger.log(format!(
                            "[LIVE GROWING] {} samples → {} points | {:.2}s recorded",
                            sample_count,
                            downsampled.len(),
                            duration_secs
                        ));

                        return Some(downsampled);
                    }

                    let samples_f64: Vec<f64> = samples.iter().map(|&s| s as f64).collect();
                    return Some(downsample_bipolar(&samples_f64));
                }
            }
        }

        // Priority 2: Show completed recording/loaded file
        let wav = self.wav_data.as_ref()?;
        let samples = wav.to_f64_samples();

        if samples.is_empty() {
            return None;
        }

        let sample_count = samples.len();
        let sample_rate = wav.header.sample_rate;
        let duration_secs = sample_count as f64 / sample_rate as f64;

        if let Some(logger) = debug_logger {
            let downsampled = downsample_bipolar(&samples);

            logger.log(format!(
                "[FILE STATIC] {} samples → {} points | {:.2}s total",
                sample_count,
                downsampled.len(),
                duration_secs
            ));

            return Some(downsampled);
        }

        Some(downsample_bipolar(&samples))
    }

    pub fn play(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Stop any existing playback first
        self.stop_playback();

        let wav = self.wav_data.as_ref().ok_or("No audio data to play")?;
        let samples: Vec<f64> = wav
            .to_f64_samples()
            .iter()
            .map(|&s| s * self.volume)
            .collect();

        let output_device = {
            let engine = AudioEngine::global();
            let engine = engine.lock().unwrap();
            engine.selected_output().map(|s| s.to_string())
        };

        let sample_rate = wav.header.sample_rate;
        let channels = wav.header.num_channels;

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
                        // Output silence when stopped/cancelled
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

                // Wait for playback to complete or be cancelled
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
}

/// Downsample audio data for bipolar UI rendering (Reaper-style).
/// Reduces the number of points to ~500 max by finding min and max in each chunk.
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

fn err_fn(err: cpal::StreamError) {
    eprintln!("Track stream error: {err}");
}
