/// Minimum execution count for a hot operation.
const HOT_COUNT: u64 = 50;
/// Maximum execution count for a cold operation.
const COLD_COUNT: u64 = 8;
/// Minimum caller relative count for a hot operation.
const HOT_RATIO: f64 = 0.10;
/// Maximum caller relative count for a cold operation.
const COLD_RATIO: f64 = 0.01;

/// Hotness classification for an operation under profile data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallsiteHotness {
    /// No profile data or no strong signal is available.
    Unknown,
    /// The operation is hot.
    Hot,
    /// The operation is cold or missing profile data.
    Cold,
}

impl CallsiteHotness {
    /// Classify hotness from a block count relative to a function entry count.
    pub fn from_counts(block_count: u64, entry_count: u64) -> Self {
        // guard against missing counts
        if block_count == 0 {
            return Self::Unknown;
        }

        // classify by absolute counts
        if block_count >= HOT_COUNT {
            return Self::Hot;
        }
        if block_count <= COLD_COUNT {
            return Self::Cold;
        }

        // fall back to ratio based classification
        if entry_count == 0 {
            return Self::Unknown;
        }

        // compare ratios against thresholds
        let ratio = block_count as f64 / entry_count as f64;
        if ratio >= HOT_RATIO {
            return Self::Hot;
        }
        if ratio <= COLD_RATIO {
            return Self::Cold;
        }

        Self::Unknown
    }
}
