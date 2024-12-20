use crate::effects::Effect;
use rustfft::num_complex::Complex64;
use rustfft::FftPlanner;
use std::f64::consts::PI;

pub struct Processor {
    fft_planner: FftPlanner<f64>,
    sample_rate: usize,
}

pub enum FrequencyEffect {
    PitchShift(f64),
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

    fn pitch_shift(&self, buffer: &mut Vec<Complex64>, shift_amount: f64) {
        let mut modified = buffer.clone();
        for i in 0..buffer.len() / 2 {
            let new_i = ((i as f64 * self.sample_rate as f64 / buffer.len() as f64 + shift_amount)
                * buffer.len() as f64
                / self.sample_rate as f64) as usize;
            if new_i < buffer.len() / 2 {
                modified[new_i] = buffer[i];
                if i != 0 && new_i != buffer.len() / 2 {
                    modified[buffer.len() - new_i] = buffer[buffer.len() - i].conj();
                }
            }
        }
        *buffer = modified;
    }
}
