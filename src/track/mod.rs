mod monitoring;
mod playback;
mod recording;

use crate::effects::EffectInstance;
use crate::wav::WavFile;
use cpal::Stream;
use ringbuf::HeapProd;
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
    pub waveform: Option<std::thread::JoinHandle<Vec<f32>>>,
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

    // Recording ring buffer producer (lock-free, written by audio callback)
    recording_producer: Option<HeapProd<f32>>,
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
            recording_producer: None,
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

    pub fn cleanup(&mut self) {
        self.stop_playback();

        if self.state == TrackState::Recording {
            let _ = self.stop_recording();
        }

        if self.armed {
            self.disarm();
        }

        self.stop_monitoring();
    }
}

/// Returns (min_peak, max_peak) tuples for drawing waveform from center axis.
/// When `exact_chunks` is true, only processes complete chunks (leftover samples ignored).
fn downsample_bipolar(samples: &[f64], chunk_size: usize, exact_chunks: bool) -> Vec<(f64, f64)> {
    let len = if exact_chunks {
        (samples.len() / chunk_size) * chunk_size
    } else {
        samples.len()
    };

    samples[..len]
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
