use crate::track::Track;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

/// Transport state for the entire session
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportState {
    Stopped,
    Playing,
}

/// Transport manages playback state for the session
pub struct Transport {
    pub state: TransportState,
    pub playback_position: Arc<Mutex<u64>>, // Position in bytes, shared for updates
    playback_start_time: Option<Instant>,
}

impl Transport {
    pub fn new() -> Self {
        Transport {
            state: TransportState::Stopped,
            playback_position: Arc::new(Mutex::new(0)),
            playback_start_time: None,
        }
    }

    pub fn play(&mut self) {
        self.state = TransportState::Playing;
        self.playback_start_time = Some(Instant::now());
    }

    pub fn stop(&mut self) {
        self.state = TransportState::Stopped;
        self.playback_start_time = None;
        if let Ok(mut pos) = self.playback_position.lock() {
            *pos = 0;
        }
    }

    pub fn is_playing(&self) -> bool {
        self.state == TransportState::Playing
    }
}

pub struct Session {
    pub name: String,
    pub tracks: Vec<Track>,
    pub sample_rate: u32,
    pub transport: Transport,
}

impl Session {
    pub fn new(name: String, sample_rate: u32) -> Self {
        Session {
            name,
            tracks: Vec::new(),
            sample_rate,
            transport: Transport::new(),
        }
    }

    /// Add a new track to the session
    pub fn add_track(&mut self, name: String) {
        self.tracks.push(Track::new(name));
    }

    /// Get a track by index (mutable)
    pub fn get_track_mut(&mut self, index: usize) -> Option<&mut Track> {
        self.tracks.get_mut(index)
    }

    /// Get a track by index (immutable)
    pub fn get_track(&self, index: usize) -> Option<&Track> {
        self.tracks.get(index)
    }

    /// Get number of tracks
    pub fn track_count(&self) -> usize {
        self.tracks.len()
    }

    /// Toggle between play and stop
    pub fn toggle_playback(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.transport.is_playing() {
            self.stop_playback()
        } else {
            self.start_playback()
        }
    }

    /// Start playing all tracks from current position
    pub fn start_playback(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.transport.play();
        
        // Find longest track duration for progress tracking
        let max_bytes = self.tracks.iter()
            .filter_map(|t| t.wav_data.as_ref())
            .map(|data| data.audio_data.len())
            .max()
            .unwrap_or(0);
        
        // Get sample rate from first track (assuming all tracks same rate)
        let sample_rate = self.tracks.iter()
            .filter_map(|t| t.wav_data.as_ref())
            .map(|data| data.header.sample_rate)
            .next()
            .unwrap_or(44100);
        
        // Calculate duration in seconds
        let bytes_per_sample = 2; // Assuming 16-bit audio
        let channels = 2; // Assuming stereo
        let bytes_per_second = sample_rate as usize * bytes_per_sample * channels;
        let duration_secs = max_bytes as f64 / bytes_per_second as f64;
        
        // Spawn thread to update playback position
        let position = Arc::clone(&self.transport.playback_position);
        let max_bytes_copy = max_bytes;
        
        thread::spawn(move || {
            let start = Instant::now();
            while start.elapsed().as_secs_f64() < duration_secs {
                thread::sleep(Duration::from_millis(50)); // Update every 50ms
                let elapsed = start.elapsed().as_secs_f64();
                let progress = (elapsed / duration_secs).min(1.0);
                let byte_position = (progress * max_bytes_copy as f64) as u64;
                
                if let Ok(mut pos) = position.lock() {
                    *pos = byte_position;
                } else {
                    break;
                }
            }
        });
        
        // Play all non-muted tracks
        for track in &self.tracks {
            if !track.muted && track.wav_data.is_some() {
                track.play()?;
            }
        }
        
        Ok(())
    }

    /// Calculate playback progress as a ratio (0.0 to 1.0)
    pub fn playback_progress(&self) -> f64 {
        // Find longest track duration in bytes
        let max_bytes = self.tracks.iter()
            .filter_map(|t| t.wav_data.as_ref())
            .map(|data| data.audio_data.len())
            .max()
            .unwrap_or(0);

        if max_bytes == 0 {
            return 0.0;
        }

        let position = self.transport.playback_position.lock().unwrap();
        (*position as f64 / max_bytes as f64).min(1.0)
    }

    /// Stop playback and reset position
    pub fn stop_playback(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.transport.stop();
        // Individual track playback will naturally stop when audio completes
        Ok(())
    }
}
