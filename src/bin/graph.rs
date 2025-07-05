use plotters::{coord::Shift, prelude::*};
use rust_audio::wav::WavFile;
use std::env;
use std::fs;

fn get_samples(file_name: &str) -> Result<Vec<f64>, Box<dyn std::error::Error>> {
    let bytes = fs::read(file_name)?;
    let wav_file = WavFile::from_bytes(bytes)?;
    let samples = wav_file.to_f64_samples();
    Ok(samples)
}

fn normalize_samples(samples: &[f64]) -> Vec<(f32, f32)> {
    samples
        .iter()
        .enumerate()
        .map(|(i, &s)| (i as f32, (s / i16::MAX as f64) as f32))
        .collect()
}

fn create_audio_chart(
    drawing_area: &DrawingArea<BitMapBackend, Shift>,
    samples: &[f64],
    caption: &str,
    color: &RGBColor,
) -> Result<(), Box<dyn std::error::Error>> {
    let norm_samples = normalize_samples(samples);

    let mut chart = ChartBuilder::on(drawing_area)
        .caption(caption, ("sans-serif", 30).into_font())
        .margin(5)
        .x_label_area_size(30)
        .y_label_area_size(30)
        .build_cartesian_2d(0f32..samples.len() as f32, -1.0f32..1.0f32)?;

    chart.configure_mesh().draw()?;
    chart.draw_series(LineSeries::new(norm_samples, color))?;

    Ok(())
}

fn create_combined_graph(
    tone_samples: Vec<f64>,
    output_samples: Vec<f64>,
    file_name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let root = BitMapBackend::new(file_name, (1280, 480)).into_drawing_area();
    root.fill(&WHITE)?;

    let (left, right) = root.split_horizontally(640);

    create_audio_chart(&left, &tone_samples, "Tone", &RED)?;
    create_audio_chart(&right, &output_samples, "Output", &BLUE)?;

    root.present()?;
    Ok(())
}

fn print_usage() {
    println!("Usage: cargo run --bin graph <tone_file.wav> <output_file.wav> [output_image.png]");
    println!("  tone_file.wav: Path to the first WAV file to compare");
    println!("  output_file.wav: Path to the second WAV file to compare");
    println!("  output_image.png: Optional output image file (default: combined.png)");
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 3 {
        println!("Error: Missing required arguments");
        print_usage();
        std::process::exit(1);
    }

    let tone_file = &args[1];
    let output_file = &args[2];
    let output_image = args.get(3).map(|s| s.as_str()).unwrap_or("combined.png");

    println!("Reading tone file: {}", tone_file);
    let tone_samples = get_samples(tone_file)?;

    println!("Reading output file: {}", output_file);
    let output_samples = get_samples(output_file)?;

    println!("Creating combined graph: {}", output_image);
    create_combined_graph(tone_samples, output_samples, output_image)?;

    println!("Graph saved successfully!");

    Ok(())
}
