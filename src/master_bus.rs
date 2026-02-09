use crate::audio_engine::AudioEngine;
use cpal::traits::{DeviceTrait, StreamTrait};
use cpal::{SampleRate, Stream, StreamConfig};
use ringbuf::traits::Consumer;
use ringbuf::HeapCons;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

const MONITOR_BUFFER_FRAMES: u32 = 32;

pub struct MasterBusConfig {
    pub playback_samples: Option<Vec<f32>>,
    pub monitor_consumer: Option<HeapCons<f32>>,
    pub sample_rate: u32,
    pub low_latency: bool,
}

pub struct MasterBus {
    stream: Option<Stream>,
    is_playing: Arc<AtomicBool>,
    frames_consumed: Arc<AtomicU64>,
    total_frames: usize,
}

impl Default for MasterBus {
    fn default() -> Self {
        MasterBus {
            stream: None,
            is_playing: Arc::new(AtomicBool::new(false)),
            frames_consumed: Arc::new(AtomicU64::new(0)),
            total_frames: 0,
        }
    }
}

impl MasterBus {
    pub fn start(&mut self, config: MasterBusConfig) -> Result<(), Box<dyn std::error::Error>> {
        self.stop();

        let output_device = AudioEngine::get_output_device()?;
        let channels = output_device.channels as usize;

        let buffer_size = if config.low_latency {
            cpal::BufferSize::Fixed(MONITOR_BUFFER_FRAMES)
        } else {
            cpal::BufferSize::Default
        };

        let stream_config = StreamConfig {
            channels: output_device.channels,
            sample_rate: SampleRate(config.sample_rate),
            buffer_size,
        };

        let playback_buf: Arc<Vec<f32>> = Arc::new(config.playback_samples.unwrap_or_default());
        let playback_len = playback_buf.len();
        self.total_frames = playback_len;

        let is_playing = Arc::clone(&self.is_playing);
        let frames_consumed = Arc::clone(&self.frames_consumed);

        is_playing.store(true, Ordering::Relaxed);
        frames_consumed.store(0, Ordering::Relaxed);

        let mut playback_pos: usize = 0;
        let mut monitor_cons = config.monitor_consumer;

        let stream = output_device.device.build_output_stream(
            &stream_config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                if !is_playing.load(Ordering::Relaxed) {
                    for sample in data.iter_mut() {
                        *sample = 0.0;
                    }
                    return;
                }

                let frames = data.len() / channels;

                for frame in 0..frames {
                    let playback_sample = if playback_pos < playback_len {
                        let s = playback_buf[playback_pos];
                        playback_pos += 1;
                        s
                    } else {
                        0.0
                    };

                    let monitor_sample = monitor_cons
                        .as_mut()
                        .and_then(|c| c.try_pop())
                        .unwrap_or(0.0);

                    let mixed = playback_sample + monitor_sample;
                    for ch in 0..channels {
                        data[frame * channels + ch] = mixed;
                    }
                }

                frames_consumed.fetch_add(frames as u64, Ordering::Relaxed);
            },
            |err| eprintln!("MasterBus stream error: {err}"),
            None,
        )?;

        stream.play()?;
        self.stream = Some(stream);

        Ok(())
    }

    pub fn stop(&mut self) {
        self.is_playing.store(false, Ordering::Relaxed);
        self.stream = None;
        self.frames_consumed.store(0, Ordering::Relaxed);
        self.total_frames = 0;
    }

    pub fn frames_consumed(&self) -> u64 {
        self.frames_consumed.load(Ordering::Relaxed)
    }

    pub fn is_finished(&self) -> bool {
        if self.total_frames == 0 {
            return false;
        }
        let consumed = self.frames_consumed.load(Ordering::Relaxed) as usize;
        consumed >= self.total_frames
    }

    pub fn is_active(&self) -> bool {
        self.stream.is_some()
    }
}
