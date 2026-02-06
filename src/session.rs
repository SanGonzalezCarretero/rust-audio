use crate::track::Track;
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportState {
    Stopped,
    Playing,
}

pub struct Transport {
    pub state: TransportState,
    pub playhead_position: u64,
    playback_origin: u64,
    playback_start_time: Option<Instant>,
}

impl Default for Transport {
    fn default() -> Self {
        Transport {
            state: TransportState::Stopped,
            playhead_position: 0,
            playback_origin: 0,
            playback_start_time: None,
        }
    }
}

impl Transport {
    pub fn play(&mut self) {
        self.state = TransportState::Playing;
        self.playback_origin = self.playhead_position;
        self.playback_start_time = Some(Instant::now());
    }

    pub fn stop(&mut self) {
        self.state = TransportState::Stopped;
        self.playback_start_time = None;
    }

    pub fn is_playing(&self) -> bool {
        self.state == TransportState::Playing
    }

    pub fn move_playhead(&mut self, delta_samples: i64) {
        if delta_samples < 0 {
            self.playhead_position = self
                .playhead_position
                .saturating_sub(delta_samples.unsigned_abs());
        } else {
            self.playhead_position = self.playhead_position.saturating_add(delta_samples as u64);
        }
    }

    pub fn playhead_seconds(&self, sample_rate: u32) -> f64 {
        self.playhead_position as f64 / sample_rate as f64
    }

    pub fn advance_playhead(&mut self, sample_rate: u32) {
        if let Some(start_time) = self.playback_start_time {
            let elapsed_secs = start_time.elapsed().as_secs_f64();
            let elapsed_samples = (elapsed_secs * sample_rate as f64) as u64;
            self.playhead_position = self.playback_origin + elapsed_samples;
        }
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

    pub fn add_track(&mut self, name: String) -> Result<(), Box<dyn std::error::Error>> {
        const MAX_TRACKS: usize = 3;
        if self.tracks.len() >= MAX_TRACKS {
            return Err(format!("Maximum of {} tracks allowed", MAX_TRACKS).into());
        }
        self.tracks.push(Track::new(name));
        Ok(())
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
        let playhead_pos = self.transport.playhead_position;
        let sample_rate = self.sample_rate;

        // Start playback on all eligible tracks (don't let one failure stop others)
        let mut any_started = false;
        for track in &mut self.tracks {
            if !track.muted
                && !track.clips.is_empty()
                && track.play_from(playhead_pos, sample_rate).is_ok()
                && track.is_playing_track()
            {
                any_started = true;
            }
        }

        // Only set transport to Playing if at least one track started
        if any_started {
            self.transport.play();
        }

        Ok(())
    }

    pub fn stop_playback(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        for track in &mut self.tracks {
            track.stop_playback();
        }

        self.transport.stop();
        Ok(())
    }

    /// Start playback on non-recording tracks so the user hears them while recording (overdub).
    pub fn start_overdub_playback(&mut self) {
        let playhead_pos = self.transport.playhead_position;
        let sample_rate = self.sample_rate;

        let mut any_started = false;
        for track in &mut self.tracks {
            if track.state != crate::track::TrackState::Recording
                && !track.muted
                && !track.clips.is_empty()
                && track.play_from(playhead_pos, sample_rate).is_ok()
                && track.is_playing_track()
            {
                any_started = true;
            }
        }

        if any_started {
            self.transport.play();
        }
    }

    /// Stop recording on all tracks that are recording, and stop playback.
    pub fn stop_all_recording(&mut self) {
        for track in &mut self.tracks {
            if track.state == crate::track::TrackState::Recording {
                let _ = track.stop_recording();
            }
            track.stop_playback();
        }
        self.transport.stop();
    }

    pub fn check_playback_status(&mut self) {
        if self.transport.is_playing() {
            // Advance playhead based on elapsed time
            self.transport.advance_playhead(self.sample_rate);

            let all_finished = self.tracks.iter().all(|track| {
                if track.clips.is_empty() || track.muted {
                    true
                } else {
                    track.is_playback_finished()
                }
            });

            if all_finished {
                self.transport.stop();
            }
        }
    }

    pub fn remove_track(&mut self, index: usize) -> Result<(), Box<dyn std::error::Error>> {
        const MIN_TRACKS: usize = 1;
        if self.tracks.len() <= MIN_TRACKS {
            return Err("Cannot remove the last track".into());
        }

        if index >= self.tracks.len() {
            return Err("Track index out of bounds".into());
        }

        // Cleanup the track before removal
        self.tracks[index].cleanup();

        // Remove the track
        self.tracks.remove(index);

        Ok(())
    }
}
