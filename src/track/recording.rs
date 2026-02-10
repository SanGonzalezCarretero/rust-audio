use super::{downsample_bipolar, generate_clip_id, Clip, Track, TrackState, RECORDING_WAVEFORM_CHUNK_SIZE, WAVEFORM_MAX_POINTS};
use crate::wav::WavFile;
use ringbuf::{
    traits::{Consumer, Split},
    HeapCons, HeapProd, HeapRb,
};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex, RwLock,
};

impl Track {
    /// Prepare this track for recording: set up buffers and waveform thread.
    /// Does NOT open any audio device or stream — the Session owns the shared input stream.
    pub fn prepare_recording(&mut self, playhead_position: u64, sample_rate: u32, channels: u16) {
        self.stop_monitoring();

        self.recording_start_position = playhead_position;

        // Create a lock-free ring buffer: 1 second of audio = huge headroom vs 50ms waveform interval
        let ring_size = sample_rate as usize * channels as usize;
        let ring = HeapRb::<f32>::new(ring_size);
        let (producer, consumer) = ring.split();

        self.recording_producer = Some(producer);
        self.recording_channels = Some(channels);
        self.recording_sample_rate = Some(sample_rate);

        // Reset waveform result for new recording
        if let Ok(mut waveform) = self.waveform.write() {
            waveform.clear();
        }

        self.state = TrackState::Recording;

        // Reset stop flag and prepare for thread spawn
        self.waveform_thread.reset_waveform_stop();

        let waveform_clone = Arc::clone(&self.waveform);
        let should_stop = self.waveform_thread.waveform_stop();

        let handle = std::thread::spawn(move || {
            Self::waveform_thread(consumer, waveform_clone, should_stop)
        });

        self.waveform_thread.waveform = Some(handle);
    }

    /// Waveform background thread: drains ring buffer consumer into a local accumulator,
    /// computes waveform peaks, and returns all accumulated samples on exit.
    fn waveform_thread(
        mut consumer: HeapCons<f32>,
        waveform: Arc<RwLock<Vec<(f64, f64)>>>,
        should_stop: Arc<AtomicBool>,
    ) -> Vec<f32> {
        const UPDATE_INTERVAL_MS: u64 = 50;

        let mut all_samples: Vec<f32> = Vec::new();
        let mut unprocessed_offset: usize = 0; // index into all_samples of first unprocessed sample
        let mut drain_buf = vec![0.0f32; 4800]; // reusable drain buffer (~100ms at 48kHz)

        loop {
            // Drain everything available from the ring buffer
            loop {
                let n = consumer.pop_slice(&mut drain_buf);
                if n == 0 {
                    break;
                }
                all_samples.extend_from_slice(&drain_buf[..n]);
            }

            // Process complete chunks for waveform display
            let unprocessed = all_samples.len() - unprocessed_offset;
            let complete_chunks = unprocessed / RECORDING_WAVEFORM_CHUNK_SIZE;

            if complete_chunks > 0 {
                let process_up_to =
                    unprocessed_offset + complete_chunks * RECORDING_WAVEFORM_CHUNK_SIZE;
                let new_samples = &all_samples[unprocessed_offset..process_up_to];
                let samples_f64: Vec<f64> = new_samples.iter().map(|&s| s as f64).collect();

                let new_peaks =
                    downsample_bipolar(&samples_f64, RECORDING_WAVEFORM_CHUNK_SIZE, true);
                unprocessed_offset = process_up_to;

                if let Ok(mut wf) = waveform.write() {
                    wf.extend(new_peaks);
                }
            }

            if should_stop.load(Ordering::Relaxed) {
                // Final drain after stop signal
                loop {
                    let n = consumer.pop_slice(&mut drain_buf);
                    if n == 0 {
                        break;
                    }
                    all_samples.extend_from_slice(&drain_buf[..n]);
                }
                break;
            }

            std::thread::sleep(std::time::Duration::from_millis(UPDATE_INTERVAL_MS));
        }

        all_samples
    }

    /// Take ownership of the recording ring buffer producer (moved into the audio callback).
    pub fn take_recording_producer(&mut self) -> Option<HeapProd<f32>> {
        self.recording_producer.take()
    }

    /// Get a clone of the monitor buffer Arc.
    pub fn monitor_buffer_handle(&self) -> Arc<Mutex<Vec<f32>>> {
        Arc::clone(&self.monitor_buffer)
    }

    pub fn stop_recording(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.state != TrackState::Recording {
            return Err("Not currently recording".into());
        }

        // Signal waveform thread to stop
        self.waveform_thread.stop_waveform();

        // Join the waveform thread to receive all accumulated samples
        let samples = if let Some(handle) = self.waveform_thread.waveform.take() {
            handle.join().unwrap_or_default()
        } else {
            Vec::new()
        };

        if !samples.is_empty() {
            let channels = self.recording_channels.unwrap_or(1);
            let sample_rate = self.recording_sample_rate.unwrap_or(48000);

            let samples_f64: Vec<f64> = samples.iter().map(|&s| s as f64).collect();
            let mut wav = WavFile::new(sample_rate, channels);
            wav.from_f64_samples(&samples_f64);

            self.clips.push(Clip {
                id: generate_clip_id(&self.name),
                wav_data: wav,
                starts_at: self.recording_start_position,
            });
        }

        self.cache_waveform();

        self.recording_channels = None;
        self.recording_sample_rate = None;
        self.state = TrackState::Armed;
        Ok(())
    }

    /// Recompute the waveform cache from all clips.
    /// Called once after recording stops, or after loading clips from a project.
    pub(crate) fn cache_waveform(&self) {
        if self.clips.is_empty() {
            if let Ok(mut wf) = self.waveform.write() {
                wf.clear();
            }
            return;
        }

        let (mixed, _) = self.mix_clips(0);
        if mixed.is_empty() {
            if let Ok(mut wf) = self.waveform.write() {
                wf.clear();
            }
            return;
        }

        let chunk_size = (mixed.len() / WAVEFORM_MAX_POINTS).max(1);
        let peaks = downsample_bipolar(&mixed, chunk_size, false);
        if let Ok(mut wf) = self.waveform.write() {
            *wf = peaks;
        }
    }

    /// Read cached waveform data. During recording, the background thread
    /// populates the cache. For non-recording tracks, the cache is populated
    /// once when recording stops (via cache_waveform).
    pub fn waveform(&self) -> Option<Vec<(f64, f64)>> {
        if let Ok(waveform) = self.waveform.read() {
            if !waveform.is_empty() {
                return Some(waveform.clone());
            }
        }
        None
    }
}
