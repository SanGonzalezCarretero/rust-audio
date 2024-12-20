mod effects;
mod processor;
mod wav;
use crate::effects::Effect;
use crate::processor::FrequencyEffect;
use crate::processor::Processor;
use rustfft::{num_complex::Complex64, FftPlanner};
use std::fs;
use wav::WavFile;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let bytes = fs::read("guitar.wav")?;
    let mut wav_file = WavFile::from_bytes(bytes)?;

    // The processor calls FFT and IFFT libraries in order to perform frequency manipulation
    let mut processor = Processor::new(44100);

    // We feed the samples to the FFT and it creates a spectrum.
    let samples = wav_file.to_f64_samples();

    // We can manipulate this spectrum and feed it back to the IFFT (inverse FFT) and get the samples
    let modified_samples =
        processor.apply_frequency_domain_effect(&samples, FrequencyEffect::PitchShift(100.0));

    // Update file with modified samples
    wav_file.from_f64_samples(&modified_samples);

    fs::write("output.wav", wav_file.export_to_bytes())?;
    Ok(())
}
