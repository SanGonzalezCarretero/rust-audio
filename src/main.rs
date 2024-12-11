mod effects;
mod utils;

use effects::*;
use utils::wav::WavFile;

use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let bytes = fs::read("input_guitar.wav")?;
    let mut wav_file = WavFile::from_bytes(bytes)?;

    // delay(&mut wav_file.audio_data, 600)?;
    // tremolo(&mut wav_file.audio_data)?;
    wav_file.apply_effects(vec![Effect::LinearFadeIn(8001)])?;

    let wav_bytes = wav_file.export_to_bytes();
    fs::write("output.wav", wav_bytes)?;

    Ok(())
}
