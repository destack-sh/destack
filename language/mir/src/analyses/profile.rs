use crate::HotnessThresholds;

/// Hotness classification for an operation under profile data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallsiteHotness {
    /// No profile data or no strong signal is available.
    Unknown,
    /// The operation is hot.
    Hot,
    /// The operation is cold.
    Cold,
}

impl HotnessThresholds {
    /// Classify hotness from a block count relative to a function entry count.
    pub fn classify(self, block_count: u64, entry_count: u64) -> CallsiteHotness {
        // guard against missing counts
        if block_count == 0 {
            return CallsiteHotness::Unknown;
        }

        // classify by absolute counts
        if block_count >= self.hot_count {
            return CallsiteHotness::Hot;
        }
        if block_count <= self.cold_count {
            return CallsiteHotness::Cold;
        }

        // classify by caller-relative ratio
        if entry_count == 0 {
            return CallsiteHotness::Unknown;
        }

        // compare ratios against thresholds
        let ratio = block_count as f64 / entry_count as f64;
        if ratio >= self.hot_ratio {
            return CallsiteHotness::Hot;
        }
        if ratio <= self.cold_ratio {
            return CallsiteHotness::Cold;
        }

        CallsiteHotness::Unknown
    }
}
