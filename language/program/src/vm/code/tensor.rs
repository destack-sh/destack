use destack_core::{EntryRange, EntryStore, SectionEntry};
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::vm::error::{Error, Result};
use crate::{CellLayout, ScalarFormat};

use super::Projection;

/// Tensor view backing memory selected by lowering.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum TensorAddress {
    /// Local heap memory.
    Heap,
    /// Shared heap memory.
    SharedHeap,
    /// Native address memory.
    Address,
    /// Stack memory.
    Stack,
    /// Frame memory.
    Frame,
    /// Static memory.
    Static,
}

impl TensorAddress {
    /// Return the tensor address for one pointer cell layout.
    pub fn from_cell_layout(cell_layout: CellLayout) -> Result<Self> {
        Ok(match cell_layout {
            CellLayout::HeapReference => Self::Heap,
            CellLayout::SharedHeapReference => Self::SharedHeap,
            CellLayout::Address => Self::Address,
            CellLayout::StackPointer => Self::Stack,
            CellLayout::FramePointer => Self::Frame,
            CellLayout::GlobalAddress => Self::Static,
            _ => return Err(Error::invalid_instruction()),
        })
    }
}

/// Flattened tensor layout compiled for VM execution.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct TensorLayout {
    /// The tensor payload byte width.
    pub byte_len: u64,
    /// The static tensor shape.
    pub shape: EntryRange<u64>,
    /// The per-dimension strides in element units.
    pub strides: EntryRange<u64>,
    /// The number of logical tensor elements.
    pub element_count: u64,
    /// The number of addressable element positions.
    pub element_span_len: u64,
    /// Whether logical elements are stored contiguously.
    pub is_contiguous: u32,
    /// The scalar layout of each element.
    pub element_layout: ScalarFormat,
    /// The frame projection for each element.
    pub element: Projection,
}

impl TensorLayout {
    /// Return the tensor payload byte width.
    pub const fn byte_len(&self) -> usize {
        self.byte_len as usize
    }

    /// Return the logical tensor element count.
    pub const fn element_count(&self) -> usize {
        self.element_count as usize
    }

    /// Return the addressable tensor element span length.
    pub const fn element_span_len(&self) -> usize {
        self.element_span_len as usize
    }

    /// Return whether the tensor is contiguous.
    pub const fn is_contiguous(&self) -> bool {
        self.is_contiguous != 0
    }
}

/// Build-time flattened tensor layout compiled for VM execution.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct TensorLayoutBuilder {
    /// The tensor payload byte width.
    pub byte_len: u64,
    /// The static tensor shape.
    pub shape: Vec<u64>,
    /// The per-dimension strides in element units.
    pub strides: Vec<u64>,
    /// The number of logical tensor elements.
    pub element_count: u64,
    /// The number of addressable element positions.
    pub element_span_len: u64,
    /// Whether logical elements are stored contiguously.
    pub is_contiguous: bool,
    /// The scalar layout of each element.
    pub element_layout: ScalarFormat,
    /// The frame projection for each element.
    pub element: Projection,
}

impl TensorLayoutBuilder {
    /// Return whether two layouts share one contiguous element order.
    pub fn has_same_contiguous_order(&self, other: &Self) -> bool {
        self.is_contiguous
            && other.is_contiguous
            && self.shape == other.shape
            && self.strides == other.strides
    }

    /// Build this tensor layout into one section entry.
    pub(crate) fn build(self, u64_entries: &mut EntryStore<u64>) -> TensorLayout {
        TensorLayout {
            byte_len: self.byte_len,
            shape: u64_entries.append(self.shape),
            strides: u64_entries.append(self.strides),
            element_count: self.element_count,
            element_span_len: self.element_span_len,
            is_contiguous: u32::from(self.is_contiguous),
            element_layout: self.element_layout,
            element: self.element,
        }
    }
}

/// Borrowed VM tensor layout.
#[derive(Clone, Copy, Debug)]
pub struct TensorLayoutView<'a> {
    /// The fixed tensor layout entry.
    pub entry: TensorLayout,
    /// The static tensor shape.
    pub shape: &'a [u64],
    /// The per-dimension strides in element units.
    pub strides: &'a [u64],
}

impl TensorLayoutView<'_> {
    /// Return the tensor payload byte width.
    pub const fn byte_len(&self) -> usize {
        self.entry.byte_len()
    }

    /// Return the logical tensor element count.
    pub const fn element_count(&self) -> usize {
        self.entry.element_count()
    }

    /// Return the addressable tensor element span length.
    pub const fn element_span_len(&self) -> usize {
        self.entry.element_span_len()
    }

    /// Return whether the tensor is contiguous.
    pub const fn is_contiguous(&self) -> bool {
        self.entry.is_contiguous()
    }
}

/// Tensor dot dimension entries compiled for VM execution.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct TensorDotDimensions {
    /// Batch dimensions on the left operand.
    pub lhs_batch: EntryRange<u32>,
    /// Batch dimensions on the right operand.
    pub rhs_batch: EntryRange<u32>,
    /// Contracting dimensions on the left operand.
    pub lhs_contracting: EntryRange<u32>,
    /// Contracting dimensions on the right operand.
    pub rhs_contracting: EntryRange<u32>,
}

/// Build-time tensor dot dimensions.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct TensorDotDimensionsBuilder {
    /// Batch dimensions on the left operand.
    pub lhs_batch: Vec<u32>,
    /// Batch dimensions on the right operand.
    pub rhs_batch: Vec<u32>,
    /// Contracting dimensions on the left operand.
    pub lhs_contracting: Vec<u32>,
    /// Contracting dimensions on the right operand.
    pub rhs_contracting: Vec<u32>,
}

impl TensorDotDimensionsBuilder {
    /// Build this tensor dot dimension entry.
    pub(crate) fn build(self, u32_entries: &mut EntryStore<u32>) -> TensorDotDimensions {
        TensorDotDimensions {
            lhs_batch: u32_entries.append(self.lhs_batch),
            rhs_batch: u32_entries.append(self.rhs_batch),
            lhs_contracting: u32_entries.append(self.lhs_contracting),
            rhs_contracting: u32_entries.append(self.rhs_contracting),
        }
    }
}

/// Tensor convolution dimension entries compiled for VM execution.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct TensorConvolutionDimensions {
    /// Batch dimension for the input tensor.
    pub input_batch: u32,
    /// Feature dimension for the input tensor.
    pub input_feature: u32,
    /// Spatial dimensions for the input tensor.
    pub input_spatial: EntryRange<u32>,
    /// Input feature dimension for the kernel tensor.
    pub kernel_input_feature: u32,
    /// Output feature dimension for the kernel tensor.
    pub kernel_output_feature: u32,
    /// Spatial dimensions for the kernel tensor.
    pub kernel_spatial: EntryRange<u32>,
    /// Batch dimension for the output tensor.
    pub output_batch: u32,
    /// Feature dimension for the output tensor.
    pub output_feature: u32,
    /// Spatial dimensions for the output tensor.
    pub output_spatial: EntryRange<u32>,
}

/// Build-time tensor convolution dimensions.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct TensorConvolutionDimensionsBuilder {
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

impl TensorConvolutionDimensionsBuilder {
    /// Build this tensor convolution dimension entry.
    pub(crate) fn build(self, u32_entries: &mut EntryStore<u32>) -> TensorConvolutionDimensions {
        TensorConvolutionDimensions {
            input_batch: self.input_batch,
            input_feature: self.input_feature,
            input_spatial: u32_entries.append(self.input_spatial),
            kernel_input_feature: self.kernel_input_feature,
            kernel_output_feature: self.kernel_output_feature,
            kernel_spatial: u32_entries.append(self.kernel_spatial),
            output_batch: self.output_batch,
            output_feature: self.output_feature,
            output_spatial: u32_entries.append(self.output_spatial),
        }
    }
}

/// Tensor convolution window entries compiled for VM execution.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct TensorConvolutionWindow {
    /// Stride for each spatial dimension.
    pub strides: EntryRange<u64>,
    /// Padding at the low end for each spatial dimension.
    pub padding_low: EntryRange<u64>,
    /// Padding at the high end for each spatial dimension.
    pub padding_high: EntryRange<u64>,
    /// Input dilation for each spatial dimension.
    pub lhs_dilation: EntryRange<u64>,
    /// Kernel dilation for each spatial dimension.
    pub rhs_dilation: EntryRange<u64>,
    /// Whether each spatial dimension is reversed.
    pub window_reversal: EntryRange<u8>,
}

/// Build-time tensor convolution window.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct TensorConvolutionWindowBuilder {
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

impl TensorConvolutionWindowBuilder {
    /// Build this tensor convolution window entry.
    pub(crate) fn build(
        self,
        u64_entries: &mut EntryStore<u64>,
        flag_entries: &mut EntryStore<u8>,
    ) -> TensorConvolutionWindow {
        let window_reversal = self
            .window_reversal
            .into_iter()
            .map(u8::from)
            .collect::<Vec<_>>();

        TensorConvolutionWindow {
            strides: u64_entries.append(self.strides),
            padding_low: u64_entries.append(self.padding_low),
            padding_high: u64_entries.append(self.padding_high),
            lhs_dilation: u64_entries.append(self.lhs_dilation),
            rhs_dilation: u64_entries.append(self.rhs_dilation),
            window_reversal: flag_entries.append(window_reversal),
        }
    }
}

/// Tensor gather dimension entries compiled for VM execution.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct TensorGatherDimensions {
    /// Offset dimensions in the output.
    pub offset_dims: EntryRange<u32>,
    /// Collapsed slice dimensions in the operand.
    pub collapsed_slice_dims: EntryRange<u32>,
    /// Mapping from index components to operand dimensions.
    pub start_index_map: EntryRange<u32>,
    /// Index vector dimension in the indices tensor.
    pub index_vector_dim: u32,
}

/// Build-time tensor gather dimensions.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct TensorGatherDimensionsBuilder {
    /// Offset dimensions in the output.
    pub offset_dims: Vec<u32>,
    /// Collapsed slice dimensions in the operand.
    pub collapsed_slice_dims: Vec<u32>,
    /// Mapping from index components to operand dimensions.
    pub start_index_map: Vec<u32>,
    /// Index vector dimension in the indices tensor.
    pub index_vector_dim: u32,
}

impl TensorGatherDimensionsBuilder {
    /// Build this tensor gather dimension entry.
    pub(crate) fn build(self, u32_entries: &mut EntryStore<u32>) -> TensorGatherDimensions {
        TensorGatherDimensions {
            offset_dims: u32_entries.append(self.offset_dims),
            collapsed_slice_dims: u32_entries.append(self.collapsed_slice_dims),
            start_index_map: u32_entries.append(self.start_index_map),
            index_vector_dim: self.index_vector_dim,
        }
    }
}

/// Tensor scatter dimension entries compiled for VM execution.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct TensorScatterDimensions {
    /// Dimensions of the update window in the updates tensor.
    pub update_window_dims: EntryRange<u32>,
    /// Dimensions inserted into the operand shape.
    pub inserted_window_dims: EntryRange<u32>,
    /// Mapping from scatter indices to operand dimensions.
    pub scatter_dims_to_operand_dims: EntryRange<u32>,
    /// Index vector dimension in the indices tensor.
    pub index_vector_dim: u32,
}

/// Build-time tensor scatter dimensions.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct TensorScatterDimensionsBuilder {
    /// Dimensions of the update window in the updates tensor.
    pub update_window_dims: Vec<u32>,
    /// Dimensions inserted into the operand shape.
    pub inserted_window_dims: Vec<u32>,
    /// Mapping from scatter indices to operand dimensions.
    pub scatter_dims_to_operand_dims: Vec<u32>,
    /// Index vector dimension in the indices tensor.
    pub index_vector_dim: u32,
}

impl TensorScatterDimensionsBuilder {
    /// Build this tensor scatter dimension entry.
    pub(crate) fn build(self, u32_entries: &mut EntryStore<u32>) -> TensorScatterDimensions {
        TensorScatterDimensions {
            update_window_dims: u32_entries.append(self.update_window_dims),
            inserted_window_dims: u32_entries.append(self.inserted_window_dims),
            scatter_dims_to_operand_dims: u32_entries.append(self.scatter_dims_to_operand_dims),
            index_vector_dim: self.index_vector_dim,
        }
    }
}

// SAFETY: tensor entries contain only fixed section entries and scalars.
unsafe impl SectionEntry for TensorDotDimensions {}
unsafe impl SectionEntry for TensorConvolutionDimensions {}
unsafe impl SectionEntry for TensorConvolutionWindow {}
unsafe impl SectionEntry for TensorGatherDimensions {}
unsafe impl SectionEntry for TensorScatterDimensions {}
unsafe impl SectionEntry for TensorLayout {}

/// Compute row-major strides for a shape.
pub fn row_major_strides(shape: &[u64]) -> Vec<u64> {
    let mut strides = vec![1; shape.len()];
    let mut stride = 1u64;
    for (index, dim) in shape.iter().enumerate().rev() {
        strides[index] = stride;
        stride *= *dim;
    }

    strides
}

/// Compute column-major strides for a shape.
pub fn column_major_strides(shape: &[u64]) -> Vec<u64> {
    let mut strides = vec![1; shape.len()];
    let mut stride = 1u64;
    for (index, dim) in shape.iter().enumerate() {
        strides[index] = stride;
        stride *= *dim;
    }

    strides
}

/// Compute the logical element count for a static tensor shape.
pub fn tensor_element_count(shape: &[u64]) -> usize {
    let mut count = 1usize;
    for dim in shape {
        count *= *dim as usize;
    }

    count
}

/// Compute the addressable element span for a shape and stride list.
pub fn tensor_element_span_len(shape: &[u64], strides: &[u64]) -> Result<usize> {
    if shape.is_empty() {
        return Ok(1);
    }

    if shape.contains(&0) {
        return Ok(0);
    }

    let mut max_index = 0u64;
    for (dim, stride) in shape.iter().zip(strides.iter()) {
        let count = dim - 1;
        let distance = count * *stride;
        max_index += distance;
    }

    Ok((max_index + 1) as usize)
}
