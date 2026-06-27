use destack_mir::TensorDimension;
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::vm::error::{Error, Result};
use crate::{AddressSpace, ScalarFormat};

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
    /// Return the tensor address for one address space.
    pub fn from_address_space(address_space: AddressSpace) -> Result<Self> {
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
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct TensorLayout {
    /// The tensor payload byte width.
    pub byte_len: usize,
    /// The static tensor shape.
    pub shape: Box<[u64]>,
    /// The per-dimension strides in element units.
    pub strides: Box<[u64]>,
    /// The number of logical tensor elements.
    pub element_count: usize,
    /// The number of addressable element positions.
    pub element_span_len: usize,
    /// Whether logical elements are stored contiguously.
    pub is_contiguous: bool,
    /// The scalar layout of each element.
    pub element_layout: ScalarFormat,
    /// The frame projection for each element.
    pub element: Projection,
}

/// Convert tensor dimensions to a static shape.
pub fn static_shape(shape: &[TensorDimension]) -> Result<Vec<u64>> {
    let mut dims = Vec::with_capacity(shape.len());
    for dim in shape {
        match dim {
            TensorDimension::Static(value) => dims.push(*value),
            TensorDimension::Dynamic => {
                return Err(Error::unsupported_instruction("tensor dynamic shape"));
            }
            TensorDimension::Symbol(name) => {
                return Err(Error::unsupported_instruction(format!(
                    "tensor symbolic shape {name}"
                )));
            }
        }
    }

    Ok(dims)
}

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
