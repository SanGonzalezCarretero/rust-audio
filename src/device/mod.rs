use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, Host, SampleRate, StreamConfig};
use std::sync::{Arc, Mutex};

// TODO: move this out of here
pub fn play_audio(samples: Vec<f64>, sample_rate: u32, channels: u16, playback_position: Arc<Mutex<f64>>) {
    std::thread::spawn(move || {
        let host = cpal::default_host();
        if let Some(device) = host.default_output_device() {
            let config = StreamConfig {
                channels,
                sample_rate: SampleRate(sample_rate),
                buffer_size: cpal::BufferSize::Default,
            };
            
            let samples = Arc::new(samples);
            let samples_clone = Arc::clone(&samples);
            let total_samples = samples.len();
            let position_update = Arc::clone(&playback_position);
            let mut sample_idx = 0;
            
            if let Ok(stream) = device.build_output_stream(
                &config,
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    for frame in data.iter_mut() {
                        *frame = if sample_idx < samples_clone.len() {
                            samples_clone[sample_idx] as f32
                        } else {
                            0.0
                        };
                        sample_idx += 1;
                    }
                    if let Ok(mut pos) = position_update.lock() {
                        *pos = sample_idx as f64 / total_samples as f64;
                    }
                },
                |err| eprintln!("Stream error: {}", err),
                None
            ) {
                let _ = stream.play();
                let duration = total_samples as f64 / sample_rate as f64;
                std::thread::sleep(std::time::Duration::from_secs_f64(duration));
            }
        }
    });
}

pub struct AudioDevice {
    pub device: Device,
    pub config: StreamConfig,
    pub sample_rate: u32,
    pub channels: u16,
}

impl AudioDevice {
    pub fn default_input() -> Result<Self, Box<dyn std::error::Error>> {
        let host: Host = cpal::default_host();
        let device: Device = host
            .default_input_device()
            .ok_or("No input device available")?;

        let config: StreamConfig = device.default_input_config()?.into();
        let SampleRate(sample_rate) = config.sample_rate;
        let channels = config.channels;

        Ok(Self {
            device,
            config,
            sample_rate,
            channels,
        })
    }

    pub fn default_output() -> Result<Self, Box<dyn std::error::Error>> {
        let host: Host = cpal::default_host();
        let device: Device = host
            .default_output_device()
            .ok_or("No output device available")?;

        let config: StreamConfig = device.default_output_config()?.into();
        let SampleRate(sample_rate) = config.sample_rate;
        let channels = config.channels;

        Ok(Self {
            device,
            config,
            sample_rate,
            channels
        })
    }
}
