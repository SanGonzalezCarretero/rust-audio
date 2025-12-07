use crate::device::AudioDevice;
use crate::wav::WavFile;
use cpal::traits::{DeviceTrait, StreamTrait};
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub fn record_input_device(
    duration_secs: u64,
    device_index: usize,
) -> Result<WavFile, Box<dyn std::error::Error>> {
    let input_device = AudioDevice::input_by_index(device_index)?;
    let output_device = AudioDevice::default_output()?;
    
    let buffer = Arc::new(Mutex::new(Vec::new()));
    let buffer_clone = buffer.clone();
    let channels = input_device.channels;
    
    let input_stream = build_input_stream(&input_device, buffer_clone)?;
    let output_stream = build_output_stream(&output_device, buffer.clone(), channels)?;
    
    input_stream.play()?;
    output_stream.play()?;
    std::thread::sleep(Duration::from_secs(duration_secs));
    drop(input_stream);
    drop(output_stream);
    
    let samples = buffer.lock().unwrap();
    let samples_f64 = convert_to_mono(&samples, channels);
    
    let mut wav_file = WavFile::new(input_device.sample_rate, channels);
    wav_file.from_f64_samples(&samples_f64);
    
    Ok(wav_file)
}

fn build_input_stream(
    device: &AudioDevice,
    buffer: Arc<Mutex<Vec<f32>>>,
) -> Result<cpal::Stream, Box<dyn std::error::Error>> {
    let stream = device.device.build_input_stream(
        &device.config,
        move |data: &[f32], _| {
            buffer.lock().unwrap().extend_from_slice(data);
        },
        |err| eprintln!("Input error: {}", err),
        None,
    )?;
    Ok(stream)
}

fn build_output_stream(
    device: &AudioDevice,
    buffer: Arc<Mutex<Vec<f32>>>,
    input_channels: u16,
) -> Result<cpal::Stream, Box<dyn std::error::Error>> {
    let mut read_idx = 0;
    let stream = device.device.build_output_stream(
        &device.config,
        move |data: &mut [f32], _| {
            let samples = buffer.lock().unwrap();
            for frame in data.iter_mut() {
                *frame = if input_channels == 2 && read_idx < samples.len() {
                    samples[(read_idx / 2) * 2]
                } else if read_idx < samples.len() {
                    samples[read_idx]
                } else {
                    0.0
                };
                read_idx += 1;
            }
        },
        |err| eprintln!("Output error: {}", err),
        None,
    )?;
    Ok(stream)
}

fn convert_to_mono(samples: &[f32], channels: u16) -> Vec<f64> {
    if channels == 2 {
        samples.chunks(2)
            .flat_map(|chunk| {
                let left = chunk[0] as f64;
                vec![left, left]
            })
            .collect()
    } else {
        samples.iter().map(|&s| s as f64).collect()
    }
}
