use crate::{Field, LocalNodeId, NodeTree, TensorDimension, TensorLayout, Type};

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
    fn new(size: u32, alignment: u32) -> Self {
        debug_assert!(alignment.is_power_of_two() || alignment == 0);
        Self { size, alignment }
    }

    /// Create a layout for a type with natural alignment (alignment == size).
    fn natural(size: u32) -> Self {
        Self {
            size,
            alignment: size,
        }
    }

    /// Compute the aligned offset for placing a value with this layout.
    /// Returns the next offset >= current_offset that satisfies alignment.
    pub(crate) fn align_offset(self, current_offset: u32) -> u32 {
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
    tree: &NodeTree,
    type_id: LocalNodeId<Type>,
    pointer_bytes: u8,
) -> TypeLayout {
    let mir_type = tree.get(type_id);
    match mir_type {
        Type::Void => TypeLayout::new(0, 1),
        Type::Boolean => TypeLayout::natural(1),

        Type::Int { width, .. } => {
            let bytes = (*width as u32).div_ceil(8);
            TypeLayout::natural(bytes)
        }
        Type::Isize | Type::Usize => TypeLayout::natural(pointer_bytes as u32),

        Type::Float { width } => {
            let bytes = (*width as u32).div_ceil(8);
            TypeLayout::natural(bytes)
        }

        Type::Type | Type::Reference { .. } | Type::FunctionPointer { .. } => {
            TypeLayout::natural(pointer_bytes as u32)
        }

        Type::Array {
            element,
            length,
            copyability: _,
        } => {
            let element_layout = compute_type_layout(tree, *element, pointer_bytes);
            let size = element_layout.size * (*length as u32);
            TypeLayout::new(size, element_layout.alignment)
        }

        Type::Tuple {
            elements,
            copyability: _,
        } => compute_tuple_layout(tree, elements, pointer_bytes),

        Type::Struct {
            fields,
            copyability: _,
        } => compute_struct_layout_from_fields(tree, fields, pointer_bytes),

        Type::Newtype { inner, .. } => compute_type_layout(tree, *inner, pointer_bytes),

        Type::Vector {
            element,
            lanes,
            copyability: _,
        } => {
            let element_layout = compute_type_layout(tree, *element, pointer_bytes);
            let size = element_layout.size * (*lanes);
            TypeLayout::new(size, element_layout.alignment)
        }

        Type::Tensor {
            element,
            shape,
            layout,
            copyability: _,
        } => {
            let element_layout = compute_type_layout(tree, *element, pointer_bytes);
            let element_count = compute_tensor_element_count(shape, layout);
            if let Some(element_count) = element_count {
                let size = element_layout.size * (element_count as u32);
                TypeLayout::new(size, element_layout.alignment)
            } else {
                TypeLayout::natural(pointer_bytes as u32)
            }
        }

        Type::TensorReference { .. } => TypeLayout::natural(pointer_bytes as u32),
    }
}

/// Compute the number of elements in a tensor.
fn compute_tensor_element_count(shape: &[TensorDimension], layout: &TensorLayout) -> Option<u64> {
    if shape.iter().any(|dim| dim.is_dynamic()) {
        return None;
    }
    match layout {
        TensorLayout::RowMajor | TensorLayout::ColumnMajor => {
            Some(shape.iter().filter_map(static_dim).product())
        }
        TensorLayout::Strided { strides } => {
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

/// Extract the static dimension size from a tensor dimension.
fn static_dim(dim: &TensorDimension) -> Option<u64> {
    match dim {
        TensorDimension::Static(value) => Some(*value),
        TensorDimension::Dynamic => None,
    }
}

/// Compute the layout of a tuple type.
fn compute_tuple_layout(
    tree: &NodeTree,
    elements: &[LocalNodeId<Type>],
    pointer_bytes: u8,
) -> TypeLayout {
    if elements.is_empty() {
        return TypeLayout::new(0, 1);
    }

    let mut offset = 0u32;
    let mut max_alignment = 1u32;

    for &element_type_id in elements {
        let element_layout = compute_type_layout(tree, element_type_id, pointer_bytes);
        offset = element_layout.align_offset(offset);
        offset += element_layout.size;
        max_alignment = max_alignment.max(element_layout.alignment);
    }

    // pad to alignment
    let final_size = TypeLayout::new(0, max_alignment).align_offset(offset);
    TypeLayout::new(final_size, max_alignment)
}

/// Compute the layout of a struct from its field definitions.
/// This uses the stored field offsets (for structs built programmatically).
fn compute_struct_layout_from_fields(
    tree: &NodeTree,
    fields: &[LocalNodeId<Field>],
    pointer_bytes: u8,
) -> TypeLayout {
    if fields.is_empty() {
        return TypeLayout::new(0, 1);
    }

    let mut max_end = 0u32;
    let mut max_alignment = 1u32;

    for &field_id in fields {
        let field = tree.get(field_id);
        let field_layout = compute_type_layout(tree, field.ty, pointer_bytes);
        let field_end = field.offset + field_layout.size;
        max_end = max_end.max(field_end);
        max_alignment = max_alignment.max(field_layout.alignment);
    }

    // pad to alignment
    let final_size = TypeLayout::new(0, max_alignment).align_offset(max_end);
    TypeLayout::new(final_size, max_alignment)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_layout_align_offset() {
        let layout = TypeLayout::natural(4);

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
