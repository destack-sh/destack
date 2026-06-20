use std::collections::HashMap;

use destack_mir as mir;
use destack_mir::{LayoutId, TraceMap};

use crate::{Cell, Error, Result};
use destack_program::vm::{address_space_from_reference, cell_layout_from_address_space};

const CELL_BITS: usize = Cell::BYTE_LEN * 8;

/// One compiled layout for one MIR type.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ValueLayout {
    /// The MIR heap layout id for this value representation.
    pub(super) layout_id: LayoutId,
    /// The byte width of the value representation.
    pub(super) byte_len: usize,
    /// The trace map for this type.
    pub(super) trace_map: TraceMap,
    /// The byte alignment of the value representation.
    alignment: usize,
    /// Field layouts when this value is field-addressable.
    fields: Option<Vec<FieldLayout>>,
    /// Element layout when this value is element-addressable.
    element: Option<ElementLayout>,
    /// Static element count when this value is element-addressable.
    element_count: Option<usize>,
    /// Whether this value is a scalar VM cell candidate.
    is_scalar: bool,
    /// Whether this value is a slice descriptor.
    is_slice: bool,
}

/// One compiled field layout.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct FieldLayout {
    /// The field value type.
    pub(super) ty: mir::LocalNodeId<mir::Type>,
    /// The byte offset of the field inside the parent value.
    pub(super) offset: usize,
    /// The byte width of the field payload.
    pub(super) byte_len: usize,
}

/// One compiled element layout.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct ElementLayout {
    /// The element value type.
    pub(super) ty: mir::LocalNodeId<mir::Type>,
    /// The byte stride between adjacent elements.
    pub(super) stride: usize,
    /// The byte width of one element payload.
    pub(super) byte_len: usize,
}

impl ValueLayout {
    /// Report whether this type is scalar.
    pub(super) fn is_scalar(&self) -> bool {
        self.is_scalar
    }

    /// Report whether this type fits in one VM cell.
    pub(super) fn is_cell(&self) -> bool {
        self.is_scalar() && self.byte_len <= Cell::BYTE_LEN
    }

    /// Return the byte alignment of this layout.
    pub(super) fn alignment(&self) -> usize {
        self.alignment
    }

    /// Return one field layout by index.
    pub(super) fn field(&self, index: u32) -> Option<FieldLayout> {
        let fields = self.fields.as_ref()?;

        fields.get(index as usize).copied()
    }

    /// Return the field count for one field-addressable layout.
    pub(super) fn field_count(&self) -> Option<usize> {
        let fields = self.fields.as_ref()?;

        Some(fields.len())
    }

    /// Report whether this layout is a slice descriptor.
    pub(super) fn is_slice(&self) -> bool {
        self.is_slice
    }

    /// Return the element layout.
    pub(super) fn element(&self) -> Option<ElementLayout> {
        self.element
    }

    /// Return the element count for one indexed layout.
    pub(super) fn element_count(&self) -> Option<usize> {
        self.element_count
    }

    /// Return the aligned stride.
    pub(super) fn stride(&self) -> usize {
        align_offset(self.byte_len, self.alignment)
    }
}

/// Return the concrete representation type for one semantic type.
fn concrete_repr_type(
    tree: &mir::Tree,
    mut ty: mir::LocalNodeId<mir::Type>,
) -> Result<mir::LocalNodeId<mir::Type>> {
    loop {
        match tree.get(ty) {
            mir::Type::Newtype { inner, .. } => {
                ty = *inner;
            }
            mir::Type::WithLifetimes { base, .. } => {
                ty = *base;
            }
            mir::Type::Error => {
                return Err(Error::invalid_program("error type"));
            }
            _ => return Ok(ty),
        };
    }
}

/// Build compiled layouts for all MIR types in the tree.
pub(super) fn build_layouts(
    tree: &mir::Tree,
    layout_id_by_type: &HashMap<mir::LocalNodeId<mir::Type>, LayoutId>,
) -> Result<HashMap<mir::LocalNodeId<mir::Type>, ValueLayout>> {
    let mut layouts = HashMap::new();

    // build one layout entry for every MIR type
    for (type_id, _) in tree.iter_nodes::<mir::Type>() {
        build_layout(tree, layout_id_by_type, &mut layouts, type_id)?;
    }

    Ok(layouts)
}

/// Build one compiled layout for one MIR type.
fn build_layout(
    tree: &mir::Tree,
    layout_id_by_type: &HashMap<mir::LocalNodeId<mir::Type>, LayoutId>,
    layouts: &mut HashMap<mir::LocalNodeId<mir::Type>, ValueLayout>,
    ty: mir::LocalNodeId<mir::Type>,
) -> Result<ValueLayout> {
    // reuse already-built layouts first
    if let Some(layout) = layouts.get(&ty) {
        return Ok(layout.clone());
    }

    let layout_id = layout_id_by_type
        .get(&ty)
        .copied()
        .ok_or_else(|| Error::internal(format!("missing heap layout id for type {ty:?}")))?;

    // peel transparent wrappers before choosing the physical representation
    let repr_ty = concrete_repr_type(tree, ty)?;
    if repr_ty != ty {
        let mut layout = build_layout(tree, layout_id_by_type, layouts, repr_ty)?;
        layout.layout_id = layout_id;
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
        | mir::Type::Float { .. } => raw_scalar_layout(tree, ty, layout_id),
        mir::Type::Uninit { value } => {
            let mut layout = build_layout(tree, layout_id_by_type, layouts, *value)?;
            layout.layout_id = layout_id;

            layout
        }
        mir::Type::TensorView { shape, format, .. } => {
            build_tensor_view_layout(shape, *format, layout_id)?
        }
        mir::Type::FunctionSignature { .. } => empty_layout(layout_id),
        mir::Type::Atomic { value } => {
            let mut layout = build_layout(tree, layout_id_by_type, layouts, *value)?;
            layout.layout_id = layout_id;

            layout
        }
        mir::Type::Dynamic { .. } => {
            let layout = tree
                .type_layout(ty)
                .ok_or_else(|| Error::invalid_program("dynamic layout"))?;
            build_mir_layout(tree, layout_id_by_type, layouts, layout_id, layout)?
        }
        mir::Type::Newtype { .. } | mir::Type::WithLifetimes { .. } => {
            unreachable!("repr_type must peel transparent type wrappers")
        }
        mir::Type::Error => return Err(Error::invalid_program("error type")),
        mir::Type::Struct { fields, .. } => {
            let field_types = fields
                .iter()
                .map(|field_id| tree.get(*field_id).ty)
                .collect::<Vec<_>>();
            build_record_layout(tree, layout_id_by_type, layouts, layout_id, ty, field_types)?
        }
        mir::Type::Variant { .. } => {
            let layout = tree
                .type_layout(ty)
                .ok_or_else(|| Error::invalid_program("variant layout"))?;
            build_mir_layout(tree, layout_id_by_type, layouts, layout_id, layout)?
        }
        mir::Type::Tuple { elements, .. } => {
            let element_types = elements.to_vec();
            build_record_layout(
                tree,
                layout_id_by_type,
                layouts,
                layout_id,
                ty,
                element_types,
            )?
        }
        mir::Type::FixedArray {
            element, length, ..
        } => build_array_layout(
            tree,
            layout_id_by_type,
            layouts,
            layout_id,
            ty,
            *element,
            *length as usize,
        )?,
        mir::Type::Slice {
            kind,
            element: _,
            space,
            ..
        } => build_slice_layout(tree, *kind, space.clone(), layout_id)?,
        mir::Type::Function { environment, .. } => {
            build_function_layout(tree, layout_id_by_type, layouts, layout_id, *environment)?
        }
        mir::Type::Vector {
            element,
            lanes: mir_element_count,
            ..
        } => {
            let element_count = *mir_element_count as usize;

            build_vector_layout(
                tree,
                layout_id_by_type,
                layouts,
                layout_id,
                ty,
                *element,
                element_count,
            )?
        }
        mir::Type::Tensor { .. } => build_handle_layout(tree, layout_id),
    };

    // register the shape before tracing recursive children
    layouts.insert(ty, layout.clone());

    // fill in the trace map after all child layouts exist
    layout.trace_map = build_trace_map(tree, layouts, ty)?;
    layouts.insert(ty, layout.clone());

    Ok(layout)
}

/// Build one scalar layout.
fn scalar_layout(byte_len: usize, alignment: usize, layout_id: LayoutId) -> ValueLayout {
    ValueLayout {
        layout_id,
        byte_len,
        trace_map: TraceMap::empty(),
        alignment,
        fields: None,
        element: None,
        element_count: None,
        is_scalar: true,
        is_slice: false,
    }
}

/// Build one layout for types with no runtime value storage.
fn empty_layout(layout_id: LayoutId) -> ValueLayout {
    ValueLayout {
        layout_id,
        byte_len: 0,
        trace_map: TraceMap::empty(),
        alignment: 1,
        fields: None,
        element: None,
        element_count: None,
        is_scalar: false,
        is_slice: false,
    }
}

/// Build one raw scalar layout.
fn raw_scalar_layout(
    tree: &mir::Tree,
    ty: mir::LocalNodeId<mir::Type>,
    layout_id: LayoutId,
) -> ValueLayout {
    let (raw_byte_len, raw_alignment) = raw_scalar_size_alignment(tree, ty);
    scalar_layout(raw_byte_len, raw_alignment, layout_id)
}

/// Build one VM layout from explicit MIR layout metadata.
fn build_mir_layout(
    tree: &mir::Tree,
    layout_id_by_type: &HashMap<mir::LocalNodeId<mir::Type>, LayoutId>,
    layouts: &mut HashMap<mir::LocalNodeId<mir::Type>, ValueLayout>,
    layout_id: LayoutId,
    layout: &mir::Layout,
) -> Result<ValueLayout> {
    let mut fields = Vec::with_capacity(layout.shape.fields().len());

    for field in layout.shape.fields() {
        build_layout(tree, layout_id_by_type, layouts, field.ty)?;
        fields.push(FieldLayout {
            ty: field.ty,
            offset: field.offset as usize,
            byte_len: field.size as usize,
        });
    }

    Ok(ValueLayout {
        layout_id,
        byte_len: layout.size as usize,
        trace_map: TraceMap::empty(),
        alignment: layout.alignment as usize,
        fields: Some(fields),
        element: None,
        element_count: None,
        is_scalar: false,
        is_slice: false,
    })
}

/// Return the raw scalar size and alignment for one MIR type.
fn raw_scalar_size_alignment(tree: &mir::Tree, ty: mir::LocalNodeId<mir::Type>) -> (usize, usize) {
    match tree.get(ty) {
        mir::Type::Void => (0, 1),
        mir::Type::Boolean => (1, 1),
        mir::Type::Int { width, .. } => {
            let byte_len = scalar_byte_len(*width as usize);

            (byte_len, byte_len.clamp(1, 8))
        }
        mir::Type::Isize | mir::Type::Usize => {
            let byte_len = tree.pointer_bytes() as usize;

            (byte_len, byte_len.clamp(1, 8))
        }
        mir::Type::Float(float_type) => {
            let byte_len = (float_type.width() as usize).div_ceil(8);

            (byte_len, byte_len.clamp(1, 8))
        }
        mir::Type::TypeDescriptor
        | mir::Type::TypeId
        | mir::Type::Reference { .. }
        | mir::Type::FunctionPointer { .. } => {
            let byte_len = tree.pointer_bytes() as usize;

            (byte_len, byte_len.max(1))
        }
        _ => unreachable!("raw scalar layout requested for non scalar type"),
    }
}

/// Return the canonical byte width for one scalar bit width.
fn scalar_byte_len(bit_width: usize) -> usize {
    if bit_width <= 8 {
        return 1;
    }

    if bit_width <= 16 {
        return 2;
    }

    if bit_width <= 32 {
        return 4;
    }

    if bit_width <= CELL_BITS {
        return Cell::BYTE_LEN;
    }

    bit_width.div_ceil(8)
}

/// Build one record layout from one ordered field type list.
fn build_record_layout(
    tree: &mir::Tree,
    layout_id_by_type: &HashMap<mir::LocalNodeId<mir::Type>, LayoutId>,
    layouts: &mut HashMap<mir::LocalNodeId<mir::Type>, ValueLayout>,
    layout_id: LayoutId,
    ty: mir::LocalNodeId<mir::Type>,
    field_types: impl IntoIterator<Item = mir::LocalNodeId<mir::Type>> + Clone,
) -> Result<ValueLayout> {
    // ensure all child layouts exist before choosing the representation
    for field_type in field_types.clone() {
        build_layout(tree, layout_id_by_type, layouts, field_type)?;
    }

    // otherwise mirror the canonical MIR record layout directly
    let Some(raw_layout) = tree.type_layout(ty) else {
        return build_runtime_fields_layout(
            tree,
            layout_id_by_type,
            layouts,
            layout_id,
            field_types,
        );
    };
    let fields = raw_fields_from_layout(raw_layout);

    Ok(ValueLayout {
        layout_id,
        byte_len: raw_layout.size as usize,
        trace_map: TraceMap::empty(),
        alignment: raw_layout.alignment as usize,
        fields: Some(fields),
        element: None,
        element_count: None,
        is_scalar: false,
        is_slice: false,
    })
}

/// Build one array layout.
fn build_array_layout(
    tree: &mir::Tree,
    layout_id_by_type: &HashMap<mir::LocalNodeId<mir::Type>, LayoutId>,
    layouts: &mut HashMap<mir::LocalNodeId<mir::Type>, ValueLayout>,
    layout_id: LayoutId,
    ty: mir::LocalNodeId<mir::Type>,
    element_type: mir::LocalNodeId<mir::Type>,
    length: usize,
) -> Result<ValueLayout> {
    let element_layout = build_layout(tree, layout_id_by_type, layouts, element_type)?;

    // otherwise mirror canonical MIR layout when present
    let (stride, byte_len, alignment) = match tree.type_layout(ty) {
        Some(raw_layout) => (
            raw_array_stride(raw_layout)?,
            raw_layout.size as usize,
            raw_layout.alignment as usize,
        ),
        None => {
            let stride = element_layout.stride();
            (
                stride,
                stride_byte_len(length, stride)?,
                element_layout.alignment,
            )
        }
    };

    Ok(repeated_layout(
        layout_id,
        element_type,
        &element_layout,
        stride,
        length,
        byte_len,
        alignment,
    ))
}

/// Build one slice descriptor layout.
fn build_slice_layout(
    tree: &mir::Tree,
    kind: mir::ReferenceKind,
    space: mir::Space,
    layout_id: LayoutId,
) -> Result<ValueLayout> {
    let address_space = address_space_from_reference(space, kind);
    let data_layout = cell_layout_from_address_space(address_space)
        .ok_or_else(|| Error::invalid_pointer_type(format!("{address_space:?}")))?;
    let pointer_bytes = tree.pointer_bytes() as usize;
    let data_byte_len = data_layout.byte_len(pointer_bytes);
    let length_offset = align_offset(data_byte_len, pointer_bytes);
    let byte_len = length_offset + pointer_bytes;

    Ok(ValueLayout {
        layout_id,
        byte_len,
        trace_map: TraceMap::empty(),
        alignment: pointer_bytes,
        fields: None,
        element: None,
        element_count: None,
        is_scalar: false,
        is_slice: true,
    })
}

/// Build one function value layout.
fn build_function_layout(
    tree: &mir::Tree,
    layout_id_by_type: &HashMap<mir::LocalNodeId<mir::Type>, LayoutId>,
    layouts: &mut HashMap<mir::LocalNodeId<mir::Type>, ValueLayout>,
    layout_id: LayoutId,
    environment_type: mir::LocalNodeId<mir::Type>,
) -> Result<ValueLayout> {
    let pointer_bytes = tree.pointer_bytes() as usize;
    let environment_layout = build_layout(tree, layout_id_by_type, layouts, environment_type)?;
    if !environment_layout.is_cell() {
        return Err(Error::invalid_program("function environment layout"));
    }

    Ok(ValueLayout {
        layout_id,
        byte_len: pointer_bytes * 2,
        trace_map: TraceMap::empty(),
        alignment: pointer_bytes,
        fields: None,
        element: None,
        element_count: None,
        is_scalar: false,
        is_slice: false,
    })
}

/// Build one opaque handle layout.
fn build_handle_layout(tree: &mir::Tree, layout_id: LayoutId) -> ValueLayout {
    let pointer_bytes = tree.pointer_bytes() as usize;

    scalar_layout(pointer_bytes, pointer_bytes, layout_id)
}

/// Build one vector layout.
fn build_vector_layout(
    tree: &mir::Tree,
    layout_id_by_type: &HashMap<mir::LocalNodeId<mir::Type>, LayoutId>,
    layouts: &mut HashMap<mir::LocalNodeId<mir::Type>, ValueLayout>,
    layout_id: LayoutId,
    ty: mir::LocalNodeId<mir::Type>,
    element_type: mir::LocalNodeId<mir::Type>,
    element_count: usize,
) -> Result<ValueLayout> {
    let element_layout = build_layout(tree, layout_id_by_type, layouts, element_type)?;
    let stride = element_layout.stride();

    // otherwise prefer canonical MIR vector size and alignment when available
    let byte_len = match tree.type_layout(ty) {
        Some(layout) => layout.size as usize,
        None => stride_byte_len(element_count, stride)?,
    };
    let alignment = tree
        .type_layout(ty)
        .map(|layout| layout.alignment as usize)
        .unwrap_or(element_layout.alignment);

    Ok(repeated_layout(
        layout_id,
        element_type,
        &element_layout,
        stride,
        element_count,
        byte_len,
        alignment,
    ))
}

/// Build one tensor view descriptor layout.
fn build_tensor_view_layout(
    shape: &[mir::TensorDimension],
    format: mir::TensorViewFormat,
    layout_id: LayoutId,
) -> Result<ValueLayout> {
    let rank = u32::try_from(shape.len())
        .map_err(|_| Error::internal(format!("tensor view rank overflow: rank={}", shape.len())))?;
    let slots = usize::try_from(format.descriptor_slots(rank)).map_err(|_| {
        Error::internal(format!("tensor view descriptor slot overflow: rank={rank}"))
    })?;
    let byte_len = slots.checked_mul(Cell::BYTE_LEN).ok_or_else(|| {
        Error::internal(format!("tensor view byte length overflow: slots={slots}"))
    })?;

    Ok(ValueLayout {
        layout_id,
        byte_len,
        trace_map: TraceMap::empty(),
        alignment: Cell::BYTE_LEN,
        fields: None,
        element: None,
        element_count: None,
        is_scalar: false,
        is_slice: false,
    })
}

/// Build one repeated element layout.
fn repeated_layout(
    layout_id: LayoutId,
    element_type: mir::LocalNodeId<mir::Type>,
    element_layout: &ValueLayout,
    stride: usize,
    element_count: usize,
    byte_len: usize,
    alignment: usize,
) -> ValueLayout {
    let element = ElementLayout {
        ty: element_type,
        stride,
        byte_len: element_layout.byte_len,
    };

    debug_assert!(element_count == 0 || stride >= element_layout.byte_len);

    ValueLayout {
        layout_id,
        byte_len,
        trace_map: TraceMap::empty(),
        alignment,
        fields: None,
        element: Some(element),
        element_count: Some(element_count),
        is_scalar: false,
        is_slice: false,
    }
}

/// Return one repeated payload byte length.
fn stride_byte_len(element_count: usize, stride: usize) -> Result<usize> {
    stride.checked_mul(element_count).ok_or_else(|| {
        Error::internal(format!(
            "repeated layout byte length overflow: stride={stride}, element_count={element_count}",
        ))
    })
}

/// Return the space when the repr type is one heap reference.
fn heap_reference_space(tree: &mir::Tree, ty: mir::LocalNodeId<mir::Type>) -> Option<mir::Space> {
    let ty = tree.repr_type(ty);

    match tree.get(ty) {
        mir::Type::Reference { kind, space, .. } if is_heap_reference_kind(*kind) => {
            Some(space.clone())
        }
        _ => None,
    }
}

/// Return whether one reference kind can name heap storage.
fn is_heap_reference_kind(kind: mir::ReferenceKind) -> bool {
    matches!(
        kind,
        mir::ReferenceKind::Managed | mir::ReferenceKind::Unique | mir::ReferenceKind::Borrowed
    )
}

/// Extract ordered field layouts from one raw MIR layout.
fn raw_fields_from_layout(layout: &mir::Layout) -> Vec<FieldLayout> {
    let mut fields: Vec<_> = layout
        .shape
        .fields()
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
    let mir::LayoutShape::Array(layout) = &layout.shape else {
        return Err(Error::internal("missing MIR array layout stride"));
    };

    Ok(layout.stride as usize)
}

/// Build one VM field layout for one record without canonical MIR metadata.
fn build_runtime_fields_layout(
    tree: &mir::Tree,
    layout_id_by_type: &HashMap<mir::LocalNodeId<mir::Type>, LayoutId>,
    layouts: &mut HashMap<mir::LocalNodeId<mir::Type>, ValueLayout>,
    layout_id: LayoutId,
    field_types: impl IntoIterator<Item = mir::LocalNodeId<mir::Type>>,
) -> Result<ValueLayout> {
    let mut fields = Vec::new();
    let mut next_offset = 0usize;
    let mut alignment = 1usize;

    // lay out each field using its runtime representation
    for field_type in field_types {
        let field_layout = build_layout(tree, layout_id_by_type, layouts, field_type)?;
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
    let layout = ValueLayout {
        layout_id,
        byte_len,
        trace_map: TraceMap::empty(),
        alignment,
        fields: Some(fields),
        element: None,
        element_count: None,
        is_scalar: false,
        is_slice: false,
    };

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

/// Build one trace map for one compiled layout.
fn build_trace_map(
    tree: &mir::Tree,
    layouts: &HashMap<mir::LocalNodeId<mir::Type>, ValueLayout>,
    ty: mir::LocalNodeId<mir::Type>,
) -> Result<TraceMap> {
    if let Some(layout) = tree.type_layout(ty)
        && matches!(layout.shape, mir::LayoutShape::Function)
    {
        return Ok(layout.trace_map.clone());
    }

    if let mir::Type::Function { environment, .. } = tree.get(tree.repr_type(ty)) {
        return function_trace_map(tree, layouts, *environment);
    }

    if let Some(layout) = tree.type_layout(ty)
        && matches!(layout.shape, mir::LayoutShape::Variant(_))
    {
        return build_variant_trace_map(tree, layouts, ty, layout);
    }

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

    let trace_map = if local_offsets.is_empty() && shared_offsets.is_empty() {
        TraceMap::empty()
    } else {
        TraceMap::Fixed {
            local_offsets: local_offsets.into_boxed_slice(),
            shared_offsets: shared_offsets.into_boxed_slice(),
        }
    };

    Ok(trace_map)
}

/// Build one trace map for a function value environment word.
fn function_trace_map(
    tree: &mir::Tree,
    layouts: &HashMap<mir::LocalNodeId<mir::Type>, ValueLayout>,
    environment_type: mir::LocalNodeId<mir::Type>,
) -> Result<TraceMap> {
    let environment_layout = layouts.get(&environment_type).ok_or_else(|| {
        Error::internal(format!(
            "missing function environment layout for {environment_type:?}"
        ))
    })?;
    let offset = u32::from(tree.pointer_bytes());

    if !environment_layout.is_cell() {
        return Err(Error::invalid_program("function environment layout"));
    }

    Ok(match heap_reference_space(tree, environment_type) {
        Some(mir::Space::Local) => TraceMap::Fixed {
            local_offsets: vec![offset].into_boxed_slice(),
            shared_offsets: Vec::new().into_boxed_slice(),
        },
        Some(mir::Space::Shared) => TraceMap::Fixed {
            local_offsets: Vec::new().into_boxed_slice(),
            shared_offsets: vec![offset].into_boxed_slice(),
        },
        _ => TraceMap::empty(),
    })
}

/// Build a tag-selected trace map for one lowered variant.
fn build_variant_trace_map(
    tree: &mir::Tree,
    layouts: &HashMap<mir::LocalNodeId<mir::Type>, ValueLayout>,
    ty: mir::LocalNodeId<mir::Type>,
    layout: &mir::Layout,
) -> Result<TraceMap> {
    let mir::LayoutShape::Variant(layout) = &layout.shape else {
        return Err(Error::internal(
            "trace map requested for non-variant layout",
        ));
    };
    let mir::Type::Variant {
        tag,
        storage,
        cases,
        ..
    } = tree.get(ty)
    else {
        return Err(Error::internal("trace map requested for non-variant type"));
    };
    let tag_type = *tag;
    let storage_type = *storage;

    let tag_bytes = variant_tag_bytes(tree, tag_type)?;
    let mut trace_variants = Vec::with_capacity(cases.len());

    for case in cases.iter() {
        let element_type = case.ty;
        let map = variant_trace_map(layouts, storage_type, element_type)?;
        trace_variants.push(mir::TraceVariant {
            tag: variant_tag_bits(tree, tag_type, &case.tag)?,
            payload_offset: layout.payload_offset,
            map,
        });
    }

    Ok(TraceMap::Tagged {
        tag_bytes,
        variants: trace_variants.into_boxed_slice(),
    })
}

/// Return one variant tag as normalized runtime bits.
fn variant_tag_bits(
    tree: &mir::Tree,
    tag_type: mir::LocalNodeId<mir::Type>,
    tag: &mir::Constant,
) -> Result<u64> {
    match (tree.get(tag_type), tag) {
        (
            mir::Type::Int {
                width,
                is_signed: true,
            },
            mir::Constant::Int { value, .. },
        ) if *width <= u64::BITS as u16 => Ok((*value as i64) as u64),
        (
            mir::Type::Int {
                width,
                is_signed: false,
            },
            mir::Constant::UInt { value, .. },
        ) if *width <= u64::BITS as u16 => u64::try_from(*value)
            .map_err(|_| Error::internal("variant tag does not fit in one cell")),
        _ => Err(Error::internal("variant tag does not match tag type")),
    }
}

/// Return the tag byte width for one variant tag type.
fn variant_tag_bytes(tree: &mir::Tree, tag_type: mir::LocalNodeId<mir::Type>) -> Result<u8> {
    let mir::Type::Int { width, .. } = tree.get(tag_type) else {
        return Err(Error::internal(format!(
            "variant tag type is not integer: {tag_type:?}"
        )));
    };

    u8::try_from(scalar_byte_len(*width as usize))
        .map_err(|_| Error::internal(format!("variant tag width is too large: {width}")))
}

/// Return the storage trace map for one variant.
fn variant_trace_map(
    layouts: &HashMap<mir::LocalNodeId<mir::Type>, ValueLayout>,
    storage_type: mir::LocalNodeId<mir::Type>,
    element_type: mir::LocalNodeId<mir::Type>,
) -> Result<TraceMap> {
    let storage_layout = layouts.get(&storage_type).ok_or_else(|| {
        Error::internal(format!(
            "missing variant storage layout for {storage_type:?}"
        ))
    })?;

    if storage_layout.is_cell() {
        return Ok(storage_layout.trace_map.clone());
    }

    layouts
        .get(&element_type)
        .map(|layout| layout.trace_map.clone())
        .ok_or_else(|| Error::internal(format!("missing variant layout for {element_type:?}")))
}

/// Append heap reference offsets for one compiled layout subtree.
fn append_reference_offsets(
    tree: &mir::Tree,
    layouts: &HashMap<mir::LocalNodeId<mir::Type>, ValueLayout>,
    ty: mir::LocalNodeId<mir::Type>,
    base_offset: u32,
    local_offsets: &mut Vec<u32>,
    shared_offsets: &mut Vec<u32>,
) -> Result<()> {
    let Some(layout) = layouts.get(&ty) else {
        return Ok(());
    };

    // scalar heap references contribute one direct offset
    if layout.is_scalar() {
        match heap_reference_space(tree, ty) {
            Some(mir::Space::Local) => local_offsets.push(base_offset),
            Some(mir::Space::Shared) => shared_offsets.push(base_offset),
            _ => {}
        }

        return Ok(());
    }

    // field layouts recurse using each field base offset
    if let Some(fields) = &layout.fields {
        for field in fields {
            let field_offset = u32::try_from(field.offset).map_err(|_| {
                Error::internal(format!(
                    "trace map field offset too large: {}",
                    field.offset
                ))
            })?;
            let field_base = base_offset.checked_add(field_offset).ok_or_else(|| {
                Error::internal(format!(
                    "trace map field base overflow: base={base_offset}, offset={field_offset}",
                ))
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

        return Ok(());
    }

    // slice descriptors trace the backing storage pointer
    let repr_ty = tree.repr_type(ty);
    if let mir::Type::Slice { kind, space, .. } = tree.get(repr_ty) {
        if is_heap_reference_kind(*kind) {
            match space {
                mir::Space::Local => local_offsets.push(base_offset),
                mir::Space::Shared => shared_offsets.push(base_offset),
                _ => {}
            }
        }

        return Ok(());
    }

    // tensor view descriptors trace the backing storage pointer
    if let mir::Type::TensorView { kind, space, .. } = tree.get(repr_ty) {
        if is_heap_reference_kind(*kind) {
            match space {
                mir::Space::Local => local_offsets.push(base_offset),
                mir::Space::Shared => shared_offsets.push(base_offset),
                _ => {}
            }
        }

        return Ok(());
    }

    // repeated layouts recurse once per logical element
    let Some(element) = layout.element else {
        return Ok(());
    };
    let Some(length) = layout.element_count else {
        return Ok(());
    };
    for index in 0..length {
        let element_offset = index.checked_mul(element.stride).ok_or_else(|| {
            Error::internal(format!(
                "trace map element offset overflow: index={index}, stride={}",
                element.stride,
            ))
        })?;
        let element_offset = u32::try_from(element_offset).map_err(|_| {
            Error::internal(format!(
                "trace map element offset too large: {element_offset}",
            ))
        })?;
        let element_base = base_offset.checked_add(element_offset).ok_or_else(|| {
            Error::internal(format!(
                "trace map element base overflow: base={base_offset}, offset={element_offset}",
            ))
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

    Ok(())
}
