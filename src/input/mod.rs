use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{Arc, Mutex};
use crate::{wav::WavFile};

pub fn record_input_device(duration_secs: u64) -> Result<WavFile, Box<dyn std::error::Error>> {
    let host = cpal::default_host();
    let input_device = host
        .default_input_device()
        .expect("No input device available");

    let config = input_device
        .default_input_config()
        .expect("Failed to get default input config");
    
    let sample_rate = config.sample_rate().0;
    let channels = config.channels() as u16;
    
    let recorded_samples = Arc::new(Mutex::new(Vec::new()));
    let recorded_samples_clone = recorded_samples.clone();

    let stream = input_device.build_input_stream(
        &config.into(),
        move |data: &[f32], info: &cpal::InputCallbackInfo| {
            recorded_samples_clone.lock().unwrap().extend_from_slice(data);
        },
        move |err| {
            eprintln!("Stream error: {}", err);
        },
        None,
    )?;

    stream.play()?;
    std::thread::sleep(std::time::Duration::from_secs(duration_secs + 1));
    drop(stream);
    
    let samples = recorded_samples.lock().unwrap();
    let samples_f64: Vec<f64> = samples.iter().map(|&s| s as f64).collect();
    
    let mut wav_file = WavFile::new(sample_rate, channels);
    wav_file.from_f64_samples(&samples_f64);
   
    Ok(wav_file)
}
