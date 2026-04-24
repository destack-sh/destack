//! Type layout calculation for target-specific memory layout.
//!
//! This module computes the size, alignment, and field/element offsets for
//! MIR types. Layout is target-dependent, primarily influenced by pointer size.
//!
//! The layout computation follows C-like rules:
//! - Scalars are naturally aligned (alignment == size)
//! - Structs are aligned to their most-aligned field
//! - Struct fields are placed at the next aligned offset
//! - Arrays are aligned to their element type
//! - Tuples are laid out like struct layouts

use destack_mir as mir;

use crate::{CodegenCraneliftError, CodegenCraneliftResult};

/// Computed layout information for a type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TypeLayout {
    /// Size in bytes.
    pub size: u32,
    /// Alignment in bytes (always a power of 2).
    pub alignment: u32,
}

impl TypeLayout {
    /// Create a layout with explicit size and alignment.
    pub(crate) fn new(size: u32, alignment: u32) -> Self {
        debug_assert!(alignment.is_power_of_two() || alignment == 0);
        Self { size, alignment }
    }

    /// Create a layout for a type with natural alignment (alignment == size).
    pub(crate) fn natural(size: u32) -> Self {
        Self {
            size,
            alignment: size,
        }
    }

    /// Compute the offset for a field with this layout, given the current offset.
    /// Returns the aligned offset where the field should be placed.
    fn align_offset(self, current_offset: u32) -> u32 {
        if self.alignment == 0 {
            return current_offset;
        }
        let misalignment = current_offset % self.alignment;
        if misalignment == 0 {
            current_offset
        } else {
            current_offset + (self.alignment - misalignment)
        }
    }
}

/// Compute the layout of a MIR type.
pub(crate) fn compute_type_layout(
    tree: &mir::NodeTree,
    type_id: mir::LocalNodeId<mir::Type>,
    pointer_bytes: u8,
) -> CodegenCraneliftResult<TypeLayout> {
    // use explicit layout metadata when available
    if let Some(layout) = tree.metadata.layout.type_layout(type_id) {
        return Ok(TypeLayout::new(layout.size, layout.alignment));
    }

    let mir_type = tree.get(type_id);
    if matches!(
        mir_type,
        mir::Type::Struct { .. }
            | mir::Type::Tuple { .. }
            | mir::Type::Array { .. }
            | mir::Type::Callable { .. }
    ) {
        return Err(CodegenCraneliftError::unsupported_type(
            "missing layout metadata",
            type_id.into(),
        ));
    }
    match mir_type {
        // void has zero size
        mir::Type::Void => Ok(TypeLayout::new(0, 1)),

        // boolean is 1 byte
        mir::Type::Boolean => Ok(TypeLayout::natural(1)),

        // integer types
        mir::Type::Int { width, .. } => {
            let bytes = (*width as u32).div_ceil(8);
            Ok(TypeLayout::natural(bytes))
        }
        mir::Type::Isize | mir::Type::Usize => {
            let bytes = pointer_bytes as u32;
            Ok(TypeLayout::natural(bytes))
        }

        // float types
        mir::Type::Float { width } => {
            let bytes = (*width as u32).div_ceil(8);
            Ok(TypeLayout::natural(bytes))
        }

        // pointers and references
        mir::Type::TypeDescriptor
        | mir::Type::TypeId
        | mir::Type::Reference { .. }
        | mir::Type::FunctionPointer { .. } => Ok(TypeLayout::natural(pointer_bytes as u32)),
        mir::Type::FunctionSignature { .. } => Err(CodegenCraneliftError::unsupported_type(
            "function signatures do not have a runtime layout",
            type_id.into_any(),
        )),

        // arrays: size = element_size * length, alignment = element alignment
        mir::Type::Array {
            element,
            length,
            copy: _,
        } => {
            let element = element
                .ty()
                .ok_or_else(|| CodegenCraneliftError::Internal {
                    message: "missing or malformed MIR type in native lowering: array element type"
                        .into(),
                })?;
            let element_layout = compute_type_layout(tree, element, pointer_bytes)?;
            let size = element_layout.size * (*length as u32);
            Ok(TypeLayout::new(size, element_layout.alignment))
        }

        // slices: laid out like a builtin two field record
        mir::Type::Slice {
            kind,
            element,
            address_space,
            mutability,
        } => {
            let (data, _length) =
                mir::slice_header_types(*kind, *element, *mutability, address_space.clone());
            let data = tree
                .iter_nodes::<mir::Type>()
                .find_map(|(type_id, ty)| (ty == &data).then_some(type_id))
                .ok_or_else(|| CodegenCraneliftError::Internal {
                    message: "missing or malformed MIR type in native lowering: slice data type"
                        .into(),
                })?;
            let length = tree.usize_type();
            let fields: [mir::TypeReference; 2] = [data.into(), length.into()];
            let fields = fields
                .iter()
                .map(|field| {
                    field.ty().ok_or_else(|| CodegenCraneliftError::Internal {
                        message:
                            "missing or malformed MIR type in native lowering: slice field type"
                                .into(),
                    })
                })
                .collect::<Result<Vec<_>, _>>()?;

            compute_tuple_layout(tree, &fields, pointer_bytes)
        }

        // tuples: laid out like a struct with sequential fields
        mir::Type::Tuple { elements, copy: _ } => {
            let elements = elements
                .iter()
                .map(|element| {
                    element.ty().ok_or_else(|| CodegenCraneliftError::Internal {
                        message:
                            "missing or malformed MIR type in native lowering: tuple element type"
                                .into(),
                    })
                })
                .collect::<CodegenCraneliftResult<Vec<_>>>()?;
            compute_tuple_layout(tree, &elements, pointer_bytes)
        }

        // structs: read canonical layout metadata
        mir::Type::Struct { fields: _, copy: _ } => {
            let Some(layout) = tree.metadata.layout.type_layout(type_id) else {
                return Err(CodegenCraneliftError::unsupported_type(
                    "missing layout metadata",
                    type_id.into(),
                ));
            };
            Ok(TypeLayout::new(layout.size, layout.alignment))
        }

        // callables: read canonical layout metadata
        mir::Type::Callable { .. } => {
            let Some(layout) = tree.metadata.layout.type_layout(type_id) else {
                return Err(CodegenCraneliftError::unsupported_type(
                    "missing layout metadata",
                    type_id.into(),
                ));
            };
            Ok(TypeLayout::new(layout.size, layout.alignment))
        }

        // newtypes are transparent
        mir::Type::Newtype { inner, .. } => {
            let inner = inner.ty().ok_or_else(|| CodegenCraneliftError::Internal {
                message: "missing or malformed MIR type in native lowering: newtype inner type"
                    .into(),
            })?;
            compute_type_layout(tree, inner, pointer_bytes)
        }

        // vectors: treat as packed elements for now
        mir::Type::Vector {
            element,
            lanes,
            copy: _,
        } => {
            let element = element
                .ty()
                .ok_or_else(|| CodegenCraneliftError::Internal {
                    message:
                        "missing or malformed MIR type in native lowering: vector element type"
                            .into(),
                })?;
            let element_layout = compute_type_layout(tree, element, pointer_bytes)?;
            let size = element_layout.size * *lanes;
            Ok(TypeLayout::new(size, element_layout.alignment))
        }

        // tensors: packed element storage based on layout
        mir::Type::Tensor {
            element,
            shape,
            layout,
            copy: _,
        } => {
            let element = element
                .ty()
                .ok_or_else(|| CodegenCraneliftError::Internal {
                    message:
                        "missing or malformed MIR type in native lowering: tensor element type"
                            .into(),
                })?;
            let element_layout = compute_type_layout(tree, element, pointer_bytes)?;
            let element_count = compute_tensor_element_count(shape, layout);
            if let Some(element_count) = element_count {
                let size = element_layout.size * (element_count as u32);
                Ok(TypeLayout::new(size, element_layout.alignment))
            } else {
                Ok(TypeLayout::natural(pointer_bytes as u32))
            }
        }

        // tensor views are reference-like
        mir::Type::TensorView { .. } => Ok(TypeLayout::natural(pointer_bytes as u32)),
    }
}

fn compute_tensor_element_count(
    shape: &[mir::TensorDimension],
    layout: &mir::TensorLayout,
) -> Option<u64> {
    if shape.iter().any(|dim| dim.is_dynamic()) {
        return None;
    }
    match layout {
        mir::TensorLayout::RowMajor | mir::TensorLayout::ColumnMajor => {
            Some(shape.iter().filter_map(static_dim).product())
        }
        mir::TensorLayout::Strided { strides } => {
            if strides.iter().any(|stride| stride.is_dynamic()) {
                return None;
            }
            let mut max_index = 0u64;
            for (dim, stride) in shape
                .iter()
                .filter_map(static_dim)
                .zip(strides.iter().filter_map(static_dim))
            {
                if dim == 0 {
                    continue;
                }
                let last_index = dim - 1;
                let offset = last_index.saturating_mul(stride);
                max_index = max_index.max(offset);
            }
            Some(max_index.saturating_add(1))
        }
    }
}

fn static_dim(dim: &mir::TensorDimension) -> Option<u64> {
    match dim {
        mir::TensorDimension::Static(value) => Some(*value),
        mir::TensorDimension::Dynamic => None,
    }
}

/// Compute the layout of a tuple type.
fn compute_tuple_layout(
    tree: &mir::NodeTree,
    elements: &[mir::LocalNodeId<mir::Type>],
    pointer_bytes: u8,
) -> CodegenCraneliftResult<TypeLayout> {
    if elements.is_empty() {
        return Ok(TypeLayout::new(0, 1));
    }

    // compute layout
    let mut max_end = 0u32;
    let mut max_alignment = 1u32;
    for &element_type_id in elements {
        // element type layout
        let element_layout = compute_type_layout(tree, element_type_id, pointer_bytes)?;

        // align to element's alignment
        max_end = element_layout.align_offset(max_end);

        // advance past the element
        max_end += element_layout.size;

        // track max alignment
        max_alignment = max_alignment.max(element_layout.alignment);
    }

    // final size is padded to alignment
    let final_size = if max_alignment > 0 {
        let padded = TypeLayout::new(0, max_alignment).align_offset(max_end);
        padded.max(max_end)
    } else {
        max_end
    };

    Ok(TypeLayout::new(final_size, max_alignment))
}

/// Compute the offset of a tuple element by index.
///
/// NOTE: Callers should typically verify bounds before calling, as they have
/// access to more information for better error reporting.
pub(crate) fn compute_tuple_element_offset(
    tree: &mir::NodeTree,
    elements: &[mir::LocalNodeId<mir::Type>],
    index: u32,
    pointer_bytes: u8,
) -> CodegenCraneliftResult<u32> {
    // callers should have already verified bounds
    assert!(
        (index as usize) < elements.len(),
        "tuple element index {index} out of bounds (len {})",
        elements.len()
    );

    // compute offset
    let mut offset = 0u32;
    for (i, &element_type_id) in elements.iter().enumerate() {
        // element type layout
        let element_layout = compute_type_layout(tree, element_type_id, pointer_bytes)?;

        // align to element's alignment
        offset = element_layout.align_offset(offset);

        // reached the desired element
        if i == index as usize {
            return Ok(offset);
        }

        // advance past the element
        offset += element_layout.size;
    }

    // should never happen
    unreachable!(
        "tuple element index {index} out of bounds (len {})",
        elements.len()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_layout_align_offset() {
        let layout = TypeLayout::natural(4); // 4-byte alignment

        assert_eq!(layout.align_offset(0), 0);
        assert_eq!(layout.align_offset(1), 4);
        assert_eq!(layout.align_offset(2), 4);
        assert_eq!(layout.align_offset(3), 4);
        assert_eq!(layout.align_offset(4), 4);
        assert_eq!(layout.align_offset(5), 8);
    }

    #[test]
    fn test_type_layout_natural() {
        let layout = TypeLayout::natural(8);
        assert_eq!(layout.size, 8);
        assert_eq!(layout.alignment, 8);
    }
}
