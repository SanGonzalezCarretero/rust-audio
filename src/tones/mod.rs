use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    SizedSample, I24,
};
use cpal::{FromSample, Sample};

pub enum Waveform {
    Sine,
    Square,
    Saw,
    Triangle,
}

pub struct Oscillator {
    pub sample_rate: f32,
    pub waveform: Waveform,
    pub current_sample_index: f32,
    pub frequency_hz: f32,
}

impl Oscillator {
    fn advance_sample(&mut self) {
        self.current_sample_index = (self.current_sample_index + 1.0) % self.sample_rate;
    }

    pub fn set_waveform(&mut self, waveform: Waveform) {
        self.waveform = waveform;
    }

    fn calculate_sine_output_from_freq(&self, freq: f32) -> f32 {
        let two_pi = 2.0 * std::f32::consts::PI;
        (self.current_sample_index * freq * two_pi / self.sample_rate).sin()
    }

    fn is_multiple_of_freq_above_nyquist(&self, multiple: f32) -> bool {
        self.frequency_hz * multiple > self.sample_rate / 2.0
    }

    fn sine_wave(&mut self) -> f32 {
        self.advance_sample();
        self.calculate_sine_output_from_freq(self.frequency_hz)
    }

    fn generative_waveform(&mut self, harmonic_index_increment: i32, gain_exponent: f32) -> f32 {
        self.advance_sample();
        let mut output = 0.0;
        let mut i = 1;
        while !self.is_multiple_of_freq_above_nyquist(i as f32) {
            let gain = 1.0 / (i as f32).powf(gain_exponent);
            output += gain * self.calculate_sine_output_from_freq(self.frequency_hz * i as f32);
            i += harmonic_index_increment;
        }
        output
    }

    fn square_wave(&mut self) -> f32 {
        self.generative_waveform(2, 1.0)
    }

    fn saw_wave(&mut self) -> f32 {
        self.generative_waveform(1, 1.0)
    }

    fn triangle_wave(&mut self) -> f32 {
        self.generative_waveform(2, 2.0)
    }

    pub fn tick(&mut self) -> f32 {
        match self.waveform {
            Waveform::Sine => self.sine_wave(),
            Waveform::Square => self.square_wave(),
            Waveform::Saw => self.saw_wave(),
            Waveform::Triangle => self.triangle_wave(),
        }
    }
}

pub fn stream_setup_for(frequency: f32, waveform_idx: usize, amplitude: f32) -> Result<cpal::Stream, anyhow::Error> {
    let (_host, device, config) = host_device_setup()?;

    match config.sample_format() {
        cpal::SampleFormat::I8 => make_stream::<i8>(&device, &config.into(), frequency, waveform_idx, amplitude),
        cpal::SampleFormat::I16 => make_stream::<i16>(&device, &config.into(), frequency, waveform_idx, amplitude),
        cpal::SampleFormat::I24 => make_stream::<I24>(&device, &config.into(), frequency, waveform_idx, amplitude),
        cpal::SampleFormat::I32 => make_stream::<i32>(&device, &config.into(), frequency, waveform_idx, amplitude),
        cpal::SampleFormat::I64 => make_stream::<i64>(&device, &config.into(), frequency, waveform_idx, amplitude),
        cpal::SampleFormat::U8 => make_stream::<u8>(&device, &config.into(), frequency, waveform_idx, amplitude),
        cpal::SampleFormat::U16 => make_stream::<u16>(&device, &config.into(), frequency, waveform_idx, amplitude),
        cpal::SampleFormat::U32 => make_stream::<u32>(&device, &config.into(), frequency, waveform_idx, amplitude),
        cpal::SampleFormat::U64 => make_stream::<u64>(&device, &config.into(), frequency, waveform_idx, amplitude),
        cpal::SampleFormat::F32 => make_stream::<f32>(&device, &config.into(), frequency, waveform_idx, amplitude),
        cpal::SampleFormat::F64 => make_stream::<f64>(&device, &config.into(), frequency, waveform_idx, amplitude),
        sample_format => Err(anyhow::Error::msg(format!(
            "Unsupported sample format '{sample_format}'"
        ))),
    }
}

pub fn host_device_setup(
) -> Result<(cpal::Host, cpal::Device, cpal::SupportedStreamConfig), anyhow::Error> {
    let host = cpal::default_host();

    let device = host
        .default_output_device()
        .ok_or_else(|| anyhow::Error::msg("Default output device is not available"))?;
    println!("Output device: {}", device.name()?);

    let config = device.default_output_config()?;
    println!("Default output config: {config:?}");

    Ok((host, device, config))
}

pub fn make_stream<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    frequency: f32,
    waveform_idx: usize,
    amplitude: f32,
) -> Result<cpal::Stream, anyhow::Error>
where
    T: SizedSample + FromSample<f32>,
{
    let num_channels = config.channels as usize;
    let waveform = match waveform_idx {
        0 => Waveform::Sine,
        1 => Waveform::Square,
        2 => Waveform::Saw,
        3 => Waveform::Triangle,
        _ => Waveform::Sine,
    };
    
    let mut oscillator = Oscillator {
        waveform,
        sample_rate: config.sample_rate.0 as f32,
        current_sample_index: 0.0,
        frequency_hz: frequency,
    };
    let err_fn = |err| eprintln!("Error building output sound stream: {err}");

    let stream = device.build_output_stream(
        config,
        move |output: &mut [T], _: &cpal::OutputCallbackInfo| {
            process_frame(output, &mut oscillator, num_channels, amplitude)
        },
        err_fn,
        None,
    )?;

    Ok(stream)
}

fn process_frame<SampleType>(
    output: &mut [SampleType],
    oscillator: &mut Oscillator,
    num_channels: usize,
    amplitude: f32,
) where
    SampleType: Sample + FromSample<f32>,
{
    for frame in output.chunks_mut(num_channels) {
        let value: SampleType = SampleType::from_sample(oscillator.tick() * amplitude);

        for sample in frame.iter_mut() {
            *sample = value;
        }
    }
}

