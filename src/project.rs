use crate::audio_engine::AudioEngine;
use crate::effects::EffectType;
use crate::session::Session;
use crate::track::Track;
use crate::wav::WavFile;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use strum::IntoEnumIterator;

#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectManifest {
    pub name: String,
    pub sample_rate: u32,
    pub tracks: Vec<TrackManifest>,
    #[serde(default)]
    pub audio_preferences: Option<AudioPreferencesManifest>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AudioPreferencesManifest {
    pub input_device: Option<String>,
    pub output_device: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TrackManifest {
    pub name: String,
    pub volume: f64,
    pub muted: bool,
    pub clips: Vec<ClipManifest>,
    pub fx_chain: Vec<FxManifest>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ClipManifest {
    pub id: String,
    pub file: String, // "clips/{id}.wav"
    pub starts_at: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FxManifest {
    pub effect_type: String,
    pub parameters: Vec<(String, String)>,
}

pub fn save_project(
    session: &Session,
    project_dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all(project_dir)?;

    let clips_dir = project_dir.join("clips");
    fs::create_dir_all(&clips_dir)?;

    let mut track_manifests = Vec::new();
    for track in session.tracks.iter() {
        let mut clip_manifests = Vec::new();

        // Save each clip as a WAV file using its unique ID
        for clip in track.clips.iter() {
            let clip_filename = format!("{}.wav", clip.id);
            let clip_path = clips_dir.join(&clip_filename);

            // Only write the file if it doesn't already exist (incremental save)
            if !clip_path.exists() {
                let mut wav = clip.wav_data.clone();
                wav.save_to_file(&clip_path)?;
            }

            clip_manifests.push(ClipManifest {
                id: clip.id.clone(),
                file: format!("clips/{}", clip_filename),
                starts_at: clip.starts_at,
            });
        }

        let fx_manifests: Vec<FxManifest> = track
            .fx_chain
            .iter()
            .map(|fx| FxManifest {
                effect_type: fx.effect_type().name(),
                parameters: fx.parameters(),
            })
            .collect();

        track_manifests.push(TrackManifest {
            name: track.name.clone(),
            volume: track.volume,
            muted: track.muted,
            clips: clip_manifests,
            fx_chain: fx_manifests,
        });
    }

    let audio_preferences = {
        let engine = AudioEngine::global();
        let engine = engine.lock().unwrap();
        AudioPreferencesManifest {
            input_device: engine.selected_input().map(String::from),
            output_device: engine.selected_output().map(String::from),
        }
    };

    let manifest = ProjectManifest {
        name: session.name.clone(),
        sample_rate: session.sample_rate,
        tracks: track_manifests,
        audio_preferences: Some(audio_preferences),
    };

    let manifest_path = project_dir.join("project.json");
    let json = serde_json::to_string_pretty(&manifest)?;
    fs::write(manifest_path, json)?;

    Ok(())
}

pub fn load_project(project_dir: &Path) -> Result<Session, Box<dyn std::error::Error>> {
    let manifest_path = project_dir.join("project.json");
    let json = fs::read_to_string(manifest_path)?;
    let manifest: ProjectManifest = serde_json::from_str(&json)?;

    let mut tracks = Vec::new();
    for track_manifest in manifest.tracks {
        let mut track = Track::new(track_manifest.name);
        track.volume = track_manifest.volume;
        track.muted = track_manifest.muted;

        for clip_manifest in track_manifest.clips {
            let clip_path = project_dir.join(&clip_manifest.file);
            let wav = WavFile::load_from_file(clip_path)?;

            track.clips.push(crate::track::Clip {
                id: clip_manifest.id,
                wav_data: wav,
                starts_at: clip_manifest.starts_at,
            });
        }

        for fx_manifest in track_manifest.fx_chain {
            // Find the EffectType by name
            let effect_type = EffectType::iter()
                .find(|et| et.name() == fx_manifest.effect_type)
                .ok_or_else(|| format!("Unknown effect type: {}", fx_manifest.effect_type))?;

            let mut effect = effect_type.create_default();

            for (param_name, param_value) in fx_manifest.parameters {
                effect = effect
                    .update_parameter(&param_name, &param_value)
                    .map_err(|e| format!("Failed to set parameter {}: {}", param_name, e))?;
            }

            track.fx_chain.push(effect);
        }

        // Recompute waveform
        track.cache_waveform();

        tracks.push(track);
    }

    if let Some(prefs) = manifest.audio_preferences {
        let engine = AudioEngine::global();
        let mut engine = engine.lock().unwrap();
        if let Some(input) = prefs.input_device {
            engine.set_input_device(input);
        }
        if let Some(output) = prefs.output_device {
            engine.set_output_device(output);
        }
    }

    let mut session = Session::new(manifest.name, manifest.sample_rate);
    session.tracks = tracks;

    Ok(session)
}

pub fn is_inside_project(dir: &Path) -> bool {
    let mut current = Some(dir);
    while let Some(d) = current {
        if d.join("project.json").exists() {
            return true;
        }
        current = d.parent();
    }
    false
}

pub fn list_projects(cwd: &Path) -> Vec<String> {
    let mut projects = Vec::new();

    if let Ok(entries) = fs::read_dir(cwd) {
        for entry in entries.flatten() {
            if let Ok(metadata) = entry.metadata() {
                if metadata.is_dir() {
                    let project_json = entry.path().join("project.json");
                    if project_json.exists() {
                        if let Some(name) = entry.file_name().to_str() {
                            projects.push(name.to_string());
                        }
                    }
                }
            }
        }
    }

    projects.sort();
    projects
}
