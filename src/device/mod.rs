use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, Host, SampleRate, StreamConfig};
use std::sync::{Arc, Mutex};

use crate::wav::WavFile;

pub struct AudioDevice {
    pub device: Device,
    pub config: StreamConfig,
    pub sample_rate: u32,
    pub channels: u16,
}

impl AudioDevice {
    fn get_host_and_device(is_input: bool) -> Result<(Host, Device), Box<dyn std::error::Error>> {
        let host = cpal::default_host();
        let device = if is_input {
            host.default_input_device()
                .ok_or("No input device available")?
        } else {
            host.default_output_device()
                .ok_or("No output device available")?
        };
        Ok((host, device))
    }

    pub fn default_input() -> Result<Self, Box<dyn std::error::Error>> {
        let (_, device) = Self::get_host_and_device(true)?;
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
        let (_, device) = Self::get_host_and_device(false)?;
        let config: StreamConfig = device.default_output_config()?.into();
        let SampleRate(sample_rate) = config.sample_rate;
        let channels = config.channels;

        Ok(Self {
            device,
            config,
            sample_rate,
            channels,
        })
    }

    pub fn play_audio(
        samples: Vec<f64>,
        sample_rate: u32,
        channels: u16,
        playback_position: Arc<Mutex<f64>>,
    ) {
        std::thread::spawn(move || {
            if let Ok((_, device)) = Self::get_host_and_device(false) {
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
                    None,
                ) {
                    let _ = stream.play();
                    let duration = total_samples as f64 / sample_rate as f64;
                    std::thread::sleep(std::time::Duration::from_secs_f64(duration));
                }
            }
        });
    }

    pub fn record_and_save_input_device(
        duration_secs: u64,
    ) -> Result<WavFile, Box<dyn std::error::Error>> {
        let audio_device = AudioDevice::default_input()?;

        let recorded_samples = Arc::new(Mutex::new(Vec::new()));
        let recorded_samples_clone = recorded_samples.clone();

        let stream = audio_device.device.build_input_stream(
            &audio_device.config,
            move |data: &[f32], _info: &cpal::InputCallbackInfo| {
                recorded_samples_clone
                    .lock()
                    .unwrap()
                    .extend_from_slice(data);
            },
            move |err| {
                eprintln!("Stream error: {}", err);
            },
            None,
        )?;

        std::thread::sleep(std::time::Duration::from_secs(duration_secs + 1));
        stream.play()?;

        let samples = recorded_samples.lock().unwrap();
        let samples_f64: Vec<f64> = samples.iter().map(|&s| s as f64).collect();

        let mut wav_file = WavFile::new(audio_device.sample_rate, audio_device.channels);
        wav_file.from_f64_samples(&samples_f64);

        Ok(wav_file)
    }
}
