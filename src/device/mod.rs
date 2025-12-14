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
}

pub trait DeviceProvider {
    fn list(&self) -> Result<Vec<String>, Box<dyn std::error::Error>>;
    fn default(&self) -> Result<AudioDevice, Box<dyn std::error::Error>>;
    fn by_index(&self, index: usize) -> Result<AudioDevice, Box<dyn std::error::Error>>;
    fn by_name(&self, name: &str) -> Result<AudioDevice, Box<dyn std::error::Error>>;
}

impl DeviceProvider for Input {
    fn list(&self) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let host = cpal::default_host();
        Ok(host
            .input_devices()?
            .filter_map(|d| d.name().ok())
            .collect())
    }

    fn default(&self) -> Result<AudioDevice, Box<dyn std::error::Error>> {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or("No input device available")?;
        let config: StreamConfig = device.default_input_config()?.into();
        let SampleRate(sample_rate) = config.sample_rate;
        let channels = config.channels;

        Ok(AudioDevice {
            device,
            config,
            sample_rate,
            channels,
        })
    }

    fn by_index(&self, index: usize) -> Result<AudioDevice, Box<dyn std::error::Error>> {
        let host = cpal::default_host();
        let device = host.input_devices()?.nth(index).ok_or("Device not found")?;
        let config: StreamConfig = device.default_input_config()?.into();
        let SampleRate(sample_rate) = config.sample_rate;
        let channels = config.channels;

        Ok(AudioDevice {
            device,
            config,
            sample_rate,
            channels,
        })
    }

    fn by_name(&self, name: &str) -> Result<AudioDevice, Box<dyn std::error::Error>> {
        let host = cpal::default_host();
        let device = host
            .input_devices()?
            .find(|d| d.name().ok().as_deref() == Some(name))
            .ok_or("Device not found")?;
        let config: StreamConfig = device.default_input_config()?.into();
        let SampleRate(sample_rate) = config.sample_rate;
        let channels = config.channels;

        Ok(AudioDevice {
            device,
            config,
            sample_rate,
            channels,
        })
    }
}

impl DeviceProvider for Output {
    fn list(&self) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let host = cpal::default_host();
        Ok(host
            .output_devices()?
            .filter_map(|d| d.name().ok())
            .collect())
    }

    fn default(&self) -> Result<AudioDevice, Box<dyn std::error::Error>> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or("No output device available")?;
        let config: StreamConfig = device.default_output_config()?.into();
        let SampleRate(sample_rate) = config.sample_rate;
        let channels = config.channels;

        Ok(AudioDevice {
            device,
            config,
            sample_rate,
            channels,
        })
    }

    fn by_index(&self, index: usize) -> Result<AudioDevice, Box<dyn std::error::Error>> {
        let host = cpal::default_host();
        let device = host
            .output_devices()?
            .nth(index)
            .ok_or("Device not found")?;
        let config: StreamConfig = device.default_output_config()?.into();
        let SampleRate(sample_rate) = config.sample_rate;
        let channels = config.channels;

        Ok(AudioDevice {
            device,
            config,
            sample_rate,
            channels,
        })
    }

    fn by_name(&self, name: &str) -> Result<AudioDevice, Box<dyn std::error::Error>> {
        let host = cpal::default_host();
        let device = host
            .output_devices()?
            .find(|d| d.name().ok().as_deref() == Some(name))
            .ok_or("Device not found")?;
        let config: StreamConfig = device.default_output_config()?.into();
        let SampleRate(sample_rate) = config.sample_rate;
        let channels = config.channels;

        Ok(AudioDevice {
            device,
            config,
            sample_rate,
            channels,
        })
    }
}
