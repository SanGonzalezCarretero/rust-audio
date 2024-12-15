mod effects;
mod wav;

use effects::*;
use wav::WavFile;

use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let bytes = fs::read("input.wav")?;
    let mut wav_file = WavFile::from_bytes(bytes)?;

    // delay(&mut wav_file.audio_data, 600)?;
    // tremolo(&mut wav_file.audio_data)?;
    wav_file.apply_effects(vec![Effect::Tremolo, Effect::Delay { ms: 30, taps: 50 }])?;

    let wav_bytes = wav_file.export_to_bytes();
    fs::write("output.wav", wav_bytes)?;

    Ok(())
}
