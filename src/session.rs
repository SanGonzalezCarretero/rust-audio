use crate::track::Track;
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportState {
    Stopped,
    Playing,
}

pub struct Transport {
    pub state: TransportState,
    playback_start_time: Option<Instant>,
}

impl Default for Transport {
    fn default() -> Self {
        Transport {
            state: TransportState::Stopped,
            playback_start_time: None,
        }
    }
}

impl Transport {
    pub fn play(&mut self) {
        self.state = TransportState::Playing;
        self.playback_start_time = Some(Instant::now());
    }

    pub fn stop(&mut self) {
        self.state = TransportState::Stopped;
        self.playback_start_time = None;
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
            transport: Transport::default(),
        }
    }

    pub fn add_track(&mut self, name: String) {
        self.tracks.push(Track::new(name));
    }

    pub fn get_track_mut(&mut self, index: usize) -> Option<&mut Track> {
        self.tracks.get_mut(index)
    }

    pub fn get_track(&self, index: usize) -> Option<&Track> {
        self.tracks.get(index)
    }

    pub fn track_count(&self) -> usize {
        self.tracks.len()
    }

    pub fn toggle_playback(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.transport.is_playing() {
            self.stop_playback()
        } else {
            self.start_playback()
        }
    }

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

    pub fn stop_playback(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.transport.stop();
        Ok(())
    }
}
