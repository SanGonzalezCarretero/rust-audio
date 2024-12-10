mod utils;

use crate::utils::header::print_header_info;
use crate::utils::header::process_header;
use utils::normalize::normalize;

use rand::Rng;

use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut data: Vec<u8> = fs::read("input.wav")?;
    print_header_info(&data)?;

    // match adjust_volume(&mut data, 2.0) {
    //     Ok(_) => println!("Successfully adjusted volume"),
    //     Err(e) => println!("Error: {}", e),
    // }
    // match reverse(&mut data) {
    //     Ok(()) => println!("Sucessfully reversed audio"),
    //     Err(e) => println!("Error: {}", e),
    // }

    delay(&mut data);
    fs::write("output.wav", data)?;
    Ok(())
}

fn adjust_volume(data: &mut Vec<u8>, volume: f32) -> Result<(), &'static str> {
    if volume > 2.0 || volume <= 0.0 {
        return Err("Not a valid volume value. Try again.");
    }

    let audio_start = process_header(data);

    let mut i = audio_start;
    while i < data.len() - 1 {
        // Convert two bytes to a 16-bit sample
        let sample = i16::from_le_bytes([data[i], data[i + 1]]);

        // Convert to float, adjust volume, and convert back to integer
        let adjusted = (sample as f32 * volume) as i16;

        // Convert back to bytes
        let adjusted_bytes = adjusted.to_le_bytes();
        data[i] = adjusted_bytes[0];
        data[i + 1] = adjusted_bytes[1];

        i += 2;
    }

    Ok(())
}

fn reverse(data: &mut Vec<u8>) -> Result<(), &'static str> {
    let audio_start = process_header(data);
    let mut rev_i = data.len() - 2;
    let mut i = audio_start;

    while i < rev_i {
        let temp1 = data[i];
        let temp2 = data[i + 1];

        data[i] = data[rev_i];
        data[i + 1] = data[rev_i + 1];

        data[rev_i] = temp1;
        data[rev_i + 1] = temp2;

        rev_i -= 2;
        i += 2;
    }

    Ok(())
}

fn duplicate(data: &mut Vec<u8>) -> Result<(), &'static str> {
    let audio_start = process_header(data);
    let original = data.clone();

    let audio_size = original.len() - audio_start;

    let new_total_size = (original.len() + audio_size - 8) as u32;

    data[4..8].copy_from_slice(&new_total_size.to_le_bytes());

    let new_audio_size = (audio_size * 2) as u32;
    data[audio_start - 4..audio_start].copy_from_slice(&new_audio_size.to_le_bytes());

    data.resize(data.len() * 2, 0);

    let mut i = audio_start;
    while i < original.len() - 1 {
        dbg!(i * 2);
        data[i * 2 - audio_start] = original[i];
        data[i * 2 + 1 - audio_start] = original[i + 1];
        data[i * 2 + 2 - audio_start] = original[i];
        data[i * 2 + 3 - audio_start] = original[i + 1];
        i += 2;
    }

    Ok(())
}

fn random_noise(data: &mut Vec<u8>) -> Result<(), &'static str> {
    let audio_start = process_header(data);

    let noise_amount = 20000;
    let original = data[audio_start..].to_vec();

    data.resize(original.len() + noise_amount, 0);

    let mut i = audio_start;
    while i < original.len() {
        let noise = rand::thread_rng().gen_range(0..4);

        data[i] = original[i];
        data[i + 1] = original[i + 1];
        data[i + 2] = noise;
        data[i + 3] = noise;

        i += 4;
    }
    Ok(())
}

fn delay(data: &mut Vec<u8>) -> Result<(), &'static str> {
    let audio_start = process_header(data);
    let offset = 50000;

    let mut original = data.clone();

    for n in 0..offset {
        original.insert(0, 0);
    }

    data.resize(data.len() + offset, 0);

    let mut i = audio_start;

    if data.len() != original.len() {
        panic!();
    }

    // normalize(&mut original);

    while i < data.len() {
        let original_sub: u16 = original[i].into();
        let data_sub: u16 = data[i].into();

        let original_sample = u16::from_le_bytes([original[i], original[i + 1]]);
        let data_sample = u16::from_le_bytes([data[i], data[i + 1]]);

        dbg!(original_sample, data_sample);
        let adjusted = u32::from(original_sample) + u32::from(data_sample);

        let adjusted_bytes = adjusted.to_le_bytes();

        data[i] = adjusted_bytes[0];
        data[i + 1] = adjusted_bytes[1];

        i += 2;
    }

    normalize(data);

    Ok(())
}
