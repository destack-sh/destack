use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;

/// Reduction operators for vector reductions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
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
