# Rust Audio

An audio processing library written in Rust.

## Supported Formats

- **WAV files only** (16-bit PCM currently supported)
- Place your audio files in the project root directory

## Quick Start

### Basic Audio Processing

```rust
mod effects;
mod processor;
mod wav;
use crate::processor::FrequencyEffect;
use crate::processor::Processor;
use std::fs;
use wav::WavFile;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Load your WAV file
    let bytes = fs::read("guitar.wav")?;
    let mut wav_file = WavFile::from_bytes(bytes)?;
    let mut samples = wav_file.to_f64_samples();

    // 2. Create processor with sample rate
    let mut processor = Processor::new(wav_file.header.sample_rate);

    // 3. Apply time domain effects
    processor.apply_time_domain_effect(&mut samples, effects::Effect::Tremolo)?;
    processor.apply_time_domain_effect(&mut samples, effects::Effect::LargeReverb)?;

    // 4. Apply frequency domain effects
    let filtered_samples =
        processor.apply_frequency_domain_effect(&samples, FrequencyEffect::LowPassFilter(1000.0));

    // 5. Save the result
    wav_file.from_f64_samples(&filtered_samples);
    fs::write("output.wav", wav_file.export_to_bytes())?;

    Ok(())
}
```

## Available Effects

### Time Domain Effects

```rust
// Volume and dynamics
Effect::AdjustVolume(0.5)              // 50% volume
Effect::TapeSaturation                 // Warm analog saturation

// Spatial effects
Effect::Pan('L', 75)                   // Pan 75% to left
Effect::Pan('R', 50)                   // Pan 50% to right

// Time-based effects
Effect::Reverse                        // Reverse audio
Effect::Delay { ms: 500, taps: 3 }    // Multi-tap delay
Effect::LargeReverb                    // Cathedral reverb
Effect::Tremolo                        // Amplitude modulation

// Creative effects
Effect::Duplicate                      // Double the audio length
Effect::RandomNoise                    // Add white noise
Effect::PitchOctaveUp                 // Octave up pitch shift
```

### Frequency Domain Effects

```rust
// Filters
FrequencyEffect::LowPassFilter(1000.0)   // Remove frequencies above 1kHz
FrequencyEffect::HighPassFilter(200.0)   // Remove frequencies below 200Hz
```

## Audio Visualization

Generate waveform graphs of your audio:

```bash
cargo run --bin graph input.wav output.wav combined.png
```

This creates a visual comparison of the before/after waveforms.

## Examples

### Simple Reverb Effect

```rust
let bytes = fs::read("dry_vocal.wav")?;
let mut wav_file = WavFile::from_bytes(bytes)?;
let mut samples = wav_file.to_f64_samples();

let processor = Processor::new(wav_file.header.sample_rate);
processor.apply_time_domain_effect(&mut samples, Effect::LargeReverb)?;

wav_file.from_f64_samples(&samples);
fs::write("wet_vocal.wav", wav_file.export_to_bytes())?;
```

### Guitar Effects Chain

```rust
let processor = Processor::new(44100);

// Effects chain
processor.apply_time_domain_effect(&mut samples, Effect::TapeSaturation)?;
processor.apply_time_domain_effect(&mut samples, Effect::Delay { ms: 250, taps: 2 })?;
processor.apply_time_domain_effect(&mut samples, Effect::Tremolo)?;
```

### Stereo Panning

```rust
// Pan hard left
processor.apply_time_domain_effect(&mut samples, Effect::Pan('L', 100))?;

// Pan slightly right
processor.apply_time_domain_effect(&mut samples, Effect::Pan('R', 30))?;
```

## Project Structure

```
src/
├── wav.rs          # WAV file parsing and generation
├── effects.rs      # Time domain effects
├── processor.rs    # FFT processing and frequency effects
└── bin/
    └── graph.rs    # Audio visualization tool
```
