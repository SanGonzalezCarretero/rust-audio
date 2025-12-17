use std::sync::{Arc, Mutex, OnceLock};

static DEBUG_LOGGER: OnceLock<Arc<Mutex<DebugLogger>>> = OnceLock::new();

#[derive(Clone)]
pub struct DebugLogger {
    logs: Arc<Mutex<Vec<String>>>,
    enabled: bool,
}

impl DebugLogger {
    /// Get the global DebugLogger instance
    pub fn global() -> Arc<Mutex<DebugLogger>> {
        DEBUG_LOGGER
            .get_or_init(|| Arc::new(Mutex::new(DebugLogger::new(false))))
            .clone()
    }

    /// Initialize the global DebugLogger with a specific enabled state
    /// This should be called early in the program, before any logging occurs
    pub fn init(enabled: bool) {
        DEBUG_LOGGER.get_or_init(|| Arc::new(Mutex::new(DebugLogger::new(enabled))));
    }

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

    /// Convenience method to log from anywhere without holding a reference
    pub fn log_global(message: impl Into<String>) {
        let logger_arc = Self::global();
        if let Ok(logger) = logger_arc.lock() {
            let msg = message.into();
            if logger.enabled {
                if let Ok(mut logs) = logger.logs.lock() {
                    logs.push(msg);
                    if logs.len() > 3 {
                        logs.remove(0);
                    }
                }
            }
        };
    }
}
