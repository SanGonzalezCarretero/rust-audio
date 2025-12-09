use crate::device::AudioDevice;
use std::sync::{Arc, Mutex, OnceLock};

static GLOBAL_AUDIO_CONTEXT: OnceLock<Arc<Mutex<AudioContext>>> = OnceLock::new();

pub struct AudioContext {
    pub input_devices: Vec<String>,
    pub output_devices: Vec<String>,
    pub selected_input_device: Option<String>,
    pub selected_output_device: Option<String>,
}

impl AudioContext {
    pub fn global() -> Arc<Mutex<AudioContext>> {
        GLOBAL_AUDIO_CONTEXT
            .get_or_init(|| Arc::new(Mutex::new(AudioContext::default())))
            .clone()
    }

    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let input_devices = AudioDevice::list_input_devices().unwrap_or_default();
        let output_devices = AudioDevice::list_output_devices().unwrap_or_default();

        // Select first device as default if available
        let selected_input_device = input_devices.first().cloned();
        let selected_output_device = output_devices.first().cloned();

        Ok(AudioContext {
            input_devices,
            output_devices,
            selected_input_device,
            selected_output_device,
        })
    }

    pub fn set_input_device(&mut self, device_name: String) {
        self.selected_input_device = Some(device_name);
    }

    pub fn set_output_device(&mut self, device_name: String) {
        self.selected_output_device = Some(device_name);
    }

    pub fn refresh_devices(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.input_devices = AudioDevice::list_input_devices().unwrap_or_default();
        self.output_devices = AudioDevice::list_output_devices().unwrap_or_default();
        Ok(())
    }
}

impl Default for AudioContext {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| AudioContext {
            input_devices: vec![],
            output_devices: vec![],
            selected_input_device: None,
            selected_output_device: None,
        })
    }
}
