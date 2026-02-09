use rust_audio::wav::WavFile;
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let bytes = fs::read("guitar.wav")?;
    let mut wav_file = WavFile::from_bytes(bytes)?;

    let samples = wav_file.to_f64_samples();

    Ok(())
}
