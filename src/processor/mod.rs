use crate::effects::Effect;
use rustfft::num_complex::Complex64;
use rustfft::FftPlanner;
use std::f64::consts::PI;
use std::io::BufReader;

pub struct Processor {
    fft_planner: FftPlanner<f64>,
    sample_rate: usize,
}

pub enum FrequencyEffect {
    PitchShift(f64),
    Log(),
    LowPassFilter(f64),
}

impl Processor {
    pub fn new(sample_rate: usize) -> Self {
        Self {
            sample_rate,
            fft_planner: FftPlanner::new(),
        }
    }

    pub fn apply_time_domain_effect(
        &self,
        audio_data: &mut Vec<u8>,
        effect: Effect,
    ) -> Result<(), &'static str> {
        effect.apply(audio_data)
    }

    pub fn apply_frequency_domain_effect(
        &mut self,
        samples: &[f64],
        effect: FrequencyEffect,
    ) -> Vec<f64> {
        let mut buffer = self.prepare_samples(samples);
        self.forward_fft(&mut buffer);

        match effect {
            FrequencyEffect::PitchShift(amount) => self.pitch_shift(&mut buffer, amount),
            FrequencyEffect::Log() => self.log(&mut buffer),
            FrequencyEffect::LowPassFilter(cutoff_freq) => {
                self.low_pass_filter(&mut buffer, cutoff_freq)
            }
        }

        self.inverse_fft(&mut buffer);
        self.finalize_samples(&buffer)
    }

    fn prepare_samples(&self, samples: &[f64]) -> Vec<Complex64> {
        samples.iter().map(|&x| Complex64::new(x, 0.0)).collect()
    }

    fn forward_fft(&mut self, buffer: &mut Vec<Complex64>) {
        let fft = self.fft_planner.plan_fft_forward(buffer.len());
        fft.process(buffer);
    }

    fn inverse_fft(&mut self, buffer: &mut Vec<Complex64>) {
        let ifft = self.fft_planner.plan_fft_inverse(buffer.len());
        ifft.process(buffer);
        let scale = 1.0 / (buffer.len() as f64);
        buffer.iter_mut().for_each(|x| *x *= scale);
    }

    fn finalize_samples(&self, buffer: &[Complex64]) -> Vec<f64> {
        buffer.iter().map(|x| x.re).collect()
    }

    fn log(&self, buffer: &mut Vec<Complex64>) {
        for i in 0..buffer.len() / 2 {
            // Which frequency this bin represents:
            let frequency = i as f64 * self.sample_rate as f64 / buffer.len() as f64;

            // From these we can get:
            let magnitude = buffer[i].norm(); // Amplitude of this frequency
            let phase = buffer[i].arg(); // Phase of this frequency

            println!(
                "Bin {}: Frequency = {:.1} Hz, Magnitude = {:.2}, Phase = {:.2} radians",
                i, frequency, magnitude, phase
            );
        }
    }

    fn apply_to_conjugate_pairs<F>(&self, buffer: &mut Vec<Complex64>, mut operation: F)
    where
        F: FnMut(usize, f64) -> f64,
    {
        let len = buffer.len();

        // Skip DC (i=0) and Nyquist (i=len/2)
        for i in 1..len / 2 {
            let frequency = i as f64 * self.sample_rate as f64 / len as f64;
            let factor = operation(i, frequency);

            dbg!(factor);

            // Modify bin i and its conjugate
            buffer[i] *= factor;
            buffer[len - i] *= factor;
        }
    }

    // Then use it in your effects:
    fn low_pass_filter(&self, buffer: &mut Vec<Complex64>, cutoff_freq: f64) {
        self.apply_to_conjugate_pairs(
            buffer,
            |_i, freq| {
                if freq > cutoff_freq {
                    0.0
                } else {
                    1.0
                }
            },
        );
    }
}
