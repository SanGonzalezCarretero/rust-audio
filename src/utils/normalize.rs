pub fn normalize(data: &mut Vec<u8>) -> Result<(), &'static str> {
    let mut i = 44;
    let mut max = 0u16;

    while i < data.len() {
        let sample = u16::from_le_bytes([data[i], data[i + 1]]);
        if sample > max {
            max = sample;
        }
        i += 2;
    }

    i = 44;

    while i < data.len() {
        let sample = u16::from_le_bytes([data[i], data[i + 1]]);
        let normalized = ((sample as f32 * 255.0) / max as f32) as u16;
        let bytes = normalized.to_le_bytes();
        data[i] = bytes[0];
        data[i + 1] = bytes[1];
        i += 2;
    }

    Ok(())
}
