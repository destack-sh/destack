use destack_serde::Schema;
use serde::{Deserialize, Serialize};

/// Compact reference to an index list stored in the MIR tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash, Serialize, Deserialize, Schema)]
pub struct IndexSlice {
    /// Start index in the index buffer.
    pub start: u32,
    /// Number of indices in the slice.
    pub count: u16,
}

impl IndexSlice {
    /// Create a new index slice.
    #[inline]
    pub const fn new(start: u32, count: u16) -> Self {
        Self { start, count }
    }

    /// Return whether this slice is empty.
    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Return the number of indices in this slice.
    #[inline]
    pub const fn len(&self) -> usize {
        self.count as usize
    }
}

/// Compact reference to an extent list stored in the MIR tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash, Serialize, Deserialize, Schema)]
pub struct ExtentSlice {
    /// Start index in the extent buffer.
    pub start: u32,
    /// Number of extents in the slice.
    pub count: u16,
}

impl ExtentSlice {
    /// Create a new extent slice.
    #[inline]
    pub const fn new(start: u32, count: u16) -> Self {
        Self { start, count }
    }

    /// Return whether this slice is empty.
    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Return the number of extents in this slice.
    #[inline]
    pub const fn len(&self) -> usize {
        self.count as usize
    }
}

/// Compact reference to a flag list stored in the MIR tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash, Serialize, Deserialize, Schema)]
pub struct FlagSlice {
    /// Start index in the flag buffer.
    pub start: u32,
    /// Number of flags in the slice.
    pub count: u16,
}

impl FlagSlice {
    /// Create a new flag slice.
    #[inline]
    pub const fn new(start: u32, count: u16) -> Self {
        Self { start, count }
    }

    /// Return whether this slice is empty.
    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Return the number of flags in this slice.
    #[inline]
    pub const fn len(&self) -> usize {
        self.count as usize
    }
}

/// Compact identity for one tensor immediate stored in the MIR tree.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Schema)]
pub struct TensorImmediateId(pub u32);

impl TensorImmediateId {
    /// Create a new tensor immediate id.
    #[inline]
    pub const fn new(id: u32) -> Self {
        Self(id)
    }

    /// Return the numeric id.
    #[inline]
    pub const fn id(self) -> u32 {
        self.0
    }
}

/// Static tensor instruction immediate.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Schema)]
pub enum TensorImmediate {
    /// Dimension numbers for a tensor dot product.
    Dot {
        /// Batch dimensions on the left operand.
        lhs_batch: IndexSlice,
        /// Batch dimensions on the right operand.
        rhs_batch: IndexSlice,
        /// Contracting dimensions on the left operand.
        lhs_contracting: IndexSlice,
        /// Contracting dimensions on the right operand.
        rhs_contracting: IndexSlice,
    },
    /// Dimension numbers and window parameters for tensor convolution.
    Convolution {
        /// Batch dimension for the input tensor.
        input_batch: u32,
        /// Feature dimension for the input tensor.
        input_feature: u32,
        /// Spatial dimensions for the input tensor.
        input_spatial: IndexSlice,
        /// Input feature dimension for the kernel tensor.
        kernel_input_feature: u32,
        /// Output feature dimension for the kernel tensor.
        kernel_output_feature: u32,
        /// Spatial dimensions for the kernel tensor.
        kernel_spatial: IndexSlice,
        /// Batch dimension for the output tensor.
        output_batch: u32,
        /// Feature dimension for the output tensor.
        output_feature: u32,
        /// Spatial dimensions for the output tensor.
        output_spatial: IndexSlice,
        /// Stride for each spatial dimension.
        strides: ExtentSlice,
        /// Padding at the low end for each spatial dimension.
        padding_low: ExtentSlice,
        /// Padding at the high end for each spatial dimension.
        padding_high: ExtentSlice,
        /// Input dilation for each spatial dimension.
        lhs_dilation: ExtentSlice,
        /// Kernel dilation for each spatial dimension.
        rhs_dilation: ExtentSlice,
        /// Whether each spatial dimension is reversed.
        window_reversal: FlagSlice,
        /// Number of feature groups.
        feature_group_count: u32,
        /// Number of batch groups.
        batch_group_count: u32,
    },
    /// Dimension numbers for tensor gather.
    Gather {
        /// Offset dimensions in the output.
        offset_dims: IndexSlice,
        /// Collapsed slice dimensions in the operand.
        collapsed_slice_dims: IndexSlice,
        /// Mapping from index components to operand dimensions.
        start_index_map: IndexSlice,
        /// Index vector dimension in the indices tensor.
        index_vector_dim: u32,
        /// Slice sizes for each operand dimension.
        slice_sizes: IndexSlice,
    },
    /// Dimension numbers for tensor scatter.
    Scatter {
        /// Dimensions of the update window in the updates tensor.
        update_window_dims: IndexSlice,
        /// Dimensions inserted into the operand shape.
        inserted_window_dims: IndexSlice,
        /// Mapping from scatter indices to operand dimensions.
        scatter_dims_to_operand_dims: IndexSlice,
        /// Index vector dimension in the indices tensor.
        index_vector_dim: u32,
    },
}
