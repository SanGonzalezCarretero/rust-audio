use rust_audio::ui;
use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let debug_mode = args.iter().any(|arg| arg == "-debug" || arg == "--debug");
    ui::run(debug_mode)
}
