use super::{EffectBox, EffectTrait};
use std::any::TypeId;
use std::fmt;

pub struct PanRight(pub u8);

impl PanRight {
    pub fn name() -> String {
        "Pan Right".to_string()
    }

    pub fn new(amount: u8) -> Self {
        PanRight(amount)
    }

    pub fn default() -> Self {
        PanRight(0)
    }
}

impl fmt::Debug for PanRight {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PanRight({})", self.0)
    }
}

impl EffectTrait for PanRight {
    fn default_instance(&self) -> EffectBox {
        Box::new(PanRight(0))
    }

    fn parameters(&self) -> Vec<(String, String)> {
        vec![("amount".to_string(), self.0.to_string())]
    }

    fn update_parameter_boxed(&self, param_name: &str, value: &str) -> Result<EffectBox, String> {
        if param_name == "amount" {
            let new_amount: u8 = value.parse().map_err(|_| "Invalid amount value")?;
            if new_amount > 100 {
                return Err("Amount must be between 0 and 100".to_string());
            }
            Ok(Box::new(PanRight(new_amount)))
        } else {
            Err(format!("Unknown parameter: {}", param_name))
        }
    }

    fn apply(&self, samples: &mut Vec<f64>, _sample_rate: u32) -> Result<(), &'static str> {
        if self.0 > 100 {
            return Err("Pan amount must be 0-100");
        }

        let gain = self.0 as f64 / 100.0;

        for chunk in samples.chunks_exact_mut(2) {
            chunk[0] *= 1.0 - gain; // Reduce left channel
        }
        Ok(())
    }

    fn type_id(&self) -> TypeId {
        TypeId::of::<PanRight>()
    }
}
