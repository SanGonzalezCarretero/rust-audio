use super::{EffectBox, EffectTrait};
use std::any::TypeId;
use std::fmt;

pub struct Delay {
    pub ms: usize,
    pub taps: usize,
}

impl Default for Delay {
    fn default() -> Self {
        Delay { ms: 1, taps: 1 }
    }
}

impl Delay {
    pub fn name() -> String {
        "Delay".to_string()
    }

    pub fn new(ms: usize, taps: usize) -> Self {
        Delay { ms, taps }
    }
}

impl fmt::Debug for Delay {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Delay(ms: {}, taps: {})", self.ms, self.taps)
    }
}

impl EffectTrait for Delay {
    fn name() -> String
    where
        Self: Sized,
    {
        Delay::name()
    }

    fn new(params: Vec<(String, String)>) -> Self
    where
        Self: Sized,
    {
        let ms = params
            .iter()
            .find(|(name, _)| name == "ms")
            .and_then(|(_, value)| value.parse::<usize>().ok())
            .unwrap_or(1);
        let taps = params
            .iter()
            .find(|(name, _)| name == "taps")
            .and_then(|(_, value)| value.parse::<usize>().ok())
            .unwrap_or(1);
        Delay { ms, taps }
    }

    fn parameters(&self) -> Vec<(String, String)> {
        vec![
            ("ms".to_string(), self.ms.to_string()),
            ("taps".to_string(), self.taps.to_string()),
        ]
    }

    fn update_parameter_boxed(&self, param_name: &str, value: &str) -> Result<EffectBox, String> {
        match param_name {
            "ms" => {
                let new_ms: usize = value.parse().map_err(|_| "Invalid ms value")?;
                if new_ms == 0 || new_ms > 5000 {
                    return Err("Delay ms must be between 1 and 5000".to_string());
                }
                Ok(Box::new(Delay {
                    ms: new_ms,
                    taps: self.taps,
                }))
            }
            "taps" => {
                let new_taps: usize = value.parse().map_err(|_| "Invalid taps value")?;
                if new_taps == 0 || new_taps > 10 {
                    return Err("Taps must be between 1 and 10".to_string());
                }
                Ok(Box::new(Delay {
                    ms: self.ms,
                    taps: new_taps,
                }))
            }
            _ => Err(format!("Unknown parameter: {}", param_name)),
        }
    }

    fn apply(&self, samples: &mut Vec<f32>, sample_rate: u32) -> Result<(), &'static str> {
        let delay_samples = (self.ms * sample_rate as usize) / 1000;
        let original = samples.clone();

        // Extend the audio to accommodate the delay
        samples.resize(samples.len() + delay_samples * self.taps, 0.0);

        for tap_idx in 0..self.taps {
            let tap = tap_idx + 1;
            let delay_offset = delay_samples * tap;
            let feedback_gain = 0.5 / tap as f32;

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

    fn type_id(&self) -> TypeId {
        TypeId::of::<Delay>()
    }
}
