use destack_serde::Reflect;

use destack_heap::{
    HeapEdge, HeapOptions, HeapReference, HeapResult, PayloadShape, ReferenceRange, RootSlot,
    SharedHeapOptions, SharedHeapReference, visit_heap_root_slots,
};
use destack_mir::{ReferenceKind, TargetLayout, TraceId, TraceMap, TraceTable};
use destack_source::ContentId;
use serde::{Deserialize, Serialize};

use crate::{
    AddressSpace, CellLayout, DispatchTable, FrameLayout, FrameLayoutId, FrameMaterialization,
    FrameSlot, FrameStateId, FrameTable, FunctionId, FunctionTable, GlobalAddress, GlobalId,
    Layout, LayoutId, LayoutShape, LayoutTable, ProgramInfo, ScalarFormat, StaticSpace, TypeId,
    TypeTable, native, vm,
};
use vm::error::{Error, Result};

use super::ProgramHeader;

/// Executable program produced by the toolchain.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct Program {
    /// Serialized program compatibility header.
    pub header: ProgramHeader,

    /// Runtime type table.
    pub types: TypeTable,
    /// Runtime layouts keyed by layout id.
    pub layouts: LayoutTable,
    /// Runtime frame table.
    pub frames: FrameTable,
    /// Executable function table.
    pub functions: FunctionTable,
    /// Runtime dispatch table.
    pub dispatch: DispatchTable,
    /// Canonical trace table used by heap tables.
    pub traces: TraceTable,
    /// Reflectable program table.
    pub info: ProgramInfo,

    /// Immutable constant storage owned by this program.
    pub constant_space: StaticSpace,
    /// Initial shared static storage for each runtime.
    pub shared_static_space: StaticSpace,
    /// Initial local static storage for each worker.
    pub local_static_space: StaticSpace,

    /// VM code used for interpretation, deoptimization, and continuation resume.
    pub vm: vm::Code,
    /// Native code used as optional acceleration.
    pub native: Option<native::Code>,
}

impl Program {
    /// Create one executable program from its durable image parts.
    pub fn new(
        header: ProgramHeader,
        types: TypeTable,
        layouts: LayoutTable,
        frames: FrameTable,
        functions: FunctionTable,
        dispatch: DispatchTable,
        traces: TraceTable,
        info: ProgramInfo,
        constant_space: StaticSpace,
        shared_static_space: StaticSpace,
        local_static_space: StaticSpace,
        vm: vm::Code,
        native: Option<native::Code>,
    ) -> Self {
        Self {
            header,
            types,
            layouts,
            frames,
            functions,
            dispatch,
            traces,
            info,
            constant_space,
            shared_static_space,
            local_static_space,
            vm,
            native,
        }
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

    /// Return executable function table.
    pub fn functions(&self) -> &FunctionTable {
        &self.functions
    }

    /// Return runtime dispatch table.
    pub fn dispatch(&self) -> &DispatchTable {
        &self.dispatch
    }

    /// Return whether one concrete type satisfies one runtime type.
    pub fn is_subtype(&self, concrete: TypeId, expected: TypeId) -> bool {
        self.types.is_subtype(concrete, expected)
    }

    /// Return immutable constant storage owned by this program.
    pub fn constants(&self) -> &StaticSpace {
        &self.constant_space
    }

    /// Return native code when this program carries it.
    pub fn native_code(&self) -> Option<&native::Code> {
        self.native.as_ref()
    }

    /// Return initial shared static storage for new runtimes.
    pub fn shared_statics(&self) -> &StaticSpace {
        &self.shared_static_space
    }

    /// Return initial local static storage for new workers.
    pub fn local_statics(&self) -> &StaticSpace {
        &self.local_static_space
    }

    /// Initialize runtime-owned static storage from this program.
    pub fn initialize_statics(
        &self,
        local_static: &mut StaticSpace,
        shared_static: &mut StaticSpace,
    ) {
        if shared_static.is_empty() {
            shared_static.clone_from(self.shared_statics());
        }

        local_static.clone_from(self.local_statics());
    }

    /// Resolve one function id by source name.
    pub fn function_id_by_name(&self, name: &str) -> Option<FunctionId> {
        self.functions.id_by_name(name)
    }

    /// Return runtime layouts for this program.
    pub fn layouts(&self) -> &LayoutTable {
        &self.layouts
    }

    /// Return the heap allocation shape for one layout id.
    pub fn allocation_shape(&self, layout_id: LayoutId) -> Result<PayloadShape<'_>> {
        let Some(layout) = self.layouts().get(layout_id) else {
            return Err(Error::internal(format!(
                "missing program layout {layout_id:?}"
            )));
        };

        let trace_map = self.trace_map(layout.trace)?;
        let trace_id = trace_map.has_reference().then_some(layout.trace);

        Ok(PayloadShape::new(
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

    /// Return the canonical program trace table.
    pub fn trace_table(&self) -> &TraceTable {
        &self.traces
    }

    /// Return the constant address for one global.
    pub fn global_address(&self, global: GlobalId) -> Option<GlobalAddress> {
        self.constant_space.address(global)
    }

    /// Return VM resume states.
    pub fn resume(&self) -> &vm::ResumeTable {
        self.vm.resume()
    }

    /// Return the lowered VM functions.
    pub fn vm_functions(&self) -> &vm::FunctionTable {
        self.vm.functions()
    }

    /// Return the compact VM side table.
    pub fn side_table(&self) -> &vm::SideTable {
        self.vm.side_table()
    }

    /// Return the frame layout for one layout id when present.
    pub fn frame_layout_by_id(&self, frame_layout: FrameLayoutId) -> Option<&FrameLayout> {
        self.frames.layout(frame_layout)
    }

    /// Return the single frame materialization for one resume state.
    pub fn frame_materialization(
        &self,
        frame_state: FrameStateId,
    ) -> Option<&FrameMaterialization> {
        self.frames.materialization(frame_state)
    }

    /// Convert one current frame location into one lowered program point.
    pub fn point(&self, function: FunctionId, block: u32, pc: u32) -> vm::ProgramPoint {
        vm::ProgramPoint::new(function, block, pc)
    }

    /// Return the canonical layout for one type.
    pub fn layout(&self, ty: TypeId) -> Option<&Layout> {
        let layout_id = self.types().layout_id(ty)?;

        Some(self.layouts().layout(layout_id))
    }

    /// Return the layout id for one type.
    pub fn layout_id_for_type(&self, ty: TypeId) -> Option<LayoutId> {
        self.types().layout_id(ty)
    }

    /// Return whether one type is stored in one VM cell.
    pub fn is_cell_type(&self, ty: TypeId) -> bool {
        self.cell_layout(ty)
            .map(|layout| layout.byte_len(self.pointer_bytes() as usize) <= vm::Cell::BYTE_LEN)
            .unwrap_or(false)
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
        let function = self.functions().get(function)?;
        let environment = function.environment?;

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
        static_space: &mut StaticSpace,
        visit: &mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>,
    ) -> Result<()> {
        let globals = static_space.globals().collect::<Vec<_>>();

        // visit each global region
        for global in globals {
            let region = static_space
                .region(global)
                .cloned()
                .ok_or_else(|| Error::internal(format!("missing global region {global:?}")))?;
            let bytes = static_space
                .bytes_mut(global)
                .ok_or_else(|| Error::internal(format!("missing global bytes {global:?}")))?;
            self.visit_byte_root_slots(region.ty, bytes, visit)?;
        }

        Ok(())
    }

    /// Return the heap edge carried by one scalar value.
    fn scalar_heap_edge(&self, ty: TypeId, value: vm::Cell) -> Result<Option<HeapEdge>> {
        if !self.is_cell_type(ty) {
            return Ok(None);
        }

        let bits = value.bits() as usize;

        let ty = self
            .types()
            .repr_type(ty)
            .ok_or_else(|| Error::internal(format!("missing program type {ty:?}")))?;
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
        self.resume().state(frame_state).map(|state| state.point)
    }

    /// Return the source instruction point for one resume state.
    pub fn source_point_for_frame_state(&self, frame_state: FrameStateId) -> Option<u32> {
        self.resume()
            .state(frame_state)
            .and_then(|state| state.source_point)
    }

    /// Return the entry data for one resume state.
    pub fn frame_entry(&self, frame_state: FrameStateId) -> Option<&vm::FrameEntry> {
        self.resume()
            .state(frame_state)
            .and_then(|state| state.entry.as_ref())
    }

    /// Return one resume state id for one lowered program point.
    pub fn frame_state_at(&self, point: vm::ProgramPoint) -> Option<FrameStateId> {
        self.resume().state_id_at(point)
    }

    /// Return the caller return destination implied by one lowered program point.
    pub fn return_destination_at(&self, point: vm::ProgramPoint) -> Result<Option<vm::MoveSlot>> {
        let Some(frame_state) = self.resume().state_id_at(point) else {
            return Ok(None);
        };

        let destination = self
            .resume()
            .state(frame_state)
            .and_then(|state| state.return_destination);

        Ok(destination)
    }

    /// Return the frame layout for one function when present.
    pub fn frame_layout(&self, function: FunctionId) -> Option<&FrameLayout> {
        let function = self.vm_functions().function_by_id(function)?;

        self.frame_layout_by_id(function.frame_layout)
    }
}
