use crate::device::AudioDevice;
use std::sync::{Arc, Mutex, OnceLock};

static AUDIO_ENGINE: OnceLock<Arc<Mutex<AudioEngine>>> = OnceLock::new();

pub struct AudioEngine {
    input_devices: Vec<String>,
    output_devices: Vec<String>,
    selected_input: Option<String>,
    selected_output: Option<String>,
}

const DEFAULT_INPUT_DEVICE_NAME: &str = "Micrófono de MacBook Pro";
const DEFAULT_OUTPUT_DEVICE_NAME: &str = "Bocinas de MacBook Pro";

impl AudioEngine {
    pub fn global() -> Arc<Mutex<AudioEngine>> {
        AUDIO_ENGINE
            .get_or_init(|| Arc::new(Mutex::new(AudioEngine::new())))
            .clone()
    }

    fn new() -> Self {
        let input_devices = AudioDevice::list_input_devices().unwrap_or_default();
        let output_devices = AudioDevice::list_output_devices().unwrap_or_default();

        let selected_input = input_devices
            .iter()
            .find(|name| name.contains(DEFAULT_INPUT_DEVICE_NAME))
            .cloned()
            .or_else(|| input_devices.first().cloned());

        let selected_output = output_devices
            .iter()
            .find(|name| name.contains(DEFAULT_OUTPUT_DEVICE_NAME))
            .cloned()
            .or_else(|| output_devices.first().cloned());

        AudioEngine {
            input_devices,
            output_devices,
            selected_input,
            selected_output,
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
        self.input_devices = AudioDevice::list_input_devices().unwrap_or_default();
        self.output_devices = AudioDevice::list_output_devices().unwrap_or_default();

        // Revalidate selections - prefer MacBook Pro built-in devices if current selection is unavailable
        if let Some(input) = &self.selected_input {
            if !self.input_devices.contains(input) {
                self.selected_input = self.input_devices
                    .iter()
                    .find(|name| name.contains("Micrófono de MacBook Pro"))
                    .cloned()
                    .or_else(|| self.input_devices.first().cloned());
            }
        }
        if let Some(output) = &self.selected_output {
            if !self.output_devices.contains(output) {
                self.selected_output = self.output_devices
                    .iter()
                    .find(|name| name.contains("Bocinas de MacBook Pro"))
                    .cloned()
                    .or_else(|| self.output_devices.first().cloned());
            }
        }
    }

    /// Get an AudioDevice for the selected input (or default)
    pub fn get_input_device() -> Result<AudioDevice, Box<dyn std::error::Error>> {
        let engine = Self::global();
        let engine = engine.lock().unwrap();
        
        if let Some(name) = &engine.selected_input {
            AudioDevice::input_by_name(name)
        } else {
            AudioDevice::default_input()
        }
    }

    /// Get an AudioDevice for the selected output (or default)
    pub fn get_output_device() -> Result<AudioDevice, Box<dyn std::error::Error>> {
        let engine = Self::global();
        let engine = engine.lock().unwrap();
        
        if let Some(name) = &engine.selected_output {
            AudioDevice::output_by_name(name)
        } else {
            AudioDevice::default_output()
        }
    }
}
