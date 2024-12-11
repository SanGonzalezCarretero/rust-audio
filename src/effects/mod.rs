use rand::Rng;

#[derive(Debug)]
pub enum Effect {
    AdjustVolume(f32),
    Reverse,
    Duplicate,
    RandomNoise,
    Delay { ms: usize, amount: usize },
    Tremolo,
    PitchOctaveUp,
}

impl Effect {
    pub fn apply(&self, audio_data: &mut Vec<u8>) -> Result<(), &'static str> {
        match self {
            Effect::AdjustVolume(volume) => adjust_volume(audio_data, *volume),
            Effect::Reverse => reverse(audio_data),
            Effect::Duplicate => duplicate(audio_data),
            Effect::RandomNoise => random_noise(audio_data),
            Effect::Delay { ms, amount } => delay(audio_data, *ms, *amount),
            Effect::Tremolo => tremolo(audio_data),
            Effect::PitchOctaveUp => pitch_octave_up(audio_data),
        }
    }
}

fn adjust_volume(audio_data: &mut Vec<u8>, volume: f32) -> Result<(), &'static str> {
    if volume > 2.0 || volume <= 0.0 {
        return Err("Not a valid volume value. Try again.");
    }

    for chunk in audio_data.chunks_exact_mut(2) {
        let sample = i16::from_le_bytes([chunk[0], chunk[1]]);
        let adjusted = (sample as f32 * volume) as i16;
        let new_bytes = adjusted.to_le_bytes();
        chunk[0] = new_bytes[0];
        chunk[1] = new_bytes[1];
    }
    Ok(())
}

fn reverse(audio_data: &mut Vec<u8>) -> Result<(), &'static str> {
    let mut rev_i = audio_data.len() - 2;
    let mut i = 0;

    while i < rev_i {
        let temp1 = audio_data[i];
        let temp2 = audio_data[i + 1];

        audio_data[i] = audio_data[rev_i];
        audio_data[i + 1] = audio_data[rev_i + 1];

        audio_data[rev_i] = temp1;
        audio_data[rev_i + 1] = temp2;

        rev_i -= 2;
        i += 2;
    }
    Ok(())
}

fn duplicate(audio_data: &mut Vec<u8>) -> Result<(), &'static str> {
    let audio_start = 44;
    let original = audio_data.clone();
    let audio_size = original.len() - audio_start;
    let new_total_size = (original.len() + audio_size - 8) as u32;

    audio_data[4..8].copy_from_slice(&new_total_size.to_le_bytes());

    let new_audio_size = (audio_size * 2) as u32;
    audio_data[audio_start - 4..audio_start].copy_from_slice(&new_audio_size.to_le_bytes());

    audio_data.resize(audio_data.len() * 2, 0);

    let mut i = audio_start;
    while i < original.len() - 1 {
        audio_data[i * 2 - audio_start] = original[i];
        audio_data[i * 2 + 1 - audio_start] = original[i + 1];
        audio_data[i * 2 + 2 - audio_start] = original[i];
        audio_data[i * 2 + 3 - audio_start] = original[i + 1];
        i += 2;
    }
    Ok(())
}

fn random_noise(data: &mut Vec<u8>) -> Result<(), &'static str> {
    let audio_start = 44;
    let noise_amount = 20000;
    let original = data[audio_start..].to_vec();

    data.resize(original.len() + noise_amount, 0);

    let mut i = audio_start;
    while i < original.len() {
        let noise = rand::thread_rng().gen_range(0..4);
        data[i] = original[i];
        data[i + 1] = original[i + 1];
        data[i + 2] = noise;
        data[i + 3] = noise;
        i += 4;
    }
    Ok(())
}

fn delay(audio_data: &mut Vec<u8>, ms: usize, amount: usize) -> Result<(), &'static str> {
    const SAMPLE_RATE: usize = 44100;
    const BYTES_PER_SAMPLE: usize = 4;
    let offset: usize = (ms * SAMPLE_RATE) / 1000 * BYTES_PER_SAMPLE;
    let mut delayed = vec![0u8; offset];
    delayed.extend(audio_data.iter());

    let original_len = audio_data.len();
    audio_data.resize(original_len + (offset * amount), 0);

    for i in (0..audio_data.len() - 1).step_by(2) {
        if i + 1 >= delayed.len() {
            break;
        }

        let original_sample = i16::from_le_bytes([audio_data[i], audio_data[i + 1]]);
        let delayed_sample = i16::from_le_bytes([delayed[i], delayed[i + 1]]);

        let delayed_adjusted = (delayed_sample as f32 * 0.5) as i16;
        let mixed_sample = original_sample.saturating_add(delayed_adjusted);
        let mixed_bytes = mixed_sample.to_le_bytes();

        audio_data[i] = mixed_bytes[0];
        audio_data[i + 1] = mixed_bytes[1];
    }
    Ok(())
}

fn tremolo(audio_data: &mut Vec<u8>) -> Result<(), &'static str> {
    const SAMPLE_RATE: f32 = 44_100.0;
    const FREQUENCY: f32 = 8.0;
    const DEPTH: f32 = 0.3;

    for i in (0..audio_data.len() - 1).step_by(2) {
        let sample = i16::from_le_bytes([audio_data[i], audio_data[i + 1]]);
        let angle = (i as f32 * FREQUENCY * 2.0 * std::f32::consts::PI) / SAMPLE_RATE;
        let mod_factor = 1.0 - (DEPTH * (0.5 + 0.5 * angle.sin()));
        let new_sample = (sample as f32 * mod_factor) as i16;
        let new_sample_bytes = new_sample.to_le_bytes();

        audio_data[i] = new_sample_bytes[0];
        audio_data[i + 1] = new_sample_bytes[1];
    }
    Ok(())
}

fn pitch_octave_up(audio_data: &mut Vec<u8>) -> Result<(), &'static str> {
    let mut j = 0;
    for i in (0..audio_data.len() - 1).step_by(4) {
        let sample = i16::from_le_bytes([audio_data[i], audio_data[i + 1]]);
        let sample_bytes = sample.to_le_bytes();
        audio_data[j] = sample_bytes[0];
        audio_data[j + 1] = sample_bytes[1];
        j += 2;
    }
    audio_data.resize(audio_data.len() / 2, 0);
    Ok(())
}
