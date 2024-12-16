use std::f64::consts::PI;

//  This is extremely slow, with a time complexity of O(n²) -- I'll add it here just for reference
//  but we shouldn't be using this for analyzing anything. Maybe your mom.
pub fn discrete_fourier_transform(audio_data: &mut Vec<u8>) -> Result<Vec<f64>, &'static str> {
    let SAMPLE_RATE: f64 = 44100.0;
    let sample_count: usize = audio_data.len() / 2;
    let freq_bins: usize = sample_count / 2;
    let mut magnitudes: Vec<f64> = Vec::with_capacity(freq_bins);

    for bin in 0..freq_bins {
        let freq: f64 = (bin as f64) * SAMPLE_RATE / (sample_count as f64);
        let mut real_sum = 0.0;
        let mut imag_sum = 0.0;

        for sample_idx in (0..audio_data.len() - 1).step_by(2) {
            let sample = i16::from_le_bytes([audio_data[sample_idx], audio_data[sample_idx + 1]]);
            let angle: f64 = 2.0 * PI * freq * (sample_idx as f64) / SAMPLE_RATE;
            let real = sample as f64 * angle.cos();
            let imaginary = sample as f64 * angle.sin();
            real_sum += real;
            imag_sum += imaginary;

            println!("Looping through sample {} in bin {}", sample_idx, bin);
        }

        let magnitude = (real_sum * real_sum + imag_sum * imag_sum).sqrt();
        magnitudes.push(magnitude);
    }

    Ok(magnitudes)
}
