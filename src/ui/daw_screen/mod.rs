mod input;
mod render;

use crossterm::event::KeyCode;
use ratatui::{layout::Rect, Frame};

use super::screen_trait::ScreenTrait;
use super::App;

pub(crate) mod layout_config {
    use ratatui::layout::Constraint;
    use ratatui::style::Color;

    pub const SELECTED_BORDER: Color = Color::Yellow;
    pub const DEFAULT_BORDER: Color = Color::White;
    pub const ARMED_BORDER: Color = Color::Red;
    pub const ARMED_SELECTED_BORDER: Color = Color::LightRed;
    pub const RECORDING_BORDER: Color = Color::Magenta;
    pub const RECORDING_SELECTED_BORDER: Color = Color::LightMagenta;
    pub const LANE_STATUS_ARMED: &str = "ARMED";
    pub const LANE_STATUS_MUTED: &str = "MUTED";
    pub const LANE_STATUS_ACTIVE: &str = "ACTIVE";
    pub const LANE_STATUS_RECORDING: &str = "\u{1f534} REC";
    pub const WAVEFORM_SENSITIVITY: f32 = 8.0;
    pub const TIMELINE_SECONDS: u64 = 20;
    pub const PLAYHEAD_DELTA_SECONDS: f64 = 0.5;
    pub const SCROLL_STEP_SECONDS: u64 = 5;
    pub const GLOBAL_INSTRUCTIONS: &str =
        "n: Add | d: Del | Space: Play | Left/Right: Playhead | [/]: Scroll | h: Reset | Tab: Clip | Bksp: Del Clip | Ctrl+S: Save";

    pub fn get_lane_constraints(track_count: usize) -> Vec<Constraint> {
        let denominator = track_count.max(3) as u32;
        (0..track_count)
            .map(|_| Constraint::Ratio(1, denominator))
            .collect()
    }

    pub fn format_lane_title(name: &str, volume: f64, input_channel: Option<u16>, status: &str) -> String {
        let input_label = match input_channel {
            None => "All",
            Some(0) => "In 1",
            Some(1) => "In 2",
            Some(n) => return format!("{} | Vol: {:.0}% | In {} | {}", name, volume * 100.0, n + 1, status),
        };
        format!(
            "{} | Vol: {:.0}% | {} | {}",
            name,
            volume * 100.0,
            input_label,
            status,
        )
    }
}

pub struct DawScreen;

impl ScreenTrait for DawScreen {
    fn render(&self, f: &mut Frame, app: &App, area: Rect) {
        render::render(f, app, area);
    }

    fn handle_input(
        &self,
        app: &mut App,
        key: KeyCode,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        input::handle_input(app, key)
    }
}
