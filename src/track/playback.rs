use super::Track;
use crate::audio_engine::AudioEngine;
use crate::device::{AudioDevice, DeviceProvider};
use cpal::traits::{DeviceTrait, StreamTrait};
use std::sync::{atomic::Ordering, Arc};

impl Track {
    pub fn play_from(&mut self, playhead_position: u64) -> Result<(), Box<dyn std::error::Error>> {
        self.stop_playback();

        if self.clips.is_empty() {
            return Ok(());
        }

        // Use the clip's own sample rate for the audio stream to preserve correct pitch
        let sample_rate = self.clips[0].wav_data.header.sample_rate;
        let channels = self.clips[0].wav_data.header.num_channels;

        // Find the end of the furthest clip
        let end_sample = self
            .clips
            .iter()
            .map(|clip| clip.starts_at + clip.wav_data.to_f64_samples().len() as u64)
            .max()
            .unwrap_or(0);

        if playhead_position >= end_sample {
            return Ok(());
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
}
