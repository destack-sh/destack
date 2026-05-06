use crate::{
    Field, LocalNodeId, TensorDimension, TensorLayout, Tree, Type, TypeReference,
    slice_header_types,
};

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

    /// Create a layout for scalar ABI alignment.
    fn scalar(size: u32) -> Self {
        Self {
            size,
            alignment: size.min(8),
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
    tree: &Tree,
    type_id: LocalNodeId<Type>,
    pointer_bytes: u8,
) -> TypeLayout {
    let mir_type = tree.get(type_id);
    match mir_type {
        Type::Void => TypeLayout::new(0, 1),
        Type::Boolean => TypeLayout::scalar(1),

        Type::Int { width, .. } => {
            let bytes = (*width as u32).div_ceil(8);
            TypeLayout::scalar(bytes)
        }
        Type::Isize | Type::Usize => TypeLayout::scalar(pointer_bytes as u32),

        Type::Float { width } => {
            let bytes = (*width as u32).div_ceil(8);
            TypeLayout::scalar(bytes)
        }

        Type::TypeDescriptor
        | Type::TypeId
        | Type::Reference { .. }
        | Type::FunctionPointer { .. } => TypeLayout::natural(pointer_bytes as u32),

        Type::Atomic { value } => compute_type_layout(
            tree,
            require_type_reference(*value, "atomic value"),
            pointer_bytes,
        ),

        Type::FunctionSignature { .. } => TypeLayout::new(0, 1),

        Type::Callable { signature } => {
            let environment = tree.callable_environment_type();
            let signature = require_type_reference(*signature, "callable signature");
            compute_tuple_layout(tree, &[signature, environment], pointer_bytes)
        }

        Type::Array {
            element,
            length,
            copy: _,
        } => {
            let element_layout = compute_type_layout(
                tree,
                require_type_reference(*element, "array element"),
                pointer_bytes,
            );
            let size = element_layout.size * (*length as u32);
            TypeLayout::new(size, element_layout.alignment)
        }
        Type::Slice {
            kind,
            element,
            address_space,
            mutability,
        } => {
            let (data, length) =
                slice_header_types(*kind, *element, *mutability, address_space.clone());
            compute_type_pair_layout(tree, [&data, &length], pointer_bytes)
        }

        Type::Tuple { elements, copy: _ } => {
            compute_tuple_layout_from_references(tree, elements, pointer_bytes)
        }

        Type::Struct { fields, copy: _ } => {
            compute_struct_layout_from_fields(tree, fields, pointer_bytes)
        }

        Type::Newtype { inner, .. } => compute_type_layout(
            tree,
            require_type_reference(*inner, "newtype inner"),
            pointer_bytes,
        ),

        Type::Vector {
            element,
            lanes,
            copy: _,
        } => {
            let element_layout = compute_type_layout(
                tree,
                require_type_reference(*element, "vector element"),
                pointer_bytes,
            );
            let size = element_layout.size * (*lanes);
            TypeLayout::new(size, element_layout.alignment)
        }

        Type::Tensor {
            element,
            shape,
            layout,
            copy: _,
        } => {
            let element_layout = compute_type_layout(
                tree,
                require_type_reference(*element, "tensor element"),
                pointer_bytes,
            );
            let element_count = compute_tensor_element_count(shape, layout);
            let size = element_layout.size * element_count;
            TypeLayout::new(size, element_layout.alignment)
        }

        Type::TensorView { .. } => TypeLayout::natural(pointer_bytes as u32),
    }
}

/// Compute the number of elements in a tensor.
fn compute_tensor_element_count(shape: &[TensorDimension], layout: &TensorLayout) -> u32 {
    let shape: Vec<u64> = shape
        .iter()
        .map(|dim| match dim {
            TensorDimension::Static(value) => *value,
            TensorDimension::Dynamic => 0,
        })
        .collect();

    match layout {
        TensorLayout::RowMajor | TensorLayout::ColumnMajor => shape
            .iter()
            .copied()
            .product::<u64>()
            .min(u64::from(u32::MAX))
            as u32,
        TensorLayout::Strided { strides } => {
            let strides: Vec<u64> = strides
                .iter()
                .map(|dim| match dim {
                    TensorDimension::Static(value) => *value,
                    TensorDimension::Dynamic => 0,
                })
                .collect();

            let mut max_index = 0u64;
            for (dim, stride) in shape.iter().copied().zip(strides.iter().copied()) {
                if dim == 0 {
                    continue;
                }
                let last_index = dim - 1;
                let offset = last_index.saturating_mul(stride);
                max_index = max_index.max(offset);
            }
            max_index.saturating_add(1).min(u64::from(u32::MAX)) as u32
        }
    }
}

/// Compute the layout of a tuple type.
fn compute_tuple_layout(
    tree: &Tree,
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

fn compute_tuple_layout_from_references(
    tree: &Tree,
    elements: &[TypeReference],
    pointer_bytes: u8,
) -> TypeLayout {
    let elements = elements
        .iter()
        .copied()
        .map(|element| require_type_reference(element, "tuple element"))
        .collect::<Vec<_>>();

    compute_tuple_layout(tree, &elements, pointer_bytes)
}

fn compute_type_pair_layout(tree: &Tree, elements: [&Type; 2], pointer_bytes: u8) -> TypeLayout {
    let mut offset = 0u32;
    let mut max_alignment = 1u32;

    for element in elements {
        let element_layout = compute_inline_type_layout(tree, element, pointer_bytes);
        offset = element_layout.align_offset(offset);
        offset += element_layout.size;
        max_alignment = max_alignment.max(element_layout.alignment);
    }

    let final_size = TypeLayout::new(0, max_alignment).align_offset(offset);
    TypeLayout::new(final_size, max_alignment)
}

fn compute_inline_type_layout(_tree: &Tree, ty: &Type, pointer_bytes: u8) -> TypeLayout {
    match ty {
        Type::Reference { .. } | Type::FunctionPointer { .. } => {
            TypeLayout::natural(pointer_bytes as u32)
        }
        Type::Usize => TypeLayout::scalar(pointer_bytes as u32),
        Type::Int {
            width: 32,
            is_signed: false,
        } => TypeLayout::scalar(4),
        _ => panic!("unsupported inline slice header type: {ty:?}"),
    }
}

/// Compute the layout of a struct from its field definitions.
/// This computes offsets from field order and field types.
fn compute_struct_layout_from_fields(
    tree: &Tree,
    fields: &[LocalNodeId<Field>],
    pointer_bytes: u8,
) -> TypeLayout {
    if fields.is_empty() {
        return TypeLayout::new(0, 1);
    }

    let mut offset = 0u32;
    let mut max_alignment = 1u32;

    for &field_id in fields {
        let field = tree.get(field_id);
        let field_layout = compute_type_layout(
            tree,
            require_type_reference(field.ty, "field type"),
            pointer_bytes,
        );
        offset = field_layout.align_offset(offset);
        offset += field_layout.size;
        max_alignment = max_alignment.max(field_layout.alignment);
    }

    // pad to alignment
    let final_size = TypeLayout::new(0, max_alignment).align_offset(offset);
    TypeLayout::new(final_size, max_alignment)
}

fn require_type_reference(reference: TypeReference, context: &str) -> LocalNodeId<Type> {
    match reference {
        TypeReference::Type(ty) => ty,
        TypeReference::Missing => {
            panic!("missing type reference while computing layout for {context}")
        }
        TypeReference::Error => {
            panic!("malformed type reference while computing layout for {context}")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::TypeLayout;

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
