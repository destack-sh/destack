use std::collections::HashMap;

use destack_mir as mir;
use destack_mir::{TraceMap, VariantTrace};

use destack_program::LayoutId;
use destack_program::vm::Cell;

use crate::LinkResult;

use super::super::ProgramLinker;

const CELL_BITS: usize = Cell::BYTE_LEN * 8;

/// One transient byte-storage layout for one MIR type during VM lowering.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct StorageLayout {
    /// The program layout id for this value representation.
    pub(crate) layout_id: LayoutId,
    /// The byte width of the value representation.
    pub(crate) byte_len: usize,
    /// The trace map for this type.
    pub(crate) trace_map: TraceMap,
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

/// One lowered field layout.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FieldLayout {
    /// The field value type.
    pub(crate) ty: mir::LocalNodeId<mir::Type>,
    /// The byte offset of the field inside the parent value.
    pub(crate) offset: usize,
    /// The byte width of the field payload.
    pub(crate) byte_len: usize,
}

/// One lowered element layout.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ElementLayout {
    /// The element value type.
    pub(crate) ty: mir::LocalNodeId<mir::Type>,
    /// The byte stride between adjacent elements.
    pub(crate) stride: usize,
    /// The byte width of one element payload.
    pub(crate) byte_len: usize,
}

/// Reference offsets grouped by traceable reference storage.
#[derive(Default)]
struct ReferenceOffsets {
    /// Byte offsets of local heap references.
    local_offsets: Vec<u32>,
    /// Byte offsets of shared heap references.
    shared_offsets: Vec<u32>,
    /// Byte offsets of frame references.
    frame_offsets: Vec<u32>,
}

impl StorageLayout {
    /// Return the byte width of the value representation.
    pub(crate) const fn byte_len(&self) -> usize {
        self.byte_len
    }

    /// Build lowered storage layouts for all MIR types in the tree.
    pub(crate) fn build_all(
        tree: &mir::Tree,
        target_layout: &mir::TargetLayout,
        table: &mir::LayoutTable,
        layout_ids: &HashMap<mir::LocalNodeId<mir::Type>, LayoutId>,
        program: &ProgramLinker,
    ) -> LinkResult<HashMap<mir::LocalNodeId<mir::Type>, StorageLayout>> {
        StorageLayoutBuilder::new(tree, target_layout, table, layout_ids, program).build()
    }

    /// Build one scalar layout.
    fn scalar(byte_len: usize, alignment: usize, layout_id: LayoutId) -> Self {
        Self {
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
    fn empty(layout_id: LayoutId) -> Self {
        Self {
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

    /// Build one repeated element layout.
    fn repeated(
        layout_id: LayoutId,
        element_type: mir::LocalNodeId<mir::Type>,
        element_layout: &Self,
        stride: usize,
        element_count: usize,
        byte_len: usize,
        alignment: usize,
    ) -> Self {
        let element = ElementLayout {
            ty: element_type,
            stride,
            byte_len: element_layout.byte_len(),
        };

        debug_assert!(element_count == 0 || stride >= element_layout.byte_len());

        Self {
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

    /// Report whether this type is scalar.
    pub(crate) fn is_scalar(&self) -> bool {
        self.is_scalar
    }

    /// Report whether this type fits in one VM cell.
    pub(crate) fn is_cell(&self) -> bool {
        self.is_scalar() && self.byte_len() <= Cell::BYTE_LEN
    }

    /// Return the byte alignment of this layout.
    pub(crate) fn alignment(&self) -> usize {
        self.alignment
    }

    /// Return the power-of-two alignment exponent.
    pub(crate) fn alignment_log2(&self) -> u32 {
        self.alignment.trailing_zeros()
    }

    /// Return one field layout by index.
    pub(crate) fn field(&self, index: u32) -> Option<FieldLayout> {
        let fields = self.fields.as_ref()?;

        fields.get(index as usize).copied()
    }

    /// Return the field count for one field-addressable layout.
    pub(crate) fn field_count(&self) -> Option<usize> {
        let fields = self.fields.as_ref()?;

        Some(fields.len())
    }

    /// Report whether this layout is a slice descriptor.
    pub(crate) fn is_slice(&self) -> bool {
        self.is_slice
    }

    /// Return the element layout.
    pub(crate) fn element(&self) -> Option<ElementLayout> {
        self.element
    }

    /// Return the element count for one indexed layout.
    pub(crate) fn element_count(&self) -> Option<usize> {
        self.element_count
    }

    /// Return the aligned stride.
    pub(crate) fn stride(&self) -> usize {
        align_offset(self.byte_len(), self.alignment)
    }
}

impl ReferenceOffsets {
    /// Return whether no offsets are present.
    fn is_empty(&self) -> bool {
        self.local_offsets.is_empty()
            && self.shared_offsets.is_empty()
            && self.frame_offsets.is_empty()
    }

    /// Push one reference offset into the selected storage column.
    fn push(&mut self, space: mir::Space, offset: u32) {
        match space {
            mir::Space::Local => self.local_offsets.push(offset),
            mir::Space::Shared => self.shared_offsets.push(offset),
            mir::Space::Frame => self.frame_offsets.push(offset),
            mir::Space::Static => unreachable!("static references cannot appear in trace maps"),
        }
    }

    /// Build one trace map from the collected columns.
    fn into_trace_map(self) -> TraceMap {
        if self.is_empty() {
            TraceMap::Empty
        } else {
            TraceMap::Fixed {
                local_offsets: self.local_offsets.into_boxed_slice(),
                shared_offsets: self.shared_offsets.into_boxed_slice(),
                frame_offsets: self.frame_offsets.into_boxed_slice(),
            }
        }
    }
}

impl FieldLayout {
    /// Return the byte width of the field payload.
    pub(crate) const fn byte_len(self) -> usize {
        self.byte_len
    }
}

impl ElementLayout {
    /// Return the byte width of one element payload.
    pub(crate) const fn byte_len(self) -> usize {
        self.byte_len
    }
}

/// Builder for transient VM storage layouts.
struct StorageLayoutBuilder<'a> {
    /// The MIR tree being lowered.
    tree: &'a mir::Tree,
    /// Target ABI layout.
    target_layout: &'a mir::TargetLayout,
    /// Canonical MIR layout table.
    table: &'a mir::LayoutTable,
    /// Executable layout ids keyed by MIR type.
    layout_ids: &'a HashMap<mir::LocalNodeId<mir::Type>, LayoutId>,
    /// Program linker owning diagnostics and program id projection.
    program: &'a ProgramLinker,
    /// Storage layouts built so far.
    layouts: HashMap<mir::LocalNodeId<mir::Type>, StorageLayout>,
}

impl<'a> StorageLayoutBuilder<'a> {
    /// Create one storage layout builder.
    fn new(
        tree: &'a mir::Tree,
        target_layout: &'a mir::TargetLayout,
        table: &'a mir::LayoutTable,
        layout_ids: &'a HashMap<mir::LocalNodeId<mir::Type>, LayoutId>,
        program: &'a ProgramLinker,
    ) -> Self {
        Self {
            tree,
            target_layout,
            table,
            layout_ids,
            program,
            layouts: HashMap::new(),
        }
    }

    /// Build storage layouts for every MIR type.
    fn build(mut self) -> LinkResult<HashMap<mir::LocalNodeId<mir::Type>, StorageLayout>> {
        for (type_id, _) in self.tree.iter_nodes::<mir::Type>() {
            self.layout(type_id)?;
        }

        Ok(self.layouts)
    }
}

impl StorageLayoutBuilder<'_> {
    /// Build one storage layout for one MIR type.
    fn layout(&mut self, ty: mir::LocalNodeId<mir::Type>) -> LinkResult<StorageLayout> {
        // reuse already-built layouts first
        if let Some(layout) = self.layouts.get(&ty) {
            return Ok(layout.clone());
        }

        let layout_id = self.layout_ids.get(&ty).copied().ok_or_else(|| {
            self.program
                .invalid_input(format!("heap layout id for type {ty:?}"))
        })?;

        // peel transparent wrappers before choosing the physical representation
        let repr_ty = self.concrete_repr_type(ty)?;
        if repr_ty != ty {
            let mut layout = self.layout(repr_ty)?;
            layout.layout_id = layout_id;
            self.layouts.insert(ty, layout.clone());

            return Ok(layout);
        }

        // build the canonical physical representation for the repr type
        let mut layout = match self.tree.get(ty) {
            mir::Type::Void
            | mir::Type::Boolean
            | mir::Type::Int { .. }
            | mir::Type::Isize
            | mir::Type::Usize
            | mir::Type::TypeDescriptor
            | mir::Type::TypeId
            | mir::Type::Reference { .. }
            | mir::Type::FunctionPointer { .. }
            | mir::Type::Float(_) => self.raw_scalar_layout(ty, layout_id),
            mir::Type::ManuallyDrop { value } | mir::Type::Uninit { value } => {
                let mut layout = self.layout(*value)?;
                layout.layout_id = layout_id;

                layout
            }
            mir::Type::TensorView { shape, format, .. } => {
                self.tensor_view_layout(shape, *format, layout_id)?
            }
            mir::Type::FunctionSignature { .. } => StorageLayout::empty(layout_id),
            mir::Type::Atomic { value } => {
                let mut layout = self.layout(*value)?;
                layout.layout_id = layout_id;

                layout
            }
            mir::Type::Dynamic { .. } => self.dynamic_layout(layout_id),
            mir::Type::Newtype { .. } | mir::Type::WithLifetimes { .. } => {
                unreachable!("repr_type must peel transparent type wrappers")
            }
            mir::Type::Error => return Err(self.program.invalid_input("error type")),
            mir::Type::Struct { fields, .. } => {
                let field_types = fields
                    .iter()
                    .map(|field_id| self.tree.get(*field_id).ty)
                    .collect::<Vec<_>>();
                self.record_layout(layout_id, ty, field_types)?
            }
            mir::Type::Variant { .. } => self.variant_layout(layout_id, ty)?,
            mir::Type::Tuple { elements, .. } => {
                let element_types = elements.to_vec();
                self.record_layout(layout_id, ty, element_types)?
            }
            mir::Type::FixedArray {
                element, length, ..
            } => self.array_layout(layout_id, ty, *element, *length as usize)?,
            mir::Type::Slice { .. } => self.slice_layout(layout_id),
            mir::Type::Function { environment, .. } => {
                self.function_layout(layout_id, *environment)?
            }
            mir::Type::Vector { element, lanes, .. } => {
                let element_count = *lanes as usize;

                self.vector_layout(layout_id, ty, *element, element_count)?
            }
            mir::Type::Tensor { .. } => self.handle_layout(layout_id),
        };

        // register the shape before tracing recursive children
        self.layouts.insert(ty, layout.clone());

        // fill in the trace map after all child layouts exist
        layout.trace_map = self.trace_map(ty)?;
        self.layouts.insert(ty, layout.clone());

        Ok(layout)
    }

    /// Return the concrete representation type for one source type.
    fn concrete_repr_type(
        &self,
        mut ty: mir::LocalNodeId<mir::Type>,
    ) -> LinkResult<mir::LocalNodeId<mir::Type>> {
        loop {
            match self.tree.get(ty) {
                mir::Type::Newtype { inner, .. } => {
                    ty = *inner;
                }
                mir::Type::WithLifetimes { base, .. } => {
                    ty = *base;
                }
                mir::Type::Error => {
                    return Err(self.program.invalid_input("error type"));
                }
                _ => return Ok(ty),
            };
        }
    }

    /// Build the trace map for one storage layout.
    fn trace_map(&self, ty: mir::LocalNodeId<mir::Type>) -> LinkResult<TraceMap> {
        self.build_trace_map(ty)
    }

    /// Build one raw scalar layout.
    fn raw_scalar_layout(
        &self,
        ty: mir::LocalNodeId<mir::Type>,
        layout_id: LayoutId,
    ) -> StorageLayout {
        let (byte_len, alignment) = self.raw_scalar_size_alignment(ty);

        StorageLayout::scalar(byte_len, alignment, layout_id)
    }

    /// Return the raw scalar size and alignment for one MIR type.
    fn raw_scalar_size_alignment(&self, ty: mir::LocalNodeId<mir::Type>) -> (usize, usize) {
        match self.tree.get(ty) {
            mir::Type::Void => (0, 1),
            mir::Type::Boolean => (1, 1),
            mir::Type::Int { width, .. } => {
                let byte_len = scalar_byte_len(*width as usize);

                (byte_len, byte_len.clamp(1, 8))
            }
            mir::Type::Isize | mir::Type::Usize => {
                let byte_len = self.target_layout.pointer_bytes() as usize;

                (byte_len, byte_len.clamp(1, 8))
            }
            mir::Type::Float(float_type) => {
                let byte_len = (float_type.width() as usize).div_ceil(8);

                (byte_len, byte_len.clamp(1, 8))
            }
            mir::Type::TypeId => {
                let byte_len = std::mem::size_of::<u32>();

                (byte_len, byte_len)
            }
            mir::Type::TypeDescriptor
            | mir::Type::Reference { .. }
            | mir::Type::FunctionPointer { .. } => {
                let byte_len = self.target_layout.pointer_bytes() as usize;

                (byte_len, byte_len.max(1))
            }
            _ => unreachable!("raw scalar layout requested for non scalar type"),
        }
    }

    /// Return one repeated payload byte length.
    fn checked_stride_byte_len(&self, element_count: usize, stride: usize) -> LinkResult<usize> {
        stride.checked_mul(element_count).ok_or_else(|| {
            self.program.layout_overflow(format!(
                "repeated layout byte length: stride={stride}, element_count={element_count}",
            ))
        })
    }

    /// Build one physical variant layout.
    fn variant_layout(
        &mut self,
        layout_id: LayoutId,
        ty: mir::TypeId,
    ) -> LinkResult<StorageLayout> {
        let layout = self
            .table
            .type_layout(ty)
            .ok_or_else(|| self.program.invalid_input("variant layout"))?;
        let mir::LayoutShape::Variant(variant) = &layout.shape else {
            return Err(self.program.invalid_input("variant layout shape"));
        };

        // build every type referenced by the physical variant layout
        self.layout(variant.discriminant)?;
        self.layout(variant.storage)?;
        for case in &variant.cases {
            self.layout(case.ty)?;
        }

        Ok(StorageLayout {
            layout_id,
            byte_len: layout.size as usize,
            trace_map: TraceMap::empty(),
            alignment: layout.alignment as usize,
            fields: None,
            element: None,
            element_count: None,
            is_scalar: false,
            is_slice: false,
        })
    }

    /// Build one record layout from one ordered field type list.
    fn record_layout(
        &mut self,
        layout_id: LayoutId,
        ty: mir::LocalNodeId<mir::Type>,
        field_types: impl IntoIterator<Item = mir::LocalNodeId<mir::Type>> + Clone,
    ) -> LinkResult<StorageLayout> {
        // ensure all child layouts exist before choosing the representation
        for field_type in field_types.clone() {
            self.layout(field_type)?;
        }

        // prefer layout tables when present
        let Some(raw_layout) = self.table.type_layout(ty) else {
            return self.record_layout_from_fields(layout_id, field_types);
        };
        let fields = Self::table_entry_fields(raw_layout);

        Ok(StorageLayout {
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
    fn array_layout(
        &mut self,
        layout_id: LayoutId,
        ty: mir::LocalNodeId<mir::Type>,
        element_type: mir::LocalNodeId<mir::Type>,
        length: usize,
    ) -> LinkResult<StorageLayout> {
        let element_layout = self.layout(element_type)?;

        // prefer layout tables when present
        let (stride, byte_len, alignment) = match self.table.type_layout(ty) {
            Some(raw_layout) => (
                self.table_entry_array_stride(raw_layout)?,
                raw_layout.size as usize,
                raw_layout.alignment as usize,
            ),
            None => {
                let stride = element_layout.stride();
                (
                    stride,
                    self.checked_stride_byte_len(length, stride)?,
                    element_layout.alignment,
                )
            }
        };

        Ok(StorageLayout::repeated(
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
    fn slice_layout(&self, layout_id: LayoutId) -> StorageLayout {
        StorageLayout {
            layout_id,
            byte_len: Cell::BYTE_LEN * 2,
            trace_map: TraceMap::empty(),
            alignment: Cell::BYTE_LEN,
            fields: None,
            element: None,
            element_count: None,
            is_scalar: false,
            is_slice: true,
        }
    }

    /// Build one function value layout.
    fn function_layout(
        &mut self,
        layout_id: LayoutId,
        environment_type: mir::LocalNodeId<mir::Type>,
    ) -> LinkResult<StorageLayout> {
        let environment_layout = self.layout(environment_type)?;
        if !environment_layout.is_cell() {
            return Err(self.program.invalid_input("function environment layout"));
        }

        Ok(StorageLayout {
            layout_id,
            byte_len: Cell::BYTE_LEN * 2,
            trace_map: TraceMap::empty(),
            alignment: Cell::BYTE_LEN,
            fields: None,
            element: None,
            element_count: None,
            is_scalar: false,
            is_slice: false,
        })
    }

    /// Build one dynamic carrier layout.
    fn dynamic_layout(&self, layout_id: LayoutId) -> StorageLayout {
        StorageLayout {
            layout_id,
            byte_len: Cell::BYTE_LEN * 2,
            trace_map: TraceMap::empty(),
            alignment: Cell::BYTE_LEN,
            fields: None,
            element: None,
            element_count: None,
            is_scalar: false,
            is_slice: false,
        }
    }

    /// Build one opaque handle layout.
    fn handle_layout(&self, layout_id: LayoutId) -> StorageLayout {
        let pointer_bytes = self.target_layout.pointer_bytes() as usize;

        StorageLayout::scalar(pointer_bytes, pointer_bytes, layout_id)
    }

    /// Build one vector layout.
    fn vector_layout(
        &mut self,
        layout_id: LayoutId,
        ty: mir::LocalNodeId<mir::Type>,
        element_type: mir::LocalNodeId<mir::Type>,
        element_count: usize,
    ) -> LinkResult<StorageLayout> {
        let element_layout = self.layout(element_type)?;
        let stride = element_layout.stride();

        // prefer layout tables when present
        let byte_len = match self.table.type_layout(ty) {
            Some(layout) => layout.size as usize,
            None => self.checked_stride_byte_len(element_count, stride)?,
        };
        let alignment = self
            .table
            .type_layout(ty)
            .map(|layout| layout.alignment as usize)
            .unwrap_or(element_layout.alignment);

        Ok(StorageLayout::repeated(
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
    fn tensor_view_layout(
        &self,
        shape: &[mir::TensorDimension],
        format: mir::TensorViewFormat,
        layout_id: LayoutId,
    ) -> LinkResult<StorageLayout> {
        let rank = u32::try_from(shape.len()).map_err(|_| {
            self.program
                .layout_overflow(format!("tensor view rank: rank={}", shape.len()))
        })?;
        let slots = usize::try_from(format.descriptor_slots(rank)).map_err(|_| {
            self.program
                .layout_overflow(format!("tensor view descriptor slot count: rank={rank}"))
        })?;
        let byte_len = slots.checked_mul(Cell::BYTE_LEN).ok_or_else(|| {
            self.program
                .layout_overflow(format!("tensor view byte length: slots={slots}"))
        })?;

        Ok(StorageLayout {
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

    /// Extract ordered field layouts from one MIR layout table entry.
    fn table_entry_fields(layout: &mir::Layout) -> Vec<FieldLayout> {
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

        // read source order from the field entries
        fields.sort_by_key(|(index, _)| *index);

        fields.into_iter().map(|(_, field)| field).collect()
    }

    /// Return the fixed-array stride from one MIR layout table entry.
    fn table_entry_array_stride(&self, layout: &mir::Layout) -> LinkResult<usize> {
        let mir::LayoutShape::Array(layout) = &layout.shape else {
            return Err(self.program.invalid_input("layout table array stride"));
        };

        Ok(layout.stride as usize)
    }

    /// Build one record layout from ordered fields.
    fn record_layout_from_fields(
        &mut self,
        layout_id: LayoutId,
        field_types: impl IntoIterator<Item = mir::LocalNodeId<mir::Type>>,
    ) -> LinkResult<StorageLayout> {
        let mut fields = Vec::new();
        let mut next_offset = 0usize;
        let mut alignment = 1usize;

        // lay out each field using its runtime representation
        for field_type in field_types {
            let field_layout = self.layout(field_type)?;
            let offset = align_offset(next_offset, field_layout.alignment);

            fields.push(FieldLayout {
                ty: field_type,
                offset,
                byte_len: field_layout.byte_len(),
            });

            next_offset = offset
                .checked_add(field_layout.byte_len())
                .ok_or_else(|| self.program.layout_overflow("record field offset"))?;
            alignment = alignment.max(field_layout.alignment);
        }

        // round the final record size up to the overall alignment
        let byte_len = align_offset(next_offset, alignment);
        let layout = StorageLayout {
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

    /// Build one trace map for one storage layout.
    fn build_trace_map(&self, ty: mir::LocalNodeId<mir::Type>) -> LinkResult<TraceMap> {
        if let Some(layout) = self.table.type_layout(ty)
            && matches!(layout.shape, mir::LayoutShape::Function)
        {
            return Ok(layout.trace_map.clone());
        }

        if let mir::Type::Function { environment, .. } = self.tree.get(self.tree.repr_type(ty)) {
            return self.function_trace_map(*environment);
        }

        if matches!(
            self.tree.get(self.tree.repr_type(ty)),
            mir::Type::Dynamic { .. }
        ) {
            // trace the boxed local payload stored in the first carrier cell
            let mut offsets = ReferenceOffsets::default();
            offsets.push(mir::Space::Local, 0);

            return Ok(offsets.into_trace_map());
        }

        if let Some(layout) = self.table.type_layout(ty)
            && matches!(layout.shape, mir::LayoutShape::Variant(_))
        {
            return self.build_variant_trace_map(layout);
        }

        let mut offsets = ReferenceOffsets::default();

        // walk the storage layout tree and collect traceable reference offsets
        self.append_reference_offsets(ty, 0, &mut offsets)?;

        Ok(offsets.into_trace_map())
    }

    /// Build one trace map for a function value environment word.
    fn function_trace_map(
        &self,
        environment_type: mir::LocalNodeId<mir::Type>,
    ) -> LinkResult<TraceMap> {
        let environment_layout = self.layouts.get(&environment_type).ok_or_else(|| {
            self.program.invalid_input(format!(
                "function environment layout for {environment_type:?}"
            ))
        })?;
        let offset = Cell::BYTE_LEN as u32;

        if !environment_layout.is_cell() {
            return Err(self.program.invalid_input("function environment layout"));
        }

        let Some(reference) = self.trace_space(environment_type) else {
            return Ok(TraceMap::Empty);
        };

        let mut offsets = ReferenceOffsets::default();
        offsets.push(reference, offset);

        Ok(offsets.into_trace_map())
    }

    /// Build a discriminant-selected trace map for one lowered variant.
    fn build_variant_trace_map(&self, layout: &mir::Layout) -> LinkResult<TraceMap> {
        let mir::LayoutShape::Variant(layout) = &layout.shape else {
            return Err(self
                .program
                .invalid_input("trace map requested for non-variant layout"));
        };
        let mut trace_variants = Vec::with_capacity(layout.cases.len());

        for case in &layout.cases {
            let map = self.variant_trace_map(case.ty)?;
            trace_variants.push(VariantTrace {
                discriminant: case.discriminant,
                payload_offset: case.payload_offset,
                map,
            });
        }

        Ok(TraceMap::Variant {
            encoding: layout.encoding,
            cases: trace_variants.into_boxed_slice(),
        })
    }

    /// Return the storage trace map for one variant.
    fn variant_trace_map(&self, element_type: mir::TypeId) -> LinkResult<TraceMap> {
        self.layouts
            .get(&element_type)
            .map(|layout| layout.trace_map.clone())
            .ok_or_else(|| {
                self.program
                    .invalid_input(format!("variant layout for {element_type:?}"))
            })
    }

    /// Append traceable reference offsets for one storage layout subtree.
    fn append_reference_offsets(
        &self,
        ty: mir::LocalNodeId<mir::Type>,
        base_offset: u32,
        offsets: &mut ReferenceOffsets,
    ) -> LinkResult<()> {
        let layout = self.layouts.get(&ty).ok_or_else(|| {
            self.program
                .invalid_input(format!("trace map layout for {ty:?}"))
        })?;

        // scalar traceable references contribute one direct offset
        if layout.is_scalar() {
            if let Some(reference) = self.trace_space(ty) {
                offsets.push(reference, base_offset);
            }

            return Ok(());
        }

        // field layouts recurse using each field base offset
        if let Some(fields) = &layout.fields {
            for field in fields {
                let field_offset = u32::try_from(field.offset).map_err(|_| {
                    self.program
                        .layout_overflow(format!("trace map field offset: {}", field.offset))
                })?;
                let field_base = base_offset.checked_add(field_offset).ok_or_else(|| {
                    self.program.layout_overflow(format!(
                        "trace map field base: base={base_offset}, offset={field_offset}",
                    ))
                })?;
                self.append_reference_offsets(field.ty, field_base, offsets)?;
            }

            return Ok(());
        }

        // slice descriptors trace the backing storage pointer
        let repr_ty = self.tree.repr_type(ty);
        if let mir::Type::Slice { kind, space, .. } = self.tree.get(repr_ty) {
            if let Some(reference) = Self::trace_space_from_parts(*kind, space) {
                offsets.push(reference, base_offset);
            }

            return Ok(());
        }

        // tensor view descriptors trace the backing storage pointer
        if let mir::Type::TensorView { kind, space, .. } = self.tree.get(repr_ty) {
            if let Some(reference) = Self::trace_space_from_parts(*kind, space) {
                offsets.push(reference, base_offset);
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
                self.program.layout_overflow(format!(
                    "trace map element offset: index={index}, stride={}",
                    element.stride,
                ))
            })?;
            let element_offset = u32::try_from(element_offset).map_err(|_| {
                self.program
                    .layout_overflow(format!("trace map element offset: {element_offset}"))
            })?;
            let element_base = base_offset.checked_add(element_offset).ok_or_else(|| {
                self.program.layout_overflow(format!(
                    "trace map element base: base={base_offset}, offset={element_offset}",
                ))
            })?;
            self.append_reference_offsets(element.ty, element_base, offsets)?;
        }

        Ok(())
    }

    /// Return the trace column selected by one traceable type.
    fn trace_space(&self, ty: mir::LocalNodeId<mir::Type>) -> Option<mir::Space> {
        let ty = self.tree.repr_type(ty);

        match self.tree.get(ty) {
            mir::Type::Reference { kind, space, .. } => Self::trace_space_from_parts(*kind, space),
            _ => None,
        }
    }

    /// Return the trace column selected by one reference kind and space.
    fn trace_space_from_parts(kind: mir::ReferenceKind, space: &mir::Space) -> Option<mir::Space> {
        match space {
            mir::Space::Frame => Some(mir::Space::Frame),
            mir::Space::Static => None,
            mir::Space::Local => match kind {
                mir::ReferenceKind::Raw => None,
                mir::ReferenceKind::Managed
                | mir::ReferenceKind::Unique
                | mir::ReferenceKind::Borrowed => Some(mir::Space::Local),
            },
            mir::Space::Shared => match kind {
                mir::ReferenceKind::Raw => None,
                mir::ReferenceKind::Managed
                | mir::ReferenceKind::Unique
                | mir::ReferenceKind::Borrowed => Some(mir::Space::Shared),
            },
        }
    }
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
