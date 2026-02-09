use super::Track;

impl Track {
    pub fn start_monitoring(&mut self) {
        if self.is_armed() {
            self.monitoring = true;
        }
    }

    pub fn stop_monitoring(&mut self) {
        self.monitoring = false;
    }
}
