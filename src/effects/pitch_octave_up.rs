use super::{EffectBox, EffectTrait};
use std::any::TypeId;
use std::fmt;

#[derive(Default)]
pub struct PitchOctaveUp;

impl PitchOctaveUp {
    pub fn name() -> String {
        "Pitch Octave Up".to_string()
    }
}

impl fmt::Debug for PitchOctaveUp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PitchOctaveUp")
    }
}

impl EffectTrait for PitchOctaveUp {
    fn name() -> String
    where
        Self: Sized,
    {
        PitchOctaveUp::name()
    }

    fn new(_params: Vec<(String, String)>) -> Self
    where
        Self: Sized,
    {
        PitchOctaveUp
    }

    fn parameters(&self) -> Vec<(String, String)> {
        vec![]
    }

    fn update_parameter_boxed(&self, _param_name: &str, _value: &str) -> Result<EffectBox, String> {
        Err("PitchOctaveUp has no configurable parameters".to_string())
    }

    fn apply(&self, samples: &mut Vec<f64>, _sample_rate: u32) -> Result<(), &'static str> {
        // Simple pitch shifting by taking every other sample
        let mut new_samples = Vec::with_capacity(samples.len() / 2);

        for i in (0..samples.len()).step_by(2) {
            new_samples.push(samples[i]);
        }

        *samples = new_samples;
        Ok(())
    }

    fn type_id(&self) -> TypeId {
        TypeId::of::<PitchOctaveUp>()
    }
}
