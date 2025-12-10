use crate::audio_engine::AudioEngine;
use crate::device::AudioDevice;
use crate::effects::EffectInstance;
use crate::wav::WavFile;
use cpal::traits::{DeviceTrait, StreamTrait};
use cpal::{BufferSize, Stream};
use ringbuf::{traits::{Consumer, Producer, Split}, HeapRb};
use std::sync::{Arc, Mutex};

// Audio configuration constants
const AUDIO_BUFFER_SIZE: u32 = 32;              // Frames per callback (~0.67ms @ 48kHz)
const RING_BUFFER_MULTIPLIER: usize = 2;        // Ring size = buffer_size * multiplier
const PREFILL_BUFFER_COUNT: usize = 0;          // Number of buffers to pre-fill with silence
const MONITOR_BUFFER_SAMPLES: usize = 4800;     // ~100ms @ 48kHz for UI visualization

const LATENCY_MS: f32 = 10.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrackState {
    Idle,
    Armed,
    Recording,
}

pub struct Track {
    pub name: String,
    pub armed: bool,
    pub monitoring: bool,
    pub state: TrackState,
    
    // FX chain
    pub fx_chain: Vec<EffectInstance>,
    
    // Monitoring buffer - for live display visualization
    pub monitor_buffer: Arc<Mutex<Vec<f32>>>,
    
    // Playback data (recorded or loaded)
    pub wav_data: Option<WavFile>,
    pub file_path: String,
    
    // Playback state
    pub volume: f64,
    pub muted: bool,
    pub is_playing: Arc<Mutex<bool>>,
    
    // Audio streams
    input_stream: Option<Stream>,
    output_stream: Option<Stream>,
    
    // Recording buffer
    recording_buffer: Option<Arc<Mutex<Vec<f32>>>>,
    recording_channels: Option<u16>,
    recording_sample_rate: Option<u32>,
}

impl Track {
    pub fn new(name: String) -> Self {
        Track {
            name,
            armed: false,
            monitoring: false,
            state: TrackState::Idle,
            fx_chain: vec![],
            monitor_buffer: Arc::new(Mutex::new(vec![0.0; MONITOR_BUFFER_SAMPLES])),
            wav_data: None,
            file_path: String::new(),
            volume: 1.0,
            muted: false,
            is_playing: Arc::new(Mutex::new(false)),
            input_stream: None,
            output_stream: None,
            recording_buffer: None,
            recording_channels: None,
            recording_sample_rate: None,
        }
    }
    
    pub fn arm(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.armed = true;
        self.state = TrackState::Armed;
        
        self.start_monitoring()?;
        
        Ok(())
    }

    pub fn disarm(&mut self) {
        self.armed = false;
        self.state = TrackState::Idle;
        self.stop_monitoring();
    }

    pub fn start_monitoring(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if !self.armed {
            return Err("Track must be armed to monitor".into());
        }
        
        let input_device = AudioEngine::get_input_device()?;
        
        let output_device = AudioDevice::default_output()?;

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
            if let Ok(mut buffer) = monitor_buffer.try_lock() {
                buffer.truncate(0);
                buffer.extend_from_slice(data);
            }
            
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
        
        let input_stream = input_device
            .device
            .build_input_stream(&config, input_data_fn, err_fn, None)?;
        
        let output_stream = output_device
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

    pub fn start_recording(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if !self.armed {
            return Err("Track must be armed to record".into());
        }
         
        let input_device = AudioEngine::get_input_device()?;
        
        let mut config = input_device.config.clone();
        let sample_rate = config.sample_rate.0;
        let recording_buffer_size = (sample_rate as f32 * LATENCY_MS / 1000.0) as u32;
    
        config.buffer_size = BufferSize::Fixed(recording_buffer_size);

        let recorded_samples = Arc::new(Mutex::new(Vec::new()));
        let recorded_samples_clone = Arc::clone(&recorded_samples);
        
        let monitor_buffer = Arc::clone(&self.monitor_buffer);
        
        let input_data_fn = move |data: &[f32], _: &cpal::InputCallbackInfo| {
            if let Ok(mut buffer) = monitor_buffer.try_lock() {
                buffer.truncate(0);
                buffer.extend_from_slice(data);
            }
            
            if let Ok(mut samples) = recorded_samples_clone.try_lock() {
                samples.extend_from_slice(data);
            }
        };
        
        let input_stream = input_device
            .device
            .build_input_stream(&config, input_data_fn, err_fn, None)?;
            
        input_stream.play()?;
        
        self.input_stream = Some(input_stream);
        self.state = TrackState::Recording;
        
        self.recording_buffer = Some(recorded_samples);
        self.recording_channels = Some(config.channels);
        self.recording_sample_rate = Some(sample_rate);
        
        Ok(())
    }

    pub fn stop_recording(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.state != TrackState::Recording {
            return Err("Not currently recording".into());
        }

        self.input_stream = None;
        self.output_stream = None;
        
        if let Some(buffer) = self.recording_buffer.take() {
            if let Ok(samples) = buffer.lock() {
                if !samples.is_empty() {
                    let channels = self.recording_channels.unwrap_or(1);
                    let sample_rate = self.recording_sample_rate.unwrap_or(48000);
                    
                    let samples_f64: Vec<f64> = samples.iter().map(|&s| s as f64).collect();
                    let mut wav = WavFile::new(sample_rate, channels as u16);
                    wav.from_f64_samples(&samples_f64);
                    
                    self.wav_data = Some(wav);
                }
            }
        }
        
        self.recording_channels = None;
        self.recording_sample_rate = None;
        self.state = TrackState::Armed;
        Ok(())
    }

    pub fn waveform(&self) -> Option<Vec<f64>> {
        let wav = self.wav_data.as_ref()?;
        let samples = wav.to_f64_samples();
        
        if samples.is_empty() {
            return None;
        }
        
        const MAX_POINTS: usize = 500;
        let chunk_size = (samples.len() / MAX_POINTS).max(1);
        
        Some(samples.chunks(chunk_size).map(|chunk| {
            chunk.iter().map(|s| s.abs()).fold(0.0, f64::max)
        }).collect())
    }

    pub fn play(&self) -> Result<(), Box<dyn std::error::Error>> {
        let wav = self.wav_data.as_ref().ok_or("No audio data to play")?;
        let samples: Vec<f64> = wav.to_f64_samples()
            .iter()
            .map(|&s| s * self.volume)
            .collect();
        
        let output_device = {
            let engine = AudioEngine::global();
            let engine = engine.lock().unwrap();
            engine.selected_output().map(|s| s.to_string())
        };
        
        AudioDevice::play_audio(
            samples,
            wav.header.sample_rate,
            wav.header.num_channels,
            output_device,
        );
        Ok(())
    }
}

fn err_fn(err: cpal::StreamError) {
    eprintln!("Track stream error: {err}");
}