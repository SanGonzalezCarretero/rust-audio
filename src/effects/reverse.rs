use super::{EffectBox, EffectTrait};
use std::any::TypeId;
use std::fmt;

#[derive(Default)]
pub struct Reverse;

impl Reverse {
    pub fn name() -> String {
        "Reverse".to_string()
    }
}

impl fmt::Debug for Reverse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Reverse")
    }
}

impl EffectTrait for Reverse {
    fn name() -> String
    where
        Self: Sized,
    {
        Reverse::name()
    }

    fn new(_params: Vec<(String, String)>) -> Self
    where
        Self: Sized,
    {
        Reverse
    }

    fn parameters(&self) -> Vec<(String, String)> {
        vec![]
    }

    fn update_parameter_boxed(&self, _param_name: &str, _value: &str) -> Result<EffectBox, String> {
        Err("Reverse has no configurable parameters".to_string())
    }

    fn apply(&self, samples: &mut Vec<f32>, _sample_rate: u32) -> Result<(), &'static str> {
        samples.reverse();
        Ok(())
    }

    fn type_id(&self) -> TypeId {
        TypeId::of::<Reverse>()
    }
}
