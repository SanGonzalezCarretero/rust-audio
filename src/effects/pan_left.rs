use super::{EffectTrait, EffectBox};
use std::any::TypeId;
use std::fmt;

pub struct PanLeft(pub u8);

impl PanLeft {
    pub fn name() -> String {
        "Pan Left".to_string()
    }

    pub fn new(amount: u8) -> Self {
        PanLeft(amount)
    }

    pub fn default() -> Self {
        PanLeft(0)
    }
}

impl fmt::Debug for PanLeft {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PanLeft({})", self.0)
    }
}

impl EffectTrait for PanLeft {
    fn default_instance(&self) -> EffectBox {
        Box::new(PanLeft(0))
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
            Ok(Box::new(PanLeft(new_amount)))
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
            chunk[1] *= 1.0 - gain; // Reduce right channel
        }
        Ok(())
    }

    fn type_id(&self) -> TypeId {
        TypeId::of::<PanLeft>()
    }
}

