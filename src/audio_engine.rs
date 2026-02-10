use crate::device::{AudioDevice, DeviceProvider};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex, OnceLock};

#[derive(Serialize, Deserialize, Default)]
struct AppConfig {
    input_device: Option<String>,
    output_device: Option<String>,
}

fn config_path() -> std::path::PathBuf {
    std::env::current_dir()
        .unwrap_or_default()
        .join("config.json")
}

static AUDIO_ENGINE: OnceLock<Arc<Mutex<AudioEngine>>> = OnceLock::new();

pub struct AudioEngine {
    input_devices: Vec<String>,
    output_devices: Vec<String>,
    selected_input: Option<String>,
    selected_output: Option<String>,
}

impl AudioEngine {
    pub fn global() -> Arc<Mutex<AudioEngine>> {
        AUDIO_ENGINE
            .get_or_init(|| Arc::new(Mutex::new(AudioEngine::new())))
            .clone()
    }

    fn new() -> Self {
        let input_devices = AudioDevice::INPUT.list().unwrap_or_default();
        let output_devices = AudioDevice::OUTPUT.list().unwrap_or_default();

        // Try loading saved preferences, fall back to OS defaults
        let config = std::fs::read_to_string(config_path())
            .ok()
            .and_then(|s| serde_json::from_str::<AppConfig>(&s).ok())
            .unwrap_or_default();

        let selected_input = config.input_device
            .filter(|name| input_devices.contains(name))
            .or_else(|| AudioDevice::INPUT.default_name())
            .or_else(|| input_devices.first().cloned());

        let selected_output = config.output_device
            .filter(|name| output_devices.contains(name))
            .or_else(|| AudioDevice::OUTPUT.default_name())
            .or_else(|| output_devices.first().cloned());

        AudioEngine {
            input_devices,
            output_devices,
            selected_input,
            selected_output,
        }
    }

    pub fn save_config(&self) {
        let config = AppConfig {
            input_device: self.selected_input.clone(),
            output_device: self.selected_output.clone(),
        };
        if let Ok(json) = serde_json::to_string_pretty(&config) {
            let _ = std::fs::write(config_path(), json);
        }
    }

    /// Get list of available input devices
    pub fn input_devices(&self) -> &[String] {
        &self.input_devices
    }

    /// Get list of available output devices
    pub fn output_devices(&self) -> &[String] {
        &self.output_devices
    }

    /// Get currently selected input device name
    pub fn selected_input(&self) -> Option<&str> {
        self.selected_input.as_deref()
    }

    /// Get currently selected output device name
    pub fn selected_output(&self) -> Option<&str> {
        self.selected_output.as_deref()
    }

    /// Select an input device by name
    pub fn set_input_device(&mut self, name: String) {
        if self.input_devices.contains(&name) {
            self.selected_input = Some(name);
        }
    }

    /// Select an output device by name
    pub fn set_output_device(&mut self, name: String) {
        if self.output_devices.contains(&name) {
            self.selected_output = Some(name);
        }
    }

    /// Refresh the list of available devices
    pub fn refresh_devices(&mut self) {
        self.input_devices = AudioDevice::INPUT.list().unwrap_or_default();
        self.output_devices = AudioDevice::OUTPUT.list().unwrap_or_default();

        // Revalidate selections — fall back to OS default if current is gone
        if let Some(input) = &self.selected_input {
            if !self.input_devices.contains(input) {
                self.selected_input = AudioDevice::INPUT.default_name()
                    .or_else(|| self.input_devices.first().cloned());
            }
        }
        if let Some(output) = &self.selected_output {
            if !self.output_devices.contains(output) {
                self.selected_output = AudioDevice::OUTPUT.default_name()
                    .or_else(|| self.output_devices.first().cloned());
            }
        }
    }

    /// Get an AudioDevice for the selected input (or default)
    pub fn get_input_device() -> Result<AudioDevice, Box<dyn std::error::Error>> {
        let engine = Self::global();
        let engine = engine.lock().unwrap();

        if let Some(name) = &engine.selected_input {
            AudioDevice::INPUT.by_name(name)
        } else {
            AudioDevice::INPUT.default()
        }
    }

    /// Get an AudioDevice for the selected output (or default)
    pub fn get_output_device() -> Result<AudioDevice, Box<dyn std::error::Error>> {
        let engine = Self::global();
        let engine = engine.lock().unwrap();

        if let Some(name) = &engine.selected_output {
            AudioDevice::OUTPUT.by_name(name)
        } else {
            AudioDevice::OUTPUT.default()
        }
    }
}
