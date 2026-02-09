use super::{
    err_fn, update_monitor_buffer, Track, AUDIO_BUFFER_SIZE, PREFILL_BUFFER_COUNT,
    RING_BUFFER_MULTIPLIER,
};
use crate::audio_engine::AudioEngine;
use cpal::traits::{DeviceTrait, StreamTrait};
use cpal::BufferSize;
use ringbuf::{
    traits::{Consumer, Producer, Split},
    HeapRb,
};
use std::sync::Arc;

impl Track {
    pub fn start_monitoring(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if !self.is_armed() {
            return Err("Track must be armed to monitor".into());
        }

        let input_device = AudioEngine::get_input_device()?;

        let output_device = AudioEngine::get_output_device()?;

        let mut config = input_device.config.clone();
        config.buffer_size = BufferSize::Fixed(AUDIO_BUFFER_SIZE);

        let buffer_samples = AUDIO_BUFFER_SIZE as usize * config.channels as usize;

        let ring_size = buffer_samples * RING_BUFFER_MULTIPLIER;
        let ring = HeapRb::<f32>::new(ring_size);
        let (mut producer, mut consumer) = ring.split();

        for _ in 0..(buffer_samples * PREFILL_BUFFER_COUNT) {
            let _ = producer.try_push(0.0);
        }

        let monitor_buffer = Arc::clone(&self.monitor_buffer);

        let input_data_fn = move |data: &[f32], _: &cpal::InputCallbackInfo| {
            update_monitor_buffer(&monitor_buffer, data);
            let _ = producer.push_slice(data);
        };

        let output_data_fn = move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
            let popped = consumer.pop_slice(data);

            for sample in &mut data[popped..] {
                *sample = 0.0;
            }

            if config.channels == 2 && popped > 0 {
                for i in (0..popped).step_by(2) {
                    if i + 1 < data.len() {
                        data[i + 1] = data[i];
                    }
                }
            }
        };

        let input_stream =
            input_device
                .device
                .build_input_stream(&config, input_data_fn, err_fn, None)?;

        let output_stream =
            output_device
                .device
                .build_output_stream(&config, output_data_fn, err_fn, None)?;

        input_stream.play()?;
        output_stream.play()?;

        self.input_stream = Some(input_stream);
        self.output_stream = Some(output_stream);
        self.monitoring = true;

        Ok(())
    }

    pub fn stop_monitoring(&mut self) {
        self.input_stream = None;
        self.output_stream = None;
        self.monitoring = false;
    }
}
