use super::{EffectBox, EffectTrait};
use std::any::TypeId;
use std::fmt;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PanDirection {
    Left,
    Right,
}

impl fmt::Display for PanDirection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PanDirection::Left => write!(f, "Left"),
            PanDirection::Right => write!(f, "Right"),
        }
    }
}

pub struct PanLeft(pub u8);
pub struct PanRight(pub u8);

impl Default for PanLeft {
    fn default() -> Self {
        PanLeft(0)
    }
}

impl Default for PanRight {
    fn default() -> Self {
        PanRight(0)
    }
}

impl fmt::Debug for PanLeft {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PanLeft({})", self.0)
    }
}

impl fmt::Debug for PanRight {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PanRight({})", self.0)
    }
}

fn pan_apply(samples: &mut Vec<f32>, amount: u8, direction: PanDirection) -> Result<(), &'static str> {
    if amount > 100 {
        return Err("Pan amount must be 0-100");
    }

    let gain = amount as f32 / 100.0;

    for chunk in samples.chunks_exact_mut(2) {
        match direction {
            PanDirection::Left => chunk[1] *= 1.0 - gain,
            PanDirection::Right => chunk[0] *= 1.0 - gain,
        }
    }
    Ok(())
}

fn pan_new_from_params(params: &[(String, String)]) -> u8 {
    params
        .iter()
        .find(|(name, _)| name == "amount")
        .and_then(|(_, value)| value.parse::<u8>().ok())
        .unwrap_or(0)
}

fn pan_update_parameter(param_name: &str, value: &str, direction: PanDirection) -> Result<EffectBox, String> {
    if param_name == "amount" {
        let new_amount: u8 = value.parse().map_err(|_| "Invalid amount value")?;
        if new_amount > 100 {
            return Err("Amount must be between 0 and 100".to_string());
        }
        match direction {
            PanDirection::Left => Ok(Box::new(PanLeft(new_amount))),
            PanDirection::Right => Ok(Box::new(PanRight(new_amount))),
        }
    } else {
        Err(format!("Unknown parameter: {}", param_name))
    }
}

impl EffectTrait for PanLeft {
    fn name() -> String where Self: Sized { "Pan Left".to_string() }

    fn new(params: Vec<(String, String)>) -> Self where Self: Sized {
        PanLeft(pan_new_from_params(&params))
    }

    fn parameters(&self) -> Vec<(String, String)> {
        vec![("amount".to_string(), self.0.to_string())]
    }

    fn update_parameter_boxed(&self, param_name: &str, value: &str) -> Result<EffectBox, String> {
        pan_update_parameter(param_name, value, PanDirection::Left)
    }

    fn apply(&self, samples: &mut Vec<f32>, _sample_rate: u32) -> Result<(), &'static str> {
        pan_apply(samples, self.0, PanDirection::Left)
    }

    fn type_id(&self) -> TypeId { TypeId::of::<PanLeft>() }
}

impl EffectTrait for PanRight {
    fn name() -> String where Self: Sized { "Pan Right".to_string() }

    fn new(params: Vec<(String, String)>) -> Self where Self: Sized {
        PanRight(pan_new_from_params(&params))
    }

    fn parameters(&self) -> Vec<(String, String)> {
        vec![("amount".to_string(), self.0.to_string())]
    }

    fn update_parameter_boxed(&self, param_name: &str, value: &str) -> Result<EffectBox, String> {
        pan_update_parameter(param_name, value, PanDirection::Right)
    }

    fn apply(&self, samples: &mut Vec<f32>, _sample_rate: u32) -> Result<(), &'static str> {
        pan_apply(samples, self.0, PanDirection::Right)
    }

    fn type_id(&self) -> TypeId { TypeId::of::<PanRight>() }
}
