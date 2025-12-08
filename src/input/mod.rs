use crate::device::AudioDevice;
use cpal::BufferSize;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use ringbuf::HeapRb;
use ringbuf::traits::{Consumer, Producer, Split};
use std::time::Duration;

const LATENCY_MS: f32 = 300.0;

pub fn record_input_device(
    duration_secs: u64,
    device_index: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    let host = cpal::default_host();

    let input_device = host.input_devices()?
        .find(|d| d.name().map(|n| n.contains("Scarlett")).unwrap_or(false))
        .or_else(|| host.default_input_device())
        .expect("No input device found");
    
    let output_device = host.output_devices()?
        .find(|d| d.name().map(|n| n.contains("Scarlett")).unwrap_or(false))
        .or_else(|| host.default_output_device())
        .expect("No output device found");

    let mut config: cpal::StreamConfig = input_device.default_input_config()?.into();
    config.buffer_size = BufferSize::Fixed(32);

    let latency_frames = (LATENCY_MS / 1_000.0) * config.sample_rate.0 as f32;
    let latency_samples = latency_frames as usize * config.channels as usize;

    let ring = HeapRb::<f32>::new(latency_samples * 2);
    let (mut producer, mut consumer) = ring.split();

    for _ in 0..latency_samples {
        producer.try_push(0.0).unwrap();
    }

     let input_data_fn = move |data: &[f32], _: &cpal::InputCallbackInfo| {
        let mut output_fell_behind = false;
        for &sample in data {
            if producer.try_push(sample).is_err() {
                output_fell_behind = true;
            }
        }
        if output_fell_behind {
            eprintln!("output stream fell behind: try increasing latency");
        }
    };

    let output_data_fn = move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
        let mut input_fell_behind = false;
        for sample in data {
            *sample = match consumer.try_pop() {
                Some(s) => s,
                None => {
                    input_fell_behind = true;
                    0.0
                }
            };
        }
        if input_fell_behind {
            eprintln!("input stream fell behind: try increasing latency");
        }
    };

    let input_stream = input_device.build_input_stream(&config, input_data_fn, err_fn, None)?;
    let output_stream = output_device.build_output_stream(&config, output_data_fn, err_fn, None)?;
    
    input_stream.play()?;
    output_stream.play()?;
    std::thread::sleep(Duration::from_secs(duration_secs));
    drop(input_stream);
    drop(output_stream);
    
    Ok(())
}

fn err_fn(err: cpal::StreamError) {
    eprintln!("Stream error: {err}");
}
