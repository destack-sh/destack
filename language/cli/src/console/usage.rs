use super::{color, dim};

/// Number of cells in one usage bar.
const BAR_WIDTH: usize = 32;
/// Partial block glyphs in eighth-cell increments.
const PARTIAL_BLOCKS: [&str; 8] = ["", "▏", "▎", "▍", "▌", "▋", "▊", "▉"];

/// One bounded resource usage bar.
#[derive(Debug)]
pub struct UsageBar {
    /// Used resource units.
    used: u64,
    /// Maximum resource units.
    maximum: u64,
}

impl UsageBar {
    /// Create one bounded resource usage bar.
    pub const fn new(used: u64, maximum: u64) -> Self {
        Self { used, maximum }
    }

    /// Return the used percentage when the maximum is nonzero.
    pub fn percentage(&self) -> Option<f64> {
        if self.maximum == 0 {
            None
        } else {
            Some(100.0 * self.used as f64 / self.maximum as f64)
        }
    }

    /// Render this usage bar.
    pub fn render(&self) -> String {
        let eighths = self.filled_eighths();
        let cells = eighths / 8;
        let partial = eighths % 8;
        let empty = BAR_WIDTH - cells - usize::from(partial > 0);

        // render the bounded used region
        let mut used = "█".repeat(cells);
        used.push_str(PARTIAL_BLOCKS[partial]);

        // style used and remaining cells independently
        let used = color(&used, "36");
        let empty = dim(&"░".repeat(empty));

        format!("{used}{empty}")
    }

    /// Return the displayed usage in eighth-cell increments.
    fn filled_eighths(&self) -> usize {
        if self.maximum == 0 {
            return usize::from(self.used > 0) * BAR_WIDTH * 8;
        }

        // clamp usage to the visible capacity of the bar
        let used = self.used.min(self.maximum) as u128;
        let maximum = self.maximum as u128;
        let eighths = used * BAR_WIDTH as u128 * 8;

        ((eighths + maximum / 2) / maximum) as usize
    }
}
