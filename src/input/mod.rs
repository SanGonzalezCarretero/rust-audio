use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::BufferSize;
use ringbuf::traits::{Consumer, Producer, Split};
use ringbuf::HeapRb;
use std::sync::Arc;
use std::time::Duration;

use crate::ui::DebugLogger;

const LATENCY_MS: f32 = 300.0;

pub fn record_input_device(
    duration_secs: u64,
    _device_index: usize,
    debug_logger: Arc<DebugLogger>,
) -> Result<(), Box<dyn std::error::Error>> {
    let host = cpal::default_host();

    let input_device = host
        .input_devices()?
        .find(|d| d.name().map(|n| n.contains("Scarlett")).unwrap_or(false))
        .or_else(|| host.default_input_device())
        .expect("No input device found");

    let output_device = host
        .output_devices()?
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

    let debug_logger_clone = debug_logger.clone();
    let input_data_fn = move |data: &[f32], _: &cpal::InputCallbackInfo| {
        let mut output_fell_behind = false;
        for &sample in data {
            if producer.try_push(sample).is_err() {
                output_fell_behind = true;
            }
        }
        if output_fell_behind {
            debug_logger_clone.log("output stream fell behind: try increasing latency".to_string());
        }
    };

    let debug_logger_clone2 = debug_logger.clone();
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
            debug_logger_clone2.log("input stream fell behind: try increasing latency".to_string());
        }
    };

    let debug_logger_clone3 = debug_logger.clone();
    let err_fn = move |err: cpal::StreamError| {
        debug_logger_clone3.log(format!("Stream error: {err}"));
    };

    let input_stream =
        input_device.build_input_stream(&config, input_data_fn, err_fn.clone(), None)?;
    let output_stream = output_device.build_output_stream(&config, output_data_fn, err_fn, None)?;

    input_stream.play()?;
    output_stream.play()?;
    std::thread::sleep(Duration::from_secs(duration_secs));
    drop(input_stream);
    drop(output_stream);

    Ok(())
}
