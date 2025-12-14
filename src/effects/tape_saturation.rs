use super::{EffectBox, EffectTrait};
use std::any::TypeId;
use std::fmt;

pub struct TapeSaturation;

impl TapeSaturation {
    pub fn name() -> String {
        "Tape Saturation".to_string()
    }
}

impl fmt::Debug for TapeSaturation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "TapeSaturation")
    }
}

impl EffectTrait for TapeSaturation {
    fn default_instance(&self) -> EffectBox {
        Box::new(TapeSaturation)
    }

    fn parameters(&self) -> Vec<(String, String)> {
        vec![]
    }

    fn update_parameter_boxed(&self, _param_name: &str, _value: &str) -> Result<EffectBox, String> {
        Err("TapeSaturation has no configurable parameters".to_string())
    }

    fn apply(&self, samples: &mut Vec<f64>, _sample_rate: u32) -> Result<(), &'static str> {
        let drive = 2.0; // How hard we push the "tape"
        let mix = 0.3; // How much saturation to blend in (30%)
        let output_gain = 0.8; // Compensate for volume increase

        for sample in samples.iter_mut() {
            let clean = *sample;
            let driven = *sample * drive;
            let saturated = driven.tanh();

            // Mix clean and saturated signals
            *sample = (clean * (1.0 - mix) + saturated * mix) * output_gain;
        }

        Ok(())
    }

    fn type_id(&self) -> TypeId {
        TypeId::of::<TapeSaturation>()
    }
}
