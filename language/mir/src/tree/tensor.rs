use serde::{Deserialize, Serialize};

/// Reduction operators for tensor reductions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TensorReduceOperator {
    /// Add all elements.
    Add,
    /// Multiply all elements.
    Multiply,
    /// Return the minimum element.
    Min,
    /// Return the maximum element.
    Max,
    /// Bitwise AND across elements.
    And,
    /// Bitwise OR across elements.
    Or,
    /// Bitwise XOR across elements.
    Xor,
}

impl TensorReduceOperator {
    /// Return the opcode name for this reduction.
    pub fn to_str(self) -> &'static str {
        match self {
            TensorReduceOperator::Add => "add",
            TensorReduceOperator::Multiply => "mul",
            TensorReduceOperator::Min => "min",
            TensorReduceOperator::Max => "max",
            TensorReduceOperator::And => "and",
            TensorReduceOperator::Or => "or",
            TensorReduceOperator::Xor => "xor",
        }
    }

    /// Parse a reduction operator from an opcode name.
    pub fn parse(text: &str) -> Option<Self> {
        <Self as std::str::FromStr>::from_str(text).ok()
    }
}

impl std::str::FromStr for TensorReduceOperator {
    type Err = ();

    /// Parse a reduction operator from an opcode name.
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let value = match text {
            "add" => TensorReduceOperator::Add,
            "mul" => TensorReduceOperator::Multiply,
            "min" => TensorReduceOperator::Min,
            "max" => TensorReduceOperator::Max,
            "and" => TensorReduceOperator::And,
            "or" => TensorReduceOperator::Or,
            "xor" => TensorReduceOperator::Xor,
            _ => return Err(()),
        };

        Ok(value)
    }
}

/// Update modes for tensor scatter operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TensorScatterMode {
    /// Replace the destination element.
    Replace,
    /// Add to the destination element.
    Add,
    /// Multiply the destination element.
    Multiply,
    /// Minimum of destination and update.
    Min,
    /// Maximum of destination and update.
    Max,
    /// Bitwise AND with destination.
    And,
    /// Bitwise OR with destination.
    Or,
    /// Bitwise XOR with destination.
    Xor,
}

impl TensorScatterMode {
    /// Return the opcode name for this scatter mode.
    pub fn to_str(self) -> &'static str {
        match self {
            TensorScatterMode::Replace => "replace",
            TensorScatterMode::Add => "add",
            TensorScatterMode::Multiply => "mul",
            TensorScatterMode::Min => "min",
            TensorScatterMode::Max => "max",
            TensorScatterMode::And => "and",
            TensorScatterMode::Or => "or",
            TensorScatterMode::Xor => "xor",
        }
    }

    /// Parse a scatter mode from an opcode name.
    pub fn parse(text: &str) -> Option<Self> {
        <Self as std::str::FromStr>::from_str(text).ok()
    }
}

impl std::str::FromStr for TensorScatterMode {
    type Err = ();

    /// Parse a scatter mode from an opcode name.
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let value = match text {
            "replace" => TensorScatterMode::Replace,
            "add" => TensorScatterMode::Add,
            "mul" => TensorScatterMode::Multiply,
            "min" => TensorScatterMode::Min,
            "max" => TensorScatterMode::Max,
            "and" => TensorScatterMode::And,
            "or" => TensorScatterMode::Or,
            "xor" => TensorScatterMode::Xor,
            _ => return Err(()),
        };

        Ok(value)
    }
}

/// Dimension numbers for tensor dot operations.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TensorDotDimensionNumbers {
    /// Batch dimensions on the left operand.
    pub lhs_batch: Vec<u32>,
    /// Batch dimensions on the right operand.
    pub rhs_batch: Vec<u32>,
    /// Contracting dimensions on the left operand.
    pub lhs_contracting: Vec<u32>,
    /// Contracting dimensions on the right operand.
    pub rhs_contracting: Vec<u32>,
}

/// Dimension numbers for tensor convolution operations.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TensorConvolutionDimensionNumbers {
    /// Batch dimension for the input tensor.
    pub input_batch: u32,
    /// Feature dimension for the input tensor.
    pub input_feature: u32,
    /// Spatial dimensions for the input tensor.
    pub input_spatial: Vec<u32>,
    /// Input feature dimension for the kernel tensor.
    pub kernel_input_feature: u32,
    /// Output feature dimension for the kernel tensor.
    pub kernel_output_feature: u32,
    /// Spatial dimensions for the kernel tensor.
    pub kernel_spatial: Vec<u32>,
    /// Batch dimension for the output tensor.
    pub output_batch: u32,
    /// Feature dimension for the output tensor.
    pub output_feature: u32,
    /// Spatial dimensions for the output tensor.
    pub output_spatial: Vec<u32>,
}

/// Window parameters for tensor convolution.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TensorConvolutionWindow {
    /// Stride for each spatial dimension.
    pub strides: Vec<u64>,
    /// Padding at the low end for each spatial dimension.
    pub padding_low: Vec<u64>,
    /// Padding at the high end for each spatial dimension.
    pub padding_high: Vec<u64>,
    /// Input dilation for each spatial dimension.
    pub lhs_dilation: Vec<u64>,
    /// Kernel dilation for each spatial dimension.
    pub rhs_dilation: Vec<u64>,
    /// Whether each spatial dimension is reversed.
    pub window_reversal: Vec<bool>,
}

/// Dimension numbers for tensor gather operations.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TensorGatherDimensionNumbers {
    /// Offset dimensions in the output.
    pub offset_dims: Vec<u32>,
    /// Collapsed slice dimensions in the operand.
    pub collapsed_slice_dims: Vec<u32>,
    /// Mapping from index components to operand dimensions.
    pub start_index_map: Vec<u32>,
    /// Index vector dimension in the indices tensor.
    pub index_vector_dim: u32,
}

/// Dimension numbers for tensor scatter operations.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TensorScatterDimensionNumbers {
    /// Dimensions of the update window in the updates tensor.
    pub update_window_dims: Vec<u32>,
    /// Dimensions inserted into the operand shape.
    pub inserted_window_dims: Vec<u32>,
    /// Mapping from scatter indices to operand dimensions.
    pub scatter_dims_to_operand_dims: Vec<u32>,
    /// Index vector dimension in the indices tensor.
    pub index_vector_dim: u32,
}
