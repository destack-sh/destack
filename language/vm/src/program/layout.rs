use std::collections::HashMap;

use destack_engine as engine;
use destack_mir as mir;
use destack_mir::{LayoutId, TraceMap};

use crate::program::{WordLayout, pointer_class_from_reference, word_layout_from_pointer_class};
use crate::{Error, Result, Word};

const WORD_BITS: usize = Word::BYTE_LEN * 8;

/// One compiled layout for one MIR type.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Layout {
    /// The MIR heap layout id for this value representation.
    pub layout_id: LayoutId,
    /// The byte width of the value representation.
    pub byte_len: usize,
    /// The structural layout shape.
    shape: LayoutShape,
    /// The trace map for this type.
    pub trace_map: TraceMap,
    /// The byte alignment of the value representation.
    alignment: usize,
}

/// VM type table for one lowered program.
pub(crate) struct TypeTable {
    /// Compiled types keyed by MIR type id.
    types: HashMap<mir::LocalNodeId<mir::Type>, Layout>,
}

/// The compiled shape for one MIR type.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum LayoutShape {
    /// One scalar or pointer value with no structural decomposition.
    Scalar,
    /// One field-addressable payload with a fixed field list.
    Fields(Vec<FieldLayout>),
    /// One slice descriptor with data and length slots.
    Slice,
    /// One element-addressable array with a fixed element stride.
    Array {
        /// The element layout.
        element: ElementLayout,
        /// The static element count.
        length: usize,
    },
    /// One vector with a fixed element count and element width.
    Vector {
        /// The element layout.
        element: ElementLayout,
        /// The element count.
        element_count: usize,
    },
    /// One tensor with a static flattened shape.
    Tensor {
        /// The element layout.
        element: ElementLayout,
        /// The flattened logical length.
        element_count: usize,
    },
    /// One tensor view descriptor with data and stride slots.
    TensorView {
        /// The number of tensor axes.
        rank: usize,
    },
}

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

/// The heap object layout for one closure value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ClosureObjectLayout {
    /// The function pointer offset.
    pub function_offset: usize,
    /// The environment pointer offset.
    pub environment_offset: usize,
    /// The closure object byte length.
    pub byte_len: usize,
    /// The closure object byte alignment.
    pub alignment: usize,
}

impl Layout {
    /// Report whether this type is scalar.
    pub(crate) fn is_scalar(&self) -> bool {
        matches!(self.shape, LayoutShape::Scalar)
    }

    /// Report whether this type fits in one VM word.
    pub(crate) fn is_word(&self) -> bool {
        self.is_scalar() && self.byte_len <= Word::BYTE_LEN
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

    /// Report whether this layout is a slice descriptor.
    pub(crate) fn is_slice(&self) -> bool {
        matches!(self.shape, LayoutShape::Slice)
    }

    /// Return the element layout.
    pub(crate) fn element(&self) -> Option<ElementLayout> {
        match &self.shape {
            LayoutShape::Array { element, .. }
            | LayoutShape::Vector { element, .. }
            | LayoutShape::Tensor { element, .. } => Some(*element),
            LayoutShape::Scalar
            | LayoutShape::Fields(_)
            | LayoutShape::Slice
            | LayoutShape::TensorView { .. } => None,
        }
    }

    /// Return the element count for one indexed layout.
    pub(crate) fn element_count(&self) -> Option<usize> {
        match &self.shape {
            LayoutShape::Array { length, .. } => Some(*length),
            LayoutShape::Vector { element_count, .. } => Some(*element_count),
            LayoutShape::Tensor { element_count, .. } => Some(*element_count),
            LayoutShape::Scalar
            | LayoutShape::Fields(_)
            | LayoutShape::Slice
            | LayoutShape::TensorView { .. } => None,
        }
    }

    /// Return the aligned stride.
    pub(crate) fn stride(&self) -> usize {
        align_offset(self.byte_len, self.alignment)
    }
}

impl TypeTable {
    /// Create one type table.
    pub(crate) fn new(types: HashMap<mir::LocalNodeId<mir::Type>, Layout>) -> Self {
        Self { types }
    }

    /// Return one compiled type layout.
    pub(crate) fn layout(&self, ty: mir::LocalNodeId<mir::Type>) -> Option<&Layout> {
        self.types.get(&ty)
    }

    /// Return one compiled layout by engine value layout id.
    pub(crate) fn layout_for_value_id(&self, layout: engine::ValueLayoutId) -> Option<&Layout> {
        self.layout(Self::type_for_value_layout(layout))
    }

    /// Return the MIR type id encoded by one engine value layout id.
    pub(crate) fn type_for_value_layout(
        layout: engine::ValueLayoutId,
    ) -> mir::LocalNodeId<mir::Type> {
        mir::LocalNodeId::new(layout.0)
    }

    /// Return the engine value layout id for one MIR type id.
    pub(crate) fn value_layout_id(ty: mir::LocalNodeId<mir::Type>) -> engine::ValueLayoutId {
        engine::ValueLayoutId(ty.id)
    }

    /// Return the MIR layout id for one MIR type.
    pub(crate) fn layout_id_for_type(&self, ty: mir::LocalNodeId<mir::Type>) -> Option<LayoutId> {
        self.types.get(&ty).map(|layout| layout.layout_id)
    }
}

impl ClosureObjectLayout {
    /// Return the heap layout table entry for closure objects.
    pub(crate) fn table_layout(self, environment_layout: WordLayout) -> mir::Layout {
        mir::Layout {
            shape: mir::LayoutShape::Closure,
            size: self.byte_len as u32,
            alignment: self.alignment as u32,
            trace_map: closure_trace_map(self.environment_offset, environment_layout),
        }
    }
}

/// Return the closure object layout for one target pointer width.
pub(crate) fn closure_object_layout(pointer_bytes: usize) -> ClosureObjectLayout {
    let function_offset = 0usize;
    let environment_offset = align_offset(pointer_bytes, pointer_bytes);
    let byte_len = environment_offset + pointer_bytes;

    ClosureObjectLayout {
        function_offset,
        environment_offset,
        byte_len,
        alignment: pointer_bytes,
    }
}

/// Return the heap trace map for one closure object.
fn closure_trace_map(environment_offset: usize, environment_layout: WordLayout) -> TraceMap {
    let environment_offset = environment_offset as u32;

    match environment_layout {
        WordLayout::HeapReference => TraceMap::Fixed {
            local_offsets: vec![environment_offset].into_boxed_slice(),
            shared_offsets: Vec::new().into_boxed_slice(),
        },
        WordLayout::SharedHeapReference => TraceMap::Fixed {
            local_offsets: Vec::new().into_boxed_slice(),
            shared_offsets: vec![environment_offset].into_boxed_slice(),
        },
        WordLayout::Void
        | WordLayout::Bool
        | WordLayout::Int { .. }
        | WordLayout::Uint { .. }
        | WordLayout::Float16
        | WordLayout::Bfloat16
        | WordLayout::Float32
        | WordLayout::Float64
        | WordLayout::Address
        | WordLayout::StackPointer
        | WordLayout::FramePointer
        | WordLayout::StaticPointer
        | WordLayout::FunctionPointer => TraceMap::empty(),
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

        ty = (*inner)
            .ty()
            .ok_or_else(|| Error::invalid_program("repr newtype inner"))?;
    }
}

/// Build compiled layouts for all MIR types in the tree.
pub(crate) fn build_layouts(
    tree: &mir::Tree,
    layout_id_by_type: &HashMap<mir::LocalNodeId<mir::Type>, LayoutId>,
) -> Result<HashMap<mir::LocalNodeId<mir::Type>, Layout>> {
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
    layouts: &mut HashMap<mir::LocalNodeId<mir::Type>, Layout>,
    ty: mir::LocalNodeId<mir::Type>,
) -> Result<Layout> {
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
            let value = value
                .ty()
                .ok_or_else(|| Error::invalid_program("uninit value type"))?;
            let mut layout = build_layout(tree, layout_id_by_type, layouts, value)?;
            layout.layout_id = layout_id;

            layout
        }
        mir::Type::TensorView { shape, .. } => build_tensor_view_layout(shape, layout_id)?,
        mir::Type::FunctionSignature { .. } => scalar_layout(0, 1, layout_id),
        mir::Type::Atomic { value } => {
            let value = value
                .ty()
                .ok_or_else(|| Error::invalid_program("atomic value type"))?;

            let mut layout = build_layout(tree, layout_id_by_type, layouts, value)?;
            layout.layout_id = layout_id;

            layout
        }
        mir::Type::Dynamic { .. } => {
            let layout = tree
                .type_layout(ty)
                .ok_or_else(|| Error::invalid_program("dynamic layout"))?;
            build_mir_layout(tree, layout_id_by_type, layouts, layout_id, layout)?
        }
        mir::Type::Newtype { .. } => unreachable!("repr_type must peel newtypes"),
        mir::Type::Struct { fields, .. } => {
            let field_types = fields
                .iter()
                .map(|field_id| {
                    let field = tree.get(*field_id);
                    (field.ty)
                        .ty()
                        .ok_or_else(|| Error::invalid_program("struct field type"))
                })
                .collect::<Result<Vec<_>>>()?;
            build_record_layout(tree, layout_id_by_type, layouts, layout_id, ty, field_types)?
        }
        mir::Type::Variant { .. } => {
            let layout = tree
                .type_layout(ty)
                .ok_or_else(|| Error::invalid_program("variant layout"))?;
            build_mir_layout(tree, layout_id_by_type, layouts, layout_id, layout)?
        }
        mir::Type::Tuple { elements, .. } => {
            let element_types = elements
                .iter()
                .map(|element| {
                    (*element)
                        .ty()
                        .ok_or_else(|| Error::invalid_program("tuple element type"))
                })
                .collect::<Result<Vec<_>>>()?;
            build_record_layout(
                tree,
                layout_id_by_type,
                layouts,
                layout_id,
                ty,
                element_types,
            )?
        }
        mir::Type::Array {
            element, length, ..
        } => build_array_layout(
            tree,
            layout_id_by_type,
            layouts,
            layout_id,
            ty,
            (*element)
                .ty()
                .ok_or_else(|| Error::invalid_program("array element type"))?,
            *length as usize,
        )?,
        mir::Type::Slice {
            kind,
            element: _,
            space,
            ..
        } => build_slice_layout(tree, *kind, space.clone(), layout_id)?,
        mir::Type::Closure { .. } => scalar_layout(
            tree.pointer_bytes() as usize,
            tree.pointer_bytes() as usize,
            layout_id,
        ),
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
                (*element)
                    .ty()
                    .ok_or_else(|| Error::invalid_program("vector element type"))?,
                element_count,
            )?
        }
        mir::Type::Tensor {
            element,
            shape,
            layout,
            ..
        } => build_tensor_layout(
            tree,
            layout_id_by_type,
            layouts,
            layout_id,
            ty,
            (*element)
                .ty()
                .ok_or_else(|| Error::invalid_program("tensor element type"))?,
            shape,
            layout,
        )?,
    };

    // publish one placeholder first so recursive tracing can see the shape graph
    layouts.insert(ty, layout.clone());

    // fill in the trace map after all child layouts exist
    layout.trace_map = build_trace_map(tree, layouts, ty)?;
    layouts.insert(ty, layout.clone());

    Ok(layout)
}

/// Build one scalar layout.
fn scalar_layout(byte_len: usize, alignment: usize, layout_id: LayoutId) -> Layout {
    Layout {
        layout_id,
        byte_len,
        shape: LayoutShape::Scalar,
        trace_map: TraceMap::empty(),
        alignment,
    }
}

/// Build one raw scalar layout.
fn raw_scalar_layout(
    tree: &mir::Tree,
    ty: mir::LocalNodeId<mir::Type>,
    layout_id: LayoutId,
) -> Layout {
    let (raw_byte_len, raw_alignment) = raw_scalar_size_alignment(tree, ty);
    scalar_layout(raw_byte_len, raw_alignment, layout_id)
}

/// Build one VM layout from explicit MIR layout metadata.
fn build_mir_layout(
    tree: &mir::Tree,
    layout_id_by_type: &HashMap<mir::LocalNodeId<mir::Type>, LayoutId>,
    layouts: &mut HashMap<mir::LocalNodeId<mir::Type>, Layout>,
    layout_id: LayoutId,
    layout: &mir::Layout,
) -> Result<Layout> {
    let mut fields = Vec::with_capacity(layout.shape.fields().len());

    for field in layout.shape.fields() {
        build_layout(tree, layout_id_by_type, layouts, field.ty)?;
        fields.push(FieldLayout {
            ty: field.ty,
            offset: field.offset as usize,
            byte_len: field.size as usize,
        });
    }

    Ok(Layout {
        layout_id,
        byte_len: layout.size as usize,
        shape: LayoutShape::Fields(fields),
        trace_map: TraceMap::empty(),
        alignment: layout.alignment as usize,
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
        | mir::Type::Closure { .. }
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

    if bit_width <= WORD_BITS {
        return Word::BYTE_LEN;
    }

    bit_width.div_ceil(8)
}

/// Build one record layout from one ordered field type list.
fn build_record_layout(
    tree: &mir::Tree,
    layout_id_by_type: &HashMap<mir::LocalNodeId<mir::Type>, LayoutId>,
    layouts: &mut HashMap<mir::LocalNodeId<mir::Type>, Layout>,
    layout_id: LayoutId,
    ty: mir::LocalNodeId<mir::Type>,
    field_types: impl IntoIterator<Item = mir::LocalNodeId<mir::Type>> + Clone,
) -> Result<Layout> {
    // ensure all child layouts exist before choosing the representation
    for field_type in field_types.clone() {
        build_layout(tree, layout_id_by_type, layouts, field_type)?;
    }

    // closure fields need the VM field representation
    for field_type in field_types.clone() {
        if contains_closure(tree, field_type)? {
            return build_runtime_fields_layout(
                tree,
                layout_id_by_type,
                layouts,
                layout_id,
                field_types,
            );
        }
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

    Ok(Layout {
        layout_id,
        byte_len: raw_layout.size as usize,
        shape: LayoutShape::Fields(fields),
        trace_map: TraceMap::empty(),
        alignment: raw_layout.alignment as usize,
    })
}

/// Build one array layout.
fn build_array_layout(
    tree: &mir::Tree,
    layout_id_by_type: &HashMap<mir::LocalNodeId<mir::Type>, LayoutId>,
    layouts: &mut HashMap<mir::LocalNodeId<mir::Type>, Layout>,
    layout_id: LayoutId,
    ty: mir::LocalNodeId<mir::Type>,
    element_type: mir::LocalNodeId<mir::Type>,
    length: usize,
) -> Result<Layout> {
    let element_layout = build_layout(tree, layout_id_by_type, layouts, element_type)?;

    // closure elements store heap handles in VM frames
    if contains_closure(tree, element_type)? {
        return Ok(repeated_layout(
            layout_id,
            element_type,
            &element_layout,
            element_layout.stride(),
            length,
            stride_byte_len(length, element_layout.stride())?,
            element_layout.alignment,
            |element| LayoutShape::Array { element, length },
        ));
    }

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
        |element| LayoutShape::Array { element, length },
    ))
}

/// Build one slice descriptor layout.
fn build_slice_layout(
    tree: &mir::Tree,
    kind: mir::ReferenceKind,
    space: mir::Space,
    layout_id: LayoutId,
) -> Result<Layout> {
    let pointer_class = pointer_class_from_reference(space, kind);
    let data_layout = word_layout_from_pointer_class(pointer_class)
        .ok_or_else(|| Error::invalid_pointer_type(format!("{pointer_class:?}")))?;
    let pointer_bytes = tree.pointer_bytes() as usize;
    let data_byte_len = data_layout.byte_len(pointer_bytes);
    let length_offset = align_offset(data_byte_len, pointer_bytes);
    let byte_len = length_offset + pointer_bytes;

    Ok(Layout {
        layout_id,
        byte_len,
        shape: LayoutShape::Slice,
        trace_map: TraceMap::empty(),
        alignment: pointer_bytes,
    })
}

/// Build one vector layout.
fn build_vector_layout(
    tree: &mir::Tree,
    layout_id_by_type: &HashMap<mir::LocalNodeId<mir::Type>, LayoutId>,
    layouts: &mut HashMap<mir::LocalNodeId<mir::Type>, Layout>,
    layout_id: LayoutId,
    ty: mir::LocalNodeId<mir::Type>,
    element_type: mir::LocalNodeId<mir::Type>,
    element_count: usize,
) -> Result<Layout> {
    let element_layout = build_layout(tree, layout_id_by_type, layouts, element_type)?;
    let stride = element_layout.stride();

    // closure elements store heap handles in VM frames
    if contains_closure(tree, element_type)? {
        return Ok(repeated_layout(
            layout_id,
            element_type,
            &element_layout,
            stride,
            element_count,
            stride_byte_len(element_count, stride)?,
            element_layout.alignment,
            |element| LayoutShape::Vector {
                element,
                element_count,
            },
        ));
    }

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
        |element| LayoutShape::Vector {
            element,
            element_count,
        },
    ))
}

/// Build one tensor layout.
fn build_tensor_layout(
    tree: &mir::Tree,
    layout_id_by_type: &HashMap<mir::LocalNodeId<mir::Type>, LayoutId>,
    layouts: &mut HashMap<mir::LocalNodeId<mir::Type>, Layout>,
    layout_id: LayoutId,
    ty: mir::LocalNodeId<mir::Type>,
    element_type: mir::LocalNodeId<mir::Type>,
    shape: &[mir::TensorDimension],
    tensor_layout: &mir::TensorLayout,
) -> Result<Layout> {
    let element_layout = build_layout(tree, layout_id_by_type, layouts, element_type)?;
    let element_count = compute_tensor_element_count(shape, tensor_layout)?;
    let stride = element_layout.stride();

    // closure elements store heap handles in VM frames
    if contains_closure(tree, element_type)? {
        return Ok(repeated_layout(
            layout_id,
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
        layout_id,
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

/// Build one tensor view descriptor layout.
fn build_tensor_view_layout(shape: &[mir::TensorDimension], layout_id: LayoutId) -> Result<Layout> {
    let rank = shape.len();
    let slots = rank
        .checked_add(1)
        .ok_or_else(|| Error::internal(format!("tensor view rank overflow: rank={rank}")))?;
    let byte_len = slots.checked_mul(Word::BYTE_LEN).ok_or_else(|| {
        Error::internal(format!("tensor view byte length overflow: slots={slots}"))
    })?;

    Ok(Layout {
        layout_id,
        byte_len,
        shape: LayoutShape::TensorView { rank },
        trace_map: TraceMap::empty(),
        alignment: Word::BYTE_LEN,
    })
}

/// Build one repeated element layout.
fn repeated_layout(
    layout_id: LayoutId,
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
        layout_id,
        byte_len,
        shape: shape(element),
        trace_map: TraceMap::empty(),
        alignment,
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

/// Report whether the repr type contains one closure value.
fn contains_closure(tree: &mir::Tree, ty: mir::LocalNodeId<mir::Type>) -> Result<bool> {
    let ty = concrete_repr_type(tree, ty)?;

    match tree.get(ty) {
        mir::Type::Closure { .. } => Ok(true),
        mir::Type::Struct { fields, .. } => {
            for field_id in fields {
                let field_type = tree
                    .get(*field_id)
                    .ty
                    .ty()
                    .ok_or_else(|| Error::invalid_program("struct field type"))?;
                if contains_closure(tree, field_type)? {
                    return Ok(true);
                }
            }

            Ok(false)
        }
        mir::Type::Tuple { elements, .. } => {
            for element_type in elements {
                let element_type = (*element_type)
                    .ty()
                    .ok_or_else(|| Error::invalid_program("tuple element type"))?;
                if contains_closure(tree, element_type)? {
                    return Ok(true);
                }
            }

            Ok(false)
        }
        mir::Type::Array { element, .. }
        | mir::Type::Vector { element, .. }
        | mir::Type::Tensor { element, .. } => contains_closure(
            tree,
            (*element)
                .ty()
                .ok_or_else(|| Error::invalid_program("element type"))?,
        ),
        _ => Ok(false),
    }
}

/// Return the space when the repr type is one heap reference.
fn heap_reference_space(tree: &mir::Tree, ty: mir::LocalNodeId<mir::Type>) -> Option<mir::Space> {
    let ty = repr_type(tree, ty);

    match tree.get(ty) {
        mir::Type::Reference { kind, space, .. } if is_heap_reference_kind(*kind) => {
            Some(space.clone())
        }
        mir::Type::Closure { .. } => Some(mir::Space::Local),
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

/// Build one VM field layout for one record with closure children.
fn build_runtime_fields_layout(
    tree: &mir::Tree,
    layout_id_by_type: &HashMap<mir::LocalNodeId<mir::Type>, LayoutId>,
    layouts: &mut HashMap<mir::LocalNodeId<mir::Type>, Layout>,
    layout_id: LayoutId,
    field_types: impl IntoIterator<Item = mir::LocalNodeId<mir::Type>>,
) -> Result<Layout> {
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
    let layout = Layout {
        layout_id,
        byte_len,
        shape: LayoutShape::Fields(fields),
        trace_map: TraceMap::empty(),
        alignment,
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
    layouts: &HashMap<mir::LocalNodeId<mir::Type>, Layout>,
    ty: mir::LocalNodeId<mir::Type>,
) -> Result<TraceMap> {
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

/// Build a tag-selected trace map for one lowered variant.
fn build_variant_trace_map(
    tree: &mir::Tree,
    layouts: &HashMap<mir::LocalNodeId<mir::Type>, Layout>,
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
    let tag_type = tag
        .ty()
        .ok_or_else(|| Error::internal("variant tag type is not concrete"))?;
    let storage_type = storage
        .ty()
        .ok_or_else(|| Error::internal("variant storage type is not concrete"))?;

    let tag_bytes = variant_tag_bytes(tree, tag_type)?;
    let mut trace_variants = Vec::with_capacity(cases.len());

    for case in cases.iter() {
        let element_type = case
            .ty
            .ty()
            .ok_or_else(|| Error::internal("variant value type is not concrete"))?;
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
            .map_err(|_| Error::internal("variant tag does not fit in one word")),
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
    layouts: &HashMap<mir::LocalNodeId<mir::Type>, Layout>,
    storage_type: mir::LocalNodeId<mir::Type>,
    element_type: mir::LocalNodeId<mir::Type>,
) -> Result<TraceMap> {
    let storage_layout = layouts.get(&storage_type).ok_or_else(|| {
        Error::internal(format!(
            "missing variant storage layout for {storage_type:?}"
        ))
    })?;

    if storage_layout.is_word() {
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
            Some(mir::Space::Local) => local_offsets.push(base_offset),
            Some(mir::Space::Shared) => shared_offsets.push(base_offset),
            _ => {}
        },

        // field layouts recurse using each field base offset
        LayoutShape::Fields(fields) => {
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
        }

        // slice descriptors trace the backing storage pointer
        LayoutShape::Slice => {
            let mir::Type::Slice { kind, space, .. } = tree.get(repr_type(tree, ty)) else {
                return Err(Error::internal("slice layout requested for non-slice type"));
            };

            if is_heap_reference_kind(*kind) {
                match space {
                    mir::Space::Local => local_offsets.push(base_offset),
                    mir::Space::Shared => shared_offsets.push(base_offset),
                    _ => {}
                }
            }
        }

        // tensor view descriptors trace the backing storage pointer
        LayoutShape::TensorView { .. } => {
            let mir::Type::TensorView { kind, space, .. } = tree.get(repr_type(tree, ty)) else {
                return Err(Error::internal(
                    "tensor view layout requested for non-tensor-view type",
                ));
            };

            if is_heap_reference_kind(*kind) {
                match space {
                    mir::Space::Local => local_offsets.push(base_offset),
                    mir::Space::Shared => shared_offsets.push(base_offset),
                    _ => {}
                }
            }
        }

        // repeated layouts recurse once per logical element
        LayoutShape::Array { element, length }
        | LayoutShape::Vector {
            element,
            element_count: length,
        }
        | LayoutShape::Tensor {
            element,
            element_count: length,
        } => {
            for index in 0..*length {
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
                let element_base =
                    base_offset
                        .checked_add(element_offset)
                        .ok_or_else(|| Error::internal(format!(
                                "trace map element base overflow: base={base_offset}, offset={element_offset}",
                            )))?;
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
    let mut static_shape = Vec::with_capacity(shape.len());
    for dimension in shape {
        match dimension {
            mir::TensorDimension::Static(value) => static_shape.push(*value),
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

    let element_count = match layout {
        mir::TensorLayout::Dense { .. } => {
            let mut element_count = 1u64;

            for dimension in static_shape.iter().copied() {
                element_count =
                    element_count
                        .checked_mul(dimension)
                        .ok_or_else(|| Error::internal(format!(
                                "tensor element count overflow: count={element_count}, dimension={dimension}",
                            )))?;
            }

            usize::try_from(element_count).map_err(|_| {
                Error::internal(format!("tensor element count too large: {element_count}"))
            })?
        }
    };

    Ok(element_count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use destack_core::StringPool;
    use destack_mir::parse::{ParseOptions, Parser};
    use destack_mir::{DataLayout, Tree, Type, TypeAlias};
    use destack_source::FileId;

    /// Parse one MIR program with the given target metadata.
    fn parse_tree_with_layout(mir_text: &str, data_layout: DataLayout) -> (Tree, StringPool) {
        let (mut tree, strings) = Parser::parse(
            FileId::new(0),
            mir_text,
            ParseOptions {
                pointer_bytes: data_layout.pointer_bytes,
            },
        )
        .finish()
        .expect("failed to parse MIR");
        tree.metadata.data_layout = data_layout;
        (tree, strings)
    }

    /// Look up one aliased type by name.
    fn lookup_type_alias(tree: &Tree, strings: &StringPool, name: &str) -> mir::LocalNodeId<Type> {
        for (_, type_alias) in tree.iter_nodes::<TypeAlias>() {
            if strings.get(type_alias.name) == name {
                return type_alias
                    .ty
                    .ty()
                    .expect("type alias should be concrete after parsing");
            }
        }

        panic!("missing type alias {name}");
    }

    /// Build test layouts with dense synthetic layout ids.
    fn build_test_layouts(tree: &Tree) -> HashMap<mir::LocalNodeId<mir::Type>, Layout> {
        let mut layout_id_by_type = HashMap::new();

        for (index, (type_id, _)) in tree.iter_nodes::<Type>().enumerate() {
            let raw = u32::try_from(index + 1).expect("test layout id should fit");
            layout_id_by_type.insert(type_id, LayoutId::new(raw));
        }

        build_layouts(tree, &layout_id_by_type).expect("failed to build layouts")
    }

    /// Struct layout uses canonical field alignment when raw metadata is absent.
    #[test]
    fn test_build_layout_aligns_struct_fields() {
        let mir_text = r#"
type Mixed {
    first: uint8;
    second: int64;
    third: uint8;
}"#;
        let (tree, strings) = parse_tree_with_layout(mir_text, DataLayout::default());
        let ty = lookup_type_alias(&tree, &strings, "Mixed");
        let layouts = build_test_layouts(&tree);
        let layout = layouts.get(&ty).expect("missing layout");

        assert_eq!(layout.byte_len, 24);

        assert_eq!(layout.field(0).expect("missing field 0").offset, 0);
        assert_eq!(layout.field(1).expect("missing field 1").offset, 8);
        assert_eq!(layout.field(2).expect("missing field 2").offset, 16);
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
        let (tree, strings) = parse_tree_with_layout(mir_text, DataLayout::default());
        let ty = lookup_type_alias(&tree, &strings, "Packed");
        let layouts = build_test_layouts(&tree);
        let layout = layouts.get(&ty).expect("missing layout");

        // struct fields follow the canonical runtime layout
        assert_eq!(layout.byte_len, 24);
        assert_eq!(layout.field(0).expect("missing field 0").offset, 0);
        assert_eq!(layout.field(1).expect("missing field 1").offset, 8);
        assert_eq!(layout.field(2).expect("missing field 2").offset, 16);

        // reference tracing should point at the heap reference field
        assert_eq!(
            layout.trace_map,
            TraceMap::Fixed {
                local_offsets: vec![8].into_boxed_slice(),
                shared_offsets: Vec::new().into_boxed_slice(),
            }
        );
    }

    /// Heap-space borrowed references are traced as interior roots.
    #[test]
    fn test_build_layout_traces_heap_borrowed_reference() {
        let mir_text = r#"
type View {
    name: ref<int32, borrowed, lifetime(static), readonly>;
}"#;
        let (tree, strings) = parse_tree_with_layout(mir_text, DataLayout::default());
        let ty = lookup_type_alias(&tree, &strings, "View");
        let layouts = build_test_layouts(&tree);
        let layout = layouts.get(&ty).expect("missing layout");

        assert_eq!(
            layout.trace_map,
            TraceMap::Fixed {
                local_offsets: vec![0].into_boxed_slice(),
                shared_offsets: Vec::new().into_boxed_slice(),
            }
        );
    }

    /// Heap-backed slice descriptors trace their backing storage pointer.
    #[test]
    fn test_build_layout_traces_heap_slice_descriptor() {
        let mir_text = r#"
type View {
    items: slice<int32, managed>;
}"#;
        let (tree, strings) = parse_tree_with_layout(mir_text, DataLayout::default());
        let ty = lookup_type_alias(&tree, &strings, "View");
        let layouts = build_test_layouts(&tree);
        let layout = layouts.get(&ty).expect("missing layout");

        assert_eq!(
            layout.trace_map,
            TraceMap::Fixed {
                local_offsets: vec![0].into_boxed_slice(),
                shared_offsets: Vec::new().into_boxed_slice(),
            }
        );
    }

    /// Heap-backed vector layout uses the same physical stride as raw layout.
    #[test]
    fn test_build_layout_uses_canonical_vector_stride() {
        let mir_text = r#"
type Vec = vector<ref<int32, managed, readonly>, 2>"#;
        let (tree, strings) = parse_tree_with_layout(mir_text, DataLayout::default());
        let ty = lookup_type_alias(&tree, &strings, "Vec");
        let layouts = build_test_layouts(&tree);
        let layout = layouts.get(&ty).expect("missing layout");
        let element = layout.element().expect("missing element layout");

        // vector stride follows the canonical physical layout
        assert_eq!(element.byte_len, 8);
        assert_eq!(element.stride, 8);
        assert_eq!(layout.byte_len, 16);

        // reference tracing should include both elements
        assert_eq!(
            layout.trace_map,
            TraceMap::Fixed {
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
        let (tree, strings) = parse_tree_with_layout(mir_text, DataLayout::default());
        let ty = lookup_type_alias(&tree, &strings, "Holder");
        let layouts = build_test_layouts(&tree);
        let layout = layouts.get(&ty).expect("missing layout");

        // newtype-wrapped heap refs should still appear in the trace map
        assert_eq!(
            layout.trace_map,
            TraceMap::Fixed {
                local_offsets: vec![0].into_boxed_slice(),
                shared_offsets: Vec::new().into_boxed_slice(),
            }
        );
    }

    /// Lowered unions trace only the active storage variant.
    #[test]
    fn test_build_layout_uses_tagged_trace_map_for_union() {
        let mir_text = r#"
type Ref = ref<int32, managed, readonly>;
type Plain = int32;
type Tag = uint8;
type Storage = [usize; 1];
type Shape = variant<Tag, Storage> { 0uint8 = Ref; 1uint8 = Plain; }"#;
        let (mut tree, strings) = parse_tree_with_layout(mir_text, DataLayout::default());
        let union_type = lookup_type_alias(&tree, &strings, "Shape");
        let layout_id = tree.metadata.layout.layout_table.insert(mir::Layout {
            shape: mir::LayoutShape::Variant(mir::VariantLayout {
                tag: mir::VariantTagLayout {
                    ty: None,
                    size: 1,
                    alignment: 1,
                },
                payload_offset: 8,
                variants: Vec::new(),
            }),
            size: 16,
            alignment: 8,
            trace_map: TraceMap::empty(),
        });
        let union_types = tree
            .iter_nodes::<Type>()
            .filter_map(|(type_id, ty)| matches!(ty, Type::Variant { .. }).then_some(type_id))
            .collect::<Vec<_>>();

        for union_type in union_types {
            tree.metadata.layout.set_layout_id(union_type, layout_id);
        }

        let layouts = build_test_layouts(&tree);
        let layout = layouts.get(&union_type).expect("missing layout");

        assert_eq!(
            layout.trace_map,
            TraceMap::Tagged {
                tag_bytes: 1,
                variants: vec![
                    mir::TraceVariant {
                        tag: 0,
                        payload_offset: 8,
                        map: TraceMap::Fixed {
                            local_offsets: vec![0].into_boxed_slice(),
                            shared_offsets: Vec::new().into_boxed_slice(),
                        },
                    },
                    mir::TraceVariant {
                        tag: 1,
                        payload_offset: 8,
                        map: TraceMap::empty(),
                    },
                ]
                .into_boxed_slice(),
            }
        );
    }

    /// Closure values stay boxed in heap payloads.
    #[test]
    fn test_build_layout_boxes_closure() {
        let mir_text = r#"
type Callable = () => int32"#;
        let (tree, strings) = parse_tree_with_layout(mir_text, DataLayout::default());
        let ty = lookup_type_alias(&tree, &strings, "Callable");
        let layouts = build_test_layouts(&tree);
        let layout = layouts.get(&ty).expect("missing layout");

        // closure fields store one heap reference to one closure object
        assert!(layout.is_scalar());
        assert_eq!(layout.byte_len, tree.pointer_bytes() as usize);
        assert_eq!(
            layout.trace_map,
            TraceMap::Fixed {
                local_offsets: vec![0].into_boxed_slice(),
                shared_offsets: Vec::new().into_boxed_slice(),
            }
        );
    }

    /// Records with closure values still trace the closure child field.
    #[test]
    fn test_build_layout_traces_closure_fields() {
        let mir_text = r#"
type Callable = () => int32;
type Holder {
    pad: uint8;
    action: Callable;
}"#;
        let (tree, strings) = parse_tree_with_layout(mir_text, DataLayout::default());
        let ty = lookup_type_alias(&tree, &strings, "Holder");
        let layouts = build_test_layouts(&tree);
        let layout = layouts.get(&ty).expect("missing layout");

        // the closure field should stay traced after the VM field rewrite
        assert_eq!(
            layout.trace_map,
            TraceMap::Fixed {
                local_offsets: vec![8].into_boxed_slice(),
                shared_offsets: Vec::new().into_boxed_slice(),
            }
        );
    }
}
