use std::collections::HashMap;

use destack_heap::ReferenceMap;
use destack_mir as mir;

/// One compiled storage field layout.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct StorageFieldLayout {
    /// The field value type.
    pub ty: mir::LocalNodeId<mir::Type>,
    /// The byte offset of the field inside the parent storage.
    pub offset: usize,
    /// The byte width of the field payload.
    pub byte_len: usize,
}

/// One compiled storage element layout.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct StorageElementLayout {
    /// The element value type.
    pub ty: mir::LocalNodeId<mir::Type>,
    /// The byte stride between adjacent elements.
    pub stride: usize,
    /// The byte width of one element payload.
    pub byte_len: usize,
}

/// One compiled composite storage component layout.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct StorageComponentLayout {
    /// The component value type.
    pub ty: mir::LocalNodeId<mir::Type>,
    /// The byte offset of the component payload.
    pub offset: usize,
    /// The byte width of the component payload.
    pub byte_len: usize,
}

/// The compiled storage shape for one MIR type.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum StorageShape {
    /// One scalar or pointer value with no structural decomposition.
    Scalar,
    /// One field-addressable composite with a fixed field list.
    Fields(Vec<StorageFieldLayout>),
    /// One element-addressable array with a fixed element stride.
    Array {
        /// The element storage layout.
        element: StorageElementLayout,
        /// The static element count.
        length: usize,
    },
    /// One vector with a fixed lane count and element width.
    Vector {
        /// The element storage layout.
        element: StorageElementLayout,
        /// The lane count.
        lanes: usize,
    },
    /// One tensor with a static flattened storage shape.
    Tensor {
        /// The element storage layout.
        element: StorageElementLayout,
        /// The flattened logical storage length.
        element_count: usize,
    },
}

/// One compiled storage layout for one MIR type.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct StorageLayout {
    /// The byte width of the storage representation.
    pub byte_len: usize,
    /// The structural storage shape.
    shape: StorageShape,
    /// The reference trace for this storage type.
    pub reference_map: ReferenceMap,
    /// The byte alignment of the storage representation.
    alignment: usize,
}

impl StorageLayout {
    /// Report whether this storage type is scalar.
    pub(crate) fn is_scalar(&self) -> bool {
        matches!(self.shape, StorageShape::Scalar)
    }

    /// Return one field layout by index.
    pub(crate) fn field(&self, index: u32) -> Option<StorageFieldLayout> {
        let StorageShape::Fields(fields) = &self.shape else {
            return None;
        };

        fields.get(index as usize).copied()
    }

    /// Return the element layout.
    pub(crate) fn element(&self) -> Option<StorageElementLayout> {
        match &self.shape {
            StorageShape::Array { element, .. }
            | StorageShape::Vector { element, .. }
            | StorageShape::Tensor { element, .. } => Some(*element),
            StorageShape::Scalar | StorageShape::Fields(_) => None,
        }
    }

    /// Return the semantic component count for this storage type.
    pub(crate) fn component_count(&self) -> Option<usize> {
        match &self.shape {
            StorageShape::Scalar => None,
            StorageShape::Fields(fields) => Some(fields.len()),
            StorageShape::Array { length, .. } => Some(*length),
            StorageShape::Vector { lanes, .. } => Some(*lanes),
            StorageShape::Tensor { element_count, .. } => Some(*element_count),
        }
    }

    /// Return one semantic component layout by index.
    pub(crate) fn component(&self, index: u32) -> Option<StorageComponentLayout> {
        let index = index as usize;

        match &self.shape {
            StorageShape::Scalar => None,
            StorageShape::Fields(fields) => fields.get(index).map(|field| StorageComponentLayout {
                ty: field.ty,
                offset: field.offset,
                byte_len: field.byte_len,
            }),
            StorageShape::Array { element, length } => {
                if index >= *length {
                    return None;
                }

                let offset = index.checked_mul(element.stride)?;
                Some(StorageComponentLayout {
                    ty: element.ty,
                    offset,
                    byte_len: element.byte_len,
                })
            }
            StorageShape::Vector { element, lanes } => {
                if index >= *lanes {
                    return None;
                }

                let offset = index.checked_mul(element.stride)?;
                Some(StorageComponentLayout {
                    ty: element.ty,
                    offset,
                    byte_len: element.byte_len,
                })
            }
            StorageShape::Tensor {
                element,
                element_count,
            } => {
                if index >= *element_count {
                    return None;
                }

                let offset = index.checked_mul(element.stride)?;
                Some(StorageComponentLayout {
                    ty: element.ty,
                    offset,
                    byte_len: element.byte_len,
                })
            }
        }
    }

    /// Return the aligned storage stride.
    pub(crate) fn stride(&self) -> usize {
        align_offset(self.byte_len, self.alignment)
    }
}

/// Return the transparent representation type for one semantic type.
pub(crate) fn repr_type(
    tree: &mir::NodeTree,
    mut ty: mir::LocalNodeId<mir::Type>,
) -> mir::LocalNodeId<mir::Type> {
    loop {
        let mir::Type::Newtype { inner, .. } = tree.get(ty) else {
            return ty;
        };

        ty = *inner;
    }
}

/// Build compiled storage layouts for all MIR types in the tree.
pub(crate) fn build_storage_layouts(
    tree: &mir::NodeTree,
) -> HashMap<mir::LocalNodeId<mir::Type>, StorageLayout> {
    let mut layouts = HashMap::new();

    for (type_id, _) in tree.iter_nodes::<mir::Type>() {
        let _ = build_storage_layout(tree, &mut layouts, type_id);
    }

    layouts
}

fn build_storage_layout(
    tree: &mir::NodeTree,
    layouts: &mut HashMap<mir::LocalNodeId<mir::Type>, StorageLayout>,
    ty: mir::LocalNodeId<mir::Type>,
) -> StorageLayout {
    if let Some(layout) = layouts.get(&ty) {
        return layout.clone();
    }

    let repr_ty = repr_type(tree, ty);
    if repr_ty != ty {
        let layout = build_storage_layout(tree, layouts, repr_ty);
        layouts.insert(ty, layout.clone());

        return layout;
    }

    let mut layout = match tree.get(ty) {
        mir::Type::Void
        | mir::Type::Boolean
        | mir::Type::Int { .. }
        | mir::Type::Isize
        | mir::Type::Usize
        | mir::Type::TypeDescriptor
        | mir::Type::TypeId
        | mir::Type::Reference { .. }
        | mir::Type::FunctionPointer { .. }
        | mir::Type::Float { .. }
        | mir::Type::TensorReference { .. } => raw_scalar_storage_layout(tree, ty),
        mir::Type::Newtype { .. } => unreachable!("repr_type must peel newtypes"),
        mir::Type::Struct { fields, .. } => {
            for field_id in fields {
                let field_type = tree.get(*field_id).ty;
                let _ = build_storage_layout(tree, layouts, field_type);
            }

            if fields.iter().any(|field_id| {
                let field_type = tree.get(*field_id).ty;
                contains_boxed_function_value(tree, field_type)
            }) {
                let field_types = fields.iter().map(|field_id| tree.get(*field_id).ty);
                let layout = build_vm_fields_layout(tree, layouts, ty, field_types);
                layouts.insert(ty, layout.clone());

                return layout;
            }

            let raw_layout = raw_layout_entry(tree, ty);
            let raw_fields = raw_fields_from_layout(raw_layout);

            StorageLayout {
                byte_len: raw_layout.size as usize,
                shape: StorageShape::Fields(raw_fields),
                reference_map: ReferenceMap::empty(),
                alignment: raw_layout.alignment as usize,
            }
        }
        mir::Type::Tuple { elements, .. } => {
            for element_type in elements {
                let _ = build_storage_layout(tree, layouts, *element_type);
            }

            if elements
                .iter()
                .any(|element_type| contains_boxed_function_value(tree, *element_type))
            {
                let element_types = elements.iter().copied();
                let layout = build_vm_fields_layout(tree, layouts, ty, element_types);
                layouts.insert(ty, layout.clone());

                return layout;
            }

            let raw_layout = raw_layout_entry(tree, ty);
            let raw_fields = raw_fields_from_layout(raw_layout);

            StorageLayout {
                byte_len: raw_layout.size as usize,
                shape: StorageShape::Fields(raw_fields),
                reference_map: ReferenceMap::empty(),
                alignment: raw_layout.alignment as usize,
            }
        }
        mir::Type::Array {
            element, length, ..
        } => {
            let element_layout = build_storage_layout(tree, layouts, *element);

            if contains_boxed_function_value(tree, *element) {
                let stride = element_layout.stride();
                let layout = StorageLayout {
                    byte_len: stride.saturating_mul(*length as usize),
                    shape: StorageShape::Array {
                        element: StorageElementLayout {
                            ty: *element,
                            stride,
                            byte_len: element_layout.byte_len,
                        },
                        length: *length as usize,
                    },
                    reference_map: ReferenceMap::empty(),
                    alignment: element_layout.alignment,
                };
                layouts.insert(ty, layout.clone());

                return layout;
            }

            let raw_layout = raw_layout_entry(tree, ty);
            let raw_stride = raw_array_stride(raw_layout);

            StorageLayout {
                byte_len: raw_layout.size as usize,
                shape: StorageShape::Array {
                    element: StorageElementLayout {
                        ty: *element,
                        stride: raw_stride,
                        byte_len: element_layout.byte_len,
                    },
                    length: *length as usize,
                },
                reference_map: ReferenceMap::empty(),
                alignment: raw_layout.alignment as usize,
            }
        }
        mir::Type::FunctionValue { .. } => {
            scalar_layout(tree.pointer_bytes() as usize, tree.pointer_bytes() as usize)
        }
        mir::Type::Vector { element, lanes, .. } => {
            let element_layout = build_storage_layout(tree, layouts, *element);

            if contains_boxed_function_value(tree, *element) {
                let stride = element_layout.stride();
                let layout = StorageLayout {
                    byte_len: stride.saturating_mul(*lanes as usize),
                    shape: StorageShape::Vector {
                        element: StorageElementLayout {
                            ty: *element,
                            stride,
                            byte_len: element_layout.byte_len,
                        },
                        lanes: *lanes as usize,
                    },
                    reference_map: ReferenceMap::empty(),
                    alignment: element_layout.alignment,
                };
                layouts.insert(ty, layout.clone());

                return layout;
            }

            let stride = element_layout.stride();
            let byte_len = tree
                .type_layout(ty)
                .map(|layout| layout.size as usize)
                .unwrap_or_else(|| stride.saturating_mul(*lanes as usize));
            let alignment = tree
                .type_layout(ty)
                .map(|layout| layout.alignment as usize)
                .unwrap_or(element_layout.alignment);

            StorageLayout {
                byte_len,
                shape: StorageShape::Vector {
                    element: StorageElementLayout {
                        ty: *element,
                        stride,
                        byte_len: element_layout.byte_len,
                    },
                    lanes: *lanes as usize,
                },
                reference_map: ReferenceMap::empty(),
                alignment,
            }
        }
        mir::Type::Tensor {
            element,
            shape,
            layout,
            ..
        } => {
            let element_layout = build_storage_layout(tree, layouts, *element);
            let element_count = compute_tensor_element_count(shape, layout);

            if contains_boxed_function_value(tree, *element) {
                let stride = element_layout.stride();
                let layout = StorageLayout {
                    byte_len: stride.saturating_mul(element_count),
                    shape: StorageShape::Tensor {
                        element: StorageElementLayout {
                            ty: *element,
                            stride,
                            byte_len: element_layout.byte_len,
                        },
                        element_count,
                    },
                    reference_map: ReferenceMap::empty(),
                    alignment: element_layout.alignment,
                };
                layouts.insert(ty, layout.clone());

                return layout;
            }

            let stride = element_layout.stride();
            let byte_len = tree
                .type_layout(ty)
                .map(|layout| layout.size as usize)
                .unwrap_or_else(|| stride.saturating_mul(element_count));
            let alignment = tree
                .type_layout(ty)
                .map(|layout| layout.alignment as usize)
                .unwrap_or(element_layout.alignment);

            StorageLayout {
                byte_len,
                shape: StorageShape::Tensor {
                    element: StorageElementLayout {
                        ty: *element,
                        stride,
                        byte_len: element_layout.byte_len,
                    },
                    element_count,
                },
                reference_map: ReferenceMap::empty(),
                alignment,
            }
        }
    };

    layouts.insert(ty, layout.clone());
    layout.reference_map = build_reference_map(tree, layouts, ty);
    layouts.insert(ty, layout.clone());
    layout
}

fn scalar_layout(byte_len: usize, alignment: usize) -> StorageLayout {
    StorageLayout {
        byte_len,
        shape: StorageShape::Scalar,
        reference_map: ReferenceMap::empty(),
        alignment,
    }
}

fn raw_scalar_storage_layout(
    tree: &mir::NodeTree,
    ty: mir::LocalNodeId<mir::Type>,
) -> StorageLayout {
    let (raw_byte_len, raw_alignment) = raw_scalar_size_alignment(tree, ty);
    scalar_layout(raw_byte_len, raw_alignment)
}

fn raw_scalar_size_alignment(
    tree: &mir::NodeTree,
    ty: mir::LocalNodeId<mir::Type>,
) -> (usize, usize) {
    match tree.get(ty) {
        mir::Type::Void => (0, 1),
        mir::Type::Boolean => (1, 1),
        mir::Type::Int { width, .. } => {
            let byte_len = (*width as usize).div_ceil(8);
            (byte_len, byte_len.clamp(1, 8))
        }
        mir::Type::Isize | mir::Type::Usize => {
            let byte_len = tree.pointer_bytes() as usize;
            (byte_len, byte_len.clamp(1, 8))
        }
        mir::Type::Float { width } => {
            let byte_len = (*width as usize).div_ceil(8);
            (byte_len, byte_len.clamp(1, 8))
        }
        mir::Type::TypeDescriptor
        | mir::Type::TypeId
        | mir::Type::Reference { .. }
        | mir::Type::FunctionValue { .. }
        | mir::Type::FunctionPointer { .. }
        | mir::Type::TensorReference { .. } => {
            let byte_len = tree.pointer_bytes() as usize;
            (byte_len, byte_len.max(1))
        }
        _ => unreachable!("raw scalar layout requested for non scalar type"),
    }
}

fn is_managed_reference_repr(tree: &mir::NodeTree, ty: mir::LocalNodeId<mir::Type>) -> bool {
    let ty = repr_type(tree, ty);

    matches!(
        tree.get(ty),
        mir::Type::Reference {
            kind: mir::ReferenceKind::Managed,
            ..
        } | mir::Type::FunctionValue { .. }
            | mir::Type::TensorReference {
                kind: mir::ReferenceKind::Managed,
                ..
            }
    )
}

fn raw_layout_entry(tree: &mir::NodeTree, ty: mir::LocalNodeId<mir::Type>) -> &mir::Layout {
    tree.type_layout(ty)
        .unwrap_or_else(|| panic!("missing MIR raw layout metadata for {ty:?}"))
}

fn raw_fields_from_layout(layout: &mir::Layout) -> Vec<StorageFieldLayout> {
    let mut fields: Vec<_> = layout
        .fields
        .iter()
        .enumerate()
        .map(|(index, field)| {
            (
                field.source_index.unwrap_or(index as u32) as usize,
                StorageFieldLayout {
                    ty: field.ty,
                    offset: field.offset as usize,
                    byte_len: field.size as usize,
                },
            )
        })
        .collect();
    fields.sort_by_key(|(index, _)| *index);

    fields.into_iter().map(|(_, field)| field).collect()
}

fn raw_array_stride(layout: &mir::Layout) -> usize {
    let mir::LayoutType::Array { element_stride, .. } = &layout.layout_type else {
        panic!("missing MIR array layout stride")
    };

    *element_stride as usize
}

fn contains_boxed_function_value(tree: &mir::NodeTree, ty: mir::LocalNodeId<mir::Type>) -> bool {
    let ty = repr_type(tree, ty);

    match tree.get(ty) {
        mir::Type::FunctionValue { .. } => true,
        mir::Type::Struct { fields, .. } => fields.iter().any(|field_id| {
            let field_type = tree.get(*field_id).ty;
            contains_boxed_function_value(tree, field_type)
        }),
        mir::Type::Tuple { elements, .. } => elements
            .iter()
            .any(|element_type| contains_boxed_function_value(tree, *element_type)),
        mir::Type::Array { element, .. }
        | mir::Type::Vector { element, .. }
        | mir::Type::Tensor { element, .. } => contains_boxed_function_value(tree, *element),
        _ => false,
    }
}

fn build_vm_fields_layout(
    tree: &mir::NodeTree,
    layouts: &mut HashMap<mir::LocalNodeId<mir::Type>, StorageLayout>,
    ty: mir::LocalNodeId<mir::Type>,
    field_types: impl IntoIterator<Item = mir::LocalNodeId<mir::Type>>,
) -> StorageLayout {
    let mut fields = Vec::new();
    let mut next_offset = 0usize;
    let mut alignment = 1usize;

    for field_type in field_types {
        let field_layout = build_storage_layout(tree, layouts, field_type);
        let field_offset = align_offset(next_offset, field_layout.alignment);

        fields.push(StorageFieldLayout {
            ty: field_type,
            offset: field_offset,
            byte_len: field_layout.byte_len,
        });

        next_offset = field_offset.saturating_add(field_layout.byte_len);
        alignment = alignment.max(field_layout.alignment);
    }

    let byte_len = align_offset(next_offset, alignment);
    let layout = StorageLayout {
        byte_len,
        shape: StorageShape::Fields(fields),
        reference_map: ReferenceMap::empty(),
        alignment,
    };

    debug_assert!(layout.byte_len <= raw_layout_entry(tree, ty).size as usize);

    layout
}

fn align_offset(offset: usize, alignment: usize) -> usize {
    if alignment <= 1 {
        return offset;
    }

    let misalignment = offset % alignment;

    if misalignment == 0 {
        offset
    } else {
        offset + (alignment - misalignment)
    }
}

fn build_reference_map(
    tree: &mir::NodeTree,
    layouts: &HashMap<mir::LocalNodeId<mir::Type>, StorageLayout>,
    ty: mir::LocalNodeId<mir::Type>,
) -> ReferenceMap {
    let mut offsets = Vec::new();
    append_reference_offsets(tree, layouts, ty, 0, &mut offsets);

    if offsets.is_empty() {
        ReferenceMap::empty()
    } else {
        ReferenceMap::ReferenceOffsets { offsets }
    }
}

fn append_reference_offsets(
    tree: &mir::NodeTree,
    layouts: &HashMap<mir::LocalNodeId<mir::Type>, StorageLayout>,
    ty: mir::LocalNodeId<mir::Type>,
    base_offset: u32,
    offsets: &mut Vec<u32>,
) {
    let Some(layout) = layouts.get(&ty) else {
        return;
    };

    match &layout.shape {
        StorageShape::Scalar => {
            if is_managed_reference_repr(tree, ty) {
                offsets.push(base_offset);
            }
        }
        StorageShape::Fields(fields) => {
            for field in fields {
                let field_base = base_offset.saturating_add(field.offset as u32);
                append_reference_offsets(tree, layouts, field.ty, field_base, offsets);
            }
        }
        StorageShape::Array { element, length }
        | StorageShape::Vector {
            element,
            lanes: length,
        }
        | StorageShape::Tensor {
            element,
            element_count: length,
        } => {
            for index in 0..*length {
                let element_base =
                    base_offset.saturating_add((index.saturating_mul(element.stride)) as u32);
                append_reference_offsets(tree, layouts, element.ty, element_base, offsets);
            }
        }
    }
}

fn compute_tensor_element_count(
    shape: &[mir::TensorDimension],
    layout: &mir::TensorLayout,
) -> usize {
    let shape: Vec<u64> = shape
        .iter()
        .map(|dimension| match dimension {
            mir::TensorDimension::Static(value) => *value,
            mir::TensorDimension::Dynamic => 0,
        })
        .collect();

    match layout {
        mir::TensorLayout::RowMajor | mir::TensorLayout::ColumnMajor => shape
            .iter()
            .copied()
            .product::<u64>()
            .min(usize::MAX as u64)
            as usize,
        mir::TensorLayout::Strided { strides } => {
            let strides: Vec<u64> = strides
                .iter()
                .map(|dimension| match dimension {
                    mir::TensorDimension::Static(value) => *value,
                    mir::TensorDimension::Dynamic => 0,
                })
                .collect();
            let mut max_index = 0u64;

            // compute the highest reachable logical element index
            for (dimension, stride) in shape.iter().copied().zip(strides.iter().copied()) {
                if dimension == 0 {
                    continue;
                }

                max_index = max_index.saturating_add((dimension - 1).saturating_mul(stride));
            }

            max_index.saturating_add(1).min(usize::MAX as u64) as usize
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use destack_core::ImmutableStringPool;
    use destack_mir::parse::{ParseOptions, Parser};
    use destack_mir::{DataLayout, NodeTree, Type, TypeAlias};
    use destack_source::FileId;

    /// Parse one MIR module with the given data layout.
    fn parse_tree_with_layout(
        mir_text: &str,
        data_layout: DataLayout,
    ) -> (NodeTree, ImmutableStringPool) {
        let (mut tree, strings) = Parser::parse(
            FileId::new(0),
            mir_text,
            ParseOptions {
                pointer_bytes: data_layout.native_pointer_bytes,
            },
        )
        .expect("failed to parse MIR");
        tree.data_layout = data_layout;

        (tree, strings)
    }

    /// Look up one aliased type by its `@name`.
    fn lookup_type_alias(
        tree: &NodeTree,
        strings: &ImmutableStringPool,
        name: &str,
    ) -> mir::LocalNodeId<Type> {
        for (_, type_alias) in tree.iter_nodes::<TypeAlias>() {
            if strings.get(type_alias.name) == name {
                return type_alias.ty;
            }
        }

        panic!("missing type alias @{name}");
    }

    /// Compiled raw struct layout matches canonical MIR raw layout metadata.
    #[test]
    fn test_build_storage_layout_imports_raw_struct_layout() {
        let mir_text = r#"
type @Mixed = { a: u8, b: i64, c: u8 }
"#;
        let (tree, strings) = parse_tree_with_layout(mir_text, DataLayout::default());
        let ty = lookup_type_alias(&tree, &strings, "Mixed");
        let layouts = build_storage_layouts(&tree);
        let layout = layouts.get(&ty).expect("missing storage layout");
        let raw_layout = tree.type_layout(ty).expect("missing MIR raw layout");

        // top-level facts
        assert_eq!(layout.byte_len, raw_layout.size as usize);

        // field facts
        let raw_fields = raw_fields_from_layout(raw_layout);
        assert_eq!(layout.field(0), raw_fields.first().copied());
        assert_eq!(layout.field(1), raw_fields.get(1).copied());
        assert_eq!(layout.field(2), raw_fields.get(2).copied());
    }

    /// Managed struct layout matches the canonical pointer-shaped runtime layout.
    #[test]
    fn test_build_storage_layout_uses_canonical_struct_reference_offsets() {
        let mir_text = r#"
type @Packed = { a: u8, b: ref<managed readonly i32>, c: u8 }
"#;
        let (tree, strings) = parse_tree_with_layout(mir_text, DataLayout::default());
        let ty = lookup_type_alias(&tree, &strings, "Packed");
        let layouts = build_storage_layouts(&tree);
        let layout = layouts.get(&ty).expect("missing storage layout");

        // struct fields follow the canonical runtime layout
        assert_eq!(layout.byte_len, 24);
        assert_eq!(layout.field(0).expect("missing field 0").offset, 0);
        assert_eq!(layout.field(1).expect("missing field 1").offset, 8);
        assert_eq!(layout.field(2).expect("missing field 2").offset, 16);

        // reference tracing should point at the managed reference field
        assert_eq!(
            layout.reference_map,
            ReferenceMap::ReferenceOffsets { offsets: vec![8] }
        );
    }

    /// Managed vector layout uses the same physical stride as raw layout.
    #[test]
    fn test_build_storage_layout_uses_canonical_vector_stride() {
        let mir_text = r#"
type @Vec = vector<ref<managed readonly i32>, 2>
"#;
        let (tree, strings) = parse_tree_with_layout(mir_text, DataLayout::default());
        let ty = lookup_type_alias(&tree, &strings, "Vec");
        let layouts = build_storage_layouts(&tree);
        let layout = layouts.get(&ty).expect("missing storage layout");
        let element = layout.element().expect("missing element layout");

        // vector stride follows the canonical physical layout
        assert_eq!(element.byte_len, 8);
        assert_eq!(element.stride, 8);
        assert_eq!(layout.byte_len, 16);

        // component offsets should reflect the canonical lane stride
        assert_eq!(layout.component(0).expect("missing lane 0").offset, 0);
        assert_eq!(layout.component(1).expect("missing lane 1").offset, 8);

        // reference tracing should include both lanes
        assert_eq!(
            layout.reference_map,
            ReferenceMap::ReferenceOffsets {
                offsets: vec![0, 8],
            }
        );
    }

    /// Transparent newtype wrappers preserve managed reference tracing.
    #[test]
    fn test_build_storage_layout_traces_newtype_wrapped_managed_reference() {
        let mir_text = r#"
type @Handle = newtype<ref<managed readonly i32>>
type @Holder = { value: @Handle }
"#;
        let (tree, strings) = parse_tree_with_layout(mir_text, DataLayout::default());
        let ty = lookup_type_alias(&tree, &strings, "Holder");
        let layouts = build_storage_layouts(&tree);
        let layout = layouts.get(&ty).expect("missing storage layout");

        // newtype-wrapped managed refs should still appear in the trace map
        assert_eq!(
            layout.reference_map,
            ReferenceMap::ReferenceOffsets { offsets: vec![0] }
        );
    }

    /// Function values stay boxed in VM storage.
    #[test]
    fn test_build_storage_layout_boxes_function_value() {
        let mir_text = r#"
type @Closure = fnvalue<fn() -> i32>
"#;
        let (tree, strings) = parse_tree_with_layout(mir_text, DataLayout::default());
        let ty = lookup_type_alias(&tree, &strings, "Closure");
        let layouts = build_storage_layouts(&tree);
        let layout = layouts.get(&ty).expect("missing storage layout");

        // fnvalue fields store one managed reference to one boxed callable object
        assert!(layout.is_scalar());
        assert_eq!(layout.byte_len, tree.pointer_bytes() as usize);
        assert_eq!(
            layout.reference_map,
            ReferenceMap::ReferenceOffsets { offsets: vec![0] }
        );
    }
}
