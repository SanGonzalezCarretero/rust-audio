use super::{EffectBox, EffectTrait};
use std::any::TypeId;
use std::fmt;

pub struct LargeReverb;

impl LargeReverb {
    pub fn name() -> String {
        "Large Reverb".to_string()
    }
}

impl fmt::Debug for LargeReverb {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "LargeReverb")
    }
}

impl EffectTrait for LargeReverb {
    fn default_instance(&self) -> EffectBox {
        Box::new(LargeReverb)
    }

    fn parameters(&self) -> Vec<(String, String)> {
        vec![]
    }

    fn update_parameter_boxed(&self, _param_name: &str, _value: &str) -> Result<EffectBox, String> {
        Err("LargeReverb has no configurable parameters".to_string())
    }

    fn apply(&self, samples: &mut Vec<f64>, sample_rate: u32) -> Result<(), &'static str> {
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

    fn type_id(&self) -> TypeId {
        TypeId::of::<LargeReverb>()
    }
}
