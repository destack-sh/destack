use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;

/// Numeric conversion policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum ConvertMode {
    /// Require exact conversion with no rounding or saturation.
    Exact,
    /// Round to the nearest even representable value.
    RoundTiesEven,
    /// Round toward zero.
    RoundTowardZero,
    /// Round toward negative infinity.
    RoundFloor,
    /// Round toward positive infinity.
    RoundCeil,
    /// Clamp values that overflow the destination range.
    Saturate,
}

impl ConvertMode {
    /// Return the MIR name for this conversion policy.
    pub const fn to_str(self) -> &'static str {
        match self {
            Self::Exact => "exact",
            Self::RoundTiesEven => "roundTiesEven",
            Self::RoundTowardZero => "roundTowardZero",
            Self::RoundFloor => "roundFloor",
            Self::RoundCeil => "roundCeil",
            Self::Saturate => "saturate",
        }
    }

    /// Parse one MIR conversion policy.
    pub fn parse(text: &str) -> Option<Self> {
        text.parse().ok()
    }
}

impl std::str::FromStr for ConvertMode {
    type Err = ();

    /// Parse one MIR conversion policy.
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        match text {
            "exact" => Ok(Self::Exact),
            "roundTiesEven" => Ok(Self::RoundTiesEven),
            "roundTowardZero" => Ok(Self::RoundTowardZero),
            "roundFloor" => Ok(Self::RoundFloor),
            "roundCeil" => Ok(Self::RoundCeil),
            "saturate" => Ok(Self::Saturate),
            _ => Err(()),
        }
    }
}
