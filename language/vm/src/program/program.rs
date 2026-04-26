use std::collections::{HashMap, HashSet};
use std::fmt;

use destack_core::ImmutableStringPool;
use destack_mir::{LayoutId, LayoutKind, LayoutTable, ReferenceMap};
use {destack_engine as engine, destack_heap as heap, destack_mir as mir};

use super::layout::{Layout, build_layouts};
use super::{CallTarget, Function, FunctionTable};
use crate::lower::{ValueType, analyze_value_types, lower_function};
use crate::{Error, FunctionPointer, Result, StaticPointer, Word};

/// Align one byte offset up to the requested byte alignment.
fn align_offset(offset: usize, alignment: usize) -> Result<usize> {
    if alignment <= 1 {
        return Ok(offset);
    }

    let remainder = offset % alignment;
    if remainder == 0 {
        return Ok(offset);
    }

    offset
        .checked_add(alignment - remainder)
        .ok_or(Error::InvalidInstruction)
}

/// Lowered MIR program and execution metadata shared across isolates.
pub struct Program {
    /// The MIR tree executed by this program.
    pub(crate) tree: mir::NodeTree,
    /// The immutable string pool for this program.
    pub(crate) strings: ImmutableStringPool,
    /// Lowered function bodies for the current interpreter backend.
    pub(crate) functions: FunctionTable,
    /// Lookup table for function ids by name.
    pub(crate) function_id_by_name: HashMap<String, mir::LocalNodeId<mir::Function>>,
    /// MIR layouts keyed by layout id.
    pub(crate) layouts: LayoutTable,
    /// Compiled type layouts keyed by MIR type id.
    pub(crate) type_layouts: HashMap<mir::LocalNodeId<mir::Type>, Layout>,
    /// Layout ids keyed by MIR type id.
    pub(crate) layout_id_by_type: HashMap<mir::LocalNodeId<mir::Type>, LayoutId>,
    /// Immutable program static data.
    pub(crate) statics: engine::StaticSpace,

    /// Logical frame layouts by dense layout id.
    pub(crate) frame_layouts: Vec<engine::FrameLayout>,
    /// Logical frame layout id by owning function.
    pub(crate) frame_layout_id_by_function:
        HashMap<mir::LocalNodeId<mir::Function>, engine::FrameLayoutId>,

    /// Resume points by dense resume point id.
    pub(crate) resume_points: Vec<engine::ResumePoint>,
    /// Resume transfers by dense transfer id.
    pub(crate) resume_transfers: Vec<engine::ResumeTransfer>,
    /// Generic resume point ids keyed by function, block, and instruction offset.
    pub(crate) resume_point_id_by_position: HashMap<
        (
            mir::LocalNodeId<mir::Function>,
            mir::LocalNodeId<mir::Block>,
            u32,
        ),
        engine::ResumePointId,
    >,

    /// Safepoints by dense safepoint id.
    pub(crate) safepoints: Vec<engine::Safepoint>,
    /// Safepoint id keyed by semantic resume point.
    pub(crate) safepoint_id_by_resume_point: HashMap<engine::ResumePointId, engine::SafepointId>,
    /// Materialization maps by dense map id.
    pub(crate) materialization_maps: Vec<engine::MaterializationMap>,
}

impl Program {
    /// Build one program from one MIR tree and immutable string pool.
    pub fn new(tree: mir::NodeTree, strings: ImmutableStringPool) -> Result<Self> {
        ProgramBuilder::new(tree, strings).build()
    }

    /// Convert one program function id into one MIR function id.
    #[inline]
    pub(crate) fn function_for_id(
        &self,
        function: engine::FunctionId,
    ) -> mir::LocalNodeId<mir::Function> {
        let _ = self;

        mir::LocalNodeId::new(function.0)
    }

    /// Convert one program block id into one MIR block id.
    #[inline]
    pub(crate) fn block_for_id(&self, block: engine::BlockId) -> mir::LocalNodeId<mir::Block> {
        let _ = self;

        mir::LocalNodeId::new(block.0)
    }

    /// Convert one MIR type id into one program type id.
    #[inline]
    pub(crate) fn type_id(&self, ty: mir::LocalNodeId<mir::Type>) -> engine::TypeId {
        let _ = self;

        engine::TypeId(ty.id)
    }

    /// Convert one program type id into one MIR type id.
    #[inline]
    pub(crate) fn type_for_id(&self, ty: engine::TypeId) -> mir::LocalNodeId<mir::Type> {
        let _ = self;

        mir::LocalNodeId::new(ty.0)
    }

    /// Convert one MIR global id into one worker static id.
    #[inline]
    pub(crate) fn static_id(&self, global: mir::LocalNodeId<mir::Global>) -> engine::StaticId {
        let _ = self;

        engine::StaticId(global.id)
    }

    /// Return the frame layout for one function when present.
    pub(crate) fn frame_layout(
        &self,
        function: mir::LocalNodeId<mir::Function>,
    ) -> Option<&engine::FrameLayout> {
        let layout_id = self.frame_layout_id_by_function.get(&function)?;
        self.frame_layouts.get(layout_id.0 as usize)
    }

    /// Return the frame layout for one layout id when present.
    pub(crate) fn frame_layout_by_id(
        &self,
        frame_layout: engine::FrameLayoutId,
    ) -> Option<&engine::FrameLayout> {
        self.frame_layouts.get(frame_layout.0 as usize)
    }

    /// Return one resume point by id.
    pub(crate) fn resume_point(
        &self,
        resume_point: engine::ResumePointId,
    ) -> Option<&engine::ResumePoint> {
        self.resume_points.get(resume_point.0 as usize)
    }

    /// Return one resume transfer by id.
    pub(crate) fn resume_transfer(
        &self,
        resume_transfer: engine::ResumeTransferId,
    ) -> Option<&engine::ResumeTransfer> {
        self.resume_transfers.get(resume_transfer.0 as usize)
    }

    /// Return one safepoint by id.
    pub(crate) fn safepoint(&self, safepoint: engine::SafepointId) -> Option<&engine::Safepoint> {
        self.safepoints.get(safepoint.0 as usize)
    }

    /// Return one safepoint id for one semantic resume point.
    pub(crate) fn safepoint_for_resume_point(
        &self,
        resume_point: engine::ResumePointId,
    ) -> Option<engine::SafepointId> {
        self.safepoint_id_by_resume_point
            .get(&resume_point)
            .copied()
    }

    /// Return one materialization map by id.
    pub(crate) fn materialization_map(
        &self,
        materialization_map: engine::MaterializationMapId,
    ) -> Option<&engine::MaterializationMap> {
        self.materialization_maps
            .get(materialization_map.0 as usize)
    }

    /// Return the single-frame materialization recipe for one resume point.
    pub(crate) fn materialization_frame(
        &self,
        resume_point: engine::ResumePointId,
    ) -> Option<&engine::MaterializationFrame> {
        let safepoint = self.safepoint_for_resume_point(resume_point)?;
        let safepoint = self.safepoint(safepoint)?;
        let materialization_map = safepoint.materialization_map?;
        let materialization_map = self.materialization_map(materialization_map)?;

        if materialization_map.frames.len() != 1 {
            return None;
        }

        materialization_map.frames.first()
    }

    /// Return the compiled layout for one MIR type.
    pub(crate) fn layout(&self, ty: mir::LocalNodeId<mir::Type>) -> Option<&Layout> {
        self.type_layouts.get(&ty)
    }

    /// Return the compiled layout for one program type id.
    pub(crate) fn layout_for_id(&self, ty: engine::TypeId) -> Option<&Layout> {
        self.layout(self.type_for_id(ty))
    }

    /// Encode one global initializer into its declared bytes.
    pub(crate) fn initializer_bytes(
        &self,
        initializer: &mir::GlobalInitializer,
        ty: mir::LocalNodeId<mir::Type>,
    ) -> Result<Vec<u8>> {
        initializer_bytes(&self.tree, &self.type_layouts, initializer, ty)
    }

    /// Return the MIR layouts for this program.
    pub(crate) fn layouts(&self) -> &LayoutTable {
        &self.layouts
    }

    /// Return the heap allocation facts for one layout id.
    pub(crate) fn allocation_layout(
        &self,
        layout_id: LayoutId,
    ) -> Result<heap::AllocationLayout<'_>> {
        let Some(layout) = self.layouts.layouts.get(layout_id.index()) else {
            return Err(Error::InvariantViolation {
                context: format!("missing allocation layout {layout_id:?}"),
            });
        };

        Ok(heap::AllocationLayout::new(
            layout.size as usize,
            &layout.reference_map,
        ))
    }

    /// Return the layout id for one MIR type.
    pub(crate) fn layout_id_for_type(&self, ty: mir::LocalNodeId<mir::Type>) -> Option<LayoutId> {
        self.layout_id_by_type.get(&ty).copied()
    }

    /// Return the program static address for one global.
    pub(crate) fn static_pointer(
        &self,
        global: mir::LocalNodeId<mir::Global>,
    ) -> Option<StaticPointer> {
        self.statics.ptr(self.static_id(global))
    }

    /// Borrow program static bytes for one global.
    pub(crate) fn static_bytes(&self, global: mir::LocalNodeId<mir::Global>) -> Option<&[u8]> {
        self.statics.bytes(self.static_id(global))
    }

    /// Return whether one global is stored in program static space.
    pub(crate) fn contains_static(&self, global: mir::LocalNodeId<mir::Global>) -> bool {
        self.statics.region(self.static_id(global)).is_some()
    }

    /// Return whether one static byte range belongs to program static space.
    pub(crate) fn owns_static_range(&self, pointer: StaticPointer, byte_len: usize) -> bool {
        self.statics.owns_pointer_range(pointer, byte_len)
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

    /// Return one generic resume point id for one execution position.
    pub(crate) fn resume_point_for_position(
        &self,
        function: mir::LocalNodeId<mir::Function>,
        block: mir::LocalNodeId<mir::Block>,
        instruction_offset: u32,
    ) -> Option<engine::ResumePointId> {
        self.resume_point_id_by_position
            .get(&(function, block, instruction_offset))
            .copied()
    }

    /// Return the caller return destination implied by one resume point.
    pub(crate) fn return_destination_for_resume_point(
        &self,
        resume_point: engine::ResumePointId,
    ) -> Result<Option<mir::Value>> {
        let Some(resume_point) = self.resume_point(resume_point) else {
            return Ok(None);
        };

        // a resume at the start of a block has no preceding call
        if resume_point.source_instruction_offset == 0 {
            return Ok(None);
        }

        // resolve the preceding MIR instruction in the resumed block
        let block = self.tree.get(self.block_for_id(resume_point.block));
        let instruction_index = resume_point.source_instruction_offset as usize - 1;
        let Some(instruction_id) = block.instructions.get(instruction_index).copied() else {
            return Ok(None);
        };
        let instruction = self.tree.get(instruction_id);

        // recover the call destination when the resumed instruction follows a call
        match instruction {
            mir::Instruction::Call { destination, .. }
            | mir::Instruction::CallVirtual { destination, .. }
            | mir::Instruction::CallInterface { destination, .. }
            | mir::Instruction::CallIndirect { destination, .. } => (*destination)
                .map(|value| {
                    value.value().ok_or_else(|| Error::ConcreteMirRequired {
                        context: "call destination".to_string(),
                    })
                })
                .transpose(),
            _ => Ok(None),
        }
    }

    /// Return the caller return destination implied by one execution position.
    pub(crate) fn return_destination_for_position(
        &self,
        function: mir::LocalNodeId<mir::Function>,
        block: mir::LocalNodeId<mir::Block>,
        instruction_offset: u32,
    ) -> Result<Option<mir::Value>> {
        let Some(resume_point) =
            self.resume_point_for_position(function, block, instruction_offset)
        else {
            return Ok(None);
        };
        self.return_destination_for_resume_point(resume_point)
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
            .field("resume_points", &self.resume_points.len())
            .field("resume_transfers", &self.resume_transfers.len())
            .field("safepoints", &self.safepoints.len())
            .field("materialization_maps", &self.materialization_maps.len())
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
    tree: &mir::NodeTree,
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
        mir::GlobalInitializer::Scalar(_) => Err(Error::TypeMismatch {
            expected: "payload initializer".to_string(),
            actual: "scalar initializer".to_string(),
        }),
    }
}

/// Encode one scalar initializer into bytes.
fn scalar_initializer_bytes(
    tree: &mir::NodeTree,
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
            if byte_len > Word::BYTE_LEN {
                return Err(Error::TypeMismatch {
                    expected: format!("at most {} scalar bytes", Word::BYTE_LEN),
                    actual: format!("{byte_len} scalar bytes"),
                });
            }

            let value = Word::from(constant);
            let bytes = value.to_byte_array();

            Ok(bytes[..byte_len].to_vec())
        }
        mir::GlobalInitializer::Aggregate(_) => Err(Error::TypeMismatch {
            expected: "scalar initializer".to_string(),
            actual: format!("{ty:?}"),
        }),
    }
}

/// Validate one zero initializer against the declared type.
fn validate_zero_initializer(
    tree: &mir::NodeTree,
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
fn validate_zero_scalar_type(tree: &mir::NodeTree, ty: mir::LocalNodeId<mir::Type>) -> Result<()> {
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
    tree: &mir::NodeTree,
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
    tree: mir::NodeTree,
    strings: ImmutableStringPool,
    frame_layouts: Vec<engine::FrameLayout>,
    frame_layout_id_by_function: HashMap<mir::LocalNodeId<mir::Function>, engine::FrameLayoutId>,
    resume_points: Vec<engine::ResumePoint>,
    resume_transfers: Vec<engine::ResumeTransfer>,
    safepoints: Vec<engine::Safepoint>,
    safepoint_id_by_resume_point: HashMap<engine::ResumePointId, engine::SafepointId>,
    materialization_maps: Vec<engine::MaterializationMap>,
    resume_point_id_by_position: HashMap<
        (
            mir::LocalNodeId<mir::Function>,
            mir::LocalNodeId<mir::Block>,
            u32,
        ),
        engine::ResumePointId,
    >,
}

impl ProgramBuilder {
    /// Create one program builder.
    fn new(tree: mir::NodeTree, strings: ImmutableStringPool) -> Self {
        Self {
            tree,
            strings,
            frame_layouts: Vec::new(),
            frame_layout_id_by_function: HashMap::new(),
            resume_points: Vec::new(),
            resume_transfers: Vec::new(),
            safepoints: Vec::new(),
            safepoint_id_by_resume_point: HashMap::new(),
            materialization_maps: Vec::new(),
            resume_point_id_by_position: HashMap::new(),
        }
    }

    /// Build the program.
    fn build(mut self) -> Result<Program> {
        let function_id_by_name = self.build_function_id_by_name();
        let (function_ids, target_by_id) = self.build_function_targets();
        let type_layouts = build_layouts(&self.tree)?;
        let layout_id_by_type = self.build_layout_id_map(&type_layouts)?;
        let layouts = self.build_layout_table(&type_layouts, &layout_id_by_type)?;
        let statics = self.build_statics(&type_layouts)?;
        let functions = self.build_functions(
            &function_ids,
            &target_by_id,
            &type_layouts,
            &layout_id_by_type,
        )?;
        let functions = FunctionTable::new(functions, target_by_id);

        Ok(Program {
            tree: self.tree,
            strings: self.strings,
            function_id_by_name,
            statics,
            layouts,
            layout_id_by_type,
            frame_layouts: self.frame_layouts,
            frame_layout_id_by_function: self.frame_layout_id_by_function,
            resume_points: self.resume_points,
            resume_transfers: self.resume_transfers,
            safepoints: self.safepoints,
            safepoint_id_by_resume_point: self.safepoint_id_by_resume_point,
            materialization_maps: self.materialization_maps,
            type_layouts,
            resume_point_id_by_position: self.resume_point_id_by_position,
            functions,
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

        // dispatch tables are immutable program statics
        for (_table_id, table) in self.tree.metadata.dispatch.iter_vtables() {
            let mir::VtableStorage::Global(global) = table.storage;
            let words = table
                .entries
                .iter()
                .map(|entry| match entry {
                    mir::VtableEntry::Method { function }
                    | mir::VtableEntry::Destructor {
                        function: Some(function),
                    } => Word::function_pointer(FunctionPointer::from_bits(function.id as usize)),
                    mir::VtableEntry::TypeDescriptor
                    | mir::VtableEntry::Destructor { function: None } => Word::VOID,
                })
                .collect::<Vec<_>>();
            let Some(ty) = self.tree.get(global).ty.ty() else {
                return Err(Error::ConcreteMirRequired {
                    context: "dispatch table global type".to_string(),
                });
            };
            self.define_static_words(&mut data, global, ty, &words)?;
        }

        // immutable globals without heap edges can share program storage
        for (global_id, global) in self.tree.iter_nodes::<mir::Global>() {
            if data.contains(engine::StaticId(global_id.id))
                || global.is_import()
                || global.is_mutable()
            {
                continue;
            }

            let Some(ty) = global.ty.ty() else {
                return Err(Error::ConcreteMirRequired {
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
        data.define(
            engine::StaticId(global.id),
            engine::TypeId(ty.id),
            alignment,
            is_mutable,
            bytes,
        )
        .ok_or_else(|| Error::InvariantViolation {
            context: format!("duplicate program static global {global:?}"),
        })
    }

    /// Define one word region in program static memory.
    fn define_static_words(
        &self,
        data: &mut engine::StaticAllocator,
        global: mir::LocalNodeId<mir::Global>,
        ty: mir::LocalNodeId<mir::Type>,
        words: &[Word],
    ) -> Result<()> {
        let mut bytes = Vec::with_capacity(words.len() * Word::BYTE_LEN);
        for word in words {
            bytes.extend_from_slice(&word.to_byte_array());
        }

        self.define_static_bytes(data, global, ty, Word::BYTE_LEN, false, &bytes)
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

    /// Build the MIR layout table from the program type layouts.
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

        // program types
        for (type_id, layout_id) in layout_id_by_type {
            let layout = layouts
                .get(type_id)
                .ok_or_else(|| Error::InvariantViolation {
                    context: format!("missing program layout for heap type {type_id:?}"),
                })?;
            let module_layout = match self.tree.get(*type_id) {
                mir::Type::Callable { .. } => self.build_callable_layout(),
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

    /// Build the layout for one boxed callable payload.
    fn build_callable_layout(&self) -> mir::Layout {
        let pointer_bytes = self.tree.pointer_bytes();

        mir::Layout {
            kind: LayoutKind::Callable,
            size: (pointer_bytes * 2) as u32,
            alignment: pointer_bytes as u32,
            reference_map: ReferenceMap::Reference {
                local_offsets: vec![pointer_bytes as u32].into_boxed_slice(),
                shared_offsets: Vec::new().into_boxed_slice(),
            },
            fields: Vec::new(),
        }
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
            // imported functions stay as import targets
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
    ) -> Result<Vec<Function>> {
        let call_targets = target_by_id.clone();

        let mut functions = Vec::with_capacity(function_ids.len());

        // build one lowered function at a time
        for function_id in function_ids {
            let function =
                self.build_function(*function_id, &call_targets, layouts, layout_id_by_type)?;
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
    ) -> Result<Function> {
        let function = self.tree.get(function_id);
        let value_types = analyze_value_types(function);

        // derive the logical frame shape before lowering
        let frame_layout = self.build_frame_layout(function_id, function, &value_types, layouts)?;
        let liveness = { mir::FunctionLiveness::build(function, &self.tree) };
        let (yield_resume_points, exceptional_call_resume_points) =
            self.build_resume_points(function_id, &frame_layout, &liveness)?;

        // lower the function with the preassigned yield resume ids
        let function = lower_function(
            &self.tree,
            function_id,
            frame_layout.id,
            &yield_resume_points,
            &exceptional_call_resume_points,
            call_targets,
            layouts,
            layout_id_by_type,
            &value_types,
        )?
        .ok_or_else(|| Error::ConcreteMirRequired {
            context: format!("program function {function_id:?}"),
        })?;

        // append the frame shape and yield resume transfers first
        self.frame_layout_id_by_function
            .insert(function_id, frame_layout.id);
        self.frame_layouts.push(frame_layout.clone());

        // append the generic lowered pc resume points
        for block in &function.blocks {
            for (instruction_offset, source_instruction_offset) in
                block.mir_instruction_offsets.iter().copied().enumerate()
            {
                let resume_point_id = engine::ResumePointId(self.resume_points.len() as u32);
                let resume_point = engine::ResumePoint {
                    id: resume_point_id,
                    function: engine::FunctionId(function_id.id),
                    frame_layout: frame_layout.id,
                    block: engine::BlockId(block.mir_block.id),
                    instruction_offset: instruction_offset as u32,
                    source_instruction_offset,
                    transfer: None,
                };

                self.resume_point_id_by_position.insert(
                    (function_id, block.mir_block, instruction_offset as u32),
                    resume_point_id,
                );
                self.append_resume_point(&frame_layout, &liveness, &resume_point)?;
            }
        }

        Ok(function)
    }

    /// Build one byte frame layout for one function.
    fn build_frame_layout(
        &self,
        function_id: mir::LocalNodeId<mir::Function>,
        function: &mir::Function,
        value_types: &[ValueType],
        layouts: &HashMap<mir::LocalNodeId<mir::Type>, Layout>,
    ) -> Result<engine::FrameLayout> {
        let mut byte_len = 0usize;
        let mut region_id = 0u32;

        let mut values = Vec::with_capacity(value_types.len());
        for value_type in value_types {
            let region = self.frame_region(
                engine::FrameRegionId(region_id),
                layouts,
                value_type.ty,
                &mut byte_len,
            )?;
            region_id = region_id.saturating_add(1);
            values.push(region);
        }

        let mut locals = Vec::with_capacity(function.locals.len());
        for local_id in &function.locals {
            let local = self.tree.get(*local_id);
            let local_type = (local.ty).ty().ok_or_else(|| Error::ConcreteMirRequired {
                context: "frame local type".to_string(),
            })?;
            let region = self.frame_region(
                engine::FrameRegionId(region_id),
                layouts,
                local_type,
                &mut byte_len,
            )?;
            region_id = region_id.saturating_add(1);
            locals.push(region);
        }

        let environment = (function.environment)
            .map(|ty| {
                ty.ty().ok_or_else(|| Error::ConcreteMirRequired {
                    context: "environment".to_string(),
                })
            })
            .transpose()?
            .map(|environment| {
                self.frame_region(
                    engine::FrameRegionId(region_id),
                    layouts,
                    environment,
                    &mut byte_len,
                )
            })
            .transpose()?;

        Ok(engine::FrameLayout {
            id: engine::FrameLayoutId(self.frame_layouts.len() as u32),
            function: engine::FunctionId(function_id.id),
            values,
            locals,
            environment,
            byte_len: u32::try_from(byte_len).map_err(|_| Error::InvalidInstruction)?,
        })
    }

    /// Allocate one typed region inside a frame layout.
    fn frame_region(
        &self,
        id: engine::FrameRegionId,
        layouts: &HashMap<mir::LocalNodeId<mir::Type>, Layout>,
        ty: mir::LocalNodeId<mir::Type>,
        byte_len: &mut usize,
    ) -> Result<engine::FrameRegion> {
        let layout = layouts.get(&ty).ok_or_else(|| Error::ConcreteMirRequired {
            context: "frame region layout".to_string(),
        })?;
        let region_alignment = if layout.is_scalar() {
            layout.alignment().max(Word::BYTE_LEN)
        } else {
            layout.alignment()
        };
        let region_len = if layout.is_scalar() {
            layout.byte_len.max(Word::BYTE_LEN)
        } else {
            layout.byte_len
        };

        let offset = align_offset(*byte_len, region_alignment)?;
        let end = offset
            .checked_add(region_len)
            .ok_or(Error::InvalidInstruction)?;
        *byte_len = end;

        Ok(engine::FrameRegion {
            id,
            offset: u32::try_from(offset).map_err(|_| Error::InvalidInstruction)?,
            byte_len: u32::try_from(region_len).map_err(|_| Error::InvalidInstruction)?,
            alignment: u16::try_from(region_alignment).map_err(|_| Error::InvalidInstruction)?,
            is_word: layout.is_scalar(),
            ty: engine::TypeId(ty.id),
        })
    }

    /// Build the semantic resume lookups for one function.
    fn build_resume_points(
        &mut self,
        function_id: mir::LocalNodeId<mir::Function>,
        frame_layout: &engine::FrameLayout,
        liveness: &mir::FunctionLiveness,
    ) -> Result<(
        HashMap<mir::LocalNodeId<mir::Block>, engine::ResumePointId>,
        HashMap<mir::LocalNodeId<mir::Block>, (engine::ResumePointId, engine::ResumePointId)>,
    )> {
        let block_ids = self.tree.get(function_id).blocks.clone();
        let mut yield_resume_points = HashMap::new();
        let mut exceptional_call_resume_points = HashMap::new();

        // assign semantic resume points to suspension and exceptional call edges
        for block_id in block_ids {
            let yield_edge = {
                let block = self.tree.get(block_id);
                let terminator = self.tree.get(block.terminator);

                match terminator {
                    mir::Terminator::Yield { resume, .. } => Some((
                        (resume.block)
                            .block()
                            .ok_or_else(|| Error::ConcreteMirRequired {
                                context: "yield resume target".to_string(),
                            })?,
                        resume
                            .arguments
                            .iter()
                            .map(|argument| {
                                (*argument)
                                    .value()
                                    .ok_or_else(|| Error::ConcreteMirRequired {
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
                let resume_value = self.infer_resume_value(resume, resume_arguments.len())?;
                let resume_point_id = self.append_resume_entry(
                    function_id,
                    frame_layout,
                    liveness,
                    resume,
                    &resume_arguments,
                    resume_value,
                )?;

                yield_resume_points.insert(block_id, resume_point_id);
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
                            Error::ConcreteMirRequired {
                                context: "invoke normal target".to_string(),
                            }
                        })?,
                        normal_target
                            .arguments
                            .iter()
                            .map(|argument| {
                                (*argument)
                                    .value()
                                    .ok_or_else(|| Error::ConcreteMirRequired {
                                        context: "invoke normal argument".to_string(),
                                    })
                            })
                            .collect::<Result<Vec<_>>>()?,
                        (unwind_target.block).block().ok_or_else(|| {
                            Error::ConcreteMirRequired {
                                context: "invoke unwind target".to_string(),
                            }
                        })?,
                        unwind_target
                            .arguments
                            .iter()
                            .map(|argument| {
                                (*argument)
                                    .value()
                                    .ok_or_else(|| Error::ConcreteMirRequired {
                                        context: "invoke unwind argument".to_string(),
                                    })
                            })
                            .collect::<Result<Vec<_>>>()?,
                    )),
                    _ => None,
                }
            };

            // exceptional call continuations branch to one normal or unwind resume point
            if let Some((normal_target, normal_arguments, unwind_target, unwind_arguments)) =
                exceptional_call_edge
            {
                // normal and unwind edges may each bind one trailing implicit value
                let normal_resume_value =
                    self.infer_resume_value(normal_target, normal_arguments.len())?;
                let normal_resume_point_id = self.append_resume_entry(
                    function_id,
                    frame_layout,
                    liveness,
                    normal_target,
                    &normal_arguments,
                    normal_resume_value,
                )?;

                let unwind_resume_value =
                    self.infer_resume_value(unwind_target, unwind_arguments.len())?;
                let unwind_resume_point_id = self.append_resume_entry(
                    function_id,
                    frame_layout,
                    liveness,
                    unwind_target,
                    &unwind_arguments,
                    unwind_resume_value,
                )?;

                exceptional_call_resume_points
                    .insert(block_id, (normal_resume_point_id, unwind_resume_point_id));
            }
        }

        Ok((yield_resume_points, exceptional_call_resume_points))
    }

    /// Return the trailing implicit resume value for one edge when present.
    fn infer_resume_value(
        &self,
        block: mir::LocalNodeId<mir::Block>,
        explicit_argument_count: usize,
    ) -> Result<Option<mir::Value>> {
        let resume_block = self.tree.get(block);

        // block edges bind the implicit transferred value after explicit arguments
        if resume_block.parameters.len() == explicit_argument_count + 1 {
            return resume_block
                .parameters
                .last()
                .map(|parameter| {
                    (parameter.value)
                        .value()
                        .ok_or_else(|| Error::ConcreteMirRequired {
                            context: "resume parameter".to_string(),
                        })
                })
                .transpose();
        }

        Ok(None)
    }

    /// Append one semantic resume point and transfer for one block entry.
    fn append_resume_entry(
        &mut self,
        function_id: mir::LocalNodeId<mir::Function>,
        frame_layout: &engine::FrameLayout,
        liveness: &mir::FunctionLiveness,
        block: mir::LocalNodeId<mir::Block>,
        arguments: &[mir::Value],
        resume_value: Option<mir::Value>,
    ) -> Result<engine::ResumePointId> {
        let resume_block = self.tree.get(block);
        let copy_parameters = if resume_value.is_some() {
            &resume_block.parameters[..resume_block.parameters.len() - 1]
        } else {
            &resume_block.parameters[..]
        };
        let copies = copy_parameters
            .iter()
            .zip(arguments.iter())
            .map(|(parameter, argument)| {
                let destination =
                    (parameter.value)
                        .value()
                        .ok_or_else(|| Error::ConcreteMirRequired {
                            context: "resume parameter".to_string(),
                        })?;

                let source = frame_layout
                    .value_region_id(engine::ValueId(argument.0))
                    .ok_or(Error::InvalidInstruction)?;
                let destination = frame_layout
                    .value_region_id(engine::ValueId(destination.0))
                    .ok_or(Error::InvalidInstruction)?;

                Ok(engine::ResumeCopy {
                    source,
                    destination,
                })
            })
            .collect::<Result<Vec<_>>>()?;

        let transfer_id = engine::ResumeTransferId(self.resume_transfers.len() as u32);
        self.resume_transfers.push(engine::ResumeTransfer {
            id: transfer_id,
            copies,
            resume_value: resume_value
                .map(|value| {
                    frame_layout
                        .value_region_id(engine::ValueId(value.0))
                        .ok_or(Error::InvalidInstruction)
                })
                .transpose()?,
        });

        let resume_point_id = engine::ResumePointId(self.resume_points.len() as u32);
        let resume_point = engine::ResumePoint {
            id: resume_point_id,
            function: engine::FunctionId(function_id.id),
            frame_layout: frame_layout.id,
            block: engine::BlockId(block.id),
            instruction_offset: 0,
            source_instruction_offset: 0,
            transfer: Some(transfer_id),
        };

        self.append_resume_point(frame_layout, liveness, &resume_point)?;

        Ok(resume_point_id)
    }

    /// Append one semantic resume point and its attached metadata.
    fn append_resume_point(
        &mut self,
        frame_layout: &engine::FrameLayout,
        liveness: &mir::FunctionLiveness,
        resume_point: &engine::ResumePoint,
    ) -> Result<()> {
        let safepoint_id = engine::SafepointId(self.safepoints.len() as u32);
        let materialization_map_id =
            engine::MaterializationMapId(self.materialization_maps.len() as u32);
        let materialized_values = self.materialized_values(frame_layout, liveness, resume_point)?;
        let materialized_locals = self.materialized_locals(liveness, resume_point);
        let regions = frame_layout
            .region_ids()
            .map(|region| {
                if self.is_materialized_region(
                    frame_layout,
                    region,
                    &materialized_values,
                    &materialized_locals,
                ) {
                    return engine::MaterializationValue::FrameRegion(region);
                }

                engine::MaterializationValue::Undefined
            })
            .collect();

        self.materialization_maps.push(engine::MaterializationMap {
            id: materialization_map_id,
            safepoint: safepoint_id,
            frames: vec![engine::MaterializationFrame {
                frame_layout: frame_layout.id,
                resume_point: resume_point.id,
                regions,
            }],
        });

        self.safepoints.push(engine::Safepoint {
            id: safepoint_id,
            function: resume_point.function,
            frame_layout: frame_layout.id,
            resume_point: resume_point.id,
            stack_map: None,
            materialization_map: Some(materialization_map_id),
        });
        self.safepoint_id_by_resume_point
            .insert(resume_point.id, safepoint_id);
        self.resume_points.push(resume_point.clone());

        Ok(())
    }

    /// Return the values materialized at one resume point.
    fn materialized_values(
        &self,
        frame_layout: &engine::FrameLayout,
        liveness: &mir::FunctionLiveness,
        resume_point: &engine::ResumePoint,
    ) -> Result<HashSet<mir::Value>> {
        let Some(resume_transfer) = resume_point
            .transfer
            .and_then(|resume_transfer| self.resume_transfers.get(resume_transfer.0 as usize))
        else {
            let block = mir::LocalNodeId::new(resume_point.block.0);

            return Ok(liveness.value_live_before_instruction(
                &self.tree,
                block,
                resume_point.source_instruction_offset as usize,
            ));
        };

        // materialize live in values that survive the resume edge
        let block = mir::LocalNodeId::new(resume_point.block.0);
        let live_in = liveness.value_live_in(block);
        let resume_block = self.tree.get(block);
        let parameter_values: HashSet<mir::Value> = resume_block
            .parameters
            .iter()
            .map(|parameter| {
                (parameter.value)
                    .value()
                    .ok_or_else(|| Error::ConcreteMirRequired {
                        context: "resume block parameter".to_string(),
                    })
            })
            .collect::<Result<HashSet<_>>>()?;
        let mut values: HashSet<mir::Value> =
            live_in.difference(&parameter_values).copied().collect();

        for copy in &resume_transfer.copies {
            let destination = frame_layout
                .value_for_region(copy.destination)
                .ok_or(Error::InvalidInstruction)?;
            let source = frame_layout
                .value_for_region(copy.source)
                .ok_or(Error::InvalidInstruction)?;
            let destination = mir::Value::new(destination.0);
            let source = mir::Value::new(source.0);

            if live_in.contains(&destination) {
                values.insert(source);
            }
        }

        Ok(values)
    }

    /// Return the locals materialized at one resume point.
    fn materialized_locals(
        &self,
        liveness: &mir::FunctionLiveness,
        resume_point: &engine::ResumePoint,
    ) -> HashSet<mir::LocalNodeId<mir::Local>> {
        let block = mir::LocalNodeId::new(resume_point.block.0);

        liveness.local_live_in(block).clone()
    }

    /// Return whether one frame region is materialized at this resume point.
    fn is_materialized_region(
        &self,
        layout: &engine::FrameLayout,
        region: engine::FrameRegionId,
        materialized_values: &HashSet<mir::Value>,
        materialized_locals: &HashSet<mir::LocalNodeId<mir::Local>>,
    ) -> bool {
        if let Some(value) = layout.value_for_region(region) {
            return materialized_values.contains(&mir::Value::new(value.0));
        }

        if let Some(local) = layout.local_for_region(region) {
            return materialized_locals.contains(&mir::LocalNodeId::new(local.0));
        }

        layout.is_environment_region(region)
    }
}
