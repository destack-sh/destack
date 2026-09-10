use std::result;
use std::sync::Arc;

use destack_bytecode as bytecode;
use destack_core::{Blob, SectionImage, SectionStorage, StringId};
use destack_heap::{
    AllocationPlan, AllocationShape, DropId, HeapOptions, HeapResult, ReferenceRange, RootSlot,
    SharedHeapOptions, TraceTable, TraceView, visit_heap_root_slots,
};
use destack_memory::{MemoryError, MemoryMap, MemoryRange, MemoryResult};
use destack_mir::{Space, Storage, TargetLayout, TraceId, TraceMap};
use destack_native as native;
use destack_serde::{Reflect, Schema, Type};
use destack_webassembly as wasm;
use serde::{Deserialize, Serialize};

use crate::{
    ActivationImage, Binding, BindingId, BindingTable, CallSite, CallSiteId, DispatchTable,
    DropEntry, DropTable, DynamicEntry, DynamicTable, DynamicTableId, EntryPoint, Error,
    FrameLayout, FrameLayoutId, FramePoint, FrameSlot, FrameState, FrameStateId, FrameTable,
    Function, FunctionId, FunctionTable, Global, GlobalId, GlobalLocation, GlobalTable,
    InitializerTable, Layout, LayoutField, LayoutId, LayoutTable, ProgramInfo, ProgramPoint,
    Result, SampleKey, SampleSite, SampleValue, ScalarFormat, Signature, SignatureEntry,
    SignatureId, SiteTable, StaticImage, StaticSpace, StringTable, Symbol, TypeFingerprint, TypeId,
    TypeTable, Value, VariantCaseLayout, VariantLayout, VirtualTable, VirtualTableId, Word,
    WordLayout,
};

/// Linked program.
#[derive(Debug)]
pub struct Program {
    /// Target ABI layout used by program layouts and pointer-sized integer types.
    pub(crate) target_layout: TargetLayout,

    /// Program string table.
    pub(crate) strings: StringTable,
    /// Runtime type table.
    pub(crate) types: TypeTable,
    /// Destructors keyed by drop id.
    pub(crate) drops: DropTable,
    /// Module initializers in dependency order.
    pub(crate) initializers: InitializerTable,
    /// Runtime layouts keyed by layout id.
    pub(crate) layouts: LayoutTable,
    /// Canonical frame states and layouts.
    pub(crate) frames: FrameTable,
    /// Program function table.
    pub(crate) functions: FunctionTable,
    /// Runtime binding declarations.
    pub(crate) bindings: BindingTable,
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
    pub(crate) constants: StaticImage,
    /// Initial shared static storage for each runtime.
    pub(crate) shared_statics: StaticImage,
    /// Initial local static storage for each worker.
    pub(crate) local_statics: StaticImage,

    /// Bytecode used for interpretation and deoptimization when included.
    pub(crate) bytecode: bytecode::Code,
    /// Native code when generated for this program.
    pub(crate) native: Option<native::Code>,
    /// WebAssembly code when generated for this program.
    pub(crate) wasm: Option<wasm::Code>,

    /// Identity of the exact encoded Program image.
    pub(crate) blob: Blob,
    /// Program section storage.
    pub(crate) storage: SectionStorage,
}

impl Serialize for Program {
    /// Serialize this Program as its exact encoded image.
    fn serialize<S>(&self, serializer: S) -> result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.storage.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Program {
    /// Deserialize and validate one exact encoded Program image.
    fn deserialize<D>(deserializer: D) -> result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let storage = SectionStorage::deserialize(deserializer)?;

        Self::load(storage).map_err(serde::de::Error::custom)
    }
}

impl Reflect for Program {
    /// Reflect this Program as its exact encoded image.
    fn reflect(schema: &mut Schema) -> Type {
        SectionStorage::reflect(schema)
    }
}

impl Program {
    /// Return the exact Blob identity of this Program image.
    pub const fn blob(&self) -> Blob {
        self.blob
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

    /// Return the execution word count for one runtime type.
    pub fn type_word_count(&self, ty: TypeId) -> Option<usize> {
        let byte_len = self.type_byte_len(ty)?;

        Some(byte_len.div_ceil(Word::BYTE_LEN))
    }

    /// Return the program function table.
    pub fn functions(&self) -> &FunctionTable {
        &self.functions
    }

    /// Return one stable function symbol.
    pub fn function_symbol(&self, function: FunctionId) -> Option<Symbol> {
        self.functions.symbol(self.sections(), function)
    }

    /// Return one stable type fingerprint.
    pub fn type_fingerprint(&self, ty: TypeId) -> Option<TypeFingerprint> {
        self.types.fingerprint(self.sections(), ty)
    }

    /// Return one stable global symbol.
    pub fn global_symbol(&self, global: GlobalId) -> Option<Symbol> {
        self.globals.symbol(self.sections(), global)
    }

    /// Return a read-only view of program sections.
    pub fn sections(&self) -> SectionImage<'_> {
        // SAFETY: Program construction builds or validates every absolute section in its header.
        unsafe { SectionImage::new(&self.storage) }
    }

    /// Return one program function entry.
    pub fn function(&self, function: FunctionId) -> Option<&Function> {
        self.functions.get(self.sections(), function)
    }

    /// Return the runtime binding attached to one function.
    pub fn function_binding(&self, function: FunctionId) -> Option<&Binding> {
        self.bindings.function(self.sections(), function)
    }

    /// Return all runtime binding declarations.
    pub fn bindings(&self) -> &[Binding] {
        self.bindings.entries(self.sections())
    }

    /// Return one runtime binding declaration by stable identity.
    pub fn binding(&self, id: BindingId) -> Option<&Binding> {
        self.bindings.get(self.sections(), id)
    }

    /// Return one binding's required runtime actions.
    pub fn binding_requires(&self, binding: &Binding) -> &[StringId] {
        self.bindings.requires(self.sections(), binding)
    }

    /// Return one binding's supported platforms.
    pub fn binding_platforms(&self, binding: &Binding) -> &[StringId] {
        self.bindings.platforms(self.sections(), binding)
    }

    /// Return one binding's supported platform families.
    pub fn binding_families(&self, binding: &Binding) -> &[StringId] {
        self.bindings.families(self.sections(), binding)
    }

    /// Return one binding's supported hosts.
    pub fn binding_hosts(&self, binding: &Binding) -> &[StringId] {
        self.bindings.hosts(self.sections(), binding)
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

    /// Return the result word count for one program function.
    pub fn function_result_word_count(&self, function: FunctionId) -> Option<usize> {
        let result = self.function_result(function)?;

        self.type_word_count(result)
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

    /// Return the call site at one program point.
    pub fn call(&self, point: ProgramPoint) -> Option<(CallSiteId, &CallSite)> {
        self.sites.call(self.sections(), point)
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
        &self.constants
    }

    /// Return the linked bytecode.
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
        &self.shared_statics
    }

    /// Return initial local static storage for new workers.
    pub fn local_statics(&self) -> &StaticImage {
        &self.local_statics
    }

    /// Materialize constant and shared static storage for one runtime.
    pub fn materialize_runtime_statics(
        &self,
        memory: Arc<MemoryMap>,
    ) -> MemoryResult<(StaticSpace, StaticSpace)> {
        let sections = self.sections();
        let constants = self
            .constants
            .materialize_constant(sections, memory.clone())?;
        let shared = self.shared_statics.materialize(sections, memory)?;

        // resolve every runtime-visible static address
        let base = |location| match location {
            GlobalLocation::Constant => Ok(constants.offset()),
            GlobalLocation::SharedStatic => Ok(shared.offset()),
            GlobalLocation::LocalStatic => Err(MemoryError::internal(
                "runtime static relocation targets local storage",
            )),
        };
        constants.relocate(self.constants.relocations(sections), base)?;
        shared.relocate(self.shared_statics.relocations(sections), base)?;
        constants.freeze()?;

        Ok((constants, shared))
    }

    /// Materialize local static storage for one worker.
    pub fn materialize_local_statics(
        &self,
        memory: Arc<MemoryMap>,
        constants: &StaticSpace,
        shared: &StaticSpace,
    ) -> MemoryResult<StaticSpace> {
        let sections = self.sections();
        let local = self.local_statics.materialize(sections, memory)?;

        // resolve every worker-visible static address
        let base = |location| match location {
            GlobalLocation::Constant => Ok(constants.offset()),
            GlobalLocation::SharedStatic => Ok(shared.offset()),
            GlobalLocation::LocalStatic => Ok(local.offset()),
        };
        local.relocate(self.local_statics.relocations(sections), base)?;

        Ok(local)
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

    /// Return the module initializers in dependency order, the entry module's last.
    pub fn initializers(&self) -> &[EntryPoint] {
        self.initializers.entries(self.sections())
    }

    /// Return the destructor for one concrete type when it requires cleanup.
    pub fn destructor(&self, ty: TypeId, storage: Storage) -> Result<Option<FunctionId>> {
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

        Ok(entry.destructor(storage))
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

    /// Build heap allocation plans for this Program and one runtime heap configuration.
    pub fn plan_allocations(
        &self,
        local: &HeapOptions,
        shared: &SharedHeapOptions,
    ) -> Result<Vec<AllocationPlan>> {
        self.sites()
            .allocations(self.sections())
            .iter()
            .map(|site| {
                let shape = self.allocation_shape(site.storage_type)?;
                let plan = match site.space {
                    Space::Local => local.allocation_plan(&shape),
                    Space::Shared => shared.allocation_plan(&shape),
                    Space::Constant | Space::Parameter(_) | Space::Slot(_) | Space::Join(_) => {
                        return Err(Error::ConstantAllocationSite);
                    }
                };

                Ok(plan)
            })
            .collect()
    }

    /// Decode one program trace map.
    pub fn trace_map(&self, id: TraceId) -> Result<TraceMap> {
        self.trace_view().trace_map(id).map_err(Error::from)
    }

    /// Return compact program trace entries.
    pub fn trace_view(&self) -> TraceView<'_> {
        self.traces.view(self.sections())
    }

    /// Return the program trace table.
    pub fn trace_table(&self) -> &TraceTable {
        &self.traces
    }

    /// Return one canonical frame state.
    pub fn frame_state(&self, frame_state: FrameStateId) -> Option<&FrameState> {
        self.frames.state(self.sections(), frame_state)
    }

    /// Return one canonical frame layout.
    pub fn frame_layout(&self, layout: FrameLayoutId) -> Option<&FrameLayout> {
        self.frames.layout(self.sections(), layout)
    }

    /// Return the live slots in one canonical frame layout.
    pub fn frame_slots<'a>(&'a self, layout: &'a FrameLayout) -> &'a [FrameSlot] {
        self.frames.slots(self.sections(), layout)
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

    /// Return whether one type is stored in one execution word.
    pub fn is_word_type(&self, ty: TypeId) -> bool {
        let Some(layout) = self.word_layout(ty) else {
            return false;
        };

        layout.byte_len(self.pointer_bytes() as usize) <= Word::BYTE_LEN
    }

    /// Return the execution word layout for one type.
    pub fn word_layout(&self, ty: TypeId) -> Option<WordLayout> {
        let layout = self.layout(ty)?;

        layout.word_layout()
    }

    /// Create one exact value for a Program type.
    pub fn value(&self, ty: TypeId, words: impl IntoIterator<Item = Word>) -> Result<Value> {
        let value = Value::new(ty, words);
        self.value_words(ty, &value)?;

        Ok(value)
    }

    /// Return one value's words after requiring the expected Program type and width.
    pub fn value_words<'a>(&self, ty: TypeId, value: &'a Value) -> Result<&'a [Word]> {
        let Some(layout) = self.layout(ty) else {
            return Err(Error::undefined_type(ty));
        };
        if value.ty() != ty {
            return Err(Error::ValueTypeMismatch {
                expected: ty,
                actual: value.ty(),
            });
        }

        // require the exact physical width selected by the Program layout
        let words = value.words();
        let expected = (layout.size as usize).div_ceil(Word::BYTE_LEN);
        if words.len() != expected {
            return Err(Error::ValueWordCountMismatch {
                ty,
                expected,
                actual: words.len(),
            });
        }

        Ok(words)
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

    /// Visit mutable heap roots retained by one activation image.
    pub fn visit_activation_root_slots(
        &self,
        memory: &MemoryMap,
        activation: &mut ActivationImage,
        visit: &mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>,
    ) -> Result<()> {
        visit(RootSlot::HeapReference(
            activation.context_mut().reference_mut(),
        ))?;
        let states = activation.frames().iter().map(|frame| frame.state());

        self.visit_memory_frame_root_slots(memory, activation.memory(), states, visit)
    }

    /// Visit roots retained by packed frames inside one memory map.
    fn visit_memory_frame_root_slots(
        &self,
        memory: &MemoryMap,
        range: MemoryRange,
        states: impl ExactSizeIterator<Item = FrameStateId> + Clone,
        visit: &mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>,
    ) -> Result<()> {
        memory.make_writable(range.offset, range.byte_len)?;

        // SAFETY: the owning machine exclusively visits this allocated frame range
        let bytes = unsafe { memory.mapped_bytes_mut(range.offset, range.byte_len) };

        self.visit_frame_root_slots(states, bytes, visit)
    }

    /// Visit roots through one exact frame state sequence.
    fn visit_frame_root_slots(
        &self,
        states: impl ExactSizeIterator<Item = FrameStateId> + Clone,
        bytes: &mut [u8],
        visit: &mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>,
    ) -> Result<()> {
        let innermost = states.clone().last().ok_or(Error::EmptyCallChain)?;
        let mut frame_byte_offset = 0usize;

        // compute the complete packed call chain byte length
        for frame_state in states.clone() {
            let state = self
                .frame_state(frame_state)
                .ok_or(Error::UndefinedFrameState { frame_state })?;
            let layout = self
                .frame_layout(state.layout)
                .ok_or(Error::UndefinedFrameLayout {
                    frame_layout: state.layout,
                })?;
            frame_byte_offset = frame_byte_offset.next_multiple_of(layout.alignment as usize);
            frame_byte_offset += layout.byte_len as usize;
        }

        // require the exact packed call chain byte length
        let expected = frame_byte_offset;
        let actual = bytes.len();
        if actual != expected {
            return Err(Error::FrameByteLengthMismatch {
                frame_state: innermost,
                expected,
                actual,
            });
        }

        // visit every retained value through its exact Program type
        let mut frame_byte_offset = 0usize;
        for frame_state in states {
            let state = self
                .frame_state(frame_state)
                .ok_or(Error::UndefinedFrameState { frame_state })?;
            let layout = self
                .frame_layout(state.layout)
                .ok_or(Error::UndefinedFrameLayout {
                    frame_layout: state.layout,
                })?;
            frame_byte_offset = frame_byte_offset.next_multiple_of(layout.alignment as usize);

            for slot in self.frame_slots(layout) {
                let start = frame_byte_offset + slot.offset as usize;
                let end = start + slot.byte_len as usize;
                let bytes = bytes
                    .get_mut(start..end)
                    .ok_or(Error::FrameSlotOutOfBounds {
                        frame_state,
                        frame_layout: state.layout,
                        offset: slot.offset,
                    })?;
                self.visit_byte_root_slots(slot.ty, bytes, visit)?;
            }

            frame_byte_offset += layout.byte_len as usize;
        }

        Ok(())
    }

    /// Visit mutable frame addresses from one byte range.
    pub fn visit_byte_frame_addresses(
        &self,
        ty: TypeId,
        bytes: &mut [u8],
        visit: &mut dyn FnMut(&mut [u8]) -> HeapResult<()>,
    ) -> Result<()> {
        let layout_id = self
            .layout_id(ty)
            .ok_or_else(|| Error::undefined_type(ty))?;
        let layout = self
            .layout_by_id(layout_id)
            .ok_or_else(|| Error::undefined_layout(layout_id))?;

        // require one complete value representation
        let layout_bytes = layout.size as usize;
        if bytes.len() != layout_bytes {
            return Err(Error::ByteLengthMismatch {
                ty,
                expected: layout_bytes,
                actual: bytes.len(),
            });
        }

        self.traces
            .view(self.sections())
            .visit_frame_address_slots(layout.trace, bytes, visit)
            .map_err(Error::from)
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

    /// Visit mutable heap roots retained by one runtime value.
    pub fn visit_value_root_slots(
        &self,
        value: &mut Value,
        visit: &mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>,
    ) -> Result<()> {
        let ty = value.ty();
        self.value_words(ty, value)?;

        // avoid materializing shared value storage when this type has no heap roots
        let layout = self.layout(ty).ok_or_else(|| Error::undefined_type(ty))?;
        let trace_map = self.trace_map(layout.trace)?;
        if !trace_map.has_heap_reference() {
            return Ok(());
        }

        // visit the exact value bytes through the linked trace
        let byte_len = layout.size as usize;
        let bytes = Word::bytes_mut(value.words_mut());
        let bytes = &mut bytes[..byte_len];

        visit_heap_root_slots(&trace_map, 0, bytes, ReferenceRange::All, visit).map_err(Error::from)
    }

    /// Return the logical coordinate for one frame state.
    pub fn frame_point(&self, frame_state: FrameStateId) -> Option<FramePoint> {
        Some(self.frame_state(frame_state)?.point)
    }

    /// Return one frame state id for one logical coordinate.
    pub fn frame_state_at(&self, point: FramePoint) -> Option<FrameStateId> {
        self.frames.state_at(self.sections(), point)
    }
}
