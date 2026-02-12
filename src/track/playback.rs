use super::Track;

impl Track {
    /// Render track audio: mix clips -> apply FX chain -> apply volume.
    /// Returns empty Vec for muted tracks or tracks with no clips.
    pub fn render(&self, from_sample: u64, sample_rate: u32) -> Vec<f32> {
        if self.muted || self.clips.is_empty() {
            return Vec::new();
        }
        let (buffer, _) = self.mix_clips(from_sample);
        let mut buffer = self.apply_fx(buffer, sample_rate);
        self.apply_volume(&mut buffer);
        buffer
    }

    /// Apply the FX chain to a buffer.
    /// Some effects (like Delay) may extend the buffer to include tails.
    fn apply_fx(&self, mut buffer: Vec<f32>, sample_rate: u32) -> Vec<f32> {
        if self.fx_chain.is_empty() {
            return buffer;
        }

        for effect in &self.fx_chain {
            let _ = effect.apply(&mut buffer, sample_rate);
        }

        buffer
    }

    /// Apply track volume to a buffer.
    fn apply_volume(&self, buffer: &mut [f32]) {
        let vol = self.volume as f32;
        for sample in buffer.iter_mut() {
            *sample *= vol;
        }
    }
}
