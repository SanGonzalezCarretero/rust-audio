use rust_audio::processor::{FrequencyEffect, Processor};
use rust_audio::wav::WavFile;
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let bytes = fs::read("guitar.wav")?;
    let mut wav_file = WavFile::from_bytes(bytes)?;
    
    let samples = wav_file.to_f64_samples();

    let mut processor = Processor::new(wav_file.header.sample_rate);
    let filtered_samples =
        processor.apply_frequency_domain_effect(&samples, FrequencyEffect::LowPassFilter(1000.0));

    wav_file.from_f64_samples(&filtered_samples);
    fs::write("output.wav", wav_file.export_to_bytes())?;

    Ok(())
}
