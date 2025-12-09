use crate::track::Track;

/// Transport state for the entire session
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportState {
    Stopped,
    Playing,
    Recording, // For future global recording
}

/// Transport manages playback state for the session
pub struct Transport {
    pub state: TransportState,
    pub playback_position: u64, // Position in samples
}

impl Transport {
    pub fn new() -> Self {
        Transport {
            state: TransportState::Stopped,
            playback_position: 0,
        }
    }

    pub fn play(&mut self) {
        self.state = TransportState::Playing;
    }

    pub fn stop(&mut self) {
        self.state = TransportState::Stopped;
        self.playback_position = 0;
    }

    pub fn is_playing(&self) -> bool {
        self.state == TransportState::Playing
    }
}

/// Session represents a project with multiple tracks
pub struct Session {
    pub name: String,
    pub tracks: Vec<Track>,
    pub sample_rate: u32,
    pub transport: Transport,
}

impl Session {
    /// Create a new session with a given sample rate
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
        
        // Play all non-muted tracks
        for track in &self.tracks {
            if !track.muted && track.wav_data.is_some() {
                track.play()?;
            }
        }
        
        Ok(())
    }

    /// Stop playback and reset position
    pub fn stop_playback(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.transport.stop();
        // Individual track playback will naturally stop when audio completes
        Ok(())
    }
}
