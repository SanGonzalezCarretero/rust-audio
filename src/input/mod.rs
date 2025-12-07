use crate::device::AudioDevice;
use crate::wav::WavFile;
use cpal::traits::{DeviceTrait, StreamTrait};
use std::sync::{Arc, Mutex};

pub fn record_input_device(
    duration_secs: u64,
    device_index: usize,
) -> Result<WavFile, Box<dyn std::error::Error>> {
    let audio_device = AudioDevice::input_by_index(device_index)?;

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
    let samples_f64: Vec<f64> = if audio_device.channels == 2 {
        samples.chunks(2)
            .flat_map(|chunk| {
                let left = chunk[0] as f64;
                vec![left, left]
            })
            .collect()
    } else {
        samples.iter().map(|&s| s as f64).collect()
    };

    let mut wav_file = WavFile::new(audio_device.sample_rate, audio_device.channels);
    wav_file.from_f64_samples(&samples_f64);

    Ok(wav_file)
}
