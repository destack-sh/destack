use serde::{Deserialize, Serialize};

/// Reduction operators for vector reductions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum VectorReduceOperator {
    /// Add all lanes.
    Add,
    /// Multiply all lanes.
    Multiply,
    /// Return the minimum lane.
    Min,
    /// Return the maximum lane.
    Max,
    /// Bitwise AND across lanes.
    And,
    /// Bitwise OR across lanes.
    Or,
    /// Bitwise XOR across lanes.
    Xor,
}

impl VectorReduceOperator {
    /// Return the opcode name for this reduction.
    pub fn to_str(self) -> &'static str {
        match self {
            VectorReduceOperator::Add => "add",
            VectorReduceOperator::Multiply => "mul",
            VectorReduceOperator::Min => "min",
            VectorReduceOperator::Max => "max",
            VectorReduceOperator::And => "and",
            VectorReduceOperator::Or => "or",
            VectorReduceOperator::Xor => "xor",
        }
    }

    /// Parse a reduction operator from an opcode name.
    pub fn parse(text: &str) -> Option<Self> {
        <Self as std::str::FromStr>::from_str(text).ok()
    }
}

impl std::str::FromStr for VectorReduceOperator {
    type Err = ();

    /// Parse a reduction operator from an opcode name.
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let value = match text {
            "add" => VectorReduceOperator::Add,
            "mul" => VectorReduceOperator::Multiply,
            "min" => VectorReduceOperator::Min,
            "max" => VectorReduceOperator::Max,
            "and" => VectorReduceOperator::And,
            "or" => VectorReduceOperator::Or,
            "xor" => VectorReduceOperator::Xor,
            _ => return Err(()),
        };

        Ok(value)
    }
}

impl TryFrom<&str> for VectorReduceOperator {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let value = value.strip_prefix("reduce.").unwrap_or(value);
        Self::parse(value).ok_or(())
    }
}

/// Conversion modes for vector element conversions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum VectorConvertMode {
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

impl VectorConvertMode {
    /// Return the opcode name for this conversion mode.
    pub fn to_str(self) -> &'static str {
        match self {
            VectorConvertMode::Exact => "exact",
            VectorConvertMode::RoundTiesEven => "round_ties_even",
            VectorConvertMode::RoundTowardZero => "round_toward_zero",
            VectorConvertMode::RoundFloor => "round_floor",
            VectorConvertMode::RoundCeil => "round_ceil",
            VectorConvertMode::Saturate => "saturate",
        }
    }

    /// Parse a conversion mode from an opcode name.
    pub fn parse(text: &str) -> Option<Self> {
        <Self as std::str::FromStr>::from_str(text).ok()
    }
}

impl std::str::FromStr for VectorConvertMode {
    type Err = ();

    /// Parse a conversion mode from an opcode name.
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let value = match text {
            "exact" => VectorConvertMode::Exact,
            "round_ties_even" => VectorConvertMode::RoundTiesEven,
            "round_toward_zero" => VectorConvertMode::RoundTowardZero,
            "round_floor" => VectorConvertMode::RoundFloor,
            "round_ceil" => VectorConvertMode::RoundCeil,
            "saturate" => VectorConvertMode::Saturate,
            _ => return Err(()),
        };

        Ok(value)
    }
}
