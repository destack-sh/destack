use crate::HotnessThresholds;

/// Profile hotness for one operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hotness {
    /// No strong profile signal is available.
    Unknown,
    /// The operation is hot.
    Hot,
    /// The operation is cold.
    Cold,
}

impl HotnessThresholds {
    /// Classify hotness from a block count relative to a function entry count.
    pub fn classify(self, block_count: u64, entry_count: u64) -> Hotness {
        // guard against missing counts
        if block_count == 0 {
            return Hotness::Unknown;
        }

        // classify by absolute counts
        if block_count >= self.hot_count {
            return Hotness::Hot;
        }
        if block_count <= self.cold_count {
            return Hotness::Cold;
        }

        // classify by caller-relative ratio
        if entry_count == 0 {
            return Hotness::Unknown;
        }

        // compare ratios against thresholds
        let ratio = block_count as f64 / entry_count as f64;
        if ratio >= self.hot_ratio {
            return Hotness::Hot;
        }
        if ratio <= self.cold_ratio {
            return Hotness::Cold;
        }

        Hotness::Unknown
    }
}
