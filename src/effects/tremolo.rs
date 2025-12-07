use super::{EffectBox, EffectTrait};
use std::any::TypeId;
use std::fmt;

pub struct Tremolo;

impl Tremolo {
    pub fn name() -> String {
        "Tremolo".to_string()
    }
}

impl fmt::Debug for Tremolo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Tremolo")
    }
}

impl EffectTrait for Tremolo {
    fn default_instance(&self) -> EffectBox {
        Box::new(Tremolo)
    }

    fn parameters(&self) -> Vec<(String, String)> {
        vec![]
    }

    fn update_parameter_boxed(&self, _param_name: &str, _value: &str) -> Result<EffectBox, String> {
        Err("Tremolo has no configurable parameters".to_string())
    }

    fn apply(&self, samples: &mut Vec<f64>, sample_rate: u32) -> Result<(), &'static str> {
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

    fn type_id(&self) -> TypeId {
        TypeId::of::<Tremolo>()
    }
}
