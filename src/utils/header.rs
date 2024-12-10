#[derive(Debug)]
pub struct WavHeader {
    riff_signature: [u8; 4],
    file_size: u32,
    wave_format: [u8; 4],
    fmt_marker: [u8; 4],
    fmt_length: u32,
    format_type: u16,
}

pub fn process_header(data: &[u8]) -> usize {
    let mut audio_start = 36; // Standard starting position

    while audio_start + 8 < data.len() {
        if &data[audio_start..audio_start + 4] == b"data" {
            audio_start += 8; // Skip "data" and chunk size
            break;
        }
        audio_start += 1;
    }

    audio_start
}

pub fn print_header_info(data: &[u8]) -> Result<WavHeader, &'static str> {
    // Check if we have enough data for the basic header
    if data.len() < 22 {
        return Err("WAV file is too short");
    }

    // RIFF signature (bytes 1-4)
    let riff = &data[0..4];
    if riff != b"RIFF" {
        return Err("Not a valid RIFF file");
    }
    println!("RIFF signature: {}", String::from_utf8_lossy(riff));

    // File size (bytes 5-8)
    let file_size = u32::from_le_bytes(data[4..8].try_into().unwrap());
    println!("File size: {} bytes", file_size);

    // WAVE format (bytes 9-12)
    let wave = &data[8..12];
    if wave != b"WAVE" {
        return Err("Not a valid WAVE file");
    }
    println!("Format: {}", String::from_utf8_lossy(wave));

    // fmt marker (bytes 13-16)
    let fmt = &data[12..16];
    if fmt != b"fmt " {
        return Err("Missing fmt marker");
    }
    println!("Format marker: {}", String::from_utf8_lossy(fmt));

    // Format length (bytes 17-20)
    let format_len = u32::from_le_bytes(data[16..20].try_into().unwrap());
    println!("Format length: {} bytes", format_len);

    // Format type (bytes 21-22)
    let format_type = u16::from_le_bytes(data[20..22].try_into().unwrap());
    println!("Format type: {} (1 = PCM)", format_type);

    // Additional PCM format information if available
    if data.len() >= 24 {
        let num_channels = u16::from_le_bytes(data[22..24].try_into().unwrap());
        println!("Number of channels: {}", num_channels);
    }

    if data.len() >= 28 {
        let sample_rate = u32::from_le_bytes(data[24..28].try_into().unwrap());
        println!("Sample rate: {} Hz", sample_rate);
    }

    if data.len() >= 32 {
        let byte_rate = u32::from_le_bytes(data[28..32].try_into().unwrap());
        println!("Byte rate: {} bytes/sec", byte_rate);
    }

    Ok(WavHeader {
        riff_signature: riff.try_into().unwrap(),
        file_size,
        wave_format: wave.try_into().unwrap(),
        fmt_marker: fmt.try_into().unwrap(),
        fmt_length: format_len,
        format_type,
    })
}

pub fn update_wav_header(data: &mut Vec<u8>, new_size: usize) -> Result<(), &'static str> {
    if data.len() < 44 {
        // Standard WAV header is 44 bytes
        return Err("Invalid WAV file: header too short");
    }

    // Update the overall file size (size - 8 bytes) at offset 4
    let file_size = (new_size - 8) as u32;
    data[4..8].copy_from_slice(&file_size.to_le_bytes());

    // Update the data chunk size at offset 40
    // This is the size of the audio data (total size - 44 bytes for header)
    let data_size = (new_size - 44) as u32;
    data[40..44].copy_from_slice(&data_size.to_le_bytes());

    Ok(())
}
