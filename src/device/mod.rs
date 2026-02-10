use cpal::traits::{DeviceTrait, HostTrait};
use cpal::{Device, SampleRate, StreamConfig};

pub struct Input;
pub struct Output;

pub struct AudioDevice {
    pub device: Device,
    pub config: StreamConfig,
    pub sample_rate: u32,
    pub channels: u16,
}

impl AudioDevice {
    pub const INPUT: Input = Input;
    pub const OUTPUT: Output = Output;

    fn from_device_with_config(
        device: Device,
        config: StreamConfig,
    ) -> Self {
        let SampleRate(sample_rate) = config.sample_rate;
        let channels = config.channels;
        AudioDevice {
            device,
            config,
            sample_rate,
            channels,
        }
    }
}

pub trait DeviceProvider {
    fn list(&self) -> Result<Vec<String>, Box<dyn std::error::Error>>;
    fn default(&self) -> Result<AudioDevice, Box<dyn std::error::Error>>;
    fn default_name(&self) -> Option<String>;
    fn by_index(&self, index: usize) -> Result<AudioDevice, Box<dyn std::error::Error>>;
    fn by_name(&self, name: &str) -> Result<AudioDevice, Box<dyn std::error::Error>>;
}

macro_rules! impl_device_provider {
    ($type:ty, $devices_fn:ident, $default_fn:ident, $config_fn:ident, $no_device_msg:expr) => {
        impl DeviceProvider for $type {
            fn list(&self) -> Result<Vec<String>, Box<dyn std::error::Error>> {
                let host = cpal::default_host();
                Ok(host
                    .$devices_fn()?
                    .filter_map(|d| d.name().ok())
                    .collect())
            }

            fn default(&self) -> Result<AudioDevice, Box<dyn std::error::Error>> {
                let host = cpal::default_host();
                let device = host
                    .$default_fn()
                    .ok_or($no_device_msg)?;
                let config: StreamConfig = device.$config_fn()?.into();
                Ok(AudioDevice::from_device_with_config(device, config))
            }

            fn default_name(&self) -> Option<String> {
                let host = cpal::default_host();
                host.$default_fn().and_then(|d| d.name().ok())
            }

            fn by_index(&self, index: usize) -> Result<AudioDevice, Box<dyn std::error::Error>> {
                let host = cpal::default_host();
                let device = host.$devices_fn()?.nth(index).ok_or("Device not found")?;
                let config: StreamConfig = device.$config_fn()?.into();
                Ok(AudioDevice::from_device_with_config(device, config))
            }

            fn by_name(&self, name: &str) -> Result<AudioDevice, Box<dyn std::error::Error>> {
                let host = cpal::default_host();
                let device = host
                    .$devices_fn()?
                    .find(|d| d.name().ok().as_deref() == Some(name))
                    .ok_or("Device not found")?;
                let config: StreamConfig = device.$config_fn()?.into();
                Ok(AudioDevice::from_device_with_config(device, config))
            }
        }
    };
}

impl_device_provider!(Input, input_devices, default_input_device, default_input_config, "No input device available");
impl_device_provider!(Output, output_devices, default_output_device, default_output_config, "No output device available");
