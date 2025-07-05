use rand::Rng;

#[derive(Debug)]
pub enum Effect {
    AdjustVolume(f32),
    Reverse,
    Duplicate,
    RandomNoise,
    Delay { ms: usize, taps: usize },
    Tremolo,
    PitchOctaveUp,
    LargeReverb,
    TapeSaturation,
    Pan(char, u8),
}

impl Effect {
    pub fn apply(&self, samples: &mut Vec<f64>, sample_rate: u32) -> Result<(), &'static str> {
        match self {
            Effect::AdjustVolume(volume) => adjust_volume(samples, *volume),
            Effect::Reverse => reverse(samples),
            Effect::Duplicate => duplicate(samples),
            Effect::RandomNoise => random_noise(samples),
            Effect::Delay { ms, taps } => delay(samples, *ms, *taps, sample_rate),
            Effect::Tremolo => tremolo(samples, sample_rate),
            Effect::PitchOctaveUp => pitch_octave_up(samples),
            Effect::LargeReverb => large_reverb(samples, sample_rate),
            Effect::TapeSaturation => tape_saturation(samples),
            Effect::Pan(direction, amount) => pan(samples, *direction, *amount),
        }
    }
}

fn adjust_volume(samples: &mut Vec<f64>, volume: f32) -> Result<(), &'static str> {
    if volume > 2.0 || volume <= 0.0 {
        return Err("Not a valid volume value. Try again.");
    }

    for sample in samples.iter_mut() {
        *sample *= volume as f64;
        *sample = sample.clamp(-1.0, 1.0); // Prevent clipping
    }
    Ok(())
}

fn reverse(samples: &mut Vec<f64>) -> Result<(), &'static str> {
    samples.reverse();
    Ok(())
}

fn duplicate(samples: &mut Vec<f64>) -> Result<(), &'static str> {
    let original = samples.clone();
    samples.extend(original);
    Ok(())
}

fn random_noise(samples: &mut Vec<f64>) -> Result<(), &'static str> {
    let mut rng = rand::thread_rng();

    for sample in samples.iter_mut() {
        // Add random noise with small amplitude
        let noise = rng.gen_range(-0.1..0.1);
        *sample += noise;
        *sample = sample.clamp(-1.0, 1.0);
    }
    Ok(())
}

fn tape_saturation(samples: &mut Vec<f64>) -> Result<(), &'static str> {
    let drive = 2.0; // How hard we push the "tape"
    let mix = 0.3; // How much saturation to blend in (30%)
    let output_gain = 0.8; // Compensate for volume increase

    for sample in samples.iter_mut() {
        let clean = *sample;
        let driven = *sample * drive;
        let saturated = driven.tanh();

        // Mix clean and saturated signals
        *sample = (clean * (1.0 - mix) + saturated * mix) * output_gain;
    }

    Ok(())
}

fn large_reverb(samples: &mut Vec<f64>, sample_rate: u32) -> Result<(), &'static str> {
    let comb_delays = vec![
        (sample_rate as f64 * 0.0897) as usize, // ~90ms
        (sample_rate as f64 * 0.1171) as usize, // ~117ms
        (sample_rate as f64 * 0.1411) as usize, // ~141ms
        (sample_rate as f64 * 0.1637) as usize, // ~164ms
        (sample_rate as f64 * 0.1893) as usize, // ~189ms
        (sample_rate as f64 * 0.2159) as usize, // ~216ms
    ];

    let feedback = 0.75;
    let wet_level = 0.6;
    let mut reverb_output = vec![0.0; samples.len()];

    for delay in comb_delays {
        let mut delay_buffer = vec![0.0; delay];
        let mut buffer_index = 0;

        for (i, &input) in samples.iter().enumerate() {
            let delayed_sample = delay_buffer[buffer_index];
            let output = input + delayed_sample * feedback;

            delay_buffer[buffer_index] = output;
            buffer_index = (buffer_index + 1) % delay;

            reverb_output[i] += output;
        }
    }

    for (i, sample) in samples.iter_mut().enumerate() {
        *sample = *sample * (1.0 - wet_level) + reverb_output[i] * wet_level / 6.0;
        *sample = sample.clamp(-1.0, 1.0);
    }

    Ok(())
}

fn delay(
    samples: &mut Vec<f64>,
    ms: usize,
    taps: usize,
    sample_rate: u32,
) -> Result<(), &'static str> {
    let delay_samples = (ms * sample_rate as usize) / 1000;
    let original = samples.clone();

    // Extend the audio to accommodate the delay
    samples.resize(samples.len() + delay_samples * taps, 0.0);

    for tap_idx in 0..taps {
        let tap = tap_idx + 1;
        let delay_offset = delay_samples * tap;
        let feedback_gain = 0.5 / tap as f64;

        for (i, &original_sample) in original.iter().enumerate() {
            let delayed_index = i + delay_offset;
            if delayed_index < samples.len() {
                samples[delayed_index] += original_sample * feedback_gain;
                // Prevent clipping
                samples[delayed_index] = samples[delayed_index].clamp(-1.0, 1.0);
            }
        }
    }
    Ok(())
}

fn tremolo(samples: &mut Vec<f64>, sample_rate: u32) -> Result<(), &'static str> {
    const FREQUENCY: f64 = 8.0; // Tremolo frequency in Hz
    const DEPTH: f64 = 0.3; // Tremolo depth (0.0 to 1.0)

    for (i, sample) in samples.iter_mut().enumerate() {
        let time = i as f64 / sample_rate as f64;
        let angle = 2.0 * std::f64::consts::PI * FREQUENCY * time;
        let mod_factor = 1.0 - (DEPTH * (0.5 + 0.5 * angle.sin()));
        *sample *= mod_factor;
    }
    Ok(())
}

fn pitch_octave_up(samples: &mut Vec<f64>) -> Result<(), &'static str> {
    // Simple pitch shifting by taking every other sample
    let mut new_samples = Vec::with_capacity(samples.len() / 2);

    for i in (0..samples.len()).step_by(2) {
        new_samples.push(samples[i]);
    }

    *samples = new_samples;
    Ok(())
}

fn pan(samples: &mut Vec<f64>, direction: char, amount: u8) -> Result<(), &'static str> {
    if amount > 100 {
        return Err("Pan amount must be 0-100");
    }

    let gain = amount as f64 / 100.0;

    for chunk in samples.chunks_exact_mut(2) {
        match direction {
            'L' | 'l' => chunk[1] *= 1.0 - gain, // Reduce right channel
            'R' | 'r' => chunk[0] *= 1.0 - gain, // Reduce left channel
            _ => return Err("Direction must be 'L' or 'R'"),
        }
    }
    Ok(())
}
