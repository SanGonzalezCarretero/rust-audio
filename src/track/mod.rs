mod monitoring;
mod playback;
mod recording;

use crate::effects::EffectInstance;
use crate::wav::WavFile;
use ringbuf::HeapProd;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex, RwLock,
};
use std::time::{SystemTime, UNIX_EPOCH};

const MONITOR_BUFFER_SAMPLES: usize = 4800; // ~100ms @ 48kHz for UI visualization

pub const LATENCY_MS: f32 = 10.0;

// Fixed chunk size for recording waveform: each peak point = this many raw samples.
// ~20ms at 48kHz → stable left-to-right waveform growth during recording.
pub const RECORDING_WAVEFORM_CHUNK_SIZE: usize = 960;
const WAVEFORM_MAX_POINTS: usize = 500;

pub struct Clip {
    pub id: String,
    pub wav_data: WavFile,
    pub starts_at: u64, // frame position on the timeline
}

pub fn generate_clip_id(track_name: &str) -> String {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format!("{}-{}", track_name, ts)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrackState {
    Idle,
    Armed,
    Recording,
}

pub struct WaveformThread {
    pub waveform: Option<std::thread::JoinHandle<Vec<f32>>>,
    waveform_stop: Arc<AtomicBool>,
}

impl Default for WaveformThread {
    fn default() -> Self {
        Self {
            waveform: None,
            waveform_stop: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl WaveformThread {
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
    pub monitoring: bool,
    pub state: TrackState,

    // FX chain
    pub fx_chain: Vec<EffectInstance>,

    // Monitoring buffer - for live display visualization
    pub monitor_buffer: Arc<Mutex<Vec<f32>>>,

    // Playback data (recorded or loaded)
    pub clips: Vec<Clip>,
    pub recording_start_position: u64,

    // Playback state
    pub volume: f64,
    pub muted: bool,

    // Recording ring buffer producer (lock-free, written by audio callback)
    recording_producer: Option<HeapProd<f32>>,
    recording_channels: Option<u16>,
    recording_sample_rate: Option<u32>,

    // Thread handles for background processing
    waveform_thread: WaveformThread,

    // Live recording waveform (written by background thread, read by UI)
    waveform: Arc<RwLock<Vec<(f32, f32)>>>,
    // Cached waveform for existing clips (persists during recording)
    clips_waveform: Vec<(f32, f32)>,
}

impl Default for Track {
    fn default() -> Self {
        Track {
            name: "Untitled".to_string(),
            monitoring: false,
            state: TrackState::Idle,
            fx_chain: vec![],
            monitor_buffer: Arc::new(Mutex::new(vec![0.0; MONITOR_BUFFER_SAMPLES])),
            clips: Vec::new(),
            recording_start_position: 0,
            volume: 1.0,
            muted: false,
            recording_producer: None,
            recording_channels: None,
            recording_sample_rate: None,
            waveform_thread: WaveformThread::new(),
            waveform: Arc::new(RwLock::new(Vec::new())),
            clips_waveform: Vec::new(),
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

    pub fn is_armed(&self) -> bool {
        matches!(self.state, TrackState::Armed | TrackState::Recording)
    }

    pub fn arm(&mut self) {
        self.state = TrackState::Armed;
        self.start_monitoring();
    }

    pub fn disarm(&mut self) {
        self.state = TrackState::Idle;
        self.stop_monitoring();
    }

    /// Frame position of the end of the furthest clip.
    pub fn clips_end(&self) -> u64 {
        self.clips
            .iter()
            .map(|clip| clip.starts_at + clip.wav_data.frame_count() as u64)
            .max()
            .unwrap_or(0)
    }

    /// Mix all clips into a single mono buffer starting from `from_frame`.
    /// Multi-channel clips are downmixed to mono by averaging channels per frame.
    pub fn mix_clips(&self, from_frame: u64) -> (Vec<f32>, u64) {
        let end_frame = self.clips_end();
        if from_frame >= end_frame {
            return (Vec::new(), end_frame);
        }

        let buffer_len = (end_frame - from_frame) as usize;
        let mut mixed = vec![0.0f32; buffer_len];

        for clip in &self.clips {
            let clip_samples = clip.wav_data.to_f32_samples();
            let channels = clip.wav_data.header.num_channels as usize;
            let frame_count = clip.wav_data.frame_count();

            for frame in 0..frame_count {
                let absolute_pos = clip.starts_at + frame as u64;
                if absolute_pos >= from_frame && absolute_pos < end_frame {
                    let buf_idx = (absolute_pos - from_frame) as usize;
                    // Downmix: average all channels for this frame
                    let start = frame * channels;
                    let sum: f32 = clip_samples[start..start + channels].iter().sum();
                    mixed[buf_idx] += sum / channels as f32;
                }
            }
        }

        (mixed, end_frame)
    }

    pub fn cleanup(&mut self) {
        if self.state == TrackState::Recording {
            let _ = self.stop_recording();
        }

        if self.is_armed() {
            self.disarm();
        }
    }
}

/// Returns (min_peak, max_peak) tuples for drawing waveform from center axis.
/// When `exact_chunks` is true, only processes complete chunks (leftover samples ignored).
fn downsample_bipolar(samples: &[f32], chunk_size: usize, exact_chunks: bool) -> Vec<(f32, f32)> {
    let len = if exact_chunks {
        (samples.len() / chunk_size) * chunk_size
    } else {
        samples.len()
    };

    samples[..len]
        .chunks(chunk_size)
        .map(|chunk| {
            let min = chunk.iter().cloned().fold(f32::INFINITY, f32::min);
            let max = chunk.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
            (min, max)
        })
        .collect()
}

pub fn update_monitor_buffer(buffer: &Mutex<Vec<f32>>, data: &[f32]) {
    if let Ok(mut buf) = buffer.try_lock() {
        buf.clear();
        buf.extend_from_slice(data);
    }
}
