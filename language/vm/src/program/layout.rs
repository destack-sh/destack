use std::collections::HashMap;

use destack_mir as mir;
use destack_mir::{LayoutKind, ReferenceMap};

use crate::{Error, Result};

const SLICE_DATA_FIELD: u32 = 0;
const SLICE_LENGTH_FIELD: u32 = 1;

/// One compiled field layout.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FieldLayout {
    /// The field value type.
    pub ty: mir::LocalNodeId<mir::Type>,
    /// The byte offset of the field inside the parent value.
    pub offset: usize,
    /// The byte width of the field payload.
    pub byte_len: usize,
}

/// One compiled element layout.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ElementLayout {
    /// The element value type.
    pub ty: mir::LocalNodeId<mir::Type>,
    /// The byte stride between adjacent elements.
    pub stride: usize,
    /// The byte width of one element payload.
    pub byte_len: usize,
}

/// One compiled slice layout.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SliceLayout {
    /// The data pointer field.
    pub data: FieldLayout,
    /// The length field.
    pub length: FieldLayout,
}

/// The heap object layout for one callable value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct CallableObjectLayout {
    /// The function pointer offset.
    pub function_offset: usize,
    /// The environment pointer offset.
    pub environment_offset: usize,
    /// The callable object byte length.
    pub byte_len: usize,
    /// The callable object byte alignment.
    pub alignment: usize,
}

/// The compiled shape for one MIR type.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum LayoutShape {
    /// One scalar or pointer value with no structural decomposition.
    Scalar,
    /// One field-addressable payload with a fixed field list.
    Fields(Vec<FieldLayout>),
    /// One element-addressable array with a fixed element stride.
    Array {
        /// The element layout.
        element: ElementLayout,
        /// The static element count.
        length: usize,
    },
    /// One vector with a fixed lane count and element width.
    Vector {
        /// The element layout.
        element: ElementLayout,
        /// The lane count.
        lanes: usize,
    },
    /// One tensor with a static flattened shape.
    Tensor {
        /// The element layout.
        element: ElementLayout,
        /// The flattened logical length.
        element_count: usize,
    },
}

/// One compiled layout for one MIR type.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Layout {
    /// The byte width of the value representation.
    pub byte_len: usize,
    /// The structural layout shape.
    shape: LayoutShape,
    /// The reference map for this type.
    pub reference_map: ReferenceMap,
    /// The byte alignment of the value representation.
    alignment: usize,
}

impl Layout {
    /// Report whether this type is scalar.
    pub(crate) fn is_scalar(&self) -> bool {
        matches!(self.shape, LayoutShape::Scalar)
    }

    /// Report whether this type fits in one VM word.
    pub(crate) fn is_word(&self) -> bool {
        self.is_scalar() && self.byte_len <= crate::Word::BYTE_LEN
    }

    /// Return the byte alignment of this layout.
    pub(crate) fn alignment(&self) -> usize {
        self.alignment
    }

    /// Return one field layout by index.
    pub(crate) fn field(&self, index: u32) -> Option<FieldLayout> {
        let LayoutShape::Fields(fields) = &self.shape else {
            return None;
        };

        fields.get(index as usize).copied()
    }

    /// Return the field count for one field-addressable layout.
    pub(crate) fn field_count(&self) -> Option<usize> {
        let LayoutShape::Fields(fields) = &self.shape else {
            return None;
        };

        Some(fields.len())
    }

    /// Return the slice fields.
    pub(crate) fn slice(&self) -> Option<SliceLayout> {
        Some(SliceLayout {
            data: self.field(SLICE_DATA_FIELD)?,
            length: self.field(SLICE_LENGTH_FIELD)?,
        })
    }

    /// Return the element layout.
    pub(crate) fn element(&self) -> Option<ElementLayout> {
        match &self.shape {
            LayoutShape::Array { element, .. }
            | LayoutShape::Vector { element, .. }
            | LayoutShape::Tensor { element, .. } => Some(*element),
            LayoutShape::Scalar | LayoutShape::Fields(_) => None,
        }
    }

    /// Return the element count for one indexed layout.
    pub(crate) fn element_count(&self) -> Option<usize> {
        match &self.shape {
            LayoutShape::Array { length, .. } => Some(*length),
            LayoutShape::Vector { lanes, .. } => Some(*lanes),
            LayoutShape::Tensor { element_count, .. } => Some(*element_count),
            LayoutShape::Scalar | LayoutShape::Fields(_) => None,
        }
    }

    /// Return the aligned stride.
    pub(crate) fn stride(&self) -> usize {
        align_offset(self.byte_len, self.alignment)
    }
}

impl CallableObjectLayout {
    /// Return the heap layout table entry for callable objects.
    pub(crate) fn table_layout(self) -> mir::Layout {
        mir::Layout {
            kind: LayoutKind::Callable,
            size: self.byte_len as u32,
            alignment: self.alignment as u32,
            reference_map: ReferenceMap::Direct {
                local_offsets: vec![self.environment_offset as u32].into_boxed_slice(),
                shared_offsets: Vec::new().into_boxed_slice(),
            },
            fields: Vec::new(),
        }
    }
}

/// Return the callable object layout for one target pointer width.
pub(crate) fn callable_object_layout(pointer_bytes: usize) -> CallableObjectLayout {
    let function_offset = 0usize;
    let environment_offset = align_offset(pointer_bytes, pointer_bytes);
    let byte_len = environment_offset + pointer_bytes;

    CallableObjectLayout {
        function_offset,
        environment_offset,
        byte_len,
        alignment: pointer_bytes,
    }
}

/// Return the transparent representation type for one semantic type.
pub(crate) fn repr_type(
    tree: &mir::Tree,
    mut ty: mir::LocalNodeId<mir::Type>,
) -> mir::LocalNodeId<mir::Type> {
    loop {
        let mir::Type::Newtype { inner, .. } = tree.get(ty) else {
            return ty;
        };

        let Some(inner) = inner.ty() else {
            return ty;
        };
        ty = inner;
    }
}

/// Return the concrete representation type for one semantic type.
fn concrete_repr_type(
    tree: &mir::Tree,
    mut ty: mir::LocalNodeId<mir::Type>,
) -> Result<mir::LocalNodeId<mir::Type>> {
    loop {
        let mir::Type::Newtype { inner, .. } = tree.get(ty) else {
            return Ok(ty);
        };

        ty = (*inner).ty().ok_or_else(|| Error::MissingRepresentation {
            context: "repr newtype inner".to_string(),
        })?;
    }
}

/// Build compiled layouts for all MIR types in the tree.
pub(crate) fn build_layouts(
    tree: &mir::Tree,
) -> Result<HashMap<mir::LocalNodeId<mir::Type>, Layout>> {
    let mut layouts = HashMap::new();

    // build one layout entry for every MIR type
    for (type_id, _) in tree.iter_nodes::<mir::Type>() {
        build_layout(tree, &mut layouts, type_id)?;
    }

    Ok(layouts)
}

/// Build one compiled layout for one MIR type.
fn build_layout(
    tree: &mir::Tree,
    layouts: &mut HashMap<mir::LocalNodeId<mir::Type>, Layout>,
    ty: mir::LocalNodeId<mir::Type>,
) -> Result<Layout> {
    // reuse already-built layouts first
    if let Some(layout) = layouts.get(&ty) {
        return Ok(layout.clone());
    }

    // peel transparent wrappers before choosing the physical representation
    let repr_ty = concrete_repr_type(tree, ty)?;
    if repr_ty != ty {
        let layout = build_layout(tree, layouts, repr_ty)?;
        layouts.insert(ty, layout.clone());

        return Ok(layout);
    }

    // build the canonical physical representation for the repr type
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
        | mir::Type::TensorView { .. } => raw_scalar_layout(tree, ty),
        mir::Type::FunctionSignature { .. } => scalar_layout(0, 1),
        mir::Type::Newtype { .. } => unreachable!("repr_type must peel newtypes"),
        mir::Type::Struct { fields, .. } => {
            let field_types = fields
                .iter()
                .map(|field_id| {
                    let field = tree.get(*field_id);
                    (field.ty).ty().ok_or_else(|| Error::MissingRepresentation {
                        context: "struct field type".to_string(),
                    })
                })
                .collect::<Result<Vec<_>>>()?;
            build_record_layout(tree, layouts, ty, field_types)?
        }
        mir::Type::Tuple { elements, .. } => {
            let element_types = elements
                .iter()
                .map(|element| {
                    (*element).ty().ok_or_else(|| Error::MissingRepresentation {
                        context: "tuple element type".to_string(),
                    })
                })
                .collect::<Result<Vec<_>>>()?;
            build_record_layout(tree, layouts, ty, element_types)?
        }
        mir::Type::Array {
            element, length, ..
        } => build_array_layout(
            tree,
            layouts,
            ty,
            (*element)
                .ty()
                .ok_or_else(|| Error::MissingRepresentation {
                    context: "array element type".to_string(),
                })?,
            *length as usize,
        )?,
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
                .ok_or_else(|| Error::MissingRepresentation {
                    context: "slice data type".to_string(),
                })?;
            let length = tree.usize_type();

            build_record_layout(tree, layouts, ty, [data, length])?
        }
        mir::Type::Callable { .. } => {
            scalar_layout(tree.pointer_bytes() as usize, tree.pointer_bytes() as usize)
        }
        mir::Type::Vector { element, lanes, .. } => build_vector_layout(
            tree,
            layouts,
            ty,
            (*element)
                .ty()
                .ok_or_else(|| Error::MissingRepresentation {
                    context: "vector element type".to_string(),
                })?,
            *lanes as usize,
        )?,
        mir::Type::Tensor {
            element,
            shape,
            layout,
            ..
        } => build_tensor_layout(
            tree,
            layouts,
            ty,
            (*element)
                .ty()
                .ok_or_else(|| Error::MissingRepresentation {
                    context: "tensor element type".to_string(),
                })?,
            shape,
            layout,
        )?,
    };

    // publish one placeholder first so recursive tracing can see the shape graph
    layouts.insert(ty, layout.clone());

    // fill in the reference map after all child layouts exist
    layout.reference_map = build_reference_map(tree, layouts, ty)?;
    layouts.insert(ty, layout.clone());

    Ok(layout)
}

/// Build one scalar layout.
fn scalar_layout(byte_len: usize, alignment: usize) -> Layout {
    Layout {
        byte_len,
        shape: LayoutShape::Scalar,
        reference_map: ReferenceMap::empty(),
        alignment,
    }
}

/// Build one raw scalar layout.
fn raw_scalar_layout(tree: &mir::Tree, ty: mir::LocalNodeId<mir::Type>) -> Layout {
    let (raw_byte_len, raw_alignment) = raw_scalar_size_alignment(tree, ty);
    scalar_layout(raw_byte_len, raw_alignment)
}

/// Return the raw scalar size and alignment for one MIR type.
fn raw_scalar_size_alignment(tree: &mir::Tree, ty: mir::LocalNodeId<mir::Type>) -> (usize, usize) {
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
        | mir::Type::Callable { .. }
        | mir::Type::FunctionPointer { .. }
        | mir::Type::TensorView { .. } => {
            let byte_len = tree.pointer_bytes() as usize;

            (byte_len, byte_len.max(1))
        }
        _ => unreachable!("raw scalar layout requested for non scalar type"),
    }
}

/// Build one record layout from one ordered field type list.
fn build_record_layout(
    tree: &mir::Tree,
    layouts: &mut HashMap<mir::LocalNodeId<mir::Type>, Layout>,
    ty: mir::LocalNodeId<mir::Type>,
    field_types: impl IntoIterator<Item = mir::LocalNodeId<mir::Type>> + Clone,
) -> Result<Layout> {
    // ensure all child layouts exist before choosing the representation
    for field_type in field_types.clone() {
        build_layout(tree, layouts, field_type)?;
    }

    // callable fields store heap handles in VM frames
    for field_type in field_types.clone() {
        if contains_callable(tree, field_type)? {
            return build_runtime_fields_layout(tree, layouts, ty, field_types);
        }
    }

    // otherwise mirror the canonical MIR record layout directly
    let raw_layout = raw_layout_entry(tree, ty)?;
    let fields = raw_fields_from_layout(raw_layout);

    Ok(Layout {
        byte_len: raw_layout.size as usize,
        shape: LayoutShape::Fields(fields),
        reference_map: ReferenceMap::empty(),
        alignment: raw_layout.alignment as usize,
    })
}

/// Build one array layout.
fn build_array_layout(
    tree: &mir::Tree,
    layouts: &mut HashMap<mir::LocalNodeId<mir::Type>, Layout>,
    ty: mir::LocalNodeId<mir::Type>,
    element_type: mir::LocalNodeId<mir::Type>,
    length: usize,
) -> Result<Layout> {
    let element_layout = build_layout(tree, layouts, element_type)?;

    // callable elements store heap handles in VM frames
    if contains_callable(tree, element_type)? {
        return Ok(repeated_layout(
            element_type,
            &element_layout,
            element_layout.stride(),
            length,
            stride_byte_len(length, element_layout.stride())?,
            element_layout.alignment,
            |element| LayoutShape::Array { element, length },
        ));
    }

    // otherwise mirror the canonical MIR array stride and size
    let raw_layout = raw_layout_entry(tree, ty)?;
    let stride = raw_array_stride(raw_layout)?;

    Ok(repeated_layout(
        element_type,
        &element_layout,
        stride,
        length,
        raw_layout.size as usize,
        raw_layout.alignment as usize,
        |element| LayoutShape::Array { element, length },
    ))
}

/// Build one vector layout.
fn build_vector_layout(
    tree: &mir::Tree,
    layouts: &mut HashMap<mir::LocalNodeId<mir::Type>, Layout>,
    ty: mir::LocalNodeId<mir::Type>,
    element_type: mir::LocalNodeId<mir::Type>,
    lanes: usize,
) -> Result<Layout> {
    let element_layout = build_layout(tree, layouts, element_type)?;
    let stride = element_layout.stride();

    // callable elements store heap handles in VM frames
    if contains_callable(tree, element_type)? {
        return Ok(repeated_layout(
            element_type,
            &element_layout,
            stride,
            lanes,
            stride_byte_len(lanes, stride)?,
            element_layout.alignment,
            |element| LayoutShape::Vector { element, lanes },
        ));
    }

    // otherwise prefer canonical MIR vector size and alignment when available
    let byte_len = match tree.type_layout(ty) {
        Some(layout) => layout.size as usize,
        None => stride_byte_len(lanes, stride)?,
    };
    let alignment = tree
        .type_layout(ty)
        .map(|layout| layout.alignment as usize)
        .unwrap_or(element_layout.alignment);

    Ok(repeated_layout(
        element_type,
        &element_layout,
        stride,
        lanes,
        byte_len,
        alignment,
        |element| LayoutShape::Vector { element, lanes },
    ))
}

/// Build one tensor layout.
fn build_tensor_layout(
    tree: &mir::Tree,
    layouts: &mut HashMap<mir::LocalNodeId<mir::Type>, Layout>,
    ty: mir::LocalNodeId<mir::Type>,
    element_type: mir::LocalNodeId<mir::Type>,
    shape: &[mir::TensorDimension],
    tensor_layout: &mir::TensorLayout,
) -> Result<Layout> {
    let element_layout = build_layout(tree, layouts, element_type)?;
    let element_count = compute_tensor_element_count(shape, tensor_layout)?;
    let stride = element_layout.stride();

    // callable elements store heap handles in VM frames
    if contains_callable(tree, element_type)? {
        return Ok(repeated_layout(
            element_type,
            &element_layout,
            stride,
            element_count,
            stride_byte_len(element_count, stride)?,
            element_layout.alignment,
            |element| LayoutShape::Tensor {
                element,
                element_count,
            },
        ));
    }

    // otherwise prefer canonical MIR tensor size and alignment when available
    let byte_len = match tree.type_layout(ty) {
        Some(layout) => layout.size as usize,
        None => stride_byte_len(element_count, stride)?,
    };
    let alignment = tree
        .type_layout(ty)
        .map(|layout| layout.alignment as usize)
        .unwrap_or(element_layout.alignment);

    Ok(repeated_layout(
        element_type,
        &element_layout,
        stride,
        element_count,
        byte_len,
        alignment,
        |element| LayoutShape::Tensor {
            element,
            element_count,
        },
    ))
}

/// Build one repeated element layout.
fn repeated_layout(
    element_type: mir::LocalNodeId<mir::Type>,
    element_layout: &Layout,
    stride: usize,
    element_count: usize,
    byte_len: usize,
    alignment: usize,
    shape: impl FnOnce(ElementLayout) -> LayoutShape,
) -> Layout {
    let element = ElementLayout {
        ty: element_type,
        stride,
        byte_len: element_layout.byte_len,
    };

    debug_assert!(element_count == 0 || stride >= element_layout.byte_len);

    Layout {
        byte_len,
        shape: shape(element),
        reference_map: ReferenceMap::empty(),
        alignment,
    }
}

/// Return one repeated payload byte length.
fn stride_byte_len(element_count: usize, stride: usize) -> Result<usize> {
    stride
        .checked_mul(element_count)
        .ok_or_else(|| Error::InvariantViolation {
            context: format!(
                "repeated layout byte length overflow: stride={stride}, element_count={element_count}",
            ),
        })
}

/// Report whether the repr type contains one callable value.
fn contains_callable(tree: &mir::Tree, ty: mir::LocalNodeId<mir::Type>) -> Result<bool> {
    let ty = concrete_repr_type(tree, ty)?;

    match tree.get(ty) {
        mir::Type::Callable { .. } => Ok(true),
        mir::Type::Struct { fields, .. } => {
            for field_id in fields {
                let field_type = tree.get(*field_id).ty;
                let field_type = (field_type)
                    .ty()
                    .ok_or_else(|| Error::MissingRepresentation {
                        context: "struct field type".to_string(),
                    })?;
                if contains_callable(tree, field_type)? {
                    return Ok(true);
                }
            }

            Ok(false)
        }
        mir::Type::Tuple { elements, .. } => {
            for element_type in elements {
                let element_type =
                    (*element_type)
                        .ty()
                        .ok_or_else(|| Error::MissingRepresentation {
                            context: "tuple element type".to_string(),
                        })?;
                if contains_callable(tree, element_type)? {
                    return Ok(true);
                }
            }

            Ok(false)
        }
        mir::Type::Array { element, .. }
        | mir::Type::Vector { element, .. }
        | mir::Type::Tensor { element, .. } => contains_callable(
            tree,
            (*element)
                .ty()
                .ok_or_else(|| Error::MissingRepresentation {
                    context: "element type".to_string(),
                })?,
        ),
        _ => Ok(false),
    }
}

/// Return the address space when the repr type is one heap reference.
fn heap_reference_space(
    tree: &mir::Tree,
    ty: mir::LocalNodeId<mir::Type>,
) -> Option<mir::AddressSpace> {
    let ty = repr_type(tree, ty);

    match tree.get(ty) {
        mir::Type::Reference {
            kind: mir::ReferenceKind::Managed | mir::ReferenceKind::Owned,
            address_space,
            ..
        } => Some(address_space.clone()),
        mir::Type::Slice {
            kind: mir::ReferenceKind::Managed | mir::ReferenceKind::Owned,
            address_space,
            ..
        } => Some(address_space.clone()),
        mir::Type::TensorView {
            kind: mir::ReferenceKind::Managed | mir::ReferenceKind::Owned,
            address_space,
            ..
        } => Some(address_space.clone()),
        mir::Type::Callable { .. } => Some(mir::AddressSpace::Local),
        _ => None,
    }
}

/// Return the raw MIR layout entry for one type.
fn raw_layout_entry(tree: &mir::Tree, ty: mir::LocalNodeId<mir::Type>) -> Result<&mir::Layout> {
    tree.type_layout(ty)
        .ok_or_else(|| Error::InvariantViolation {
            context: format!("missing MIR raw layout metadata for {ty:?}"),
        })
}

/// Extract ordered field layouts from one raw MIR layout.
fn raw_fields_from_layout(layout: &mir::Layout) -> Vec<FieldLayout> {
    let mut fields: Vec<_> = layout
        .fields
        .iter()
        .enumerate()
        .map(|(index, field)| {
            (
                field.source_index.unwrap_or(index as u32) as usize,
                FieldLayout {
                    ty: field.ty,
                    offset: field.offset as usize,
                    byte_len: field.size as usize,
                },
            )
        })
        .collect();

    // read source order from MIR field metadata
    fields.sort_by_key(|(index, _)| *index);

    fields.into_iter().map(|(_, field)| field).collect()
}

/// Return the raw MIR array stride.
fn raw_array_stride(layout: &mir::Layout) -> Result<usize> {
    let mir::LayoutKind::Array { element_stride, .. } = &layout.kind else {
        return Err(Error::InvariantViolation {
            context: "missing MIR array layout stride".to_string(),
        });
    };

    Ok(*element_stride as usize)
}

/// Build one VM field layout for one record with callable children.
fn build_runtime_fields_layout(
    tree: &mir::Tree,
    layouts: &mut HashMap<mir::LocalNodeId<mir::Type>, Layout>,
    ty: mir::LocalNodeId<mir::Type>,
    field_types: impl IntoIterator<Item = mir::LocalNodeId<mir::Type>>,
) -> Result<Layout> {
    let mut fields = Vec::new();
    let mut next_offset = 0usize;
    let mut alignment = 1usize;

    // lay out each field using its runtime representation
    for field_type in field_types {
        let field_layout = build_layout(tree, layouts, field_type)?;
        let offset = align_offset(next_offset, field_layout.alignment);

        fields.push(FieldLayout {
            ty: field_type,
            offset,
            byte_len: field_layout.byte_len,
        });

        next_offset = offset.saturating_add(field_layout.byte_len);
        alignment = alignment.max(field_layout.alignment);
    }

    // round the final record size up to the overall alignment
    let byte_len = align_offset(next_offset, alignment);
    let layout = Layout {
        byte_len,
        shape: LayoutShape::Fields(fields),
        reference_map: ReferenceMap::empty(),
        alignment,
    };

    debug_assert!(layout.byte_len <= raw_layout_entry(tree, ty)?.size as usize);

    Ok(layout)
}

/// Align one byte offset up to the requested alignment.
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

/// Build one reference map for one compiled layout.
fn build_reference_map(
    tree: &mir::Tree,
    layouts: &HashMap<mir::LocalNodeId<mir::Type>, Layout>,
    ty: mir::LocalNodeId<mir::Type>,
) -> Result<ReferenceMap> {
    let mut local_offsets = Vec::new();
    let mut shared_offsets = Vec::new();

    // walk the compiled layout tree and collect heap reference offsets
    append_reference_offsets(
        tree,
        layouts,
        ty,
        0,
        &mut local_offsets,
        &mut shared_offsets,
    )?;

    let reference_map = if local_offsets.is_empty() && shared_offsets.is_empty() {
        ReferenceMap::empty()
    } else {
        ReferenceMap::Direct {
            local_offsets: local_offsets.into_boxed_slice(),
            shared_offsets: shared_offsets.into_boxed_slice(),
        }
    };

    Ok(reference_map)
}

/// Append heap reference offsets for one compiled layout subtree.
fn append_reference_offsets(
    tree: &mir::Tree,
    layouts: &HashMap<mir::LocalNodeId<mir::Type>, Layout>,
    ty: mir::LocalNodeId<mir::Type>,
    base_offset: u32,
    local_offsets: &mut Vec<u32>,
    shared_offsets: &mut Vec<u32>,
) -> Result<()> {
    let Some(layout) = layouts.get(&ty) else {
        return Ok(());
    };

    match &layout.shape {
        // scalar heap references contribute one direct offset
        LayoutShape::Scalar => match heap_reference_space(tree, ty) {
            Some(mir::AddressSpace::Local) => local_offsets.push(base_offset),
            Some(mir::AddressSpace::Shared) => shared_offsets.push(base_offset),
            _ => {}
        },

        // field layouts recurse using each field base offset
        LayoutShape::Fields(fields) => {
            for field in fields {
                let field_offset =
                    u32::try_from(field.offset).map_err(|_| Error::InvariantViolation {
                        context: format!("reference map field offset too large: {}", field.offset),
                    })?;
                let field_base =
                    base_offset
                        .checked_add(field_offset)
                        .ok_or_else(|| Error::InvariantViolation {
                            context: format!(
                                "reference map field base overflow: base={base_offset}, offset={field_offset}",
                            ),
                        })?;
                append_reference_offsets(
                    tree,
                    layouts,
                    field.ty,
                    field_base,
                    local_offsets,
                    shared_offsets,
                )?;
            }
        }

        // repeated layouts recurse once per logical element
        LayoutShape::Array { element, length }
        | LayoutShape::Vector {
            element,
            lanes: length,
        }
        | LayoutShape::Tensor {
            element,
            element_count: length,
        } => {
            for index in 0..*length {
                let element_offset =
                    index
                        .checked_mul(element.stride)
                        .ok_or_else(|| Error::InvariantViolation {
                            context: format!(
                                "reference map element offset overflow: index={index}, stride={}",
                                element.stride,
                            ),
                        })?;
                let element_offset =
                    u32::try_from(element_offset).map_err(|_| Error::InvariantViolation {
                        context: format!(
                            "reference map element offset too large: {element_offset}",
                        ),
                    })?;
                let element_base =
                    base_offset
                        .checked_add(element_offset)
                        .ok_or_else(|| Error::InvariantViolation {
                            context: format!(
                                "reference map element base overflow: base={base_offset}, offset={element_offset}",
                            ),
                        })?;
                append_reference_offsets(
                    tree,
                    layouts,
                    element.ty,
                    element_base,
                    local_offsets,
                    shared_offsets,
                )?;
            }
        }
    }

    Ok(())
}

/// Compute the flattened element count for one tensor layout.
fn compute_tensor_element_count(
    shape: &[mir::TensorDimension],
    layout: &mir::TensorLayout,
) -> Result<usize> {
    let shape: Vec<u64> = shape
        .iter()
        .map(|dimension| match dimension {
            mir::TensorDimension::Static(value) => *value,
            mir::TensorDimension::Dynamic => 0,
        })
        .collect();

    let element_count = match layout {
        mir::TensorLayout::RowMajor | mir::TensorLayout::ColumnMajor => {
            let mut element_count = 1u64;

            for dimension in shape.iter().copied() {
                element_count =
                    element_count
                        .checked_mul(dimension)
                        .ok_or_else(|| Error::InvariantViolation {
                            context: format!(
                                "tensor element count overflow: count={element_count}, dimension={dimension}",
                            ),
                        })?;
            }

            usize::try_from(element_count).map_err(|_| Error::InvariantViolation {
                context: format!("tensor element count too large: {element_count}"),
            })?
        }
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

                let extent = (dimension - 1)
                    .checked_mul(stride)
                    .ok_or_else(|| Error::InvariantViolation {
                        context: format!(
                            "strided tensor extent overflow: dimension={dimension}, stride={stride}",
                        ),
                    })?;
                max_index =
                    max_index
                        .checked_add(extent)
                        .ok_or_else(|| Error::InvariantViolation {
                            context: format!(
                                "strided tensor element count overflow: max_index={max_index}, extent={extent}",
                            ),
                        })?;
            }

            let element_count =
                max_index
                    .checked_add(1)
                    .ok_or_else(|| Error::InvariantViolation {
                        context: format!(
                            "strided tensor element count overflow: max_index={max_index}"
                        ),
                    })?;

            usize::try_from(element_count).map_err(|_| Error::InvariantViolation {
                context: format!("strided tensor element count too large: {element_count}"),
            })?
        }
    };

    Ok(element_count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use destack_core::ImmutableStringPool;
    use destack_mir::parse::{ParseOptions, Parser};
    use destack_mir::{Storage, Tree, Type, TypeAlias};
    use destack_source::FileId;

    /// Parse one MIR program with the given storage metadata.
    fn parse_tree_with_layout(mir_text: &str, storage: Storage) -> (Tree, ImmutableStringPool) {
        let (mut tree, strings) = Parser::parse(
            FileId::new(0),
            mir_text,
            ParseOptions {
                pointer_bytes: storage.native_pointer_bytes,
            },
        )
        .validate()
        .expect("failed to parse MIR");
        tree.metadata.layout.storage = storage;
        (tree, strings)
    }

    /// Look up one aliased type by name.
    fn lookup_type_alias(
        tree: &Tree,
        strings: &ImmutableStringPool,
        name: &str,
    ) -> mir::LocalNodeId<Type> {
        for (_, type_alias) in tree.iter_nodes::<TypeAlias>() {
            if strings.get(type_alias.name) == name {
                return type_alias
                    .ty
                    .ty()
                    .expect("type alias should be concrete after validation");
            }
        }

        panic!("missing type alias {name}");
    }

    /// Compiled raw struct layout matches canonical MIR raw layout metadata.
    #[test]
    fn test_build_layout_imports_raw_struct_layout() {
        let mir_text = r#"
type Mixed {
    first: uint8;
    second: int64;
    third: uint8;
}"#;
        let (tree, strings) = parse_tree_with_layout(mir_text, Storage::default());
        let ty = lookup_type_alias(&tree, &strings, "Mixed");
        let layouts = build_layouts(&tree).expect("failed to build layouts");
        let layout = layouts.get(&ty).expect("missing layout");
        let raw_layout = tree.type_layout(ty).expect("missing MIR raw layout");

        // top-level facts
        assert_eq!(layout.byte_len, raw_layout.size as usize);

        // field facts
        let raw_fields = raw_fields_from_layout(raw_layout);
        assert_eq!(layout.field(0), raw_fields.first().copied());
        assert_eq!(layout.field(1), raw_fields.get(1).copied());
        assert_eq!(layout.field(2), raw_fields.get(2).copied());
    }

    /// Heap-backed struct layout matches the canonical pointer-shaped runtime layout.
    #[test]
    fn test_build_layout_uses_canonical_struct_reference_offsets() {
        let mir_text = r#"
type Packed {
    first: uint8;
    inner: ref<int32, managed, readonly>;
    third: uint8;
}"#;
        let (tree, strings) = parse_tree_with_layout(mir_text, Storage::default());
        let ty = lookup_type_alias(&tree, &strings, "Packed");
        let layouts = build_layouts(&tree).expect("failed to build layouts");
        let layout = layouts.get(&ty).expect("missing layout");

        // struct fields follow the canonical runtime layout
        assert_eq!(layout.byte_len, 24);
        assert_eq!(layout.field(0).expect("missing field 0").offset, 0);
        assert_eq!(layout.field(1).expect("missing field 1").offset, 8);
        assert_eq!(layout.field(2).expect("missing field 2").offset, 16);

        // reference tracing should point at the heap reference field
        assert_eq!(
            layout.reference_map,
            ReferenceMap::Direct {
                local_offsets: vec![8].into_boxed_slice(),
                shared_offsets: Vec::new().into_boxed_slice(),
            }
        );
    }

    /// Heap-backed vector layout uses the same physical stride as raw layout.
    #[test]
    fn test_build_layout_uses_canonical_vector_stride() {
        let mir_text = r#"
type Vec = vector<ref<int32, managed, readonly>, 2>"#;
        let (tree, strings) = parse_tree_with_layout(mir_text, Storage::default());
        let ty = lookup_type_alias(&tree, &strings, "Vec");
        let layouts = build_layouts(&tree).expect("failed to build layouts");
        let layout = layouts.get(&ty).expect("missing layout");
        let element = layout.element().expect("missing element layout");

        // vector stride follows the canonical physical layout
        assert_eq!(element.byte_len, 8);
        assert_eq!(element.stride, 8);
        assert_eq!(layout.byte_len, 16);

        // reference tracing should include both elements
        assert_eq!(
            layout.reference_map,
            ReferenceMap::Direct {
                local_offsets: vec![0, 8].into_boxed_slice(),
                shared_offsets: Vec::new().into_boxed_slice(),
            }
        );
    }

    /// Transparent newtype wrappers preserve heap reference tracing.
    #[test]
    fn test_build_layout_traces_newtype_wrapped_heap_reference() {
        let mir_text = r#"
type Handle = newtype<ref<int32, managed, readonly>>;
type Holder {
    value: Handle;
}"#;
        let (tree, strings) = parse_tree_with_layout(mir_text, Storage::default());
        let ty = lookup_type_alias(&tree, &strings, "Holder");
        let layouts = build_layouts(&tree).expect("failed to build layouts");
        let layout = layouts.get(&ty).expect("missing layout");

        // newtype-wrapped heap refs should still appear in the trace map
        assert_eq!(
            layout.reference_map,
            ReferenceMap::Direct {
                local_offsets: vec![0].into_boxed_slice(),
                shared_offsets: Vec::new().into_boxed_slice(),
            }
        );
    }

    /// Callable values stay boxed in heap payloads.
    #[test]
    fn test_build_layout_boxes_callable() {
        let mir_text = r#"
type Callable = () => int32"#;
        let (tree, strings) = parse_tree_with_layout(mir_text, Storage::default());
        let ty = lookup_type_alias(&tree, &strings, "Callable");
        let layouts = build_layouts(&tree).expect("failed to build layouts");
        let layout = layouts.get(&ty).expect("missing layout");

        // callable fields store one heap reference to one callable object
        assert!(layout.is_scalar());
        assert_eq!(layout.byte_len, tree.pointer_bytes() as usize);
        assert_eq!(
            layout.reference_map,
            ReferenceMap::Direct {
                local_offsets: vec![0].into_boxed_slice(),
                shared_offsets: Vec::new().into_boxed_slice(),
            }
        );
    }

    /// Records with callable values still trace the callable child field.
    #[test]
    fn test_build_layout_traces_callable_fields() {
        let mir_text = r#"
type Callable = () => int32;
type Holder {
    pad: uint8;
    action: Callable;
}"#;
        let (tree, strings) = parse_tree_with_layout(mir_text, Storage::default());
        let ty = lookup_type_alias(&tree, &strings, "Holder");
        let layouts = build_layouts(&tree).expect("failed to build layouts");
        let layout = layouts.get(&ty).expect("missing layout");

        // the callable field should stay traced after the VM field rewrite
        assert_eq!(
            layout.reference_map,
            ReferenceMap::Direct {
                local_offsets: vec![8].into_boxed_slice(),
                shared_offsets: Vec::new().into_boxed_slice(),
            }
        );
    }
}
