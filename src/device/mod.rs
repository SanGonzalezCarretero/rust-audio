use cpal::traits::{DeviceTrait, HostTrait};
use cpal::{Device, Host, SampleRate, StreamConfig};

pub struct AudioDevice {
    pub device: Device,
    pub config: StreamConfig,
    pub sample_rate: u32,
    pub channels: u16,
}

impl AudioDevice {
    fn get_host_and_device(is_input: bool) -> Result<(Host, Device), Box<dyn std::error::Error>> {
        let host = cpal::default_host();
        let device = if is_input {
            host.default_input_device()
                .ok_or("No input device available")?
        } else {
            host.default_output_device()
                .ok_or("No output device available")?
        };
        Ok((host, device))
    }

    pub fn default_input() -> Result<Self, Box<dyn std::error::Error>> {
        let (_, device) = Self::get_host_and_device(true)?;
        let config: StreamConfig = device.default_input_config()?.into();
        let SampleRate(sample_rate) = config.sample_rate;
        let channels = config.channels;

        Ok(Self {
            device,
            config,
            sample_rate,
            channels,
        })
    }

    pub fn default_output() -> Result<Self, Box<dyn std::error::Error>> {
        let (_, device) = Self::get_host_and_device(false)?;
        let config: StreamConfig = device.default_output_config()?.into();
        let SampleRate(sample_rate) = config.sample_rate;
        let channels = config.channels;

        Ok(Self {
            device,
            config,
            sample_rate,
            channels,
        })
    }

    pub fn input_by_index(index: usize) -> Result<Self, Box<dyn std::error::Error>> {
        let host = cpal::default_host();
        let device = host.input_devices()?.nth(index).ok_or("Device not found")?;
        let config: StreamConfig = device.default_input_config()?.into();
        let SampleRate(sample_rate) = config.sample_rate;
        let channels = config.channels;

        Ok(Self {
            device,
            config,
            sample_rate,
            channels,
        })
    }

    pub fn output_by_index(index: usize) -> Result<Self, Box<dyn std::error::Error>> {
        let host = cpal::default_host();
        let device = host
            .output_devices()?
            .nth(index)
            .ok_or("Device not found")?;
        let config: StreamConfig = device.default_output_config()?.into();
        let SampleRate(sample_rate) = config.sample_rate;
        let channels = config.channels;

        Ok(Self {
            device,
            config,
            sample_rate,
            channels,
        })
    }

    pub fn list_input_devices() -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let host = cpal::default_host();
        Ok(host
            .input_devices()?
            .filter_map(|d| d.name().ok())
            .collect())
    }

    pub fn list_output_devices() -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let host = cpal::default_host();
        Ok(host
            .output_devices()?
            .filter_map(|d| d.name().ok())
            .collect())
    }

    pub fn input_by_name(name: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let host = cpal::default_host();
        let device = host.input_devices()?
            .find(|d| d.name().ok().as_deref() == Some(name))
            .ok_or("Device not found")?;
        let config: StreamConfig = device.default_input_config()?.into();
        let SampleRate(sample_rate) = config.sample_rate;
        let channels = config.channels;

        Ok(Self {
            device,
            config,
            sample_rate,
            channels,
        })
    }

    pub fn output_by_name(name: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let host = cpal::default_host();
        let device = host.output_devices()?
            .find(|d| d.name().ok().as_deref() == Some(name))
            .ok_or("Device not found")?;
        let config: StreamConfig = device.default_output_config()?.into();
        let SampleRate(sample_rate) = config.sample_rate;
        let channels = config.channels;

        Ok(Self {
            device,
            config,
            sample_rate,
            channels,
        })
    }
}
