use destack_serde::Reflect;
use std::sync::Arc;

use destack_heap as heap;
use destack_heap::{
    HeapEdge, HeapReference, HeapResult, ReferenceRange, RootSlot, SharedHeapReference,
    visit_heap_root_slots,
};
use destack_mir as mir;
use destack_mir::LayoutId;
use serde::{Deserialize, Serialize};

use crate::{
    EntryPoint, FunctionId, FunctionTable, StaticAddress, StaticId, StaticSpace, TypeId, TypeTable,
    native, vm,
};
use vm::error::{Error, Result};
use vm::{CellLayout, FrameEntry, ProgramPoint, ResumeTable, SideTable, ValueShape};

use super::ProgramHeader;

/// Durable executable program produced by the toolchain.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct Program {
    /// Serialized program compatibility header.
    pub header: ProgramHeader,

    /// Runtime type metadata.
    pub types: TypeTable,
    /// Runtime layouts keyed by layout id.
    pub layouts: mir::LayoutTable,
    /// Runtime frame metadata.
    pub frames: mir::FrameMetadata,
    /// Runtime function metadata.
    pub functions: FunctionTable,
    /// Canonical trace table used by heap metadata.
    pub traces: Arc<mir::TraceTable>,

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
    /// Create one durable program.
    pub fn new(
        header: ProgramHeader,
        types: TypeTable,
        layouts: mir::LayoutTable,
        frames: mir::FrameMetadata,
        functions: FunctionTable,
        constant_space: StaticSpace,
        shared_static_space: StaticSpace,
        local_static_space: StaticSpace,
        traces: Arc<mir::TraceTable>,
        vm: vm::Code,
        native: Option<native::Code>,
    ) -> Self {
        Self {
            header,
            types,
            layouts,
            frames,
            functions,
            constant_space,
            shared_static_space,
            local_static_space,
            traces,
            vm,
            native,
        }
    }

    /// Return all content ids referenced by this program.
    pub fn content_ids(&self) -> Vec<destack_source::ContentId> {
        let mut ids = self.vm.content_ids();

        if let Some(native) = &self.native {
            ids.extend(native.content_ids());
        }

        ids
    }

    /// Return runtime type metadata.
    pub fn types(&self) -> &TypeTable {
        &self.types
    }

    /// Return the pointer byte width used by this program.
    pub const fn pointer_bytes(&self) -> u8 {
        self.header.pointer_bytes
    }

    /// Return local heap options required by this program.
    pub fn heap_options(&self) -> &heap::HeapOptions {
        &self.header.local_heap
    }

    /// Return shared heap options required by this program.
    pub fn shared_heap_options(&self) -> &heap::SharedHeapOptions {
        &self.header.shared_heap
    }

    /// Return runtime function metadata.
    pub fn functions(&self) -> &FunctionTable {
        &self.functions
    }

    /// Return immutable constant storage owned by this program.
    pub fn constants(&self) -> &StaticSpace {
        &self.constant_space
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

    /// Resolve one execution entry into its function id.
    pub fn function_for_entry(&self, entry: EntryPoint) -> FunctionId {
        entry.function()
    }

    /// Resolve one function id by source name.
    pub fn function_id_by_name(&self, name: &str) -> Option<FunctionId> {
        self.functions.id_by_name(name)
    }

    /// Return the MIR layouts for this program.
    pub fn layouts(&self) -> &mir::LayoutTable {
        &self.layouts
    }

    /// Return the heap allocation shape for one layout id.
    pub fn allocation_shape(&self, layout_id: LayoutId) -> Result<heap::AllocationShape<'_>> {
        let Some(layout) = self.layouts().entries.get(layout_id.index()) else {
            return Err(Error::internal(format!("missing MIR layout {layout_id:?}")));
        };

        if layout.trace_map.has_reference() {
            let Some(trace_id) = self.traces.id(&layout.trace_map) else {
                return Err(Error::internal(format!(
                    "missing MIR trace map for layout {layout_id:?}"
                )));
            };

            return Ok(heap::AllocationShape::new(
                layout.size as usize,
                layout.alignment as usize,
                Some(trace_id),
                &layout.trace_map,
            ));
        }

        Ok(heap::AllocationShape::new(
            layout.size as usize,
            layout.alignment as usize,
            None,
            &layout.trace_map,
        ))
    }

    /// Borrow one program trace map.
    pub fn trace_map(&self, id: mir::TraceId) -> Result<&mir::TraceMap> {
        self.traces
            .trace(id)
            .ok_or_else(|| Error::internal(format!("missing program trace map {id:?}")))
    }

    /// Return the canonical program trace table.
    pub fn trace_table(&self) -> &mir::TraceTable {
        self.traces.as_ref()
    }

    /// Return the canonical program trace table handle.
    pub fn trace_table_handle(&self) -> Arc<mir::TraceTable> {
        self.traces.clone()
    }

    /// Return the constant address for one static id.
    pub fn static_address(&self, id: StaticId) -> Option<StaticAddress> {
        self.constant_space.address(id)
    }

    /// Return VM resume states.
    pub fn resume(&self) -> &ResumeTable {
        self.vm.resume()
    }

    /// Return the lowered VM functions.
    pub fn vm_functions(&self) -> &vm::FunctionTable {
        self.vm.functions()
    }

    /// Return the compact VM side table.
    pub fn side_table(&self) -> &SideTable {
        self.vm.side_table()
    }

    /// Return the frame layout for one layout id when present.
    pub fn frame_layout_by_id(
        &self,
        frame_layout: mir::FrameLayoutId,
    ) -> Option<&mir::FrameLayout> {
        self.frames.layout(frame_layout)
    }

    /// Return the single frame materialization for one resume state.
    pub fn frame_materialization(
        &self,
        frame_state: mir::FrameStateId,
    ) -> Option<&mir::FrameMaterialization> {
        self.frames.materialization(frame_state)
    }

    /// Convert one current frame location into one lowered program point.
    pub fn point(&self, function: FunctionId, block: u32, pc: u32) -> ProgramPoint {
        ProgramPoint::new(function, block, pc)
    }

    /// Return the canonical layout for one type.
    pub fn layout(&self, ty: TypeId) -> Option<&mir::Layout> {
        let layout_id = self.types.layout_id(ty)?;

        Some(self.layouts.layout(layout_id))
    }

    /// Return the layout id for one type.
    pub fn layout_id_for_type(&self, ty: TypeId) -> Option<LayoutId> {
        self.types.layout_id(ty)
    }

    /// Return whether one type is stored in one VM cell.
    pub fn is_cell_type(&self, ty: TypeId) -> bool {
        self.types.is_cell_type(ty, self.pointer_bytes())
    }

    /// Return the native cell layout for one type.
    pub fn cell_layout(&self, ty: TypeId) -> Option<CellLayout> {
        self.types.cell_layout(ty, self.pointer_bytes())
    }

    /// Return the runtime value shape for one type.
    pub fn value_shape(&self, ty: TypeId) -> Option<ValueShape> {
        self.types.value_shape(ty, self.pointer_bytes())
    }

    /// Return whether one frame slot is stored in one VM cell.
    pub fn frame_slot_is_cell(&self, slot: &mir::FrameSlot) -> bool {
        self.is_cell_type(self.types.type_id(slot.ty))
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

        visit_heap_root_slots(&layout.trace_map, 0, bytes, ReferenceRange::All, visit)
            .map_err(|error| Error::internal(error.to_string()))
    }

    /// Visit mutable heap root slots from one static space.
    pub fn visit_static_root_slots(
        &self,
        static_space: &mut StaticSpace,
        visit: &mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>,
    ) -> Result<()> {
        let static_ids = static_space.ids().collect::<Vec<_>>();

        // static regions
        for id in static_ids {
            let region = static_space
                .region(id)
                .cloned()
                .ok_or_else(|| Error::internal(format!("missing static region {id:?}")))?;
            let bytes = static_space
                .bytes_mut(id)
                .ok_or_else(|| Error::internal(format!("missing static bytes {id:?}")))?;
            self.visit_byte_root_slots(region.ty, bytes, visit)?;
        }

        Ok(())
    }

    /// Return the heap edge carried by one scalar value.
    fn scalar_heap_edge(&self, ty: TypeId, value: vm::Cell) -> Result<Option<HeapEdge>> {
        if !self.is_cell_type(ty) {
            return Ok(None);
        }

        let repr_ty = self.types.repr_type(ty);
        let bits = value.bits() as usize;

        match self.types.get(repr_ty) {
            Some(mir::Type::Reference {
                kind:
                    mir::ReferenceKind::Managed
                    | mir::ReferenceKind::Unique
                    | mir::ReferenceKind::Borrowed,
                space: mir::Space::Local,
                ..
            }) if bits == 0 => Ok(None),
            Some(mir::Type::Reference {
                kind:
                    mir::ReferenceKind::Managed
                    | mir::ReferenceKind::Unique
                    | mir::ReferenceKind::Borrowed,
                space: mir::Space::Local,
                ..
            }) => Ok(Some(HeapEdge::Local(HeapReference::from_bits(bits)))),
            Some(mir::Type::Reference {
                kind:
                    mir::ReferenceKind::Managed
                    | mir::ReferenceKind::Unique
                    | mir::ReferenceKind::Borrowed,
                space: mir::Space::Shared,
                ..
            }) if bits == 0 => Ok(None),
            Some(mir::Type::Reference {
                kind:
                    mir::ReferenceKind::Managed
                    | mir::ReferenceKind::Unique
                    | mir::ReferenceKind::Borrowed,
                space: mir::Space::Shared,
                ..
            }) => Ok(Some(HeapEdge::Shared(SharedHeapReference::from_bits(bits)))),
            _ => Ok(None),
        }
    }

    /// Return the lowered program point for one resume state.
    pub fn point_for_frame_state(&self, frame_state: mir::FrameStateId) -> Option<ProgramPoint> {
        self.resume().state(frame_state).map(|state| state.point)
    }

    /// Return the source MIR point for one resume state.
    pub fn mir_point_for_frame_state(&self, frame_state: mir::FrameStateId) -> Option<u32> {
        self.resume()
            .state(frame_state)
            .map(|state| state.mir_point)
    }

    /// Return the entry data for one resume state.
    pub fn frame_entry(&self, frame_state: mir::FrameStateId) -> Option<&FrameEntry> {
        self.resume()
            .state(frame_state)
            .and_then(|state| state.entry.as_ref())
    }

    /// Return one resume state id for one lowered program point.
    pub fn frame_state_at(&self, point: ProgramPoint) -> Option<mir::FrameStateId> {
        self.resume().state_id_at(point)
    }

    /// Return the caller return destination implied by one lowered program point.
    pub fn return_destination_at(&self, point: ProgramPoint) -> Result<Option<mir::Value>> {
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
    pub fn frame_layout(&self, function: FunctionId) -> Option<&mir::FrameLayout> {
        let function = self.vm_functions().function_by_id(function)?;

        self.frame_layout_by_id(function.frame_layout)
    }
}
