use super::{EffectBox, EffectTrait};
use rand::Rng;
use std::any::TypeId;
use std::fmt;

pub struct RandomNoise;

impl RandomNoise {
    pub fn name() -> String {
        "Random Noise".to_string()
    }
}

impl fmt::Debug for RandomNoise {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "RandomNoise")
    }
}

impl EffectTrait for RandomNoise {
    fn default_instance(&self) -> EffectBox {
        Box::new(RandomNoise)
    }

    fn parameters(&self) -> Vec<(String, String)> {
        vec![]
    }

    fn update_parameter_boxed(&self, _param_name: &str, _value: &str) -> Result<EffectBox, String> {
        Err("RandomNoise has no configurable parameters".to_string())
    }

    fn apply(&self, samples: &mut Vec<f64>, _sample_rate: u32) -> Result<(), &'static str> {
        let mut rng = rand::thread_rng();

        for sample in samples.iter_mut() {
            // Add random noise with small amplitude
            let noise = rng.gen_range(-0.1..0.1);
            *sample += noise;
            *sample = sample.clamp(-1.0, 1.0);
        }
        Ok(())
    }

    fn type_id(&self) -> TypeId {
        TypeId::of::<RandomNoise>()
    }
}
