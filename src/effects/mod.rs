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
