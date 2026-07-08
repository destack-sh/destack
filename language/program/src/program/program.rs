use destack_core::{SectionDirectory, SectionImage, SectionStorage, StringId};
use destack_heap::{
    AllocationShape, HeapEdge, HeapOptions, HeapReference, HeapResult, ReferenceRange, RootSlot,
    SharedHeapOptions, SharedHeapReference, TraceTable, TraceView, visit_heap_root_slots,
};
use destack_mir::{ReferenceKind, TargetLayout, TraceId, TraceMap};
use destack_serde::Reflect;
use destack_source::ContentId;
use serde::{Deserialize, Serialize};

use crate::{
    AddressSpace, CellLayout, DispatchTable, FrameLayout, FrameLayoutId, FrameMaterialization,
    FrameSlot, FrameSlotId, FrameStateId, FrameTable, Function, FunctionId, FunctionSignature,
    FunctionTable, Global, GlobalAddress, GlobalId, GlobalLocation, GlobalTable, Layout,
    LayoutField, LayoutId, LayoutShape, LayoutTable, ProgramInfo, ProgramPoint, ScalarFormat,
    Signature, SiteTable, StaticImage, StaticSpace, StringTable, TypeId, TypeTable, native, vm,
};
use vm::error::{Error, Result};

use super::ProgramLoadError;

/// Program produced by the toolchain.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
#[reflect(module = "destack_program::program")]
pub struct Program {
    /// Program section directory.
    pub(crate) sections: SectionDirectory,
    /// Target ABI layout used by program layouts and pointer-sized integer types.
    pub(crate) target_layout: TargetLayout,
    /// Local heap geometry used by lowered allocation plans.
    pub(crate) local_heap: HeapOptions,
    /// Shared heap geometry used by lowered allocation plans.
    pub(crate) shared_heap: SharedHeapOptions,

    /// Program string table.
    pub(crate) strings: StringTable,
    /// Runtime type table.
    pub(crate) types: TypeTable,
    /// Runtime layouts keyed by layout id.
    pub(crate) layouts: LayoutTable,
    /// Runtime frame table.
    pub(crate) frames: FrameTable,
    /// Program function table.
    pub(crate) functions: FunctionTable,
    /// Runtime dispatch table.
    pub(crate) dispatch: DispatchTable,
    /// Executable sites used by debugging, probes, and observations.
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

    /// VM code used for interpretation, deoptimization, and continuation resume.
    pub(crate) vm: vm::Code,
    /// Native code used as optional acceleration.
    pub(crate) native: Option<native::Code>,

    /// Program section storage.
    pub(crate) storage: SectionStorage,
}

impl Program {
    /// Create one program from executable tables and section storage.
    pub fn new(
        sections: SectionDirectory,
        target_layout: TargetLayout,
        local_heap: HeapOptions,
        shared_heap: SharedHeapOptions,
        strings: StringTable,
        types: TypeTable,
        layouts: LayoutTable,
        frames: FrameTable,
        functions: FunctionTable,
        dispatch: DispatchTable,
        sites: SiteTable,
        traces: TraceTable,
        globals: GlobalTable,
        info: Option<ProgramInfo>,
        constant_space: StaticImage,
        shared_static_space: StaticImage,
        local_static_space: StaticImage,
        vm: vm::Code,
        native: Option<native::Code>,
        storage: SectionStorage,
    ) -> std::result::Result<Self, ProgramLoadError> {
        SectionImage::load(&sections, &storage)?;

        Ok(Self {
            sections,
            target_layout,
            local_heap,
            shared_heap,
            strings,
            types,
            layouts,
            frames,
            functions,
            dispatch,
            sites,
            traces,
            globals,
            info,
            constant_space,
            shared_static_space,
            local_static_space,
            vm,
            native,
            storage,
        })
    }

    /// Return all content ids referenced by this program.
    pub fn content_ids(&self) -> Vec<ContentId> {
        let mut ids = self.vm.content_ids();

        if let Some(native) = &self.native {
            ids.extend(native.content_ids());
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

    /// Return local heap options required by this program.
    pub fn heap_options(&self) -> &HeapOptions {
        &self.local_heap
    }

    /// Return shared heap options required by this program.
    pub fn shared_heap_options(&self) -> &SharedHeapOptions {
        &self.shared_heap
    }

    /// Return the program function table.
    pub fn functions(&self) -> &FunctionTable {
        &self.functions
    }

    /// Return a read-only view of program sections.
    pub fn sections(&self) -> SectionImage<'_> {
        // SAFETY: Program::new loads and checks the section image before storing it.
        unsafe { SectionImage::new_unchecked(&self.sections, &self.storage) }
    }

    /// Return one program function record.
    pub fn function(&self, function: FunctionId) -> Option<&Function> {
        self.functions.get(self.sections(), function)
    }

    /// Return parameter types for one program function.
    pub fn function_parameters(&self, function: FunctionId) -> Option<&[TypeId]> {
        let sections = self.sections();

        self.functions
            .get(sections, function)
            .map(|function| self.functions.parameters(sections, function))
    }

    /// Check that one function matches one signature type.
    pub fn check_function_signature(
        &self,
        function: FunctionId,
        signature: &Signature,
    ) -> Result<()> {
        self.functions
            .check_signature(self.sections(), function, signature)
    }

    /// Check that one function matches one packed signature entry.
    pub fn check_function_signature_entry(
        &self,
        function: FunctionId,
        signature: FunctionSignature,
        parameters: &[TypeId],
    ) -> Result<()> {
        let sections = self.sections();

        self.functions
            .check_signature_entry(sections, function, signature, parameters)
    }

    /// Return runtime dispatch table.
    pub fn dispatch(&self) -> &DispatchTable {
        &self.dispatch
    }

    /// Return executable program sites.
    pub fn sites(&self) -> &SiteTable {
        &self.sites
    }

    /// Return whether one concrete type satisfies one runtime type.
    pub fn is_subtype(&self, concrete: TypeId, expected: TypeId) -> Result<bool> {
        self.types
            .is_subtype(self.sections(), concrete, expected)
            .ok_or_else(|| {
                Error::internal(format!("missing runtime type {concrete:?} or {expected:?}"))
            })
    }

    /// Return one program global by id.
    pub fn global(&self, global: GlobalId) -> Option<&Global> {
        self.globals.get(self.sections(), global)
    }

    /// Return immutable constant storage owned by this program.
    pub fn constants(&self) -> &StaticImage {
        &self.constant_space
    }

    /// Return native code when this program carries it.
    pub fn native_code(&self) -> Option<&native::Code> {
        self.native.as_ref()
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
    pub fn materialize_shared_statics(&self) -> StaticSpace {
        self.shared_static_space.materialize(self.sections())
    }

    /// Materialize initial local static storage.
    pub fn materialize_local_statics(&self) -> StaticSpace {
        self.local_static_space.materialize(self.sections())
    }

    /// Initialize runtime-owned static storage from this program.
    pub fn initialize_statics(
        &self,
        local_static: &mut StaticSpace,
        shared_static: &mut StaticSpace,
    ) {
        if shared_static.is_empty() {
            *shared_static = self.materialize_shared_statics();
        }

        *local_static = self.materialize_local_statics();
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

    /// Return the heap allocation shape for one layout id.
    pub fn allocation_shape(&self, layout_id: LayoutId) -> Result<AllocationShape> {
        let sections = self.sections();
        let Some(layout) = self.layouts().get(sections, layout_id) else {
            return Err(Error::internal(format!(
                "missing program layout {layout_id:?}"
            )));
        };

        let trace_map = self.trace_map(layout.trace)?;
        let trace_id = trace_map.has_heap_reference().then_some(layout.trace);

        Ok(AllocationShape::new(
            layout.size as usize,
            layout.alignment as usize,
            trace_id,
            trace_map,
        ))
    }

    /// Decode one program trace map.
    pub fn trace_map(&self, id: TraceId) -> Result<TraceMap> {
        self.trace_view()
            .trace_map(id)
            .map_err(|error| Error::internal(error.to_string()))
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

    /// Return VM resume states.
    pub fn resume(&self) -> &vm::ResumeTable {
        self.vm.resume()
    }

    /// Return the lowered VM functions.
    pub fn vm_functions(&self) -> &vm::FunctionTable {
        self.vm.functions()
    }

    /// Return one VM call target by function id.
    pub fn vm_call_target(&self, function: FunctionId) -> Option<vm::CallTarget> {
        self.vm_functions().call_target(self.sections(), function)
    }

    /// Return one lowered VM function by dense local index.
    pub fn vm_function_by_index(&self, index: u32) -> Option<vm::FunctionCode<'_>> {
        self.vm_functions()
            .function_by_index(self.sections(), index)
    }

    /// Return one lowered VM function by function id.
    pub fn vm_function_by_id(&self, function: FunctionId) -> Option<vm::FunctionCode<'_>> {
        self.vm_functions()
            .function_by_id(self.sections(), function)
    }

    /// Return the compact VM side table.
    pub fn side_table(&self) -> &vm::SideTable {
        self.vm.side_table()
    }

    /// Return the frame layout for one layout id when present.
    pub fn frame_layout_by_id(&self, frame_layout: FrameLayoutId) -> Option<&FrameLayout> {
        self.frames.layout(self.sections(), frame_layout)
    }

    /// Return the single frame materialization for one resume state.
    pub fn frame_materialization(
        &self,
        frame_state: FrameStateId,
    ) -> Option<&FrameMaterialization> {
        self.frames.materialization(self.sections(), frame_state)
    }

    /// Return one frame slot by id.
    pub fn frame_slot(&self, layout: &FrameLayout, slot: FrameSlotId) -> Option<&FrameSlot> {
        self.frames.slot(self.sections(), layout, slot)
    }

    /// Return all value slots for one frame layout.
    pub fn frame_value_slots<'a>(&'a self, layout: &FrameLayout) -> &'a [FrameSlot] {
        let sections = self.sections();
        let slots = self.frames.slots(sections, layout);

        layout.values(slots)
    }

    /// Return all local slots for one frame layout.
    pub fn frame_local_slots<'a>(&'a self, layout: &FrameLayout) -> &'a [FrameSlot] {
        let sections = self.sections();
        let slots = self.frames.slots(sections, layout);

        layout.locals(slots)
    }

    /// Return one local slot for one frame layout.
    pub fn frame_local_slot(&self, layout: &FrameLayout, local: u32) -> Option<&FrameSlot> {
        let sections = self.sections();
        let slots = self.frames.slots(sections, layout);

        layout.local(slots, local)
    }

    /// Return the closure environment slot for one frame layout.
    pub fn frame_environment_slot<'a>(&'a self, layout: &FrameLayout) -> Option<&'a FrameSlot> {
        let sections = self.sections();
        let slots = self.frames.slots(sections, layout);

        layout.environment(slots)
    }

    /// Return copied frame slots for one materialization.
    pub fn frame_copied_slots<'a>(
        &'a self,
        materialization: &'a FrameMaterialization,
    ) -> &'a [FrameSlotId] {
        self.frames.copied_slots(self.sections(), materialization)
    }

    /// Return the canonical layout for one type.
    pub fn layout(&self, ty: TypeId) -> Option<&Layout> {
        let sections = self.sections();
        let layout_id = self.types().layout_id(sections, ty)?;

        self.layouts().get(sections, layout_id)
    }

    /// Return one field by layout index.
    pub fn layout_field_at(&self, layout: &Layout, index: u32) -> Option<&LayoutField> {
        self.layouts.field_at(self.sections(), layout, index)
    }

    /// Return the field count for one layout when it is field-addressable.
    pub fn layout_field_count(&self, layout: &Layout) -> Option<usize> {
        self.layouts.field_count(layout)
    }

    /// Return all fields for one layout when it is field-addressable.
    pub fn layout_fields(&self, layout: &Layout) -> &[LayoutField] {
        self.layouts.fields(self.sections(), layout)
    }

    /// Return the layout id for one type.
    pub fn layout_id_for_type(&self, ty: TypeId) -> Option<LayoutId> {
        self.types().layout_id(self.sections(), ty)
    }

    /// Return whether one type is stored in one VM cell.
    pub fn is_cell_type(&self, ty: TypeId) -> bool {
        let Some(layout) = self.cell_layout(ty) else {
            return false;
        };

        layout.byte_len(self.pointer_bytes() as usize) <= vm::Cell::BYTE_LEN
    }

    /// Return the native cell layout for one type.
    pub fn cell_layout(&self, ty: TypeId) -> Option<CellLayout> {
        let layout = self.layout(ty)?;

        layout.cell_layout()
    }

    /// Return the scalar layout for one type.
    pub fn scalar_format(&self, ty: TypeId) -> Option<ScalarFormat> {
        let layout = self.layout(ty)?;

        layout.scalar_format()
    }

    /// Return the environment cell layout for one function closure.
    pub fn function_environment_layout(&self, function: FunctionId) -> Option<CellLayout> {
        let sections = self.sections();
        let function = self.functions().get(sections, function)?;
        let environment = function.environment()?;

        self.cell_layout(environment)
    }

    /// Return whether one frame slot is stored in one VM cell.
    pub fn frame_slot_is_cell(&self, slot: &FrameSlot) -> bool {
        self.is_cell_type(slot.ty)
    }

    /// Return the byte width for one type in a VM frame.
    pub fn type_byte_len(&self, ty: TypeId) -> Option<usize> {
        if let Some(layout) = self.cell_layout(ty) {
            return Some(layout.byte_len(self.pointer_bytes() as usize));
        }

        self.layout(ty).map(|layout| layout.size as usize)
    }

    /// Return whether one scalar type carries a worker heap root.
    pub fn is_local_root_type(&self, ty: TypeId) -> Result<bool> {
        Ok(matches!(
            self.scalar_heap_edge(ty, vm::Cell::ZERO)?,
            Some(HeapEdge::Local(_))
        ))
    }

    /// Return whether one scalar type carries a shared heap root.
    pub fn is_shared_root_type(&self, ty: TypeId) -> Result<bool> {
        Ok(matches!(
            self.scalar_heap_edge(ty, vm::Cell::ZERO)?,
            Some(HeapEdge::Shared(_))
        ))
    }

    /// Visit mutable heap root slots from one byte range.
    pub fn visit_byte_root_slots(
        &self,
        ty: TypeId,
        bytes: &mut [u8],
        visit: &mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>,
    ) -> Result<()> {
        let layout = self.layout(ty).ok_or_else(|| {
            Error::internal(format!("missing layout for byte heap roots: type={ty:?}"))
        })?;

        let layout_bytes = layout.size as usize;
        if bytes.len() != layout_bytes {
            return Err(Error::internal(format!(
                "byte heap root length mismatch: type={ty:?}, bytes={}, layout_bytes={}",
                bytes.len(),
                layout_bytes,
            )));
        }

        let trace_map = self.trace_map(layout.trace)?;

        visit_heap_root_slots(&trace_map, 0, bytes, ReferenceRange::All, visit)
            .map_err(|error| Error::internal(error.to_string()))
    }

    /// Visit mutable heap root slots from one static space.
    pub fn visit_static_root_slots(
        &self,
        location: GlobalLocation,
        static_space: &mut StaticSpace,
        visit: &mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>,
    ) -> Result<()> {
        let sections = self.sections();
        let globals = self
            .globals
            .iter_location(sections, location)
            .map(|(global_id, global)| (global_id, *global))
            .collect::<Vec<_>>();

        // visit each static global
        for (global_id, global) in globals {
            let bytes = static_space
                .bytes_mut(&global)
                .ok_or_else(|| Error::internal(format!("missing global bytes {global_id:?}")))?;
            self.visit_byte_root_slots(global.ty, bytes, visit)?;
        }

        Ok(())
    }

    /// Return the heap edge carried by one scalar value.
    fn scalar_heap_edge(&self, ty: TypeId, value: vm::Cell) -> Result<Option<HeapEdge>> {
        if !self.is_cell_type(ty) {
            return Ok(None);
        }

        let bits = value.bits() as usize;

        let Some(LayoutShape::Reference(reference)) = self.layout(ty).map(|layout| &layout.shape)
        else {
            return Ok(None);
        };
        let Some(ReferenceKind::Managed | ReferenceKind::Unique | ReferenceKind::Borrowed) =
            reference.flags.kind()
        else {
            return Ok(None);
        };

        // null references are not roots
        if bits == 0 {
            return Ok(None);
        }

        match reference.address_space() {
            Some(AddressSpace::Local) => Ok(Some(HeapEdge::Local(HeapReference::from_bits(bits)))),
            Some(AddressSpace::Shared) => {
                Ok(Some(HeapEdge::Shared(SharedHeapReference::from_bits(bits))))
            }
            _ => Ok(None),
        }
    }

    /// Return the program point for one resume state.
    pub fn point_for_frame_state(&self, frame_state: FrameStateId) -> Option<ProgramPoint> {
        self.resume()
            .state(self.sections(), frame_state)
            .map(|state| state.point)
    }

    /// Return the source instruction point for one resume state.
    pub fn source_point_for_frame_state(&self, frame_state: FrameStateId) -> Option<u32> {
        self.resume()
            .state(self.sections(), frame_state)
            .and_then(|state| state.source_point.get())
    }

    /// Return the frame entry code for one resume state.
    pub fn frame_entry(&self, frame_state: FrameStateId) -> Option<vm::FrameEntryCode<'_>> {
        let sections = self.sections();
        let state = self.resume().state(sections, frame_state)?;

        self.resume().entry(sections, state)
    }

    /// Return one resume state id for one program point.
    pub fn frame_state_at(&self, point: ProgramPoint) -> Option<FrameStateId> {
        self.resume().state_id_at(self.sections(), point)
    }

    /// Return the caller return destination implied by one program point.
    pub fn return_destination_at(&self, point: ProgramPoint) -> Result<Option<vm::MoveSlot>> {
        let sections = self.sections();
        let Some(frame_state) = self.resume().state_id_at(sections, point) else {
            return Ok(None);
        };

        let destination = self
            .resume()
            .state(sections, frame_state)
            .and_then(|state| state.return_destination.get());

        Ok(destination)
    }

    /// Return the frame layout for one function when present.
    pub fn frame_layout(&self, function: FunctionId) -> Option<&FrameLayout> {
        let function = self.vm_function_by_id(function)?;

        self.frame_layout_by_id(function.function.frame_layout)
    }
}
