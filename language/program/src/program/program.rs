use destack_core::{SectionImage, SectionStorage, StringId};
use destack_heap::{
    AllocationShape, HeapEdge, HeapOptions, HeapReference, HeapResult, ReferenceRange, RootSlot,
    SharedHeapOptions, SharedHeapReference, TraceView, visit_heap_root_slots,
};
use destack_mir::{ReferenceKind, TargetLayout, TraceId, TraceMap};
use destack_source::ContentId;

use crate::{
    AddressSpace, CellLayout, DispatchTable, FrameLayout, FrameLayoutId, FrameMaterialization,
    FrameSlot, FrameSlotId, FrameStateId, Function, FunctionId, FunctionSignature, FunctionTable,
    Global, GlobalAddress, GlobalId, GlobalLocation, Layout, LayoutField, LayoutId, LayoutShape,
    LayoutTable, ProgramInfo, ScalarFormat, Signature, StaticImage, StaticSpace, StringTable,
    TraceCache, TraceTable, TypeId, TypeTable, native, vm,
};
use vm::error::{Error, Result};

use super::{ProgramHeader, ProgramLoadError};

/// Program produced by the toolchain.
#[derive(Debug, Clone)]
pub struct Program {
    /// Root program descriptor.
    header: ProgramHeader,
    /// Program section storage.
    storage: SectionStorage,
    /// Decoded trace maps for heap and GC paths.
    traces: TraceCache,
}

impl Program {
    /// Create one program from its root descriptor and section storage.
    pub fn new(
        header: ProgramHeader,
        storage: SectionStorage,
    ) -> std::result::Result<Self, ProgramLoadError> {
        let sections = SectionImage::load(&header.sections, &storage)?;
        let trace_cache = TraceCache::decode(&header.traces, sections)?;

        Ok(Self {
            header,
            storage,
            traces: trace_cache,
        })
    }

    /// Return the root program descriptor.
    pub(crate) fn header(&self) -> &ProgramHeader {
        &self.header
    }

    /// Return all content ids referenced by this program.
    pub fn content_ids(&self) -> Vec<ContentId> {
        let mut ids = self.header.vm.content_ids();

        if let Some(native) = &self.header.native {
            ids.extend(native.content_ids());
        }

        ids
    }

    /// Return runtime type table.
    pub fn types(&self) -> &TypeTable {
        &self.header.types
    }

    /// Return the target ABI layout used by this program.
    pub fn target_layout(&self) -> TargetLayout {
        self.header.target_layout
    }

    /// Return the pointer byte width used by this program.
    pub fn pointer_bytes(&self) -> u8 {
        self.target_layout().pointer_bytes()
    }

    /// Return local heap options required by this program.
    pub fn heap_options(&self) -> &HeapOptions {
        &self.header.local_heap
    }

    /// Return shared heap options required by this program.
    pub fn shared_heap_options(&self) -> &SharedHeapOptions {
        &self.header.shared_heap
    }

    /// Return the program function table.
    pub fn functions(&self) -> &FunctionTable {
        &self.header.functions
    }

    /// Return a read-only view of program sections.
    pub fn sections(&self) -> SectionImage<'_> {
        // SAFETY: Program::new loads and checks the section image before storing it.
        unsafe { SectionImage::new_unchecked(&self.header.sections, &self.storage) }
    }

    /// Return one program function record.
    pub fn function(&self, function: FunctionId) -> Option<&Function> {
        self.header.functions.get(self.sections(), function)
    }

    /// Return parameter types for one program function.
    pub fn function_parameters(&self, function: FunctionId) -> Option<&[TypeId]> {
        let sections = self.sections();

        self.header
            .functions
            .get(sections, function)
            .map(|function| self.header.functions.parameters(sections, function))
    }

    /// Check that one function matches one signature type.
    pub fn check_function_signature(
        &self,
        function: FunctionId,
        signature: &Signature,
    ) -> Result<()> {
        self.header
            .functions
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

        self.header
            .functions
            .check_signature_entry(sections, function, signature, parameters)
    }

    /// Return runtime dispatch table.
    pub fn dispatch(&self) -> &DispatchTable {
        &self.header.dispatch
    }

    /// Return whether one concrete type satisfies one runtime type.
    pub fn is_subtype(&self, concrete: TypeId, expected: TypeId) -> Result<bool> {
        self.header
            .types
            .is_subtype(self.sections(), concrete, expected)
            .ok_or_else(|| {
                Error::internal(format!("missing runtime type {concrete:?} or {expected:?}"))
            })
    }

    /// Return one program global by id.
    pub fn global(&self, global: GlobalId) -> Option<&Global> {
        self.header.globals.get(self.sections(), global)
    }

    /// Return immutable constant storage owned by this program.
    pub fn constants(&self) -> &StaticImage {
        &self.header.constant_space
    }

    /// Return native code when this program carries it.
    pub fn native_code(&self) -> Option<&native::Code> {
        self.header.native.as_ref()
    }

    /// Return initial shared static storage for new runtimes.
    pub fn shared_statics(&self) -> &StaticImage {
        &self.header.shared_static_space
    }

    /// Return initial local static storage for new workers.
    pub fn local_statics(&self) -> &StaticImage {
        &self.header.local_static_space
    }

    /// Materialize initial shared static storage.
    pub fn materialize_shared_statics(&self) -> StaticSpace {
        self.header.shared_static_space.materialize(self.sections())
    }

    /// Materialize initial local static storage.
    pub fn materialize_local_statics(&self) -> StaticSpace {
        self.header.local_static_space.materialize(self.sections())
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
        self.header
            .functions
            .id_by_name(self.sections(), StringId::for_text(name))
    }

    /// Return the program string table.
    pub fn strings(&self) -> &StringTable {
        &self.header.strings
    }

    /// Return source reflection when this program ships it.
    pub fn info(&self) -> Option<&ProgramInfo> {
        self.header.info.as_ref()
    }

    /// Return one program string by stable id when present.
    pub fn string(&self, id: StringId) -> Option<&str> {
        self.header.strings.string(self.sections(), id)
    }

    /// Return runtime layouts for this program.
    pub fn layouts(&self) -> &LayoutTable {
        &self.header.layouts
    }

    /// Return the heap allocation shape for one layout id.
    pub fn allocation_shape(&self, layout_id: LayoutId) -> Result<AllocationShape<'_>> {
        let sections = self.sections();
        let Some(layout) = self.layouts().get(sections, layout_id) else {
            return Err(Error::internal(format!(
                "missing program layout {layout_id:?}"
            )));
        };

        let trace_map = self.trace_map(layout.trace)?;
        let trace_id = trace_map.has_reference().then_some(layout.trace);

        Ok(AllocationShape::new(
            layout.size as usize,
            layout.alignment as usize,
            trace_id,
            trace_map,
        ))
    }

    /// Borrow one program trace map.
    pub fn trace_map(&self, id: TraceId) -> Result<&TraceMap> {
        self.traces
            .trace(id)
            .ok_or_else(|| Error::internal(format!("missing program trace map {id:?}")))
    }

    /// Return decoded program trace maps.
    pub fn trace_maps(&self) -> TraceView<'_> {
        self.traces.maps()
    }

    /// Return the program trace table.
    pub fn trace_table(&self) -> &TraceTable {
        &self.header.traces
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

        self.header
            .constant_space
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

        self.header
            .constant_space
            .owns_address_range(sections, global, address, byte_len)
    }

    /// Return VM resume states.
    pub fn resume(&self) -> &vm::ResumeTable {
        self.header.vm.resume()
    }

    /// Return the lowered VM functions.
    pub fn vm_functions(&self) -> &vm::FunctionTable {
        self.header.vm.functions()
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
        self.header.vm.side_table()
    }

    /// Return the frame layout for one layout id when present.
    pub fn frame_layout_by_id(&self, frame_layout: FrameLayoutId) -> Option<&FrameLayout> {
        self.header.frames.layout(self.sections(), frame_layout)
    }

    /// Return the single frame materialization for one resume state.
    pub fn frame_materialization(
        &self,
        frame_state: FrameStateId,
    ) -> Option<&FrameMaterialization> {
        self.header
            .frames
            .materialization(self.sections(), frame_state)
    }

    /// Return one frame slot by id.
    pub fn frame_slot(&self, layout: &FrameLayout, slot: FrameSlotId) -> Option<&FrameSlot> {
        self.header.frames.slot(self.sections(), layout, slot)
    }

    /// Return all value slots for one frame layout.
    pub fn frame_value_slots<'a>(&'a self, layout: &FrameLayout) -> &'a [FrameSlot] {
        let sections = self.sections();
        let slots = self.header.frames.slots(sections, layout);

        layout.values(slots)
    }

    /// Return all local slots for one frame layout.
    pub fn frame_local_slots<'a>(&'a self, layout: &FrameLayout) -> &'a [FrameSlot] {
        let sections = self.sections();
        let slots = self.header.frames.slots(sections, layout);

        layout.locals(slots)
    }

    /// Return one local slot for one frame layout.
    pub fn frame_local_slot(&self, layout: &FrameLayout, local: u32) -> Option<&FrameSlot> {
        let sections = self.sections();
        let slots = self.header.frames.slots(sections, layout);

        layout.local(slots, local)
    }

    /// Return the closure environment slot for one frame layout.
    pub fn frame_environment_slot<'a>(&'a self, layout: &FrameLayout) -> Option<&'a FrameSlot> {
        let sections = self.sections();
        let slots = self.header.frames.slots(sections, layout);

        layout.environment(slots)
    }

    /// Return copied frame slots for one materialization.
    pub fn frame_copied_slots<'a>(
        &'a self,
        materialization: &'a FrameMaterialization,
    ) -> &'a [FrameSlotId] {
        self.header
            .frames
            .copied_slots(self.sections(), materialization)
    }

    /// Convert one current frame location into one lowered program point.
    pub fn point(&self, function: FunctionId, block: u32, pc: u32) -> vm::ProgramPoint {
        vm::ProgramPoint::new(function, block, pc)
    }

    /// Return the canonical layout for one type.
    pub fn layout(&self, ty: TypeId) -> Option<&Layout> {
        let sections = self.sections();
        let layout_id = self.types().layout_id(sections, ty)?;

        self.layouts().get(sections, layout_id)
    }

    /// Return one field by layout index.
    pub fn layout_field_at(&self, layout: &Layout, index: u32) -> Option<&LayoutField> {
        self.header.layouts.field_at(self.sections(), layout, index)
    }

    /// Return the field count for one layout when it is field-addressable.
    pub fn layout_field_count(&self, layout: &Layout) -> Option<usize> {
        self.header.layouts.field_count(layout)
    }

    /// Return all fields for one layout when it is field-addressable.
    pub fn layout_fields(&self, layout: &Layout) -> &[LayoutField] {
        self.header.layouts.fields(self.sections(), layout)
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

        visit_heap_root_slots(trace_map, 0, bytes, ReferenceRange::All, visit)
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
            .header
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

    /// Return the lowered program point for one resume state.
    pub fn point_for_frame_state(&self, frame_state: FrameStateId) -> Option<vm::ProgramPoint> {
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

    /// Return one resume state id for one lowered program point.
    pub fn frame_state_at(&self, point: vm::ProgramPoint) -> Option<FrameStateId> {
        self.resume().state_id_at(self.sections(), point)
    }

    /// Return the caller return destination implied by one lowered program point.
    pub fn return_destination_at(&self, point: vm::ProgramPoint) -> Result<Option<vm::MoveSlot>> {
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
