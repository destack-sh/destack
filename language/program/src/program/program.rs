use std::sync::Arc;

use destack_bytecode as bytecode;
use destack_bytecode::Word;
use destack_core::{EntryRange, SectionImage, SectionStorage, StringId};
use destack_heap::{
    AllocationShape, DropId, HeapResult, ReferenceRange, RootSlot, TraceTable, TraceView,
    visit_heap_root_slots,
};
use destack_memory::{MemoryMap, MemoryResult};
use destack_mir::{TargetLayout, TraceId, TraceMap};
use destack_serde::Reflect;
use destack_source::ContentId;
use serde::{Deserialize, Serialize};

use crate::{
    BindingId, Continuation, DispatchTable, DropEntry, DropTable, DynamicEntry, DynamicTable,
    DynamicTableId, FrameLayout, FrameLayoutId, FrameSlot, FrameSlotId, FrameState, FrameStateId,
    FrameTable, Function, FunctionId, FunctionTable, Global, GlobalAddress, GlobalId,
    GlobalLocation, GlobalTable, Layout, LayoutField, LayoutId, LayoutShape, LayoutTable,
    ProgramInfo, ProgramPoint, SampleKey, SampleSite, SampleValue, ScalarFormat, Signature,
    SignatureEntry, SignatureId, SiteTable, StaticImage, StaticSpace, StringTable, TensorDimension,
    TensorLayout, TensorViewLayout, TypeId, TypeTable, Value, VariantCaseLayout, VariantLayout,
    VirtualTable, VirtualTableId, WordLayout, native, wasm,
};

use super::{Error, Result};

/// Linked program.
#[derive(Debug, Serialize, Deserialize, Reflect)]
#[reflect(module = "destack_program::program")]
pub struct Program {
    /// Target ABI layout used by program layouts and pointer-sized integer types.
    pub(crate) target_layout: TargetLayout,

    /// Program string table.
    pub(crate) strings: StringTable,
    /// Runtime type table.
    pub(crate) types: TypeTable,
    /// Destructors keyed by drop id.
    pub(crate) drops: DropTable,
    /// Runtime layouts keyed by layout id.
    pub(crate) layouts: LayoutTable,
    /// Runtime frame table.
    pub(crate) frames: FrameTable,
    /// Program function table.
    pub(crate) functions: FunctionTable,
    /// Runtime dispatch table.
    pub(crate) dispatch: DispatchTable,
    /// Program sites used by debugging, probes, and observations.
    pub(crate) sites: SiteTable,
    /// Canonical trace table used by heap tables.
    pub(crate) traces: TraceTable,
    /// Program globals keyed by dense global id.
    pub(crate) globals: GlobalTable,
    /// Optional source reflection table.
    pub(crate) info: Option<ProgramInfo>,

    /// Immutable constant storage owned by this program.
    pub(crate) constant_space: StaticImage,
    /// Initial shared static storage for each runtime.
    pub(crate) shared_static_space: StaticImage,
    /// Initial local static storage for each worker.
    pub(crate) local_static_space: StaticImage,

    /// Bytecode used for interpretation, deoptimization, and continuation resume.
    pub(crate) bytecode: bytecode::Code,
    /// Native code when generated for this program.
    pub(crate) native: Option<native::Code>,
    /// WebAssembly code when generated for this program.
    pub(crate) wasm: Option<wasm::Code>,

    /// Program section storage.
    pub(crate) storage: SectionStorage,
}

impl Program {
    /// Return all content ids referenced by this program.
    pub fn content_ids(&self) -> Vec<ContentId> {
        let mut ids = Vec::new();

        if let Some(native) = &self.native {
            ids.extend(native.content_ids());
        }

        if let Some(wasm) = &self.wasm {
            ids.extend(wasm.content_ids());
        }

        ids
    }

    /// Return runtime type table.
    pub fn types(&self) -> &TypeTable {
        &self.types
    }

    /// Return the target ABI layout used by this program.
    pub fn target_layout(&self) -> TargetLayout {
        self.target_layout
    }

    /// Return the pointer byte width used by this program.
    pub fn pointer_bytes(&self) -> u8 {
        self.target_layout().pointer_bytes()
    }

    /// Return the program function table.
    pub fn functions(&self) -> &FunctionTable {
        &self.functions
    }

    /// Return a read-only view of program sections.
    pub fn sections(&self) -> SectionImage<'_> {
        SectionImage::new(&self.storage)
    }

    /// Return one program function entry.
    pub fn function(&self, function: FunctionId) -> Option<&Function> {
        self.functions.get(self.sections(), function)
    }

    /// Return the runtime binding attached to one function.
    pub fn function_binding(&self, function: FunctionId) -> Option<BindingId> {
        self.functions.binding(self.sections(), function)
    }

    /// Return parameter types for one program function.
    pub fn function_parameters(&self, function: FunctionId) -> Option<&[TypeId]> {
        let sections = self.sections();
        let function = self.functions.get(sections, function)?;
        let signature = self.functions.signature(sections, function.signature)?;

        Some(self.functions.parameters(sections, signature))
    }

    /// Return the result type for one program function.
    pub fn function_result(&self, function: FunctionId) -> Option<TypeId> {
        let sections = self.sections();
        let function = self.functions.get(sections, function)?;
        let signature = self.functions.signature(sections, function.signature)?;

        Some(signature.result)
    }

    /// Return one callable signature entry.
    pub fn signature(&self, signature: SignatureId) -> Option<&SignatureEntry> {
        self.functions.signature(self.sections(), signature)
    }

    /// Return one expanded callable signature.
    pub fn expand_signature(&self, signature: SignatureId) -> Option<Signature> {
        self.functions.expand_signature(self.sections(), signature)
    }

    /// Check that one function matches one call signature.
    pub fn check_function_signature<I>(
        &self,
        function: FunctionId,
        result: TypeId,
        parameters: I,
    ) -> Result<()>
    where
        I: Clone + Iterator<Item = TypeId>,
    {
        self.functions
            .check_signature(self.sections(), function, result, parameters)
    }

    /// Return runtime dispatch table.
    pub fn dispatch(&self) -> &DispatchTable {
        &self.dispatch
    }

    /// Return one virtual table by its durable id.
    pub fn virtual_table(&self, table: VirtualTableId) -> Option<&VirtualTable> {
        self.dispatch.virtual_table(self.sections(), table)
    }

    /// Return one virtual method by table id and slot.
    pub fn virtual_method(&self, table: VirtualTableId, slot: u32) -> Option<FunctionId> {
        let table = self.virtual_table(table)?;

        self.dispatch
            .virtual_methods(self.sections(), table)
            .get(slot as usize)
            .copied()
    }

    /// Return one dynamic table by its durable id.
    pub fn dynamic_table(&self, table: DynamicTableId) -> Option<&DynamicTable> {
        self.dispatch.dynamic_table(self.sections(), table)
    }

    /// Return one dynamic entry by table id and slot.
    pub fn dynamic_entry(&self, table: DynamicTableId, slot: u32) -> Option<&DynamicEntry> {
        let table = self.dynamic_table(table)?;

        self.dispatch
            .dynamic_entries(self.sections(), table)
            .get(slot as usize)
    }

    /// Return program sites.
    pub fn sites(&self) -> &SiteTable {
        &self.sites
    }

    /// Decode one profile sample key using its program site type.
    pub fn sample_value(&self, site: &SampleSite, key: SampleKey) -> Option<SampleValue> {
        let layout = self.word_layout(site.value_type)?;

        key.decode(layout)
    }

    /// Return whether one concrete type satisfies one runtime type.
    pub fn is_subtype(&self, concrete: TypeId, expected: TypeId) -> Result<bool> {
        self.types
            .is_subtype(self.sections(), concrete, expected)
            .map_err(Error::undefined_type)
    }

    /// Return one program global by id.
    pub fn global(&self, global: GlobalId) -> Option<&Global> {
        self.globals.get(self.sections(), global)
    }

    /// Return globals stored in one static location.
    pub fn globals(&self, location: GlobalLocation) -> impl Iterator<Item = (GlobalId, &Global)> {
        self.globals.iter_location(self.sections(), location)
    }

    /// Return immutable constant storage owned by this program.
    pub fn constants(&self) -> &StaticImage {
        &self.constant_space
    }

    /// Return linked bytecode carried by this program.
    pub fn bytecode(&self) -> &bytecode::Code {
        &self.bytecode
    }

    /// Return native code when this program carries it.
    pub fn native(&self) -> Option<&native::Code> {
        self.native.as_ref()
    }

    /// Return WebAssembly code when this program carries it.
    pub fn wasm(&self) -> Option<&wasm::Code> {
        self.wasm.as_ref()
    }

    /// Return initial shared static storage for new runtimes.
    pub fn shared_statics(&self) -> &StaticImage {
        &self.shared_static_space
    }

    /// Return initial local static storage for new workers.
    pub fn local_statics(&self) -> &StaticImage {
        &self.local_static_space
    }

    /// Materialize initial shared static storage.
    pub fn materialize_shared_statics(&self, memory: Arc<MemoryMap>) -> MemoryResult<StaticSpace> {
        self.shared_static_space
            .materialize(self.sections(), memory)
    }

    /// Materialize initial local static storage.
    pub fn materialize_local_statics(&self, memory: Arc<MemoryMap>) -> MemoryResult<StaticSpace> {
        self.local_static_space.materialize(self.sections(), memory)
    }

    /// Resolve one function id by source name.
    pub fn function_id_by_name(&self, name: &str) -> Option<FunctionId> {
        self.functions
            .id_by_name(self.sections(), StringId::for_text(name))
    }

    /// Return the program string table.
    pub fn strings(&self) -> &StringTable {
        &self.strings
    }

    /// Return source reflection when present.
    pub fn info(&self) -> Option<&ProgramInfo> {
        self.info.as_ref()
    }

    /// Return one program string by stable id when present.
    pub fn string(&self, id: StringId) -> Option<&str> {
        self.strings.string(self.sections(), id)
    }

    /// Return runtime layouts for this program.
    pub fn layouts(&self) -> &LayoutTable {
        &self.layouts
    }

    /// Return one complete drop entry.
    pub fn drop_entry(&self, drop: DropId) -> Option<DropEntry> {
        self.drops.entry(self.sections(), drop)
    }

    /// Return the program drop table.
    pub fn drops(&self) -> &DropTable {
        &self.drops
    }

    /// Return the destructor for one concrete type when it requires cleanup.
    pub fn destructor(&self, ty: TypeId) -> Result<Option<FunctionId>> {
        let descriptor = self
            .types()
            .descriptor(self.sections(), ty)
            .ok_or_else(|| Error::undefined_type(ty))?;

        // return types without destructors directly
        let Some(drop) = descriptor.drop_id() else {
            return Ok(None);
        };

        // resolve the destructor through the program drop table
        let entry = self
            .drop_entry(drop)
            .ok_or_else(|| Error::undefined_drop(drop))?;

        Ok(Some(entry.function))
    }

    /// Return the heap allocation shape for one concrete type.
    pub fn allocation_shape(&self, ty: TypeId) -> Result<AllocationShape> {
        let sections = self.sections();
        let descriptor = self
            .types()
            .descriptor(sections, ty)
            .ok_or_else(|| Error::undefined_type(ty))?;
        let layout_id = descriptor.layout;
        let Some(layout) = self.layouts().get(sections, layout_id) else {
            return Err(Error::undefined_layout(layout_id));
        };

        let trace_map = self.trace_map(layout.trace)?;
        let trace_id = trace_map.has_heap_reference().then_some(layout.trace);
        let shape = AllocationShape::new(
            layout.size as usize,
            layout.alignment as usize,
            trace_id,
            trace_map,
        );

        match descriptor.drop_id() {
            Some(drop) => shape.with_drop(drop).map_err(Error::from),
            None => Ok(shape),
        }
    }

    /// Decode one program trace map.
    pub fn trace_map(&self, id: TraceId) -> Result<TraceMap> {
        self.trace_view().trace_map(id).map_err(Error::from)
    }

    /// Return compact program trace rows.
    pub fn trace_view(&self) -> TraceView<'_> {
        self.traces.view(self.sections())
    }

    /// Return the program trace table.
    pub fn trace_table(&self) -> &TraceTable {
        &self.traces
    }

    /// Return the constant address for one global.
    pub fn global_address(&self, global: GlobalId) -> Option<GlobalAddress> {
        self.global(global)?;

        Some(GlobalAddress::new(global, 0))
    }

    /// Resolve one constant byte range to a native address.
    pub fn constant_native_address(
        &self,
        address: GlobalAddress,
        byte_len: usize,
    ) -> Option<usize> {
        let sections = self.sections();
        let global = self.global(address.global())?;
        if global.location != GlobalLocation::Constant {
            return None;
        }

        self.constant_space
            .native_address(sections, global, address, byte_len)
    }

    /// Return whether constants own one byte range.
    pub fn constants_own_address_range(&self, address: GlobalAddress, byte_len: usize) -> bool {
        let sections = self.sections();
        let Some(global) = self.global(address.global()) else {
            return false;
        };
        if global.location != GlobalLocation::Constant {
            return false;
        }

        self.constant_space
            .owns_address_range(sections, global, address, byte_len)
    }

    /// Return the frame layout for one layout id when present.
    pub fn frame_layout_by_id(&self, frame_layout: FrameLayoutId) -> Option<&FrameLayout> {
        self.frames.layout(self.sections(), frame_layout)
    }

    /// Return one frame state.
    pub fn frame_state(&self, frame_state: FrameStateId) -> Option<&FrameState> {
        self.frames.state(self.sections(), frame_state)
    }

    /// Return one frame slot by id.
    pub fn frame_slot(&self, layout: &FrameLayout, slot: FrameSlotId) -> Option<&FrameSlot> {
        self.frames.slot(self.sections(), layout, slot)
    }

    /// Return the closure environment slot for one frame layout.
    pub fn frame_environment_slot<'a>(&'a self, layout: &FrameLayout) -> Option<&'a FrameSlot> {
        let sections = self.sections();
        let slots = self.frames.slots(sections, layout);

        layout.environment(slots)
    }

    /// Return the live slots for one frame state.
    pub fn frame_live_slots<'a>(&'a self, state: &'a FrameState) -> &'a [FrameSlotId] {
        self.frames.live_slots(self.sections(), state)
    }

    /// Return the canonical layout for one type.
    pub fn layout(&self, ty: TypeId) -> Option<&Layout> {
        let sections = self.sections();
        let layout_id = self.types().layout_id(sections, ty)?;

        self.layouts().get(sections, layout_id)
    }

    /// Return one runtime layout by id.
    pub fn layout_by_id(&self, layout: LayoutId) -> Option<&Layout> {
        self.layouts().get(self.sections(), layout)
    }

    /// Return the layout for one owning tensor type.
    pub fn tensor_layout(&self, ty: TypeId) -> Option<TensorLayout> {
        let layout = self.layout(ty)?;
        let LayoutShape::Tensor(tensor) = layout.shape else {
            return None;
        };

        Some(tensor)
    }

    /// Return the layout for one tensor view type.
    pub fn tensor_view_layout(&self, ty: TypeId) -> Option<TensorViewLayout> {
        let layout = self.layout(ty)?;
        let LayoutShape::TensorView(view) = layout.shape else {
            return None;
        };

        Some(view)
    }

    /// Return the dimensions for one tensor or tensor view layout.
    pub fn tensor_dimensions(&self, dimensions: EntryRange<TensorDimension>) -> &[TensorDimension] {
        self.layouts.tensor_dimensions(self.sections(), dimensions)
    }

    /// Return one field by logical source index.
    pub fn layout_field(&self, layout: &Layout, index: u32) -> Option<&LayoutField> {
        self.layouts.field(self.sections(), layout, index)
    }

    /// Return the field count for one layout when it is field-addressable.
    pub fn layout_field_count(&self, layout: &Layout) -> Option<usize> {
        self.layouts.field_count(layout)
    }

    /// Return the cases for one variant layout.
    pub fn variant_cases(&self, layout: VariantLayout) -> &[VariantCaseLayout] {
        self.layouts.cases(self.sections(), layout)
    }

    /// Return all fields for one layout when it is field-addressable.
    pub fn layout_fields(&self, layout: &Layout) -> &[LayoutField] {
        self.layouts.fields(self.sections(), layout)
    }

    /// Return the layout id for one type.
    pub fn layout_id(&self, ty: TypeId) -> Option<LayoutId> {
        self.types().layout_id(self.sections(), ty)
    }

    /// Return whether one type is stored in one bytecode word.
    pub fn is_word_type(&self, ty: TypeId) -> bool {
        let Some(layout) = self.word_layout(ty) else {
            return false;
        };

        layout.byte_len(self.pointer_bytes() as usize) <= Word::BYTE_LEN
    }

    /// Return the bytecode word layout for one type.
    pub fn word_layout(&self, ty: TypeId) -> Option<WordLayout> {
        let layout = self.layout(ty)?;

        layout.word_layout()
    }

    /// Encode one program value through its runtime layout.
    pub fn encode_value(&self, ty: TypeId, value: &Value) -> Result<Option<Word>> {
        let Some(layout) = self.layout(ty) else {
            return Err(Error::undefined_type(ty));
        };

        // accept only values represented by one bytecode word
        let Some(layout) = layout.word_layout() else {
            return Err(Error::UnsupportedValue { ty });
        };

        value.encode(layout)
    }

    /// Decode one program value from its bytecode result words.
    pub fn decode_value(&self, ty: TypeId, words: &[Word]) -> Result<Value> {
        let Some(layout) = self.layout(ty) else {
            return Err(Error::undefined_type(ty));
        };

        // accept only values represented by one bytecode word
        let Some(layout) = layout.word_layout() else {
            return Err(Error::UnsupportedValue { ty });
        };

        // require exactly the words implied by the selected layout
        let expected = usize::from(layout != WordLayout::Void);
        if words.len() != expected {
            return Err(Error::ValueWordCountMismatch {
                ty,
                expected,
                actual: words.len(),
            });
        }

        // decode void without indexing the empty result range
        if layout == WordLayout::Void {
            return Ok(Value::Void);
        }

        Value::decode(layout, words[0])
    }

    /// Return the scalar layout for one type.
    pub fn scalar_format(&self, ty: TypeId) -> Option<ScalarFormat> {
        let layout = self.layout(ty)?;

        layout.scalar_format()
    }

    /// Return the environment word layout for one function closure.
    pub fn function_environment_layout(&self, function: FunctionId) -> Option<WordLayout> {
        let sections = self.sections();
        let function = self.functions().get(sections, function)?;
        let environment = function.environment()?;

        self.word_layout(environment)
    }

    /// Return whether one frame slot is stored in one bytecode word.
    pub fn frame_slot_is_word(&self, slot: &FrameSlot) -> bool {
        self.is_word_type(slot.ty)
    }

    /// Return the byte width for one type in a runtime frame.
    pub fn type_byte_len(&self, ty: TypeId) -> Option<usize> {
        if let Some(layout) = self.word_layout(ty) {
            return Some(layout.byte_len(self.pointer_bytes() as usize));
        }

        self.layout(ty).map(|layout| layout.size as usize)
    }

    /// Visit mutable heap root slots from one byte range.
    pub fn visit_byte_root_slots(
        &self,
        ty: TypeId,
        bytes: &mut [u8],
        visit: &mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>,
    ) -> Result<()> {
        let layout_id = self
            .layout_id(ty)
            .ok_or_else(|| Error::undefined_type(ty))?;
        let layout = self
            .layout_by_id(layout_id)
            .ok_or_else(|| Error::undefined_layout(layout_id))?;

        let layout_bytes = layout.size as usize;
        if bytes.len() != layout_bytes {
            return Err(Error::ByteLengthMismatch {
                ty,
                expected: layout_bytes,
                actual: bytes.len(),
            });
        }

        let trace_map = self.trace_map(layout.trace)?;

        visit_heap_root_slots(&trace_map, 0, bytes, ReferenceRange::All, visit).map_err(Error::from)
    }

    /// Visit mutable heap root slots from one static space.
    pub fn visit_static_root_slots(
        &self,
        location: GlobalLocation,
        static_space: &mut StaticSpace,
        visit: &mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>,
    ) -> Result<()> {
        let sections = self.sections();

        // visit each static global
        for (global_id, global) in self.globals.iter_location(sections, location) {
            // SAFETY: root walking holds exclusive access to this static space
            let bytes = unsafe { static_space.bytes_mut(global) }
                .ok_or(Error::MissingGlobalStorage { global: global_id })?;
            self.visit_byte_root_slots(global.ty, bytes, visit)?;
        }

        Ok(())
    }

    /// Visit mutable heap root slots retained by one continuation.
    pub fn visit_continuation_root_slots(
        &self,
        continuation: &mut Continuation,
        visit: &mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>,
    ) -> Result<()> {
        for frame_index in 0..continuation.frames.len() {
            let frame = continuation.frames[frame_index];
            let state = self
                .frame_state(frame.frame_state)
                .ok_or(Error::UndefinedFrameState {
                    frame_state: frame.frame_state,
                })?;
            let layout =
                self.frame_layout_by_id(state.frame_layout)
                    .ok_or(Error::UndefinedFrameLayout {
                        frame_layout: state.frame_layout,
                    })?;
            let bytes =
                continuation
                    .frame_bytes_mut(frame_index)
                    .ok_or(Error::InvalidFrameRange {
                        frame_state: frame.frame_state,
                    })?;

            // reject frame bytes that do not match the linked frame layout
            if bytes.len() != layout.byte_len() as usize {
                return Err(Error::FrameByteLengthMismatch {
                    frame_state: frame.frame_state,
                    expected: layout.byte_len() as usize,
                    actual: bytes.len(),
                });
            }

            // visit only values live at the captured program point
            for slot_id in self.frame_live_slots(state) {
                let slot = self
                    .frame_slot(layout, *slot_id)
                    .ok_or(Error::UndefinedFrameSlot {
                        frame_layout: state.frame_layout,
                        slot: *slot_id,
                    })?;
                let start = slot.offset as usize;
                let end = start + slot.byte_len as usize;
                let bytes = bytes
                    .get_mut(start..end)
                    .ok_or(Error::FrameSlotOutOfBounds {
                        frame_state: frame.frame_state,
                        slot: *slot_id,
                    })?;

                self.visit_byte_root_slots(slot.ty, bytes, visit)?;
            }
        }

        Ok(())
    }

    /// Return the program point for one resume state.
    pub fn frame_point(&self, frame_state: FrameStateId) -> Option<ProgramPoint> {
        self.frame_state(frame_state).map(|state| state.point)
    }

    /// Return one resume state id for one program point.
    pub fn frame_state_at(&self, point: ProgramPoint) -> Option<FrameStateId> {
        self.frames.state_at(self.sections(), point)
    }

    /// Return the function containing the innermost continuation frame.
    pub fn continuation_function(&self, continuation: &Continuation) -> Result<FunctionId> {
        let frame = continuation.innermost().ok_or(Error::EmptyContinuation)?;
        let state = self
            .frame_state(frame.frame_state)
            .ok_or(Error::UndefinedFrameState {
                frame_state: frame.frame_state,
            })?;

        Ok(state.point.function)
    }

    /// Return the value type yielded by one suspended continuation.
    pub fn continuation_yield_type(&self, continuation: &Continuation) -> Result<TypeId> {
        let frame = continuation.innermost().ok_or(Error::EmptyContinuation)?;
        let (_, site) = self
            .sites
            .continuation_state(self.sections(), frame.frame_state)
            .ok_or(Error::UndefinedContinuationSite {
                frame_state: frame.frame_state,
            })?;

        Ok(site.yielded_type)
    }

    /// Return the value type received by one suspended continuation.
    pub fn continuation_resume_type(&self, continuation: &Continuation) -> Result<TypeId> {
        let frame = continuation.innermost().ok_or(Error::EmptyContinuation)?;
        let (_, site) = self
            .sites
            .continuation_state(self.sections(), frame.frame_state)
            .ok_or(Error::UndefinedContinuationSite {
                frame_state: frame.frame_state,
            })?;

        Ok(site.resumed_type)
    }

    /// Return the frame layout for one function when present.
    pub fn frame_layout(&self, function: FunctionId) -> Option<&FrameLayout> {
        let frame_layout = self.function(function)?.frame_layout()?;

        self.frame_layout_by_id(frame_layout)
    }
}
