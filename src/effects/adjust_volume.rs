use super::{EffectBox, EffectTrait};
use std::any::TypeId;
use std::fmt;

pub struct AdjustVolume(pub f32);

impl AdjustVolume {
    pub fn name() -> String {
        "Adjust Volume".to_string()
    }

    pub fn new(volume: f32) -> Self {
        AdjustVolume(volume)
    }

    pub fn default() -> Self {
        AdjustVolume(1.0)
    }
}

impl fmt::Debug for AdjustVolume {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "AdjustVolume({})", self.0)
    }
}

impl EffectTrait for AdjustVolume {
    fn default_instance(&self) -> EffectBox {
        Box::new(AdjustVolume(1.0))
    }

    fn parameters(&self) -> Vec<(String, String)> {
        vec![("volume".to_string(), self.0.to_string())]
    }

    fn update_parameter_boxed(&self, param_name: &str, value: &str) -> Result<EffectBox, String> {
        if param_name == "volume" {
            let volume: f32 = value.parse().map_err(|_| "Invalid volume value")?;
            if volume <= 0.0 || volume > 2.0 {
                return Err("Volume must be between 0.0 and 2.0".to_string());
            }
            Ok(Box::new(AdjustVolume(volume)))
        } else {
            Err(format!("Unknown parameter: {}", param_name))
        }
    }

    fn apply(&self, samples: &mut Vec<f64>, _sample_rate: u32) -> Result<(), &'static str> {
        if self.0 > 2.0 || self.0 <= 0.0 {
            return Err("Not a valid volume value. Try again.");
        }

        for sample in samples.iter_mut() {
            *sample *= self.0 as f64;
            *sample = sample.clamp(-1.0, 1.0); // Prevent clipping
        }
        Ok(())
    }

    fn type_id(&self) -> TypeId {
        TypeId::of::<AdjustVolume>()
    }
}
