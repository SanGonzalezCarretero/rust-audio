use super::Track;

impl Track {
    /// Return volume-applied samples from `from_sample` to end of clips.
    /// Returns empty Vec for muted tracks or tracks with no clips.
    pub fn render(&self, from_sample: u64) -> Vec<f32> {
        if self.muted || self.clips.is_empty() {
            return Vec::new();
        }
        let (mixed, _) = self.mix_clips(from_sample);
        mixed.iter().map(|&s| s * self.volume as f32).collect()
    }
}
