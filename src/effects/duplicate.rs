use super::{EffectBox, EffectTrait};
use std::any::TypeId;
use std::fmt;

#[derive(Default)]
pub struct Duplicate;

impl Duplicate {
    pub fn name() -> String {
        "Duplicate".to_string()
    }
}

impl fmt::Debug for Duplicate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Duplicate")
    }
}

impl EffectTrait for Duplicate {
    fn name() -> String
    where
        Self: Sized,
    {
        Duplicate::name()
    }

    fn new(_params: Vec<(String, String)>) -> Self
    where
        Self: Sized,
    {
        Duplicate
    }

    fn parameters(&self) -> Vec<(String, String)> {
        vec![]
    }

    fn update_parameter_boxed(&self, _param_name: &str, _value: &str) -> Result<EffectBox, String> {
        Err("Duplicate has no configurable parameters".to_string())
    }

    fn apply(&self, samples: &mut Vec<f64>, _sample_rate: u32) -> Result<(), &'static str> {
        let original = samples.clone();
        samples.extend(original);
        Ok(())
    }

    fn type_id(&self) -> TypeId {
        TypeId::of::<Duplicate>()
    }
}
