use std::collections::HashMap;
use std::sync::Arc;

use destack_core::StringPool;
use destack_heap as heap;
use destack_heap::{
    HeapEdge, HeapReference, HeapResult, ReferenceRange, RootSlot, SharedHeapReference,
    visit_heap_root_slots,
};
use destack_mir as mir;
use destack_mir::{LayoutId, LayoutTable};
use destack_source::ContentId;
use serde::{Deserialize, Serialize};

use crate::{
    EntryPoint, FrameLayout, FrameLayoutId, FrameMaterialization, FrameStateId, ProgramLayout,
    StaticAddress, StaticId, StaticSpace, StorageLayoutId, native, vm,
};
use vm::error::{Error, Result};
use vm::{
    FrameEntry, FunctionObjectLayout, FunctionTable, Layout, ProgramPoint, ResumeTable, SideTable,
    TypeTable, function_object_layout, repr_type,
};

/// Durable executable program produced by the toolchain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Program<T> {
    /// The program identity and compatibility header.
    pub header: ProgramHeader,
    /// The MIR tree executed by this program.
    pub tree: mir::Tree,
    /// The immutable string pool used by this program.
    pub string_pool: StringPool,
    /// Immutable constant storage owned by this program.
    pub constant_space: StaticSpace,
    /// Initial shared static storage for each runtime.
    pub shared_static_space: StaticSpace,
    /// Initial local static storage for each worker.
    pub local_static_space: StaticSpace,
    /// Execution-visible program layout.
    pub layout: ProgramLayout,
    /// MIR layout table used by heap metadata.
    pub layout_table: LayoutTable,
    /// Canonical trace table used by heap metadata.
    pub trace_table: Arc<mir::TraceTable>,
    /// Fast function lookup by source name.
    pub function_by_name: HashMap<String, mir::LocalNodeId<mir::Function>>,
    /// Executable material for one backend.
    pub executable: T,
}

impl<T> Program<T> {
    /// Create one durable program.
    pub fn new(
        header: ProgramHeader,
        tree: mir::Tree,
        string_pool: StringPool,
        constant_space: StaticSpace,
        shared_static_space: StaticSpace,
        local_static_space: StaticSpace,
        layout: ProgramLayout,
        layout_table: LayoutTable,
        trace_table: Arc<mir::TraceTable>,
        function_by_name: HashMap<String, mir::LocalNodeId<mir::Function>>,
        executable: T,
    ) -> Self {
        Self {
            header,
            tree,
            string_pool,
            constant_space,
            shared_static_space,
            local_static_space,
            layout,
            layout_table,
            trace_table,
            function_by_name,
            executable,
        }
    }
}

impl Program<Executable> {
    /// Return this program's executable format.
    pub fn format(&self) -> ProgramFormat {
        self.executable.format()
    }

    /// Return all content ids referenced by this program.
    pub fn content_ids(&self) -> Vec<ContentId> {
        let mut contents = Vec::new();

        // collect referenced executable content
        contents.extend(self.executable.content_ids());

        contents
    }
}

impl<T> Program<T> {
    /// Return the MIR tree executed by this program.
    pub fn tree(&self) -> &mir::Tree {
        &self.tree
    }

    /// Return the immutable string pool for this program.
    pub fn strings(&self) -> &StringPool {
        &self.string_pool
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

    /// Return execution-visible program layout metadata.
    pub fn program_layout(&self) -> &ProgramLayout {
        &self.layout
    }

    /// Resolve one execution entry into its MIR function id.
    pub fn function_for_entry(&self, entry: EntryPoint) -> mir::LocalNodeId<mir::Function> {
        mir::LocalNodeId::new(entry.index())
    }

    /// Resolve one function id by source name.
    pub fn function_id_by_name(&self, name: &str) -> Option<mir::LocalNodeId<mir::Function>> {
        self.function_by_name.get(name).copied()
    }

    /// Convert one MIR global id into one worker static id.
    pub fn static_id(&self, global: mir::LocalNodeId<mir::Global>) -> StaticId {
        StaticId(global.id)
    }

    /// Return the frame layout for one layout id when present.
    pub fn frame_layout_by_id(&self, frame_layout: FrameLayoutId) -> Option<&FrameLayout> {
        self.layout.frame_layouts.get(frame_layout.0 as usize)
    }

    /// Return the single frame materialization for one resume state.
    pub fn frame_materialization(
        &self,
        frame_state: FrameStateId,
    ) -> Option<&FrameMaterialization> {
        self.layout
            .frame_states
            .get(frame_state.0 as usize)
            .map(|state| &state.materialization)
    }

    /// Return the MIR layouts for this program.
    pub fn layouts(&self) -> &LayoutTable {
        &self.layout_table
    }

    /// Return the heap allocation shape for one layout id.
    pub fn allocation_shape(&self, layout_id: LayoutId) -> Result<heap::AllocationShape<'_>> {
        let Some(layout) = self.layout_table.layouts.get(layout_id.index()) else {
            return Err(Error::internal(format!("missing MIR layout {layout_id:?}")));
        };

        if layout.trace_map.has_reference() {
            let Some(trace_id) = self.trace_table.id(&layout.trace_map) else {
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
        self.trace_table
            .trace(id)
            .ok_or_else(|| Error::internal(format!("missing program trace map {id:?}")))
    }

    /// Return the canonical program trace table.
    pub fn trace_table(&self) -> &mir::TraceTable {
        self.trace_table.as_ref()
    }

    /// Return the canonical program trace table handle.
    pub fn trace_table_handle(&self) -> Arc<mir::TraceTable> {
        self.trace_table.clone()
    }

    /// Return the constant address for one global.
    pub fn static_address(&self, global: mir::LocalNodeId<mir::Global>) -> Option<StaticAddress> {
        self.constant_space.address(self.static_id(global))
    }
}

impl Program<vm::Executable> {
    /// Return this program's executable format.
    pub fn format(&self) -> ProgramFormat {
        ProgramFormat::Vm
    }

    /// Return VM resume recipes.
    pub fn resume(&self) -> &ResumeTable {
        self.executable.resume()
    }

    /// Return the lowered VM functions.
    pub fn functions(&self) -> &FunctionTable {
        self.executable.functions()
    }

    /// Return the compact VM side table.
    pub fn side_table(&self) -> &SideTable {
        self.executable.side_table()
    }

    /// Return the VM type table.
    pub fn type_table(&self) -> &TypeTable {
        self.executable.type_table()
    }

    /// Convert one current frame location into one lowered program point.
    pub fn point(
        &self,
        function: mir::LocalNodeId<mir::Function>,
        block: mir::LocalNodeId<mir::Block>,
        pc: u32,
    ) -> ProgramPoint {
        ProgramPoint::new(function, block, pc)
    }

    /// Convert one MIR type id into one execution storage layout id.
    pub fn storage_layout_id(&self, ty: mir::LocalNodeId<mir::Type>) -> StorageLayoutId {
        TypeTable::storage_layout_id(ty)
    }

    /// Convert one execution storage layout id into one MIR type id.
    pub fn type_for_storage_id(&self, layout: StorageLayoutId) -> mir::LocalNodeId<mir::Type> {
        TypeTable::type_for_storage_id(layout)
    }

    /// Return the function heap object layout for this program.
    pub fn function_object_layout(&self) -> FunctionObjectLayout {
        function_object_layout(self.tree.pointer_bytes() as usize)
    }

    /// Return the compiled layout for one MIR type.
    pub fn layout(&self, ty: mir::LocalNodeId<mir::Type>) -> Option<&Layout> {
        self.type_table().layout(ty)
    }

    /// Return the compiled layout for one program layout id.
    pub fn layout_for_storage_id(&self, layout: StorageLayoutId) -> Option<&Layout> {
        self.type_table().layout_for_storage_id(layout)
    }

    /// Return the layout id for one MIR type.
    pub fn layout_id_for_type(&self, ty: mir::LocalNodeId<mir::Type>) -> Option<LayoutId> {
        self.type_table().layout_id_for_type(ty)
    }

    /// Return whether one scalar type carries a worker heap root.
    pub fn is_local_root_type(&self, ty: mir::LocalNodeId<mir::Type>) -> Result<bool> {
        Ok(matches!(
            self.scalar_heap_edge(ty, vm::Cell::ZERO)?,
            Some(HeapEdge::Local(_))
        ))
    }

    /// Return whether one scalar type carries a shared heap root.
    pub fn is_shared_root_type(&self, ty: mir::LocalNodeId<mir::Type>) -> Result<bool> {
        Ok(matches!(
            self.scalar_heap_edge(ty, vm::Cell::ZERO)?,
            Some(HeapEdge::Shared(_))
        ))
    }

    /// Visit mutable heap root slots from one byte range.
    pub fn visit_byte_root_slots(
        &self,
        ty: mir::LocalNodeId<mir::Type>,
        bytes: &mut [u8],
        visit: &mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>,
    ) -> Result<()> {
        let layout = self.layout(ty).ok_or_else(|| {
            Error::internal(format!("missing layout for byte heap roots: type={ty:?}"))
        })?;

        if bytes.len() != layout.byte_len {
            return Err(Error::internal(format!(
                "byte heap root length mismatch: type={ty:?}, bytes={}, layout_bytes={}",
                bytes.len(),
                layout.byte_len,
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
            let ty = self.type_for_storage_id(region.layout);

            self.visit_byte_root_slots(ty, bytes, visit)?;
        }

        Ok(())
    }

    /// Return the heap edge carried by one scalar value.
    fn scalar_heap_edge(
        &self,
        ty: mir::LocalNodeId<mir::Type>,
        value: vm::Cell,
    ) -> Result<Option<HeapEdge>> {
        let layout = self.layout(ty).ok_or_else(|| {
            Error::internal(format!("missing scalar layout for root scan: type={ty:?}"))
        })?;

        if !layout.is_cell() {
            return Ok(None);
        }

        let repr_ty = repr_type(&self.tree, ty);
        let bits = value.bits() as usize;

        match self.tree.get(repr_ty) {
            mir::Type::Reference {
                kind:
                    mir::ReferenceKind::Managed
                    | mir::ReferenceKind::Unique
                    | mir::ReferenceKind::Borrowed,
                space: mir::Space::Local,
                ..
            } if bits == 0 => Ok(None),
            mir::Type::Reference {
                kind:
                    mir::ReferenceKind::Managed
                    | mir::ReferenceKind::Unique
                    | mir::ReferenceKind::Borrowed,
                space: mir::Space::Local,
                ..
            } => Ok(Some(HeapEdge::Local(HeapReference::from_bits(bits)))),
            mir::Type::Reference {
                kind:
                    mir::ReferenceKind::Managed
                    | mir::ReferenceKind::Unique
                    | mir::ReferenceKind::Borrowed,
                space: mir::Space::Shared,
                ..
            } if bits == 0 => Ok(None),
            mir::Type::Reference {
                kind:
                    mir::ReferenceKind::Managed
                    | mir::ReferenceKind::Unique
                    | mir::ReferenceKind::Borrowed,
                space: mir::Space::Shared,
                ..
            } => Ok(Some(HeapEdge::Shared(SharedHeapReference::from_bits(bits)))),
            _ => Ok(None),
        }
    }

    /// Return the lowered program point for one resume state.
    pub fn point_for_frame_state(&self, frame_state: FrameStateId) -> Option<ProgramPoint> {
        self.resume().state(frame_state).map(|state| state.point)
    }

    /// Return the source MIR point for one resume state.
    pub fn mir_point_for_frame_state(&self, frame_state: FrameStateId) -> Option<u32> {
        self.resume()
            .state(frame_state)
            .map(|state| state.mir_point)
    }

    /// Return the entry data for one resume state.
    pub fn frame_entry(&self, frame_state: FrameStateId) -> Option<&FrameEntry> {
        self.resume()
            .state(frame_state)
            .and_then(|state| state.entry.as_ref())
    }

    /// Return one resume state id for one lowered program point.
    pub fn frame_state_at(&self, point: ProgramPoint) -> Option<FrameStateId> {
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
    pub fn frame_layout(&self, function: mir::LocalNodeId<mir::Function>) -> Option<&FrameLayout> {
        let function = self.functions().function_by_id(function)?;

        self.frame_layout_by_id(function.frame_layout)
    }
}

/// Program identity and compatibility header.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgramHeader {
    /// Human-facing program name.
    pub name: Option<String>,
    /// Build fingerprint that produced this program.
    pub fingerprint: Option<String>,
    /// Target triple or equivalent target identity.
    pub target: Option<String>,
}

impl ProgramHeader {
    /// Create one program header.
    pub fn new(name: Option<String>, fingerprint: Option<String>, target: Option<String>) -> Self {
        Self {
            name,
            fingerprint,
            target,
        }
    }

    /// Create one anonymous in-memory program header.
    pub fn anonymous() -> Self {
        Self {
            name: None,
            fingerprint: None,
            target: None,
        }
    }
}

/// Program format marker.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ProgramFormat {
    /// VM executable program.
    Vm,
    /// Native executable program.
    Native,
}

/// Executable material for one backend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Executable {
    /// VM executable program.
    Vm(Box<vm::Executable>),
    /// Native executable program.
    Native(Box<native::Executable>),
}

impl Executable {
    /// Return this executable's program format.
    pub fn format(&self) -> ProgramFormat {
        match self {
            Self::Vm(_) => ProgramFormat::Vm,
            Self::Native(_) => ProgramFormat::Native,
        }
    }

    /// Return all content ids referenced by this executable.
    pub fn content_ids(&self) -> Vec<ContentId> {
        match self {
            Self::Vm(program) => program.content_ids(),
            Self::Native(program) => program.content_ids(),
        }
    }
}
