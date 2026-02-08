mod input;
mod render;

use crossterm::event::KeyCode;
use ratatui::{layout::Rect, Frame};

use super::screen_trait::ScreenTrait;
use super::App;

mod layout_config {
    use ratatui::layout::Constraint;
    use ratatui::style::Color;

    pub const SELECTED_BORDER: Color = Color::Yellow;
    pub const DEFAULT_BORDER: Color = Color::White;
    pub const ARMED_BORDER: Color = Color::Red;
    pub const RECORDING_BORDER: Color = Color::Magenta;
    pub const LANE_STATUS_EMPTY: &str = "Empty";
    pub const LANE_STATUS_ARMED: &str = "ARMED";
    pub const LANE_STATUS_MUTED: &str = "MUTED";
    pub const LANE_STATUS_ACTIVE: &str = "ACTIVE";
    pub const LANE_STATUS_RECORDING: &str = "\u{1f534} REC";
    pub const GLOBAL_INSTRUCTIONS: &str =
        "n: Add new track | d: Delete track | Space: Play all tracks | Left/Right: Move playhead";

    pub fn get_lane_constraints(track_count: usize) -> Vec<Constraint> {
        let denominator = track_count.max(3) as u32;
        (0..track_count)
            .map(|_| Constraint::Ratio(1, denominator))
            .collect()
    }

    pub fn format_lane_title(
        lane_num: usize,
        volume: f64,
        status: &str,
        file_path: &str,
    ) -> String {
        format!(
            "Lane {} | Vol: {:.0}% | {} | {}",
            lane_num,
            volume * 100.0,
            status,
            if file_path.is_empty() {
                LANE_STATUS_EMPTY
            } else {
                file_path
            }
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
