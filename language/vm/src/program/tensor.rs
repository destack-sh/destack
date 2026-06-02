use std::collections::HashMap;

use destack_mir as mir;

use crate::{Error, Result};

use super::{
    AddressSpace, Layout, Projection, ScalarLayout, cell_layout_from_type, scalar_layout_from_type,
};

/// Tensor view backing memory selected by lowering.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TensorAddress {
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
    /// Return the tensor address for one address space.
    pub(crate) fn from_address_space(address_space: AddressSpace) -> Result<Self> {
        Ok(match address_space {
            AddressSpace::Local => Self::Heap,
            AddressSpace::Shared => Self::SharedHeap,
            AddressSpace::Raw => Self::Address,
            AddressSpace::Stack => Self::Stack,
            AddressSpace::Frame => Self::Frame,
            AddressSpace::Static => Self::Static,
        })
    }
}

/// Flattened tensor layout compiled for VM execution.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct TensorLayout {
    /// The tensor value byte width.
    pub(crate) byte_len: usize,
    /// The static tensor shape.
    pub(crate) shape: Box<[u64]>,
    /// The per-dimension strides in element units.
    pub(crate) strides: Box<[u64]>,
    /// The number of logical tensor elements.
    pub(crate) element_count: usize,
    /// The number of addressable element positions.
    pub(crate) element_span_len: usize,
    /// Whether logical elements are stored contiguously.
    pub(crate) is_contiguous: bool,
    /// The scalar layout of each element.
    pub(crate) element_layout: ScalarLayout,
    /// The frame projection for each element.
    pub(crate) element: Projection,
}

impl TensorLayout {
    /// Compile one tensor layout from one MIR tensor type.
    pub(crate) fn from_type(
        tree: &mir::Tree,
        layouts: &HashMap<mir::LocalNodeId<mir::Type>, Layout>,
        ty: mir::LocalNodeId<mir::Type>,
    ) -> Result<Self> {
        // resolve tensor type data
        let (shape, element) = match tree.get(ty) {
            mir::Type::Tensor { shape, element, .. } => (shape, element),
            mir::Type::TensorView { shape, element, .. } => (shape, element),
            _ => {
                return Err(Error::type_mismatch("tensor type", format!("{ty:?}")));
            }
        };
        let element = element
            .ty()
            .ok_or_else(|| Error::invalid_program("tensor element type"))?;

        // compile shape
        let shape = static_shape(shape)?;
        let strides = match tree.get(ty) {
            mir::Type::Tensor { layout, .. } => static_tensor_strides(&shape, layout)?,
            mir::Type::TensorView { layout, .. } => static_tensor_view_strides(&shape, layout)?,
            _ => return Err(Error::invalid_instruction()),
        };
        let element_count = tensor_element_count(&shape);
        let element_span_len = tensor_element_span_len(&shape, &strides)?;
        let is_contiguous = element_count == element_span_len;

        // compile frame element projection
        let value_shape = layouts.get(&ty).ok_or(Error::invalid_instruction())?;
        let element_layout = layouts.get(&element).ok_or(Error::invalid_instruction())?;
        let element_projection = Projection::indexed(
            element,
            element_span_len as u64,
            element_layout.stride(),
            element_layout.byte_len,
            cell_layout_from_type(tree, element),
        );
        let element_layout = scalar_layout_from_type(tree, element)
            .ok_or_else(|| Error::type_mismatch("tensor scalar element", format!("{element:?}")))?;

        Ok(Self {
            byte_len: value_shape.byte_len,
            shape: shape.into_boxed_slice(),
            strides: strides.into_boxed_slice(),
            element_count,
            element_span_len,
            is_contiguous,
            element_layout,
            element: element_projection,
        })
    }
}

/// Compile a static owning tensor stride list.
fn static_tensor_strides(shape: &[u64], layout: &mir::TensorLayout) -> Result<Vec<u64>> {
    match layout {
        mir::TensorLayout::Dense {
            order: mir::TensorDimensionOrder::RowMajor,
        } => Ok(row_major_strides(shape)),
        mir::TensorLayout::Dense {
            order: mir::TensorDimensionOrder::ColumnMajor,
        } => Ok(column_major_strides(shape)),
    }
}

/// Compile a static tensor view stride list.
fn static_tensor_view_strides(shape: &[u64], layout: &mir::TensorViewLayout) -> Result<Vec<u64>> {
    match layout {
        mir::TensorViewLayout::Dense {
            order: mir::TensorDimensionOrder::RowMajor,
        } => Ok(row_major_strides(shape)),
        mir::TensorViewLayout::Dense {
            order: mir::TensorDimensionOrder::ColumnMajor,
        } => Ok(column_major_strides(shape)),
        mir::TensorViewLayout::Strided => Ok(row_major_strides(shape)),
    }
}

/// Convert tensor dimensions to a static shape.
pub(crate) fn static_shape(shape: &[mir::TensorDimension]) -> Result<Vec<u64>> {
    let mut dims = Vec::with_capacity(shape.len());
    for dim in shape {
        match dim {
            mir::TensorDimension::Static(value) => dims.push(*value),
            mir::TensorDimension::Dynamic => {
                return Err(Error::unsupported_instruction("tensor dynamic shape"));
            }
            mir::TensorDimension::Symbol(name) => {
                return Err(Error::unsupported_instruction(format!(
                    "tensor symbolic shape {name}"
                )));
            }
        }
    }

    Ok(dims)
}

/// Compute row-major strides for a shape.
pub(crate) fn row_major_strides(shape: &[u64]) -> Vec<u64> {
    let mut strides = vec![1; shape.len()];
    let mut stride = 1u64;
    for (index, dim) in shape.iter().enumerate().rev() {
        strides[index] = stride;
        stride *= *dim;
    }

    strides
}

/// Compute column-major strides for a shape.
pub(crate) fn column_major_strides(shape: &[u64]) -> Vec<u64> {
    let mut strides = vec![1; shape.len()];
    let mut stride = 1u64;
    for (index, dim) in shape.iter().enumerate() {
        strides[index] = stride;
        stride *= *dim;
    }

    strides
}

/// Compute the logical element count for a static tensor shape.
pub(crate) fn tensor_element_count(shape: &[u64]) -> usize {
    let mut count = 1usize;
    for dim in shape {
        count *= *dim as usize;
    }

    count
}

/// Compute the addressable element span for a shape and stride list.
pub(crate) fn tensor_element_span_len(shape: &[u64], strides: &[u64]) -> Result<usize> {
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
