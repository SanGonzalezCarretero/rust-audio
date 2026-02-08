# rust-audio Codebase Deep Dive

## Table of Contents

1. [What This Project Is](#1-what-this-project-is)
2. [Architecture Overview](#2-architecture-overview)
3. [Module Map & Dependency Graph](#3-module-map--dependency-graph)
4. [Core Data Structures](#4-core-data-structures)
5. [The Recording Pipeline](#5-the-recording-pipeline)
6. [The Playback Pipeline](#6-the-playback-pipeline)
7. [The Drawing / UI Pipeline](#7-the-drawing--ui-pipeline)
8. [Thread Model & Shared State](#8-thread-model--shared-state)
9. [Audio Device Management](#9-audio-device-management)
10. [Effects System](#10-effects-system)
11. [WAV File Handling](#11-wav-file-handling)
12. [Strong Points](#12-strong-points)
13. [Weak Points & Technical Debt](#13-weak-points--technical-debt)
14. [Necessary Changes for a Production DAW](#14-necessary-changes-for-a-production-daw)
15. [Constants Reference](#15-constants-reference)

---

## 1. What This Project Is

A terminal-based Digital Audio Workstation (DAW) built entirely in Rust. It renders a multi-track timeline in the terminal using `ratatui`, captures audio via the system microphone using `cpal`, plays it back through speakers, and supports a chain of audio effects. Think of it as a proof-of-concept Ableton/Logic but inside your terminal.

**Tech stack:**

- **cpal** - cross-platform audio I/O (recording and playback streams)
- **ratatui + crossterm** - terminal UI framework
- **ringbuf** - lock-free ring buffer for low-latency monitoring
- **rustfft** - FFT for frequency-domain effects
- **strum** - enum iteration for the effects registry

---

## 2. Architecture Overview

```
                    +-----------+
                    |  main.rs  |  Entry point, parses --debug flag
                    +-----+-----+
                          |
                    +-----v-----+
                    |  ui/mod   |  Event loop: poll input -> handle -> render
                    +-----+-----+
                          |
            +-------------+-------------+
            |             |             |
     +------v------+ +---v---+ +------v-------+
     | daw_screen  | |effects| |audio_prefs   |
     | (timeline,  | |screen | |(device       |
     |  tracks,    | |       | | selection)   |
     |  controls)  | +-------+ +--------------+
     +------+------+
            |
     +------v------+
     |  Session    |  Owns tracks[], transport, shared_input_stream
     +------+------+
            |
     +------v------+
     |   Track     |  Owns clips[], streams, buffers, state
     +------+------+
            |
    +-------+-------+
    |               |
+---v---+     +-----v-----+
|  WAV  |     |  Effects  |
| files |     |  chain    |
+-------+     +-----------+
```

The app follows a straightforward **single-threaded event loop** on the main thread, with **spawned std::threads** for audio playback and waveform processing. There is no async runtime.

---

## 3. Module Map & Dependency Graph

```
src/
  main.rs                    Entry point
  lib.rs                     Module declarations
  audio_engine.rs            Singleton device manager
  session.rs                 Session + Transport (owns tracks)
  device/mod.rs              cpal device abstraction
  track/mod.rs               Track, Clip, recording/playback logic
  processor/mod.rs           FFT-based audio processor
  wav/mod.rs                 WAV file read/write/convert
  effects/
    mod.rs                   EffectTrait, EffectInstance, EffectType registry
    adjust_volume.rs         Volume scaling
    reverse.rs               Reverse samples
    duplicate.rs             Double the audio
    random_noise.rs          Add noise
    delay.rs                 Multi-tap delay
    tremolo.rs               Amplitude modulation
    pitch_octave_up.rs       Primitive pitch shift
    tape_saturation.rs       Soft clipping via tanh
    large_reverb.rs          Comb filter reverb
    pan_left.rs              Pan left (stereo)
    pan_right.rs             Pan right (stereo)
  ui/
    mod.rs                   App struct, run() loop
    screen_trait.rs          ScreenTrait interface
    view.rs                  Layout routing (title, content, status, debug)
    event_handler.rs         Input polling and dispatch
    main_menu_screen.rs      Main menu
    daw_screen.rs            Timeline, tracks, transport controls
    effects_screen.rs        Effect selection & parameter editing
    audio_preferences_screen Audio device picker
    debug_logger.rs          In-app log overlay
```

**Who depends on whom:**

| Module         | Depends On                                                    |
| -------------- | ------------------------------------------------------------- |
| `main`         | `ui`                                                          |
| `ui`           | `session`, `audio_engine`, `effects`, `wav`                   |
| `daw_screen`   | `session`, `track` (types/constants)                          |
| `session`      | `track`, `audio_engine`, `cpal`                               |
| `track`        | `audio_engine`, `device`, `effects`, `wav`, `cpal`, `ringbuf` |
| `processor`    | `effects`, `rustfft`                                          |
| `wav`          | `effects`                                                     |
| `effects`      | nothing (leaf module)                                         |
| `device`       | `cpal`                                                        |
| `audio_engine` | `device`                                                      |

---

## 4. Core Data Structures

### Session (`session.rs`)

The top-level container. One session = one project.

```
Session
  |-- name: String                        ("Untitled Project")
  |-- tracks: Vec<Track>                  (1 to 3 tracks)
  |-- sample_rate: u32                    (usually 48000)
  |-- transport: Transport                (playhead, play/stop state)
  |-- shared_input_stream: Option<Stream> (one stream for all recording tracks)
```

### Transport (`session.rs`)

Manages the playhead and playback timing.

```
Transport
  |-- state: TransportState     (Stopped | Playing)
  |-- playhead_position: u64    (absolute sample position)
  |-- playback_origin: u64      (position when Play was pressed)
  |-- playback_start_time: Option<Instant>
```

The playhead advances using wall-clock time: `origin + (elapsed_secs * sample_rate)`. This means it drifts independently of actual audio stream progress.

### Track (`track/mod.rs`)

The core unit. Each track can be armed, recording, playing, or idle.

```
Track
  |-- name, armed, monitoring, state (Idle|Armed|Recording)
  |-- fx_chain: Vec<EffectInstance>
  |
  |-- clips: Vec<Clip>                         # Recorded audio segments
  |-- recording_start_position: u64            # Timeline position where rec started
  |-- volume: f64, muted: bool
  |
  |-- monitor_buffer: Arc<Mutex<Vec<f32>>>     # Live input samples for UI display
  |-- recording_buffer: Option<Arc<RwLock<Vec<f32>>>>  # Growing vec during recording
  |-- waveform: Arc<RwLock<Vec<(f64,f64)>>>    # Downsampled peaks for drawing
  |
  |-- input_stream: Option<Stream>             # Monitoring input stream (per-track)
  |-- output_stream: Option<Stream>            # Monitoring output stream (per-track)
  |-- playback_thread: Option<JoinHandle<()>>  # Spawned playback thread
  |-- is_playing: Arc<AtomicBool>              # Signal to stop playback thread
  |
  |-- thread_handles: ThreadHandles            # Waveform background thread + stop flag
  |-- recording_channels: Option<u16>
  |-- recording_sample_rate: Option<u32>
```

### Clip (`track/mod.rs`)

A segment of recorded or loaded audio placed on the timeline.

```
Clip
  |-- wav_data: WavFile      # The actual audio
  |-- starts_at: u64         # Sample position on the timeline
```

### TrackState

```
Idle     -> the track is doing nothing
Armed    -> the track is ready to record (monitoring may be on)
Recording -> audio is being captured into recording_buffer
```

---

## 5. The Recording Pipeline

This is the most complex flow in the codebase. Here it is step-by-step:

### Step 1: User presses 'r' (daw_screen.rs)

```
KeyCode::Char('r') -> session.start_recording()
```

### Step 2: Session creates ONE shared input stream (session.rs)

```rust
// 1. Get the input device ONCE (avoids per-track OS calls)
let input_device = AudioEngine::get_input_device()?;

// 2. Configure buffer size from LATENCY_MS (10ms)
config.buffer_size = BufferSize::Fixed(recording_buffer_size);

// 3. Prepare each armed track (sets up buffers + waveform thread)
for track in armed_tracks {
    track.prepare_recording(playhead_pos, sample_rate, channels);
}

// 4. Collect Arc handles to all recording + monitor buffers
// 5. Build ONE cpal input stream whose callback fans data to ALL tracks
// 6. stream.play() - start capturing
// 7. Start overdub playback on non-recording tracks
```

### Step 3: Track prepares its buffers (track/mod.rs:prepare_recording)

```rust
// 1. Stop monitoring (releases per-track input/output streams)
self.stop_monitoring();

// 2. Create empty recording buffer wrapped in Arc<RwLock>
self.recording_buffer = Some(Arc::new(RwLock::new(Vec::new())));

// 3. Store sample_rate and channels for later WAV conversion
// 4. Clear waveform, set state to Recording
// 5. Spawn waveform processing thread (see below)
```

### Step 4: The shared input stream callback fires (~every 10ms)

Every time the OS delivers a buffer of audio samples, this closure runs:

```rust
move |data: &[f32], _: &cpal::InputCallbackInfo| {
    // Fan out to ALL recording tracks:
    for mon in &mon_buffers {
        mon.try_lock() -> truncate + extend_from_slice(data)  // for UI
    }
    for rec in &rec_buffers {
        rec.write() -> extend_from_slice(data)  // for recording
    }
}
```

**Key detail:** `monitor_buffer` uses `try_lock()` (non-blocking) because this callback runs on a real-time audio thread. Blocking would cause audio glitches. The `recording_buffer` uses `.write()` (blocking) because losing recording data is worse than a brief stall.

### Step 5: Waveform thread processes samples in the background

Spawned during `prepare_recording()`, runs in a loop:

```
Every 50ms:
  1. Check stop flag (AtomicBool)
  2. Read recording_buffer (RwLock read lock)
  3. Count complete chunks of 960 samples (~20ms @ 48kHz)
  4. For each chunk: compute (min_peak, max_peak)
  5. Append to self.waveform (RwLock write lock)
```

This thread exists so the UI can show a growing waveform in real-time without blocking the audio callback.

### Step 6: User presses 'r' again to stop

```
session.stop_all_recording():
  1. Drop shared_input_stream = None   (stops OS audio capture)
  2. For each recording track:
     a. Signal waveform thread to stop (AtomicBool)
     b. Join waveform thread (wait for it to finish)
     c. Take recording_buffer out of the track
     d. Convert Vec<f32> -> Vec<f64> -> WavFile
     e. Create Clip { wav_data, starts_at } and push to track.clips
     f. Set state back to Armed
  3. Stop overdub playback on all tracks
  4. Stop transport
```

### Data flow diagram:

```
  Microphone
      |
  [OS audio driver]
      |
  [cpal input stream callback]  <-- runs on audio thread, ~every 10ms
      |
      +---> track1.monitor_buffer (Arc<Mutex>)  ---> UI reads for live meter
      +---> track1.recording_buffer (Arc<RwLock>) ---> waveform thread reads
      |                                                      |
      +---> track2.monitor_buffer                     downsample to peaks
      +---> track2.recording_buffer                          |
      |                                               track.waveform (Arc<RwLock>)
      +---> track3.monitor_buffer                            |
      +---> track3.recording_buffer                   UI reads for waveform drawing
```

---

## 6. The Playback Pipeline

### Step 1: User presses Space (daw_screen.rs)

```
KeyCode::Char(' ') -> session.toggle_playback()
                    -> session.start_playback()
```

### Step 2: Session iterates tracks (session.rs)

```rust
for track in &mut self.tracks {
    if !track.muted && !track.clips.is_empty() {
        track.play_from(playhead_pos, sample_rate);
    }
}
self.transport.play();  // start advancing playhead
```

### Step 3: Track mixes clips and spawns a playback thread (track/mod.rs:play_from)

```rust
// 1. Stop any existing playback
// 2. Find the end of the furthest clip
// 3. Build mixed buffer: all clips overlaid from playhead_position to end
// 4. Apply volume scaling
// 5. Spawn a new std::thread:
//    a. Get output device
//    b. Build output stream with callback
//    c. Callback reads sequentially from samples array
//    d. stream.play()
//    e. Sleep-loop until duration elapsed or is_playing flag cleared
//    f. Set is_playing = false
```

### Step 4: The output callback fires repeatedly

```rust
move |data: &mut [f32], _| {
    for frame in data.iter_mut() {
        *frame = if sample_idx < samples.len() {
            samples[sample_idx] as f32
        } else {
            0.0
        };
        sample_idx += 1;
    }
}
```

### Step 5: Transport advances the playhead

In the main UI loop, `check_playback_status()` runs every 100ms:

```rust
transport.advance_playhead(sample_rate);
// playhead = origin + (elapsed_time * sample_rate)
```

When all tracks report `is_playback_finished()`, transport stops.

### Step 6: User presses Space again to stop

```
session.stop_playback():
  for track in tracks:
    track.is_playing.store(false)  // signal thread
    drop(playback_thread handle)   // don't wait
  transport.stop()
```

### Data flow diagram:

```
  track.clips[0..n]
      |
  [mix all clips from playhead to end]
      |
  Vec<f64> samples (pre-mixed, volume-applied)
      |
  [spawned std::thread]
      |
  [cpal output stream callback]  <-- reads sequentially from samples
      |
  [OS audio driver]
      |
  Speakers
```

**Important:** Each track gets its own playback thread and its own output stream. There is no master mix bus. Tracks play independently and mix acoustically through the OS audio mixer.

---

## 7. The Drawing / UI Pipeline

### The Main Loop (ui/mod.rs:run)

```
loop {
    terminal.draw(|f| AppView::render(f, &app, area))   // draw frame
    if AppEventHandler::process_events(&mut app)?        // handle input
        { break; }                                        // quit if requested
}
```

This runs at ~10 FPS (100ms poll timeout in `event_handler.rs`).

### Frame Layout (view.rs + daw_screen.rs)

```
+--------------------------------------------------+
| Title bar: "rust-audio | Screen: Daw"            |  3 lines
+--------------------------------------------------+
| Transport: "Playing 00:03.50" [progress gauge]   |  3 lines
+--------------------------------------------------+
| Instructions: "n: Add | d: Delete | ..."         |  1 line
+--------------------------------------------------+
| Track 1 | Vol: 100% | ARMED | Empty              |
|  ---- timeline with waveform + playhead ----     |  1/3 height
|                                                  |
+--------------------------------------------------+
| Track 2 | Vol: 100% | ACTIVE | song.wav          |
|  ---- timeline with waveform + playhead ----     |  1/3 height
|                                                  |
+--------------------------------------------------+
| Track 3 | Vol: 80% | MUTED | drums.wav           |
|  ---- timeline with waveform + playhead ----     |  1/3 height
|                                                  |
+--------------------------------------------------+
| Status: "Recording 2 armed track(s)"             |  1 line
+--------------------------------------------------+
```

### Track Canvas Rendering

Each track renders a `ratatui::Canvas` widget:

```
X-axis: 0 to (sample_rate * 20) = 20 seconds of timeline
Y-axis: -1.0 to 1.0 (audio amplitude)

Drawn elements:
  1. Center line (y=0, DarkGray) - timeline axis
  2. Second markers - ticks at each second, taller at 5s intervals
  3. Waveform - vertical lines from min_peak to max_peak
     - During recording: fixed chunk positioning (stable left-to-right growth)
     - During idle: proportional scaling across clip duration
  4. Playhead - cyan vertical line at current position
```

### How waveform data reaches the screen

**During recording:**

```
recording_buffer (written by audio callback)
    -> waveform thread reads every 50ms
    -> downsamples to (min, max) tuples at RECORDING_WAVEFORM_CHUNK_SIZE
    -> writes to track.waveform (Arc<RwLock>)
    -> UI reads track.waveform() every ~100ms
    -> Canvas draws vertical lines with 8x sensitivity boost
```

**During idle (clips loaded):**

```
track.clips[].wav_data.to_f64_samples()
    -> composite all clips into one mixed buffer
    -> downsample_bipolar() to 500 points
    -> Canvas draws proportionally scaled vertical lines
```

### Border color logic

- **Yellow** - track is actively playing audio
- **Magenta** - track is recording
- **Red** - track is armed (ready to record)
- **Yellow** - track is selected (when not playing)
- **White** - default

---

## 8. Thread Model & Shared State

### Thread inventory

| Thread                 | Created by                  | Purpose                            | Lifetime               |
| ---------------------- | --------------------------- | ---------------------------------- | ---------------------- |
| Main thread            | OS                          | UI loop, event handling, rendering | App lifetime           |
| cpal input callback    | `session.start_recording()` | Capture mic audio, fan to buffers  | While recording        |
| cpal monitoring input  | `track.start_monitoring()`  | Route mic to ring buffer           | While armed+monitoring |
| cpal monitoring output | `track.start_monitoring()`  | Route ring buffer to speakers      | While armed+monitoring |
| Waveform thread        | `track.prepare_recording()` | Downsample recording for display   | While recording        |
| Playback thread        | `track.play_from()`         | Play audio through output stream   | While playing          |

**Note:** cpal callbacks run on OS-managed audio threads, not `std::thread::spawn`. They have real-time constraints.

### Shared state map

```
                        Main Thread (UI)
                             |
              reads          |          reads
         +-------------------+-------------------+
         |                                       |
   track.waveform                          track.monitor_buffer
   Arc<RwLock<Vec<(f64,f64)>>>             Arc<Mutex<Vec<f32>>>
         |                                       |
      writes                                  writes
         |                                       |
   Waveform Thread                    cpal Input Callback
         |                                       |
      reads                                   writes
         |                                       |
   track.recording_buffer              track.recording_buffer
   Arc<RwLock<Vec<f32>>>               Arc<RwLock<Vec<f32>>>
```

### Synchronization primitives

| Primitive                     | Where              | Why                                                                   |
| ----------------------------- | ------------------ | --------------------------------------------------------------------- |
| `Arc<Mutex<Vec<f32>>>`        | `monitor_buffer`   | UI needs latest input samples; `try_lock` in callback avoids blocking |
| `Arc<RwLock<Vec<f32>>>`       | `recording_buffer` | Multiple readers (waveform thread), one writer (audio callback)       |
| `Arc<RwLock<Vec<(f64,f64)>>>` | `waveform`         | Waveform thread writes, UI reads                                      |
| `Arc<AtomicBool>`             | `is_playing`       | Lock-free playback stop signal                                        |
| `Arc<AtomicBool>`             | `waveform_stop`    | Lock-free waveform thread stop signal                                 |
| `HeapRb` (ringbuf)            | monitoring         | Lock-free SPSC queue between input and output callbacks               |
| `OnceLock<Arc<Mutex<>>>`      | `AudioEngine`      | Thread-safe singleton                                                 |

---

## 9. Audio Device Management

### AudioEngine (audio_engine.rs)

A **singleton** managing which input/output devices are selected. Uses `OnceLock` for lazy initialization.

```
AudioEngine::global() -> Arc<Mutex<AudioEngine>>
AudioEngine::get_input_device() -> Result<AudioDevice>
AudioEngine::get_output_device() -> Result<AudioDevice>
```

### AudioDevice (device/mod.rs)

Wraps a `cpal::Device` with its `StreamConfig`:

```rust
AudioDevice {
    device: cpal::Device,     // the OS audio device handle
    config: StreamConfig,     // sample rate, channels, buffer size
    sample_rate: u32,
    channels: u16,
}
```

Two marker types (`Input`, `Output`) implement a `DeviceProvider` trait, giving uniform access to device listing, default selection, and lookup by name/index.

### Device selection flow

1. `AudioEngine::new()` enumerates all devices at startup
2. Tries to select hardcoded default devices ("Microfono de MacBook Pro")
3. Falls back to first available device
4. User can change via Audio Preferences screen
5. `get_input_device()` / `get_output_device()` lock the singleton and return an `AudioDevice`

---

## 10. Effects System

### Architecture

```
EffectTrait (trait)
    |-- name(), new(), parameters(), apply(), update_parameter_boxed()
    |
    |-- impl: AdjustVolume, Reverse, Duplicate, RandomNoise,
    |         Delay, Tremolo, PitchOctaveUp, TapeSaturation,
    |         LargeReverb, PanLeft, PanRight

EffectType (enum)
    |-- maps variant to concrete type
    |-- create_default() -> EffectInstance

EffectInstance (wrapper)
    |-- effect: Box<dyn EffectTrait>
    |-- effect_type: EffectType
```

### How effects are applied

Effects are **applied destructively** to WAV sample data:

```rust
fn apply(&self, samples: &mut Vec<f64>, sample_rate: u32) -> Result<(), &'static str>
```

They modify the samples in-place. Some effects change the length (Duplicate doubles it, PitchOctaveUp halves it).

### Parameter system

Effects expose parameters as `Vec<(String, String)>` key-value pairs. Updates create a new instance (immutable pattern):

```rust
effect.update_parameter("volume", "0.5") -> Result<EffectInstance, String>
```

### Where effects live

- `Track.fx_chain: Vec<EffectInstance>` - per-track effect chain
- `App.selected_effects: Vec<EffectInstance>` - effects selected in the effects screen
- Effects are applied to `WavFile.apply_effects()` which calls each effect's `apply()` sequentially

---

## 11. WAV File Handling

### WavFile (wav/mod.rs)

Handles 16-bit PCM WAV files.

```
WavFile
  |-- header: WavHeader (44 bytes of RIFF/fmt metadata)
  |-- audio_data: Vec<u8>  (raw i16 little-endian samples)
```

### Key operations

| Method                       | What it does                                      |
| ---------------------------- | ------------------------------------------------- |
| `new(sample_rate, channels)` | Create empty WAV                                  |
| `from_bytes(bytes)`          | Parse WAV file (handles chunk searching)          |
| `to_bytes()`                 | Serialize to WAV format                           |
| `to_f64_samples()`           | Convert i16 bytes to normalized f64 `[-1.0, 1.0]` |
| `from_f64_samples(samples)`  | Convert f64 back to i16 bytes                     |
| `apply_effects(effects)`     | Run effect chain on audio data                    |
| `export_to_bytes()`          | Finalize headers and export                       |

### Recording to WAV conversion

When recording stops:

```
Vec<f32> (recording_buffer)
  -> map to Vec<f64>
  -> WavFile::from_f64_samples()
  -> stores as i16 LE bytes in audio_data
  -> wrapped in Clip { wav_data, starts_at }
```

---

## 12. Strong Points

### Architecture & Design

1. **Shared input stream** - Recording uses ONE cpal input stream for all armed tracks. All tracks receive the exact same audio data at the exact same time, eliminating inter-track timing skew. This is how professional DAWs work.

2. **Lock-free monitoring** - The monitoring path uses `ringbuf` (a lock-free SPSC ring buffer) between input and output callbacks. This is the correct approach for real-time audio.

3. **Separation of concerns** - Clean module boundaries: `device` knows cpal, `track` knows recording/playback, `session` knows transport, `ui` knows rendering. Each module has a focused responsibility.

4. **RwLock for recording buffer** - Allows the waveform thread to read concurrently while the audio callback writes. Better than Mutex for this read-heavy pattern.

5. **Background waveform processing** - The waveform downsampling runs in a dedicated thread with fixed-size chunks, preventing UI stalls and giving smooth left-to-right waveform growth during recording.

6. **AtomicBool for control signals** - Lock-free stop signals for playback and waveform threads. No chance of deadlock on the hot path.

7. **Singleton AudioEngine** - Centralizes device management. Any part of the codebase can access the selected devices without passing references around.

8. **Effects trait system** - Clean abstraction. Adding a new effect requires implementing one trait and adding an enum variant. The parameter system supports runtime configuration.

9. **Fixed timeline canvas** - 20-second view with tick marks gives a familiar DAW-like visual reference.

10. **Minimal dependencies** - Only essential crates, no bloated framework.

### Code Quality

11. **Consistent error handling** - Uses `Result<T, Box<dyn std::error::Error>>` throughout for composable errors.

12. **Safe cleanup** - `Track::cleanup()` and `Session::stop_all_recording()` ensure streams and threads are properly torn down.

13. **try_lock in callbacks** - Audio callbacks use `try_lock()` on the monitor buffer, which correctly avoids blocking the real-time audio thread.

---

## 13. Weak Points & Technical Debt

### Critical Issues

1. **RwLock::write() in audio callback** - The recording buffer uses `.write()` (blocking) inside the cpal input callback. If the waveform thread holds a read lock, the audio callback will block, potentially causing audio dropouts. **Should use a lock-free structure (e.g., ring buffer or triple buffer) for the recording path too.**

2. **No master mix bus** - Each track opens its own output stream for playback. The OS mixes them. This means:
   - No control over the final mix (can't apply master effects)
   - No guarantee of sample-accurate synchronization between tracks
   - Multiple output streams consume more system resources
   - Volume/pan on the master bus is impossible

3. **Playhead drift** - The transport uses wall-clock time (`Instant::now()`) to advance the playhead, but each track's playback stream advances at its own rate. Over time, the visual playhead and actual audio position will diverge.

4. **No sample-accurate sync** - Playback threads are spawned sequentially. Each thread independently opens an output stream and starts playing. There's no synchronization point, so tracks can be offset by milliseconds.

5. **Hardcoded device names** - `"Microfono de MacBook Pro"` and `"Bocinas de MacBook Pro"` are hardcoded as defaults. This won't work on any non-Spanish macOS or on Linux/Windows.

### Architectural Weaknesses

6. **f64 -> f32 -> f64 conversions** - Recording captures f32 (cpal native), waveform processing converts to f64, WAV storage converts to i16, playback converts back to f64 then f32. Each conversion loses precision and burns CPU.

7. **Full waveform recomputation on every frame** - `track.waveform()` for idle tracks composites ALL clips and downsamples to 500 points on every UI render call (~10 FPS). This should be cached and only recomputed when clips change.

8. **MAX_TRACKS = 3 hardcoded** - The track limit is an arbitrary constant, not derived from system capabilities.

9. **No file I/O for projects** - There's no save/load. All recorded audio exists only in memory. Closing the app loses everything.

10. **Thread handle leak in stop_playback** - `stop_playback()` drops the `JoinHandle` without joining, which means the thread continues running until the `is_playing` flag takes effect. Not a leak per se, but not clean.

11. **No input validation on playback** - If clips have different sample rates, the first clip's rate is used for the stream config. Other clips play at the wrong speed.

12. **Vec<f32> grows unbounded during recording** - The recording buffer is a `Vec` that grows continuously. A 10-minute recording at 48kHz stereo = ~230MB. No streaming to disk.

### UI Weaknesses

13. **10 FPS refresh rate** - The 100ms poll timeout means the UI updates at most 10 times per second. Waveform growth during recording looks choppy.

14. **No scrollable timeline** - Fixed 20-second window. Recordings longer than 20s are drawn but clipped visually.

15. **No clip editing** - Can't move, trim, split, or delete individual clips on the timeline.

16. **No undo/redo** - Any destructive action (clear, delete track) is permanent.

### Code Smells

17. **Large track/mod.rs** - This file (~600 lines) handles monitoring, recording preparation, playback, waveform computation, and cleanup. Should be split into focused modules.

18. **Public fields everywhere** - Most struct fields are `pub`, allowing any code to mutate state freely. Encapsulation is minimal.

19. **Unused `file_path` field** - Tracks have a `file_path: String` that's never set during recording. It's a vestige of a file-loading feature that was partially removed.

20. **Effects not applied during recording** - The `fx_chain` exists on Track but is never used during recording or playback. Effects are only applied through the separate effects screen on loaded WAV files.

---

## 14. Necessary Changes for a Production DAW

### Priority 1: Correctness & Stability

- [ ] **Replace RwLock in recording callback with lock-free buffer** - Use a ring buffer or atomic triple buffer for the recording path. The audio callback must never block.

- [ ] **Single output stream (master bus)** - Mix all tracks into one output buffer and play through a single output stream. This gives sample-accurate sync, master volume, and master effects.

- [ ] **Sample-accurate transport** - Drive the playhead from the audio callback's sample counter, not wall-clock time. The audio callback knows exactly how many samples have been rendered.

- [ ] **Stream to disk during recording** - Write recorded audio to a temp WAV file in a background thread. Keep only a tail buffer in memory for waveform display. This prevents OOM on long recordings.

### Priority 2: Core Features

- [ ] **Project save/load** - Serialize session state (tracks, clips, effects, transport position) to a project file. Store audio as WAV files in a project directory.

- [ ] **File import** - Load existing WAV/MP3 files as clips on the timeline.

- [ ] **Clip editing** - Move clips on the timeline, trim start/end, split at playhead, delete individual clips.

- [ ] **Undo/redo stack** - Track state changes as commands. Essential for any editor.

- [ ] **Scrollable/zoomable timeline** - Allow timeline to extend beyond 20 seconds with zoom controls.

### Priority 3: Audio Quality

- [ ] **Sample rate consistency** - Resample clips to the session sample rate when loading. Don't mix sample rates.

- [ ] **Cache waveform data** - Compute waveforms once per clip change, not every render frame.

- [ ] **Proper latency compensation** - Account for input latency when placing recorded clips on the timeline. Currently, recordings start at the playhead position but actual audio arrives LATENCY_MS later.

- [ ] **Crossfade on boundaries** - When clips overlap or start/stop, apply short crossfades to prevent clicks.

### Priority 4: Architecture

- [ ] **Split track/mod.rs** - Separate recording, playback, waveform, and monitoring into sub-modules.

- [ ] **Reduce pub fields** - Use proper accessor methods. Internal state should be private.

- [ ] **Remove hardcoded device names** - Use system defaults without language-specific strings.

- [ ] **Event-driven architecture** - Consider a command/event system instead of direct mutation. This enables undo/redo and cleaner state management.

- [ ] **Audio graph** - Replace ad-hoc stream management with a proper audio graph where nodes (tracks, effects, mixers) connect to each other and process in a defined order.

### Priority 5: Polish

- [ ] **Higher UI refresh rate** - Reduce poll timeout to 16ms (60 FPS) or use a separate render timer.

- [ ] **Metering** - Show VU meters / peak meters per track using the monitor buffer.

- [ ] **Solo button** - Mute all tracks except the soloed one.

- [ ] **Track reordering** - Drag tracks up/down.

- [ ] **Export mix** - Render all tracks to a single WAV file (currently a stub).

---

## 15. Constants Reference

| Constant                        | Value     | Location         | Meaning                                    |
| ------------------------------- | --------- | ---------------- | ------------------------------------------ |
| `AUDIO_BUFFER_SIZE`             | 32 frames | track/mod.rs     | Monitoring callback size (~0.67ms @ 48kHz) |
| `RING_BUFFER_MULTIPLIER`        | 2         | track/mod.rs     | Ring buffer = buffer_size \* 2             |
| `PREFILL_BUFFER_COUNT`          | 0         | track/mod.rs     | Pre-filled silence buffers (disabled)      |
| `MONITOR_BUFFER_SAMPLES`        | 4800      | track/mod.rs     | ~100ms of samples for UI display           |
| `LATENCY_MS`                    | 10.0      | track/mod.rs     | Recording stream buffer size in ms         |
| `RECORDING_WAVEFORM_CHUNK_SIZE` | 960       | track/mod.rs     | Samples per waveform point (~20ms @ 48kHz) |
| `MAX_TRACKS`                    | 3         | session.rs       | Maximum tracks per session                 |
| `MIN_TRACKS`                    | 1         | session.rs       | Minimum tracks (can't delete last)         |
| `MAX_WAVEFORM_POINTS`           | 500       | track/mod.rs     | Resolution for idle waveform display       |
| `UPDATE_INTERVAL_MS`            | 50        | track/mod.rs     | Waveform thread sleep interval             |
| `POLL_TIMEOUT`                  | 100ms     | event_handler.rs | UI event polling interval                  |
| `TIMELINE_SECONDS`              | 20        | daw_screen.rs    | Fixed timeline duration                    |
