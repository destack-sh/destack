use serde::{Deserialize, Serialize};

/// Optimization level for builds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum OptimizeLevel {
    /// No optimization (O0).
    #[default]
    O0,
    /// Basic optimization (O1).
    O1,
    /// Standard optimization (O2).
    O2,
    /// Aggressive optimization (O3).
    O3,
    /// Maximal optimization (O4).
    O4,
}

impl From<u8> for OptimizeLevel {
    fn from(level: u8) -> Self {
        match level {
            0 => Self::O0,
            1 => Self::O1,
            2 => Self::O2,
            3 => Self::O3,
            _ => Self::O4,
        }
    }
}

impl From<OptimizeLevel> for u8 {
    fn from(level: OptimizeLevel) -> Self {
        match level {
            OptimizeLevel::O0 => 0,
            OptimizeLevel::O1 => 1,
            OptimizeLevel::O2 => 2,
            OptimizeLevel::O3 => 3,
            OptimizeLevel::O4 => 4,
        }
    }
}

/// Debug info emission policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum DebugInfoLevel {
    /// No debug info.
    #[default]
    None,
    /// Line tables only.
    Line,
    /// Full debug info.
    Full,
}

impl std::str::FromStr for DebugInfoLevel {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().replace('-', "_").as_str() {
            "none" => Ok(Self::None),
            "line" | "lines" => Ok(Self::Line),
            "full" => Ok(Self::Full),
            _ => Err(()),
        }
    }
}

impl DebugInfoLevel {
    /// Parse from a string value.
    pub fn parse(s: &str) -> Option<Self> {
        s.parse().ok()
    }
}
