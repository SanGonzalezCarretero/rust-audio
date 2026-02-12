use std::fs;
use std::io::Cursor;
use std::io::Read;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct WavHeader {
    pub chunk_id: [u8; 4],     // "RIFF"
    pub chunk_size: u32,       // File size - 8
    pub format: [u8; 4],       // "WAVE"
    pub subchunk1_id: [u8; 4], // "fmt "
    pub subchunk1_size: u32,   // 16 for PCM
    pub audio_format: u16,     // 1 for PCM
    pub num_channels: u16,     // 1 for mono, 2 for stereo
    pub sample_rate: u32,      // e.g., 44100
    pub byte_rate: u32,        // SampleRate * NumChannels * BitsPerSample/8
    pub block_align: u16,      // NumChannels * BitsPerSample/8
    pub bits_per_sample: u16,  // 8 bits = 8, 16 bits = 16, etc.
}

#[derive(Clone)]
pub struct WavFile {
    pub header: WavHeader,
    pub audio_data: Vec<u8>,
}

use crate::effects::EffectInstance;

impl WavFile {
    pub fn new(sample_rate: u32, num_channels: u16) -> Self {
        WavFile {
            header: WavHeader {
                chunk_id: *b"RIFF",
                chunk_size: 0,
                format: *b"WAVE",
                subchunk1_id: *b"fmt ",
                subchunk1_size: 16,
                audio_format: 1,
                num_channels,
                sample_rate,
                byte_rate: sample_rate * num_channels as u32 * 2,
                block_align: num_channels * 2,
                bits_per_sample: 16,
            },
            audio_data: Vec::new(),
        }
    }

    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self, Box<dyn std::error::Error>> {
        let mut cursor = Cursor::new(&bytes);
        let mut header = WavHeader {
            chunk_id: [0; 4],
            chunk_size: 0,
            format: [0; 4],
            subchunk1_id: [0; 4],
            subchunk1_size: 0,
            audio_format: 0,
            num_channels: 0,
            sample_rate: 0,
            byte_rate: 0,
            block_align: 0,
            bits_per_sample: 0,
        };

        cursor.read_exact(&mut header.chunk_id)?;
        header.chunk_size = read_u32(&mut cursor)?;
        cursor.read_exact(&mut header.format)?;
        cursor.read_exact(&mut header.subchunk1_id)?;
        header.subchunk1_size = read_u32(&mut cursor)?;
        header.audio_format = read_u16(&mut cursor)?;
        header.num_channels = read_u16(&mut cursor)?;
        header.sample_rate = read_u32(&mut cursor)?;
        header.byte_rate = read_u32(&mut cursor)?;
        header.block_align = read_u16(&mut cursor)?;
        header.bits_per_sample = read_u16(&mut cursor)?;

        // Validate header
        if header.chunk_id != *b"RIFF" || header.format != *b"WAVE" {
            return Err("Not a valid WAV file".into());
        }
        if header.audio_format != 1 {
            return Err(format!(
                "Unsupported audio format: {} (only PCM is supported)",
                header.audio_format
            )
            .into());
        }
        if header.bits_per_sample != 16 {
            return Err(format!(
                "Unsupported bits per sample: {} (only 16-bit is supported)",
                header.bits_per_sample
            )
            .into());
        }

        // Search for the data chunk
        let mut data_chunk_size: u32 = 0;
        let mut position = cursor.position() as usize;

        while position + 8 <= bytes.len() {
            let chunk_id = &bytes[position..position + 4];
            let chunk_size = u32::from_le_bytes([
                bytes[position + 4],
                bytes[position + 5],
                bytes[position + 6],
                bytes[position + 7],
            ]);

            if chunk_id == b"data" {
                position += 8; // Skip "data" + size
                data_chunk_size = chunk_size;
                break;
            }

            // Skip this chunk and move to the next one
            position += 8 + chunk_size as usize;
        }

        if data_chunk_size == 0 {
            return Err("Data chunk not found".into());
        }

        let data_end = (position + data_chunk_size as usize).min(bytes.len());
        let audio_data = bytes[position..data_end].to_vec();

        Ok(WavFile { header, audio_data })
    }

    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        // Write RIFF header
        bytes.extend_from_slice(&self.header.chunk_id); // "RIFF"
        bytes.extend_from_slice(&self.header.chunk_size.to_le_bytes());
        bytes.extend_from_slice(&self.header.format); // "WAVE"

        // Write fmt subchunk
        bytes.extend_from_slice(&self.header.subchunk1_id); // "fmt "
        bytes.extend_from_slice(&self.header.subchunk1_size.to_le_bytes());
        bytes.extend_from_slice(&self.header.audio_format.to_le_bytes());
        bytes.extend_from_slice(&self.header.num_channels.to_le_bytes());
        bytes.extend_from_slice(&self.header.sample_rate.to_le_bytes());
        bytes.extend_from_slice(&self.header.byte_rate.to_le_bytes());
        bytes.extend_from_slice(&self.header.block_align.to_le_bytes());
        bytes.extend_from_slice(&self.header.bits_per_sample.to_le_bytes());

        // Write data subchunk
        bytes.extend_from_slice(b"data"); // data chunk identifier
        bytes.extend_from_slice(&(self.audio_data.len() as u32).to_le_bytes()); // size of data
        bytes.extend(&self.audio_data); // actual audio data

        bytes
    }

    pub fn sample_count(&self) -> usize {
        let bytes_per_sample = (self.header.bits_per_sample / 8) as usize;
        if bytes_per_sample == 0 {
            return 0;
        }
        self.audio_data.len() / bytes_per_sample
    }

    pub fn to_f32_samples(&self) -> Vec<f32> {
        let mut samples = Vec::with_capacity(self.audio_data.len() / 2);

        for i in (0..self.audio_data.len()).step_by(2) {
            if i + 1 >= self.audio_data.len() {
                break;
            }

            let sample = i16::from_le_bytes([self.audio_data[i], self.audio_data[i + 1]]);
            samples.push(sample as f32 / 32768.0);
        }

        samples
    }

    pub fn from_f32_samples(&mut self, samples: &[f32]) {
        let mut new_audio_data = Vec::with_capacity(samples.len() * 2);

        for &sample in samples {
            let clamped = sample.clamp(-1.0, 1.0);
            let sample_i16 = (clamped * 32768.0).clamp(-32768.0, 32767.0) as i16;
            new_audio_data.extend_from_slice(&sample_i16.to_le_bytes());
        }

        self.audio_data = new_audio_data;
        self.resize();
    }

    pub fn to_f64_samples(&self) -> Vec<f64> {
        self.to_f32_samples().iter().map(|&s| s as f64).collect()
    }

    pub fn from_f64_samples(&mut self, samples: &[f64]) {
        let f32_samples: Vec<f32> = samples.iter().map(|&s| s as f32).collect();
        self.from_f32_samples(&f32_samples);
    }

    fn resize(&mut self) {
        let new_size = &self.audio_data.len() + 44 - 8;

        self.header.chunk_size = new_size as u32;
    }

    pub fn export_to_bytes(&mut self) -> Vec<u8> {
        self.resize();
        self.to_bytes()
    }

    pub fn save_to_file<P: AsRef<Path>>(
        &mut self,
        path: P,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let bytes = self.export_to_bytes();
        fs::write(path, bytes)?;
        Ok(())
    }

    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let bytes = fs::read(path)?;
        Self::from_bytes(bytes)
    }

    pub fn apply_effects(
        &mut self,
        effects: Vec<EffectInstance>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut samples = self.to_f32_samples();

        for effect in effects {
            effect.apply(&mut samples, self.header.sample_rate)?;
        }

        self.from_f32_samples(&samples);

        Ok(())
    }
}

fn read_u32(cursor: &mut Cursor<&Vec<u8>>) -> Result<u32, std::io::Error> {
    let mut buffer = [0; 4];
    cursor.read_exact(&mut buffer)?;
    Ok(u32::from_le_bytes(buffer))
}

fn read_u16(cursor: &mut Cursor<&Vec<u8>>) -> Result<u16, std::io::Error> {
    let mut buffer = [0; 2];
    cursor.read_exact(&mut buffer)?;
    Ok(u16::from_le_bytes(buffer))
}
