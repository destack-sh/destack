use std::collections::HashMap;

use destack_mir as mir;

use crate::{Error, ReferenceMeta, Result};

use super::{
    ElementAccess, Layout, PointerClass, ScalarLayout, scalar_layout_from_type,
    word_layout_from_type,
};

/// Flattened tensor layout compiled for VM execution.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct TensorLayout {
    /// The tensor value byte width.
    pub(crate) byte_len: usize,
    /// The static tensor shape.
    pub(crate) shape: Box<[u64]>,
    /// The per-dimension strides in element units.
    pub(crate) strides: Box<[u64]>,
    /// The number of addressable element positions.
    pub(crate) element_span_len: usize,
    /// The scalar layout of each element.
    pub(crate) element_layout: ScalarLayout,
    /// The frame access for each element.
    pub(crate) element: ElementAccess,
}

impl TensorLayout {
    /// Compile one tensor layout from one MIR tensor type.
    pub(crate) fn from_type(
        tree: &mir::Tree,
        layouts: &HashMap<mir::LocalNodeId<mir::Type>, Layout>,
        ty: mir::LocalNodeId<mir::Type>,
    ) -> Result<Self> {
        // resolve tensor type data
        let (shape, layout, element) = match tree.get(ty) {
            mir::Type::Tensor {
                shape,
                layout,
                element,
                ..
            }
            | mir::Type::TensorView {
                shape,
                layout,
                element,
                ..
            } => (shape, layout, element),
            _ => {
                return Err(Error::TypeMismatch {
                    expected: "tensor type".to_string(),
                    actual: format!("{ty:?}"),
                });
            }
        };
        let element = element.ty().ok_or_else(|| Error::MissingRepresentation {
            context: "tensor element type".to_string(),
        })?;

        // compile shape and strides
        let shape = static_shape(shape)?;
        let strides = match layout {
            mir::TensorLayout::RowMajor => Ok(row_major_strides(&shape)),
            mir::TensorLayout::ColumnMajor => Ok(column_major_strides(&shape)),
            mir::TensorLayout::Strided { strides } => static_strides(strides),
        }?;
        let element_span_len = tensor_element_span_len(&shape, &strides)?;

        // compile frame storage access
        let value_layout = layouts.get(&ty).ok_or(Error::InvalidInstruction)?;
        let element_layout = layouts.get(&element).ok_or(Error::InvalidInstruction)?;
        let element_access = ElementAccess {
            pointer_class: PointerClass::Frame,
            reference: ReferenceMeta::NONE,
            value_type: element,
            length: element_span_len as u64,
            byte_stride: element_layout.stride(),
            byte_len: element_layout.byte_len,
            word_layout: word_layout_from_type(tree, element),
        };
        let element_layout =
            scalar_layout_from_type(tree, element).ok_or_else(|| Error::TypeMismatch {
                expected: "tensor scalar element".to_string(),
                actual: format!("{element:?}"),
            })?;

        Ok(Self {
            byte_len: value_layout.byte_len,
            shape: shape.into_boxed_slice(),
            strides: strides.into_boxed_slice(),
            element_span_len,
            element_layout,
            element: element_access,
        })
    }
}

/// Convert tensor dimensions to a static shape.
pub(crate) fn static_shape(shape: &[mir::TensorDimension]) -> Result<Vec<u64>> {
    let mut dims = Vec::with_capacity(shape.len());
    for dim in shape {
        match dim {
            mir::TensorDimension::Static(value) => dims.push(*value),
            mir::TensorDimension::Dynamic => {
                return Err(Error::UnsupportedInstruction {
                    name: "tensor dynamic shape".to_string(),
                });
            }
        }
    }

    Ok(dims)
}

/// Convert tensor strides to a static list.
pub(crate) fn static_strides(strides: &[mir::TensorDimension]) -> Result<Vec<u64>> {
    let mut values = Vec::with_capacity(strides.len());
    for dim in strides {
        match dim {
            mir::TensorDimension::Static(value) => values.push(*value),
            mir::TensorDimension::Dynamic => {
                return Err(Error::UnsupportedInstruction {
                    name: "tensor dynamic stride".to_string(),
                });
            }
        }
    }

    Ok(values)
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
