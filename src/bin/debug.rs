use rust_audio::wav::WavFile;
use std::f32::consts::PI;

// --- 1. The Biquad Struct (Same as before) ---
#[derive(Debug, Default, Clone)]
pub struct Biquad {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    x1: f32,
    x2: f32,
    y1: f32,
    y2: f32,
}

impl Biquad {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_lowpass(&mut self, cutoff_hz: f32, sample_rate: f32, q: f32) {
        let w0 = 2.0 * PI * cutoff_hz / sample_rate;
        let alpha = w0.sin() / (2.0 * q);
        let cos_w0 = w0.cos();
        let a0 = 1.0 + alpha;

        self.b0 = ((1.0 - cos_w0) / 2.0) / a0;
        self.b1 = (1.0 - cos_w0) / a0;
        self.b2 = ((1.0 - cos_w0) / 2.0) / a0;
        self.a1 = (-2.0 * cos_w0) / a0;
        self.a2 = (1.0 - alpha) / a0;
    }

    pub fn process(&mut self, sample: f32) -> f32 {
        let x = sample;
        let output = (self.b0 * x) + (self.b1 * self.x1) + (self.b2 * self.x2)
            - (self.a1 * self.y1)
            - (self.a2 * self.y2);

        // Update history
        self.x2 = self.x1;
        self.x1 = x;
        self.y2 = self.y1;
        self.y1 = output;

        output
    }
}

// --- 2. The File Processor ---
fn main() {
    // 1. Load the input file using WavFile
    let input_bytes = match std::fs::read("guitar.wav") {
        Ok(bytes) => bytes,
        Err(e) => {
            eprintln!("Error reading guitar.wav: {}", e);
            eprintln!("Please ensure 'guitar.wav' exists in the project root.");
            return;
        }
    };

    let wav_file = match WavFile::from_bytes(input_bytes) {
        Ok(wav) => wav,
        Err(e) => {
            eprintln!("Error parsing WAV file: {}", e);
            return;
        }
    };

    let sample_rate = wav_file.header.sample_rate as f32;
    let channels = wav_file.header.num_channels as usize;

    println!("Processing: {} Hz, {} channels", sample_rate, channels);

    // 2. Convert to f64 samples
    let input_samples = wav_file.to_f64_samples();
    let total_samples = input_samples.len();

    // 3. Setup Filters
    // We need ONE filter per channel (e.g., Left and Right)
    let mut filters = vec![Biquad::new(); channels];

    // Cutoff sweep parameters
    let start_cutoff = 200.0; // Start at 200 Hz (muffled)
    let end_cutoff = 8000.0; // End at 8000 Hz (bright)
    let update_interval = 512; // Update filter every 512 samples

    // Initialize filters with starting cutoff
    for filter in filters.iter_mut() {
        filter.set_lowpass(start_cutoff, sample_rate, 0.707);
    }

    // 4. Process Samples
    let mut output_samples = Vec::with_capacity(total_samples);

    for (i, &sample_f64) in input_samples.iter().enumerate() {
        // Update cutoff frequency periodically
        if i % update_interval == 0 {
            // Calculate progress (0.0 to 1.0)
            let progress = (i as f32) / (total_samples as f32);

            // Exponential sweep - spends more time on lower frequencies
            // This creates a logarithmic frequency sweep
            let current_cutoff = start_cutoff * (end_cutoff / start_cutoff).powf(progress);
            println!("Sample {}: Cutoff = {:.2} Hz", i, current_cutoff);
            // Update all filters with new cutoff
            for filter in filters.iter_mut() {
                filter.set_lowpass(current_cutoff, sample_rate, 0.707);
            }
        }

        // Convert f64 to f32 for processing
        let input_float = sample_f64 as f32;

        // Determine which channel this sample belongs to
        let channel_idx = i % channels;

        // Run through the correct filter
        let output_float = filters[channel_idx].process(input_float);

        // Convert back to f64
        output_samples.push(output_float as f64);
    }

    // 5. Create output WAV file
    let mut output_wav = WavFile::new(sample_rate as u32, channels as u16);
    output_wav.from_f64_samples(&output_samples);

    // 6. Write to file
    let output_bytes = output_wav.export_to_bytes();
    match std::fs::write("output_lowpass.wav", output_bytes) {
        Ok(_) => println!("Done! Created 'output_lowpass.wav'"),
        Err(e) => eprintln!("Error writing output file: {}", e),
    }
}
