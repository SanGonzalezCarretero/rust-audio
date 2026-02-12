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
    /// Effects use f64 internally, so we convert back and forth.
    /// Some effects (like Delay) may extend the buffer to include tails.
    fn apply_fx(&self, buffer: Vec<f32>, sample_rate: u32) -> Vec<f32> {
        if self.fx_chain.is_empty() {
            return buffer;
        }

        // Convert to f64 for effect processing
        let mut samples_f64: Vec<f64> = buffer.iter().map(|&s| s as f64).collect();

        for effect in &self.fx_chain {
            let _ = effect.apply(&mut samples_f64, sample_rate);
        }

        // Convert back to f32 (may be longer than original due to effect tails)
        samples_f64.iter().map(|&s| s as f32).collect()
    }

    /// Apply track volume to a buffer.
    fn apply_volume(&self, buffer: &mut [f32]) {
        let vol = self.volume as f32;
        for sample in buffer.iter_mut() {
            *sample *= vol;
        }
    }
}
