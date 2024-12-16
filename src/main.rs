mod analysis;
mod effects;
mod wav;

use analysis::discrete_fourier_transform;
use wav::WavFile;

use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let bytes = fs::read("tone.wav")?;
    let mut wav_file = WavFile::from_bytes(bytes)?;

    // wav_file.apply_effects(vec![Effect::Tremolo, Effect::Delay { ms: 30, taps: 50 }])?;

    discrete_fourier_transform(&mut wav_file.audio_data);

    // let wav_bytes = wav_file.export_to_bytes();
    // fs::write("output.wav", wav_bytes)?;

    Ok(())
}
