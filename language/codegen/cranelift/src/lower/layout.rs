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
//! - Tuples are laid out like anonymous structs

use destack_mir as mir;

use crate::CodegenCraneliftResult;

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
    let mir_type = tree.get(type_id);
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

        // float types
        mir::Type::Float { width } => {
            let bytes = (*width as u32).div_ceil(8);
            Ok(TypeLayout::natural(bytes))
        }

        // pointers and references
        mir::Type::RawPointer { .. }
        | mir::Type::ManagedReference { .. }
        | mir::Type::FunctionPointer { .. } => Ok(TypeLayout::natural(pointer_bytes as u32)),

        // arrays: size = element_size * length, alignment = element alignment
        mir::Type::Array { element, length } => {
            let element_layout = compute_type_layout(tree, *element, pointer_bytes)?;
            let size = element_layout.size * (*length as u32);
            Ok(TypeLayout::new(size, element_layout.alignment))
        }

        // tuples: laid out like a struct with sequential fields
        mir::Type::Tuple { elements } => compute_tuple_layout(tree, elements, pointer_bytes),

        // structs: fields have explicit offsets, but we still need to compute size/alignment
        mir::Type::Struct { fields } => compute_struct_layout(tree, fields, pointer_bytes),
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
    let mut offset = 0u32;
    let mut max_alignment = 1u32;
    for &element_type_id in elements {
        // element type layout
        let element_layout = compute_type_layout(tree, element_type_id, pointer_bytes)?;

        // align to element's alignment
        offset = element_layout.align_offset(offset);

        // advance past the element
        offset += element_layout.size;

        // track max alignment
        max_alignment = max_alignment.max(element_layout.alignment);
    }

    // final size is padded to alignment
    let final_size = if max_alignment > 0 {
        let padded = TypeLayout::new(0, max_alignment).align_offset(offset);
        padded.max(offset)
    } else {
        offset
    };

    Ok(TypeLayout::new(final_size, max_alignment))
}

/// Compute the layout of a struct type.
fn compute_struct_layout(
    tree: &mir::NodeTree,
    fields: &[mir::Field],
    pointer_bytes: u8,
) -> CodegenCraneliftResult<TypeLayout> {
    if fields.is_empty() {
        return Ok(TypeLayout::new(0, 1));
    }

    // compute layout
    let mut max_end = 0u32;
    let mut max_alignment = 1u32;
    for field in fields {
        let field_layout = compute_type_layout(tree, field.ty, pointer_bytes)?;

        // end of this field
        let field_end = field.offset + field_layout.size;
        max_end = max_end.max(field_end);

        // track max alignment
        max_alignment = max_alignment.max(field_layout.alignment);
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
