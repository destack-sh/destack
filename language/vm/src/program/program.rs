use std::collections::{HashMap, HashSet};
use std::fmt;

use destack_core::ImmutableStringPool;
use destack_mir::{LayoutId, LayoutKind, LayoutTable, ReferenceMap};
use {destack_engine as engine, destack_heap as heap, destack_mir as mir};

use super::layout::{Layout, LayoutIndex, build_layouts, callable_object_layout};
use super::{
    CallTarget, CallableObjectLayout, FrameBinding, FrameEntry, FrameState, FrameStateTable,
    Function, FunctionTable, ProgramPoint, SideTable, SideTableBuilder,
};
use crate::lower::{ValueType, analyze_value_types, lower_function};
use crate::{Error, FunctionPointer, Result, StaticPointer, Word};

/// Lowered MIR program and execution metadata shared across isolates.
pub struct Program {
    /// The MIR tree executed by this program.
    pub(crate) tree: mir::Tree,
    /// The immutable string pool for this program.
    pub(crate) strings: ImmutableStringPool,
    /// Lowered function bodies for the current interpreter backend.
    pub(crate) functions: FunctionTable,
    /// Side table referenced by compact side records.
    pub(crate) side_table: SideTable,
    /// Lookup table for function ids by name.
    pub(crate) function_id_by_name: HashMap<String, mir::LocalNodeId<mir::Function>>,
    /// Layout metadata for types, heap allocation, and binding views.
    layout_index: LayoutIndex,
    /// Immutable program static data.
    pub(crate) statics: engine::StaticSpace,
    /// Callable heap object field layout.
    pub(crate) callable_object_layout: CallableObjectLayout,

    /// Logical frame layouts by dense layout id.
    pub(crate) frame_layouts: Vec<engine::FrameLayout>,
    /// Dense frame-state metadata.
    frame_states: FrameStateTable,
}

impl Program {
    /// Build one program from one MIR tree and immutable string pool.
    pub fn new(tree: mir::Tree, strings: ImmutableStringPool) -> Result<Self> {
        Self::with_heap_options(
            tree,
            strings,
            heap::HeapOptions::local(),
            heap::HeapOptions::shared(),
        )
    }

    /// Build one program for concrete heap allocation geometry.
    pub(crate) fn with_heap_options(
        tree: mir::Tree,
        strings: ImmutableStringPool,
        heap_options: heap::HeapOptions,
        shared_heap_options: heap::HeapOptions,
    ) -> Result<Self> {
        ProgramBuilder::new(tree, strings, heap_options, shared_heap_options).build()
    }

    /// Resolve one engine entry into its MIR function id.
    #[inline]
    pub(crate) fn function_for_entry(
        &self,
        entry: engine::Entry,
    ) -> mir::LocalNodeId<mir::Function> {
        mir::LocalNodeId::new(entry.index())
    }

    /// Convert one current frame location into one lowered program point.
    #[inline]
    pub(crate) fn point(
        &self,
        function: mir::LocalNodeId<mir::Function>,
        block: mir::LocalNodeId<mir::Block>,
        instruction_index: u32,
    ) -> ProgramPoint {
        ProgramPoint::new(function, block, instruction_index)
    }

    /// Convert one MIR type id into one program layout id.
    #[inline]
    pub(crate) fn layout_id(&self, ty: mir::LocalNodeId<mir::Type>) -> engine::LayoutId {
        LayoutIndex::engine_layout_id(ty)
    }

    /// Convert one program layout id into one MIR type id.
    #[inline]
    pub(crate) fn type_for_layout(&self, layout: engine::LayoutId) -> mir::LocalNodeId<mir::Type> {
        LayoutIndex::type_for_layout(layout)
    }

    /// Convert one MIR global id into one worker static id.
    #[inline]
    pub(crate) fn static_id(&self, global: mir::LocalNodeId<mir::Global>) -> engine::StaticId {
        engine::StaticId(global.id)
    }

    /// Return the frame layout for one function when present.
    pub(crate) fn frame_layout(
        &self,
        function: mir::LocalNodeId<mir::Function>,
    ) -> Option<&engine::FrameLayout> {
        let function = self.functions.function_for(function)?;

        self.frame_layout_by_id(function.frame_layout)
    }

    /// Return the frame layout for one layout id when present.
    pub(crate) fn frame_layout_by_id(
        &self,
        frame_layout: engine::FrameLayoutId,
    ) -> Option<&engine::FrameLayout> {
        self.frame_layouts.get(frame_layout.0 as usize)
    }

    /// Return the lowered program point for one frame state.
    pub(crate) fn point_for_frame_state(
        &self,
        frame_state: engine::FrameStateId,
    ) -> Option<ProgramPoint> {
        self.frame_states.get(frame_state).map(|state| state.point)
    }

    /// Return the entry data for one frame state.
    pub(crate) fn frame_entry(&self, frame_state: engine::FrameStateId) -> Option<&FrameEntry> {
        self.frame_states
            .get(frame_state)
            .and_then(|state| state.entry.as_ref())
    }

    /// Return the single-frame materialization for one frame state.
    pub(crate) fn frame_materialization(
        &self,
        frame_state: engine::FrameStateId,
    ) -> Option<&engine::FrameMaterialization> {
        self.frame_states
            .get(frame_state)
            .map(|state| &state.materialization)
    }

    /// Return the compiled layout for one MIR type.
    pub(crate) fn layout(&self, ty: mir::LocalNodeId<mir::Type>) -> Option<&Layout> {
        self.layout_index.layout(ty)
    }

    /// Return the compiled layout for one program layout id.
    pub(crate) fn layout_for_layout(&self, layout: engine::LayoutId) -> Option<&Layout> {
        self.layout_index.layout_for_layout(layout)
    }

    /// Encode one global initializer into its declared bytes.
    pub(crate) fn initializer_bytes(
        &self,
        initializer: &mir::GlobalInitializer,
        ty: mir::LocalNodeId<mir::Type>,
    ) -> Result<Vec<u8>> {
        initializer_bytes(
            &self.tree,
            self.layout_index.type_layouts(),
            initializer,
            ty,
        )
    }

    /// Return the MIR layouts for this program.
    pub(crate) fn layouts(&self) -> &LayoutTable {
        self.layout_index.table()
    }

    /// Return the heap allocation shape for one layout id.
    pub(crate) fn allocation_shape(
        &self,
        layout_id: LayoutId,
    ) -> Result<heap::AllocationShape<'_>> {
        self.layout_index.allocation_shape(layout_id)
    }

    /// Return the layout id for one MIR type.
    pub(crate) fn layout_id_for_type(&self, ty: mir::LocalNodeId<mir::Type>) -> Option<LayoutId> {
        self.layout_index.layout_id_for_type(ty)
    }

    /// Return the program static address for one global.
    pub(crate) fn static_pointer(
        &self,
        global: mir::LocalNodeId<mir::Global>,
    ) -> Option<StaticPointer> {
        self.statics.pointer(self.static_id(global))
    }

    /// Return whether one global is stored in program static space.
    pub(crate) fn contains_static(&self, global: mir::LocalNodeId<mir::Global>) -> bool {
        self.statics.region(self.static_id(global)).is_some()
    }

    /// Return one MIR type by display name.
    pub(crate) fn type_by_display_name(&self, name: &str) -> Option<mir::LocalNodeId<mir::Type>> {
        for (type_id, _) in self.tree.iter_nodes::<mir::Type>() {
            let Some(display_name) = self.tree.type_display_name(type_id) else {
                continue;
            };
            if self.strings.get(display_name) == name {
                return Some(type_id);
            }
        }

        None
    }

    /// Return one frame state id for one lowered program point.
    pub(crate) fn frame_state_at(&self, point: ProgramPoint) -> Option<engine::FrameStateId> {
        self.frame_states.state_at(point)
    }

    /// Return the caller return destination implied by one lowered program point.
    pub(crate) fn return_destination_at(&self, point: ProgramPoint) -> Result<Option<mir::Value>> {
        let Some(frame_state) = self.frame_states.state_at(point) else {
            return Ok(None);
        };
        let destination = self
            .frame_states
            .get(frame_state)
            .and_then(|state| state.return_destination);

        Ok(destination)
    }
}

impl fmt::Debug for Program {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Program")
            .field(
                "functions",
                &format!("<{} functions>", self.function_id_by_name.len()),
            )
            .field("frame_layouts", &self.frame_layouts.len())
            .field("frame_states", &self.frame_states.states.len())
            .field("statics", &self.statics.len())
            .finish_non_exhaustive()
    }
}

/// One initialized byte range inside a global payload.
#[derive(Clone, Copy, Debug)]
struct InitializerRange {
    /// The value type for this range.
    ty: mir::LocalNodeId<mir::Type>,
    /// The byte offset inside the payload.
    offset: usize,
    /// The byte width of this range.
    byte_len: usize,
}

/// Encode one static initializer into bytes.
fn initializer_bytes(
    tree: &mir::Tree,
    layouts: &HashMap<mir::LocalNodeId<mir::Type>, Layout>,
    initializer: &mir::GlobalInitializer,
    ty: mir::LocalNodeId<mir::Type>,
) -> Result<Vec<u8>> {
    let layout = layouts.get(&ty).ok_or_else(|| Error::TypeMismatch {
        expected: "compiled initializer layout".to_string(),
        actual: format!("{ty:?}"),
    })?;

    if layout.is_scalar() {
        return scalar_initializer_bytes(tree, initializer, ty, layout.byte_len);
    }

    match initializer {
        mir::GlobalInitializer::Zero => {
            validate_zero_initializer(tree, layouts, ty)?;

            Ok(vec![0; layout.byte_len])
        }
        mir::GlobalInitializer::Bytes(bytes) => {
            if bytes.len() != layout.byte_len {
                return Err(Error::TypeMismatch {
                    expected: format!("{} initializer bytes", layout.byte_len),
                    actual: format!("{} initializer bytes", bytes.len()),
                });
            }

            Ok(bytes.clone())
        }
        mir::GlobalInitializer::Aggregate(elements) => {
            payload_initializer_bytes(tree, layouts, elements, ty)
        }
        mir::GlobalInitializer::Scalar(_) | mir::GlobalInitializer::FunctionAddress(_) => {
            Err(Error::TypeMismatch {
                expected: "payload initializer".to_string(),
                actual: "scalar initializer".to_string(),
            })
        }
    }
}

/// Encode one scalar initializer into bytes.
fn scalar_initializer_bytes(
    tree: &mir::Tree,
    initializer: &mir::GlobalInitializer,
    ty: mir::LocalNodeId<mir::Type>,
    byte_len: usize,
) -> Result<Vec<u8>> {
    match initializer {
        mir::GlobalInitializer::Zero => {
            validate_zero_scalar_type(tree, ty)?;

            Ok(vec![0; byte_len])
        }
        mir::GlobalInitializer::Bytes(bytes) => {
            if bytes.len() != byte_len {
                return Err(Error::TypeMismatch {
                    expected: format!("{byte_len} initializer bytes"),
                    actual: format!("{} initializer bytes", bytes.len()),
                });
            }

            Ok(bytes.clone())
        }
        mir::GlobalInitializer::Scalar(constant) => {
            let bytes = constant_scalar_bytes(constant, byte_len)?;

            Ok(bytes)
        }
        mir::GlobalInitializer::FunctionAddress(function) => {
            let bytes = function_address_initializer_bytes(*function, byte_len)?;

            Ok(bytes)
        }
        mir::GlobalInitializer::Aggregate(_) => Err(Error::TypeMismatch {
            expected: "scalar initializer".to_string(),
            actual: format!("{ty:?}"),
        }),
    }
}

/// Encode one function address initializer as bytes.
fn function_address_initializer_bytes(
    function: mir::FunctionReference,
    byte_len: usize,
) -> Result<Vec<u8>> {
    if byte_len > Word::BYTE_LEN {
        return Err(Error::TypeMismatch {
            expected: "address-sized initializer".to_string(),
            actual: format!("{byte_len} byte initializer"),
        });
    }

    let Some(function) = function.function() else {
        return Err(Error::MissingRepresentation {
            context: "function address initializer".to_string(),
        });
    };
    let function = FunctionPointer::from_bits(function.id as usize);
    let raw = function.bits() as u64;
    let bytes = raw.to_le_bytes();

    Ok(bytes[..byte_len].to_vec())
}

/// Encode one scalar initializer as bytes.
fn constant_scalar_bytes(constant: &mir::Constant, byte_len: usize) -> Result<Vec<u8>> {
    let mut bytes = vec![0; byte_len];

    match constant {
        mir::Constant::Null => {}
        mir::Constant::Boolean { value } => {
            bytes[0] = u8::from(*value);
        }
        mir::Constant::Int {
            value,
            is_signed: true,
            ..
        } => {
            let source = value.to_le_bytes();
            let copied = source.len().min(byte_len);
            bytes[..copied].copy_from_slice(&source[..copied]);

            if *value < 0 && byte_len > source.len() {
                bytes[source.len()..].fill(0xff);
            }
        }
        mir::Constant::Int { value, .. } => {
            let source = (*value as u128).to_le_bytes();
            let copied = source.len().min(byte_len);
            bytes[..copied].copy_from_slice(&source[..copied]);
        }
        mir::Constant::UInt { value, .. } => {
            let source = value.to_le_bytes();
            let copied = source.len().min(byte_len);
            bytes[..copied].copy_from_slice(&source[..copied]);
        }
        mir::Constant::Float { bits, .. } => {
            let source = bits.to_le_bytes();
            let copied = source.len().min(byte_len);
            bytes[..copied].copy_from_slice(&source[..copied]);
        }
        mir::Constant::Char { value } => {
            let source = (*value as u32).to_le_bytes();
            let copied = source.len().min(byte_len);
            bytes[..copied].copy_from_slice(&source[..copied]);
        }
    }

    Ok(bytes)
}

/// Validate one zero initializer against the declared type.
fn validate_zero_initializer(
    tree: &mir::Tree,
    layouts: &HashMap<mir::LocalNodeId<mir::Type>, Layout>,
    ty: mir::LocalNodeId<mir::Type>,
) -> Result<()> {
    let layout = layouts.get(&ty).ok_or_else(|| Error::TypeMismatch {
        expected: "compiled initializer layout".to_string(),
        actual: format!("{ty:?}"),
    })?;

    if layout.is_scalar() {
        validate_zero_scalar_type(tree, ty)?;

        return Ok(());
    }

    for range in initializer_ranges(layouts, ty)? {
        validate_zero_initializer(tree, layouts, range.ty)?;
    }

    Ok(())
}

/// Validate whether one scalar type accepts a zero initializer.
fn validate_zero_scalar_type(tree: &mir::Tree, ty: mir::LocalNodeId<mir::Type>) -> Result<()> {
    let ty_node = tree.get(ty).clone();

    match ty_node {
        mir::Type::Void
        | mir::Type::Int { .. }
        | mir::Type::Isize
        | mir::Type::Usize
        | mir::Type::Float { .. }
        | mir::Type::Boolean => Ok(()),
        mir::Type::Reference {
            kind: _,
            address_space: _,
            mutability: _,
            is_nullable,
            ..
        } => {
            if !is_nullable {
                return Err(Error::UnsupportedZeroValue {
                    ty: format!("{ty_node:?}"),
                });
            }

            Ok(())
        }
        _ => Err(Error::UnsupportedZeroValue {
            ty: format!("{ty_node:?}"),
        }),
    }
}

/// Encode one payload initializer into bytes.
fn payload_initializer_bytes(
    tree: &mir::Tree,
    layouts: &HashMap<mir::LocalNodeId<mir::Type>, Layout>,
    elements: &[mir::GlobalInitializer],
    ty: mir::LocalNodeId<mir::Type>,
) -> Result<Vec<u8>> {
    let layout = layouts.get(&ty).ok_or_else(|| Error::TypeMismatch {
        expected: "compiled payload layout".to_string(),
        actual: format!("{ty:?}"),
    })?;
    let ranges = initializer_ranges(layouts, ty)?;
    if elements.len() != ranges.len() {
        return Err(Error::TypeMismatch {
            expected: format!("{} initializer elements", ranges.len()),
            actual: format!("{} initializer elements", elements.len()),
        });
    }

    let mut bytes = vec![0u8; layout.byte_len];
    for (element, range) in elements.iter().zip(ranges.into_iter()) {
        let value_bytes = initializer_bytes(tree, layouts, element, range.ty)?;
        if value_bytes.len() != range.byte_len {
            return Err(Error::TypeMismatch {
                expected: format!("{} initializer bytes", range.byte_len),
                actual: format!("{} initializer bytes", value_bytes.len()),
            });
        }

        let end = range
            .offset
            .checked_add(range.byte_len)
            .ok_or(Error::InvalidInstruction)?;
        let target = bytes
            .get_mut(range.offset..end)
            .ok_or(Error::InvalidInstruction)?;
        target.copy_from_slice(&value_bytes);
    }

    Ok(bytes)
}

/// Return initializer byte ranges for one payload type.
fn initializer_ranges(
    layouts: &HashMap<mir::LocalNodeId<mir::Type>, Layout>,
    ty: mir::LocalNodeId<mir::Type>,
) -> Result<Vec<InitializerRange>> {
    let layout = layouts.get(&ty).ok_or_else(|| Error::TypeMismatch {
        expected: "compiled payload layout".to_string(),
        actual: format!("{ty:?}"),
    })?;

    if let Some(field_count) = layout.field_count() {
        let mut ranges = Vec::with_capacity(field_count);
        for index in 0..field_count {
            let field = layout
                .field(index as u32)
                .ok_or(Error::InvalidInstruction)?;
            ranges.push(InitializerRange {
                ty: field.ty,
                offset: field.offset,
                byte_len: field.byte_len,
            });
        }

        return Ok(ranges);
    }

    let element = layout.element().ok_or_else(|| Error::TypeMismatch {
        expected: "indexed initializer layout".to_string(),
        actual: format!("{ty:?}"),
    })?;
    let element_count = layout.element_count().ok_or(Error::InvalidInstruction)?;
    let mut ranges = Vec::with_capacity(element_count);
    for index in 0..element_count {
        let offset = element
            .stride
            .checked_mul(index)
            .ok_or(Error::InvalidInstruction)?;
        ranges.push(InitializerRange {
            ty: element.ty,
            offset,
            byte_len: element.byte_len,
        });
    }

    Ok(ranges)
}

/// Build one program from one MIR tree and immutable string pool.
struct ProgramBuilder {
    heap_options: heap::HeapOptions,
    shared_heap_options: heap::HeapOptions,
    tree: mir::Tree,
    strings: ImmutableStringPool,
    frame_layouts: Vec<engine::FrameLayout>,
    frame_states: FrameStateTable,
}

impl ProgramBuilder {
    /// Create one program builder.
    fn new(
        tree: mir::Tree,
        strings: ImmutableStringPool,
        heap_options: heap::HeapOptions,
        shared_heap_options: heap::HeapOptions,
    ) -> Self {
        Self {
            heap_options,
            shared_heap_options,
            tree,
            strings,
            frame_layouts: Vec::new(),
            frame_states: FrameStateTable::default(),
        }
    }

    /// Build the program.
    fn build(mut self) -> Result<Program> {
        let function_id_by_name = self.build_function_id_by_name();
        let (function_ids, target_by_id) = self.build_function_targets();
        let type_layouts = build_layouts(&self.tree)?;
        let layout_id_by_type = self.build_layout_id_map(&type_layouts)?;
        let layouts = self.build_layout_table(&type_layouts, &layout_id_by_type)?;
        let callable_object_layout = callable_object_layout(self.tree.pointer_bytes() as usize);
        let layout_index = LayoutIndex::new(layouts, type_layouts, layout_id_by_type);
        let statics = self.build_statics(layout_index.type_layouts())?;
        let mut side_table = SideTableBuilder::default();
        let functions = self.build_functions(
            &function_ids,
            &target_by_id,
            layout_index.type_layouts(),
            layout_index.layout_ids(),
            &mut side_table,
        )?;
        let side_table = side_table.finish();
        let functions = FunctionTable::new(&self.tree, functions, target_by_id);

        Ok(Program {
            tree: self.tree,
            strings: self.strings,
            function_id_by_name,
            statics,
            callable_object_layout,
            layout_index,
            frame_layouts: self.frame_layouts,
            frame_states: self.frame_states,
            functions,
            side_table,
        })
    }

    /// Build the function name lookup table.
    fn build_function_id_by_name(&self) -> HashMap<String, mir::LocalNodeId<mir::Function>> {
        let mut map = HashMap::new();

        for (function_id, function) in self.tree.iter_nodes::<mir::Function>() {
            let name = self.strings.get(function.name).to_string();
            map.entry(name).or_insert(function_id);
        }

        map
    }

    /// Build immutable program static space.
    fn build_statics(
        &self,
        layouts: &HashMap<mir::LocalNodeId<mir::Type>, Layout>,
    ) -> Result<engine::StaticSpace> {
        let mut data = engine::StaticSpace::allocator();

        // immutable globals without heap edges can share program storage
        for (global_id, global) in self.tree.iter_nodes::<mir::Global>() {
            if data.contains(engine::StaticId(global_id.id))
                || global.is_import()
                || global.is_mutable()
            {
                continue;
            }

            let Some(ty) = global.ty.ty() else {
                return Err(Error::MissingRepresentation {
                    context: "global type".to_string(),
                });
            };
            let layout = layouts.get(&ty).ok_or_else(|| Error::TypeMismatch {
                expected: "compiled global layout".to_string(),
                actual: format!("{ty:?}"),
            })?;
            if layout.reference_map.has_reference() {
                continue;
            }

            let bytes = match global.initializer.as_ref() {
                Some(initializer) => initializer_bytes(&self.tree, layouts, initializer, ty)?,
                None => vec![0; layout.byte_len],
            };
            self.define_static_bytes(&mut data, global_id, ty, layout.alignment(), false, &bytes)?;
        }

        Ok(data.finish())
    }

    /// Define one byte region in program static memory.
    fn define_static_bytes(
        &self,
        data: &mut engine::StaticAllocator,
        global: mir::LocalNodeId<mir::Global>,
        ty: mir::LocalNodeId<mir::Type>,
        alignment: usize,
        is_mutable: bool,
        bytes: &[u8],
    ) -> Result<()> {
        let was_defined = data.define(
            engine::StaticId(global.id),
            engine::LayoutId(ty.id),
            alignment,
            is_mutable,
            bytes,
        );
        if !was_defined {
            return Err(Error::InvariantViolation {
                context: format!("duplicate program static global {global:?}"),
            });
        }

        Ok(())
    }

    /// Build the layout id map for all compiled MIR types.
    fn build_layout_id_map(
        &self,
        layouts: &HashMap<mir::LocalNodeId<mir::Type>, Layout>,
    ) -> Result<HashMap<mir::LocalNodeId<mir::Type>, LayoutId>> {
        let next_layout_id = layouts
            .keys()
            .filter_map(|type_id| self.tree.type_layout_id(*type_id))
            .map(|layout_id| layout_id.raw())
            .max()
            .unwrap_or(0);
        let mut next_layout_id = next_layout_id.saturating_add(1);
        let mut layout_id_by_type = HashMap::new();

        for type_id in layouts.keys().copied() {
            let layout_id = if let Some(layout_id) = self.tree.type_layout_id(type_id) {
                layout_id
            } else {
                let layout_id = LayoutId::new(next_layout_id);
                next_layout_id =
                    next_layout_id
                        .checked_add(1)
                        .ok_or_else(|| Error::InvariantViolation {
                            context: "program layout id space exhausted".to_string(),
                        })?;

                layout_id
            };

            layout_id_by_type.insert(type_id, layout_id);
        }

        Ok(layout_id_by_type)
    }

    /// Build the MIR layout table from the program layouts.
    fn build_layout_table(
        &self,
        layouts: &HashMap<mir::LocalNodeId<mir::Type>, Layout>,
        layout_id_by_type: &HashMap<mir::LocalNodeId<mir::Type>, LayoutId>,
    ) -> Result<LayoutTable> {
        let max_layout_id = layout_id_by_type
            .values()
            .map(|layout_id| layout_id.raw() as usize)
            .max()
            .unwrap_or(0);
        let mut table = LayoutTable::new();
        table.layouts.resize_with(max_layout_id, || mir::Layout {
            kind: LayoutKind::Struct,
            size: 0,
            alignment: 1,
            reference_map: ReferenceMap::empty(),
            fields: Vec::new(),
        });

        // program layouts
        for (type_id, layout_id) in layout_id_by_type {
            let layout = layouts
                .get(type_id)
                .ok_or_else(|| Error::InvariantViolation {
                    context: format!("missing program layout for heap type {type_id:?}"),
                })?;
            let module_layout = match self.tree.get(*type_id) {
                mir::Type::Callable { .. } => {
                    callable_object_layout(self.tree.pointer_bytes() as usize).table_layout()
                }
                _ => mir::Layout {
                    kind: LayoutKind::Struct,
                    size: layout.byte_len as u32,
                    alignment: layout.alignment() as u32,
                    reference_map: layout.reference_map.clone(),
                    fields: Vec::new(),
                },
            };
            let layout_index = layout_id.index();

            if layout_index >= table.layouts.len() {
                return Err(Error::InvariantViolation {
                    context: format!("layout id out of range: {layout_id:?}"),
                });
            }

            table.layouts[layout_index] = module_layout;
        }

        Ok(table)
    }

    /// Build the lowered function order and callable target map.
    fn build_function_targets(
        &self,
    ) -> (
        Vec<mir::LocalNodeId<mir::Function>>,
        HashMap<mir::LocalNodeId<mir::Function>, CallTarget>,
    ) {
        let mut function_ids = Vec::new();
        let mut target_by_id = HashMap::new();

        // collect imported and lowerable callables
        for (function_id, function) in self.tree.iter_nodes::<mir::Function>() {
            // binding functions stay as import targets
            if function.is_import() {
                target_by_id.insert(function_id, CallTarget::Import);
                continue;
            }

            // declarations without bodies are not call targets
            if function.entry.is_none() {
                continue;
            }

            function_ids.push(function_id);
        }

        // assign stable lowered indices in build order
        for (index, function_id) in function_ids.iter().enumerate() {
            target_by_id.insert(*function_id, CallTarget::Local(index as u32));
        }

        (function_ids, target_by_id)
    }

    /// Build the lowered functions and append their execution metadata.
    fn build_functions(
        &mut self,
        function_ids: &[mir::LocalNodeId<mir::Function>],
        target_by_id: &HashMap<mir::LocalNodeId<mir::Function>, CallTarget>,
        layouts: &HashMap<mir::LocalNodeId<mir::Type>, Layout>,
        layout_id_by_type: &HashMap<mir::LocalNodeId<mir::Type>, mir::LayoutId>,
        side_table: &mut SideTableBuilder,
    ) -> Result<Vec<Function>> {
        let call_targets = target_by_id.clone();

        let mut functions = Vec::with_capacity(function_ids.len());

        // build one lowered function at a time
        for function_id in function_ids {
            let function = self.build_function(
                *function_id,
                &call_targets,
                layouts,
                layout_id_by_type,
                side_table,
            )?;
            functions.push(function);
        }

        Ok(functions)
    }

    /// Build one lowered function and append its execution metadata.
    fn build_function(
        &mut self,
        function_id: mir::LocalNodeId<mir::Function>,
        call_targets: &HashMap<mir::LocalNodeId<mir::Function>, CallTarget>,
        layouts: &HashMap<mir::LocalNodeId<mir::Type>, Layout>,
        layout_id_by_type: &HashMap<mir::LocalNodeId<mir::Type>, mir::LayoutId>,
        side_table: &mut SideTableBuilder,
    ) -> Result<Function> {
        let function = self.tree.get(function_id);
        let value_types = analyze_value_types(function);

        // derive the logical frame shape before lowering
        let frame_layout = self.build_frame_layout(function, &value_types, layouts)?;
        let liveness = { mir::FunctionLiveness::build(function, &self.tree) };
        let (yield_frame_states, exceptional_call_frame_states) =
            self.build_frame_states(function_id, &frame_layout, &liveness)?;

        // lower the function with the preassigned yield resume ids
        let function = lower_function(
            &self.tree,
            function_id,
            &frame_layout,
            &yield_frame_states,
            &exceptional_call_frame_states,
            call_targets,
            layouts,
            layout_id_by_type,
            &self.heap_options,
            &self.shared_heap_options,
            &value_types,
            side_table,
        )?
        .ok_or_else(|| Error::MissingRepresentation {
            context: format!("program function {function_id:?}"),
        })?;

        // append the frame layout before assigning frame states
        self.frame_layouts.push(frame_layout.clone());

        // append states for every lowered instruction boundary
        for block in &function.blocks {
            for (instruction_index, source_boundary) in
                block.source_boundary_by_pc.iter().copied().enumerate()
            {
                let point =
                    ProgramPoint::new(function_id, block.mir_block, instruction_index as u32);
                let frame_state_id = self.frame_states.next_id();
                let return_destination =
                    self.return_destination(block.mir_block, source_boundary)?;

                self.append_frame_state(
                    &frame_layout,
                    &liveness,
                    block.mir_block,
                    point,
                    frame_state_id,
                    None,
                    return_destination,
                    Some(source_boundary),
                )?;
            }
        }

        Ok(function)
    }

    /// Return the call destination before one source instruction boundary.
    fn return_destination(
        &self,
        block: mir::LocalNodeId<mir::Block>,
        completed_instruction_count: u32,
    ) -> Result<Option<mir::Value>> {
        // block entry has no preceding call
        if completed_instruction_count == 0 {
            return Ok(None);
        }

        // inspect the source instruction immediately before this boundary
        let block = self.tree.get(block);
        let instruction_index = completed_instruction_count as usize - 1;
        let Some(instruction_id) = block.instructions.get(instruction_index).copied() else {
            return Ok(None);
        };
        let instruction = self.tree.get(instruction_id);

        match instruction {
            mir::Instruction::Call { destination, .. }
            | mir::Instruction::CallVirtual { destination, .. }
            | mir::Instruction::CallInterface { destination, .. }
            | mir::Instruction::CallIndirect { destination, .. } => (*destination)
                .map(|value| {
                    value.value().ok_or_else(|| Error::MissingRepresentation {
                        context: "call destination".to_string(),
                    })
                })
                .transpose(),
            _ => Ok(None),
        }
    }

    /// Build one byte frame layout for one function.
    fn build_frame_layout(
        &self,
        function: &mir::Function,
        value_types: &[ValueType],
        layouts: &HashMap<mir::LocalNodeId<mir::Type>, Layout>,
    ) -> Result<engine::FrameLayout> {
        let mut byte_len = 0usize;
        let mut slot_id = 0u32;

        let slot_count =
            value_types.len() + function.locals.len() + usize::from(function.environment.is_some());
        let mut slots = Vec::with_capacity(slot_count);

        for value_type in value_types {
            let slot = self.frame_slot(
                engine::FrameSlotId(slot_id),
                layouts,
                value_type.ty,
                &mut byte_len,
            )?;
            slot_id += 1;
            slots.push(slot);
        }

        let value_count = slot_id;
        for local_id in &function.locals {
            let local = self.tree.get(*local_id);
            let local_type = (local.ty)
                .ty()
                .ok_or_else(|| Error::MissingRepresentation {
                    context: "frame local type".to_string(),
                })?;
            let slot = self.frame_slot(
                engine::FrameSlotId(slot_id),
                layouts,
                local_type,
                &mut byte_len,
            )?;
            slot_id += 1;
            slots.push(slot);
        }

        let local_count = slot_id - value_count;
        let environment_slot = function
            .environment
            .map(|ty| {
                ty.ty().ok_or_else(|| Error::MissingRepresentation {
                    context: "environment".to_string(),
                })
            })
            .transpose()?
            .map(|environment| {
                self.frame_slot(
                    engine::FrameSlotId(slot_id),
                    layouts,
                    environment,
                    &mut byte_len,
                )
            })
            .transpose()?;
        let environment_slot = environment_slot.map(|slot| {
            let id = slot.id;
            slots.push(slot);

            id
        });

        Ok(engine::FrameLayout {
            id: engine::FrameLayoutId(self.frame_layouts.len() as u32),
            slots,
            value_count,
            local_count,
            environment_slot,
            byte_len: u32::try_from(byte_len).map_err(|_| Error::InvalidInstruction)?,
        })
    }

    /// Allocate one typed slot inside a frame layout.
    fn frame_slot(
        &self,
        id: engine::FrameSlotId,
        layouts: &HashMap<mir::LocalNodeId<mir::Type>, Layout>,
        ty: mir::LocalNodeId<mir::Type>,
        byte_len: &mut usize,
    ) -> Result<engine::FrameSlot> {
        let layout = layouts
            .get(&ty)
            .ok_or_else(|| Error::MissingRepresentation {
                context: "frame slot layout".to_string(),
            })?;
        let is_word = layout.is_word();
        let slot_alignment = if is_word {
            layout.alignment().max(Word::BYTE_LEN)
        } else {
            layout.alignment()
        };
        let slot_len = if is_word {
            layout.byte_len.max(Word::BYTE_LEN)
        } else {
            layout.byte_len
        };

        let offset = align_offset(*byte_len, slot_alignment);
        let end = offset + slot_len;
        *byte_len = end;

        Ok(engine::FrameSlot {
            id,
            offset: u32::try_from(offset).map_err(|_| Error::InvalidInstruction)?,
            byte_len: u32::try_from(slot_len).map_err(|_| Error::InvalidInstruction)?,
            alignment: u16::try_from(slot_alignment).map_err(|_| Error::InvalidInstruction)?,
            is_word,
            layout: engine::LayoutId(ty.id),
        })
    }

    /// Build the semantic resume lookups for one function.
    fn build_frame_states(
        &mut self,
        function_id: mir::LocalNodeId<mir::Function>,
        frame_layout: &engine::FrameLayout,
        liveness: &mir::FunctionLiveness,
    ) -> Result<(
        HashMap<mir::LocalNodeId<mir::Block>, engine::FrameStateId>,
        HashMap<mir::LocalNodeId<mir::Block>, (engine::FrameStateId, engine::FrameStateId)>,
    )> {
        let block_ids = self.tree.get(function_id).blocks.clone();
        let mut yield_frame_states = HashMap::new();
        let mut exceptional_call_frame_states = HashMap::new();

        // assign frame states to suspension and exceptional call edges
        for block_id in block_ids {
            let yield_edge = {
                let block = self.tree.get(block_id);
                let terminator = self.tree.get(block.terminator);

                match terminator {
                    mir::Terminator::Yield { resume, .. } => Some((
                        (resume.block)
                            .block()
                            .ok_or_else(|| Error::MissingRepresentation {
                                context: "yield resume target".to_string(),
                            })?,
                        resume
                            .arguments
                            .iter()
                            .map(|argument| {
                                (*argument)
                                    .value()
                                    .ok_or_else(|| Error::MissingRepresentation {
                                        context: "yield resume argument".to_string(),
                                    })
                            })
                            .collect::<Result<Vec<_>>>()?,
                    )),
                    _ => None,
                }
            };

            // yield resumes into one single continuation block
            if let Some((resume, resume_arguments)) = yield_edge {
                // yield resumes may bind one trailing resume value
                let received_value = self.received_value(resume, resume_arguments.len())?;
                let frame_state_id = self.append_entry_state(
                    function_id,
                    frame_layout,
                    liveness,
                    resume,
                    &resume_arguments,
                    received_value,
                )?;

                yield_frame_states.insert(block_id, frame_state_id);
                continue;
            }

            let exceptional_call_edge = {
                let block = self.tree.get(block_id);
                let terminator = self.tree.get(block.terminator);
                match terminator {
                    mir::Terminator::Invoke {
                        normal_target,
                        unwind_target,
                        ..
                    }
                    | mir::Terminator::InvokeIndirect {
                        normal_target,
                        unwind_target,
                        ..
                    }
                    | mir::Terminator::InvokeVirtual {
                        normal_target,
                        unwind_target,
                        ..
                    }
                    | mir::Terminator::InvokeInterface {
                        normal_target,
                        unwind_target,
                        ..
                    } => Some((
                        (normal_target.block).block().ok_or_else(|| {
                            Error::MissingRepresentation {
                                context: "invoke normal target".to_string(),
                            }
                        })?,
                        normal_target
                            .arguments
                            .iter()
                            .map(|argument| {
                                (*argument)
                                    .value()
                                    .ok_or_else(|| Error::MissingRepresentation {
                                        context: "invoke normal argument".to_string(),
                                    })
                            })
                            .collect::<Result<Vec<_>>>()?,
                        (unwind_target.block).block().ok_or_else(|| {
                            Error::MissingRepresentation {
                                context: "invoke unwind target".to_string(),
                            }
                        })?,
                        unwind_target
                            .arguments
                            .iter()
                            .map(|argument| {
                                (*argument)
                                    .value()
                                    .ok_or_else(|| Error::MissingRepresentation {
                                        context: "invoke unwind argument".to_string(),
                                    })
                            })
                            .collect::<Result<Vec<_>>>()?,
                    )),
                    _ => None,
                }
            };

            // exceptional call continuations branch to normal or unwind states
            if let Some((normal_target, normal_arguments, unwind_target, unwind_arguments)) =
                exceptional_call_edge
            {
                // normal and unwind edges may each bind one trailing implicit value
                let normal_received_value =
                    self.received_value(normal_target, normal_arguments.len())?;
                let normal_state_id = self.append_entry_state(
                    function_id,
                    frame_layout,
                    liveness,
                    normal_target,
                    &normal_arguments,
                    normal_received_value,
                )?;

                let unwind_received_value =
                    self.received_value(unwind_target, unwind_arguments.len())?;
                let unwind_state_id = self.append_entry_state(
                    function_id,
                    frame_layout,
                    liveness,
                    unwind_target,
                    &unwind_arguments,
                    unwind_received_value,
                )?;

                exceptional_call_frame_states.insert(block_id, (normal_state_id, unwind_state_id));
            }
        }

        Ok((yield_frame_states, exceptional_call_frame_states))
    }

    /// Return the trailing received value for one edge when present.
    fn received_value(
        &self,
        block: mir::LocalNodeId<mir::Block>,
        explicit_argument_count: usize,
    ) -> Result<Option<mir::Value>> {
        let entry_block = self.tree.get(block);

        // block edges bind the received value after explicit arguments
        if entry_block.parameters.len() == explicit_argument_count + 1 {
            return entry_block
                .parameters
                .last()
                .map(|parameter| {
                    (parameter.value)
                        .value()
                        .ok_or_else(|| Error::MissingRepresentation {
                            context: "resume parameter".to_string(),
                        })
                })
                .transpose();
        }

        Ok(None)
    }

    /// Append one frame state and its entry recipe.
    fn append_entry_state(
        &mut self,
        function_id: mir::LocalNodeId<mir::Function>,
        frame_layout: &engine::FrameLayout,
        liveness: &mir::FunctionLiveness,
        block: mir::LocalNodeId<mir::Block>,
        arguments: &[mir::Value],
        received_value: Option<mir::Value>,
    ) -> Result<engine::FrameStateId> {
        let entry_block = self.tree.get(block);
        let entry_parameters = if received_value.is_some() {
            &entry_block.parameters[..entry_block.parameters.len() - 1]
        } else {
            &entry_block.parameters[..]
        };
        let bindings = entry_parameters
            .iter()
            .zip(arguments.iter())
            .map(|(parameter, argument)| {
                let destination =
                    (parameter.value)
                        .value()
                        .ok_or_else(|| Error::MissingRepresentation {
                            context: "resume parameter".to_string(),
                        })?;

                let source = frame_layout
                    .value_slot_id(argument.0)
                    .ok_or(Error::InvalidInstruction)?;
                let destination = frame_layout
                    .value_slot_id(destination.0)
                    .ok_or(Error::InvalidInstruction)?;

                Ok(FrameBinding {
                    source,
                    destination,
                })
            })
            .collect::<Result<Vec<_>>>()?;

        let frame_entry = FrameEntry {
            bindings,
            received_value: received_value
                .map(|value| {
                    frame_layout
                        .value_slot_id(value.0)
                        .ok_or(Error::InvalidInstruction)
                })
                .transpose()?,
        };

        let frame_state_id = self.frame_states.next_id();
        let point = ProgramPoint::new(function_id, block, 0);

        self.append_frame_state(
            frame_layout,
            liveness,
            block,
            point,
            frame_state_id,
            Some(frame_entry),
            None,
            None,
        )?;

        Ok(frame_state_id)
    }

    /// Append one frame state and its attached metadata.
    fn append_frame_state(
        &mut self,
        frame_layout: &engine::FrameLayout,
        liveness: &mir::FunctionLiveness,
        block: mir::LocalNodeId<mir::Block>,
        point: ProgramPoint,
        frame_state: engine::FrameStateId,
        frame_entry: Option<FrameEntry>,
        return_destination: Option<mir::Value>,
        source_boundary: Option<u32>,
    ) -> Result<()> {
        let materialized_values = self.materialized_values(
            frame_layout,
            liveness,
            block,
            frame_entry.as_ref(),
            source_boundary,
        )?;
        let materialized_locals = self.materialized_locals(liveness, block);
        let sources = frame_layout
            .slot_ids()
            .filter_map(|slot| {
                if !self.is_materialized_slot(
                    frame_layout,
                    slot,
                    &materialized_values,
                    &materialized_locals,
                ) {
                    return None;
                }

                Some(engine::SlotSource {
                    slot,
                    source: engine::ValueSource::Slot(slot),
                })
            })
            .collect();

        let materialization = engine::FrameMaterialization {
            frame_layout: frame_layout.id,
            frame_state,
            sources,
        };
        self.frame_states.push(
            frame_state,
            FrameState {
                point,
                entry: frame_entry,
                return_destination,
                materialization,
            },
        );

        Ok(())
    }

    /// Return the values materialized at one frame state.
    fn materialized_values(
        &self,
        frame_layout: &engine::FrameLayout,
        liveness: &mir::FunctionLiveness,
        block: mir::LocalNodeId<mir::Block>,
        frame_entry: Option<&FrameEntry>,
        source_boundary: Option<u32>,
    ) -> Result<HashSet<mir::Value>> {
        let Some(frame_entry) = frame_entry else {
            let source_boundary = source_boundary.ok_or(Error::InvalidInstruction)?;

            return Ok(liveness.value_live_before_instruction(
                &self.tree,
                block,
                source_boundary as usize,
            ));
        };

        // materialize live in values that survive the entry edge
        let live_in = liveness.value_live_in(block);
        let entry_block = self.tree.get(block);
        let parameter_values: HashSet<mir::Value> = entry_block
            .parameters
            .iter()
            .map(|parameter| {
                (parameter.value)
                    .value()
                    .ok_or_else(|| Error::MissingRepresentation {
                        context: "entry block parameter".to_string(),
                    })
            })
            .collect::<Result<HashSet<_>>>()?;
        let mut values: HashSet<mir::Value> =
            live_in.difference(&parameter_values).copied().collect();

        // entry bindings keep their source values live
        for binding in &frame_entry.bindings {
            let source = frame_layout
                .value_for_slot(binding.source)
                .ok_or(Error::InvalidInstruction)?;
            let source = mir::Value::new(source);

            values.insert(source);
        }

        Ok(values)
    }

    /// Return the locals materialized at one frame state.
    fn materialized_locals(
        &self,
        liveness: &mir::FunctionLiveness,
        block: mir::LocalNodeId<mir::Block>,
    ) -> HashSet<mir::LocalNodeId<mir::Local>> {
        liveness.local_live_in(block).clone()
    }

    /// Return whether one frame slot is materialized at this frame state.
    fn is_materialized_slot(
        &self,
        layout: &engine::FrameLayout,
        slot: engine::FrameSlotId,
        materialized_values: &HashSet<mir::Value>,
        materialized_locals: &HashSet<mir::LocalNodeId<mir::Local>>,
    ) -> bool {
        if let Some(value) = layout.value_for_slot(slot) {
            return materialized_values.contains(&mir::Value::new(value));
        }

        if let Some(local) = layout.local_for_slot(slot) {
            return materialized_locals.contains(&mir::LocalNodeId::new(local));
        }

        layout.is_environment_slot(slot)
    }
}

/// Align one byte offset up to the requested byte alignment.
fn align_offset(offset: usize, alignment: usize) -> usize {
    if alignment <= 1 {
        return offset;
    }

    let remainder = offset % alignment;
    if remainder == 0 {
        return offset;
    }

    offset + alignment - remainder
}
