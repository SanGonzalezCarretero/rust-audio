use std::any::TypeId;
use strum::EnumIter;

pub mod adjust_volume;
pub use adjust_volume::AdjustVolume;
pub mod reverse;
pub use reverse::Reverse;
pub mod duplicate;
pub use duplicate::Duplicate;
pub mod random_noise;
pub use random_noise::RandomNoise;
pub mod tremolo;
pub use tremolo::Tremolo;
pub mod pitch_octave_up;
pub use pitch_octave_up::PitchOctaveUp;
pub mod tape_saturation;
pub use tape_saturation::TapeSaturation;
pub mod large_reverb;
pub use large_reverb::LargeReverb;
pub mod delay;
pub use delay::Delay;
pub mod pan;
pub use pan::{PanLeft, PanRight};

// Type alias for trait objects - trait must be object-safe
pub type EffectBox = Box<dyn EffectTrait>;

pub trait EffectTrait: std::fmt::Debug + Send + Sync {
    fn name() -> String
    where
        Self: Sized;
    fn new(params: Vec<(String, String)>) -> Self
    where
        Self: Sized;
    fn parameters(&self) -> Vec<(String, String)>;
    fn update_parameter_boxed(&self, param_name: &str, value: &str) -> Result<EffectBox, String>;
    fn apply(&self, samples: &mut Vec<f32>, sample_rate: u32) -> Result<(), &'static str>;

    // Type identification for trait objects
    fn type_id(&self) -> TypeId;
}

// EffectType enum for registry iteration
#[derive(Debug, Clone, EnumIter, PartialEq, Eq)]
pub enum EffectType {
    Reverse,
    Duplicate,
    RandomNoise,
    Delay,
    Tremolo,
    PitchOctaveUp,
    LargeReverb,
    TapeSaturation,
    PanLeft,
    PanRight,
    AdjustVolume,
}

// Wrapper for effects that stores type information
#[derive(Debug)]
pub struct EffectInstance {
    effect: EffectBox,
    effect_type: EffectType,
}

impl EffectInstance {
    pub fn new(effect: EffectBox, effect_type: EffectType) -> Self {
        EffectInstance {
            effect,
            effect_type,
        }
    }

    pub fn name(&self) -> String {
        self.effect_type.name()
    }

    pub fn display_name(&self) -> String {
        let base_name = self.effect_type.name();
        let params = self.parameters();
        if params.is_empty() {
            base_name
        } else {
            let param_strs: Vec<String> = params
                .iter()
                .map(|(name, value)| format!("{}={}", name, value))
                .collect();
            format!("{} ({})", base_name, param_strs.join(", "))
        }
    }

    pub fn parameters(&self) -> Vec<(String, String)> {
        self.effect.parameters()
    }

    pub fn apply(&self, samples: &mut Vec<f32>, sample_rate: u32) -> Result<(), &'static str> {
        self.effect.apply(samples, sample_rate)
    }

    pub fn effect_type(&self) -> EffectType {
        self.effect_type.clone()
    }

    pub fn same_variant(&self, other: &EffectInstance) -> bool {
        self.effect_type == other.effect_type
    }

    pub fn update_parameter(&self, param_name: &str, value: &str) -> Result<Self, String> {
        let updated_effect = self.effect.update_parameter_boxed(param_name, value)?;
        Ok(EffectInstance {
            effect: updated_effect,
            effect_type: self.effect_type.clone(),
        })
    }
}

impl Clone for EffectInstance {
    fn clone(&self) -> Self {
        // Create a default instance, then apply all current parameters to preserve state
        let mut cloned = self.effect_type.create_default();
        for (param_name, param_value) in self.parameters() {
            if let Ok(updated) = cloned.update_parameter(&param_name, &param_value) {
                cloned = updated;
            }
        }
        cloned
    }
}

impl EffectType {
    pub fn name(&self) -> String {
        match self {
            EffectType::Reverse => Reverse::name(),
            EffectType::Duplicate => Duplicate::name(),
            EffectType::RandomNoise => RandomNoise::name(),
            EffectType::Delay => Delay::name(),
            EffectType::Tremolo => Tremolo::name(),
            EffectType::PitchOctaveUp => PitchOctaveUp::name(),
            EffectType::LargeReverb => LargeReverb::name(),
            EffectType::TapeSaturation => TapeSaturation::name(),
            EffectType::PanLeft => PanLeft::name(),
            EffectType::PanRight => PanRight::name(),
            EffectType::AdjustVolume => AdjustVolume::name(),
        }
    }

    pub fn create_default(&self) -> EffectInstance {
        let effect = match self {
            EffectType::AdjustVolume => Box::new(AdjustVolume::default()) as EffectBox,
            EffectType::Reverse => Box::new(Reverse) as EffectBox,
            EffectType::Duplicate => Box::new(Duplicate) as EffectBox,
            EffectType::RandomNoise => Box::new(RandomNoise) as EffectBox,
            EffectType::Delay => Box::new(Delay::default()) as EffectBox,
            EffectType::Tremolo => Box::new(Tremolo) as EffectBox,
            EffectType::PitchOctaveUp => Box::new(PitchOctaveUp) as EffectBox,
            EffectType::LargeReverb => Box::new(LargeReverb) as EffectBox,
            EffectType::TapeSaturation => Box::new(TapeSaturation) as EffectBox,
            EffectType::PanLeft => Box::new(PanLeft::default()) as EffectBox,
            EffectType::PanRight => Box::new(PanRight::default()) as EffectBox,
        };
        EffectInstance::new(effect, self.clone())
    }
}
