use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct DebugLogger {
    logs: Arc<Mutex<Vec<String>>>,
    enabled: bool,
}

impl DebugLogger {
    pub fn new(enabled: bool) -> Self {
        DebugLogger {
            logs: Arc::new(Mutex::new(Vec::new())),
            enabled,
        }
    }

    pub fn log(&self, message: String) {
        if self.enabled {
            if let Ok(mut logs) = self.logs.lock() {
                logs.push(message);
                if logs.len() > 3 {
                    logs.remove(0);
                }
            }
        }
    }

    pub fn get_logs(&self) -> Vec<String> {
        if let Ok(logs) = self.logs.lock() {
            logs.clone()
        } else {
            Vec::new()
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
}
