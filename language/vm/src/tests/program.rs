use std::collections::HashMap;

use destack_bytecode as bytecode;
use destack_core::Optional;
use destack_mir::{Access, Reference, Space, Storage, TraceMap};
use destack_program as program;
use destack_program::{
    AllocationSite, BreakpointId, CallDispatch, CallMode, CallSite, CounterId, CounterSite,
    DynamicEntry, DynamicTableBuilder, EdgeSite, FunctionId, LayoutId, LayoutShapeBuilder,
    MemoryAccess, MemorySite, MemoryStop, MemoryTarget, ObjectLayoutBuilder, ProgramPoint,
    ReferenceLayout, SampleSite, SamplerId, ScalarFormat, Signature, SignatureId, SiteTableBuilder,
    StopPoint, StopReason, TypeId, VirtualTableBuilder, WatchpointId, Word,
};

pub(super) const TEST_GLOBAL_BYTES: usize = Word::BYTE_LEN;

/// Program tables used by one bytecode machine fixture.
pub(crate) struct TestProgram {
    /// Runtime bindings keyed by bytecode function name.
    pub(super) bindings: HashMap<u32, String>,
    /// Program signatures keyed by bytecode function id.
    pub(super) signatures: Vec<Option<Signature>>,
    /// Signature used by explicitly word-only fixtures.
    pub(super) default_signature: Option<Signature>,
    /// Program global locations in dense global id order.
    pub(super) globals: Vec<program::GlobalLocation>,
    /// Program sites under test.
    pub(super) sites: SiteTableBuilder,
    /// Virtual tables in dense runtime id order.
    pub(super) virtual_tables: Vec<VirtualTableBuilder>,
    /// Dynamic tables in dense runtime id order.
    pub(super) dynamic_tables: Vec<DynamicTableBuilder>,
    /// Dense dynamic table ids keyed by bytecode type pair.
    pub(super) dynamic_table_ids: HashMap<(bytecode::TypeId, bytecode::TypeId), u32>,
    /// Destructor functions keyed by object-local type id and storage.
    pub(super) drops: Vec<(TypeId, Storage, FunctionId)>,
    /// Concrete type layouts under test.
    pub(super) layouts: Vec<TestLayout>,
    /// Canonical frames required by retained execution tests.
    pub(super) frames: Vec<TestFrame>,
}

/// One canonical frame required by a machine test.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct TestFrame {
    /// The function containing the frame state.
    pub(super) function: u32,
    /// The logical operation represented by the frame state.
    pub(super) operation: u32,
    /// Physical register spans and their Program types.
    pub(super) values: Vec<(bytecode::RegisterSpan, TypeId)>,
}

/// One concrete test type layout.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct TestLayout {
    /// The dense test type id.
    pub(super) ty: TypeId,
    /// The concrete layout shape.
    pub(super) shape: LayoutShapeBuilder,
    /// The inline value byte length.
    pub(super) byte_len: u32,
    /// The inline value alignment.
    pub(super) alignment: u32,
    /// References reachable from the inline value.
    pub(super) trace: TraceMap,
}

impl TestLayout {
    /// Create one canonical word-sized scalar layout.
    pub(super) fn word(ty: TypeId) -> Self {
        Self {
            ty,
            shape: LayoutShapeBuilder::Scalar(ScalarFormat::int(64, false)),
            byte_len: Word::BYTE_LEN as u32,
            alignment: Word::BYTE_LEN as u32,
            trace: TraceMap::empty(),
        }
    }
}

impl TestProgram {
    /// Create one dense test program point.
    pub(crate) const fn point(function: u32, operation: u32) -> ProgramPoint {
        ProgramPoint::new(FunctionId(function), operation)
    }

    /// Create one word-sized allocation site.
    pub(crate) const fn value_allocation(
        function: u32,
        operation: u32,
        space: Space,
        ty: u32,
    ) -> AllocationSite {
        AllocationSite {
            point: Self::point(function, operation),
            space,
            result_type: TypeId(ty),
            storage_type: TypeId(ty),
            layout: LayoutId::new(ty + 1),
            virtual_table: Optional::none(),
        }
    }

    /// Create one virtual object allocation site.
    pub(crate) const fn virtual_allocation(
        function: u32,
        operation: u32,
        space: Space,
        ty: u32,
        table: u32,
    ) -> AllocationSite {
        AllocationSite {
            point: Self::point(function, operation),
            space,
            result_type: TypeId(ty),
            storage_type: TypeId(ty),
            layout: LayoutId::new(ty + 1),
            virtual_table: Optional::some(program::VirtualTableId(table)),
        }
    }

    /// Create one dynamically sized slice allocation site.
    pub(crate) const fn slice_allocation(
        function: u32,
        operation: u32,
        space: Space,
        element: u32,
    ) -> AllocationSite {
        AllocationSite {
            point: Self::point(function, operation),
            space,
            result_type: TypeId(element),
            storage_type: TypeId(element),
            layout: LayoutId::new(element + 1),
            virtual_table: Optional::none(),
        }
    }

    /// Create one word-sized memory site.
    pub(crate) fn memory_site(
        function: u32,
        operation: u32,
        access: MemoryAccess,
        storage: Option<Storage>,
    ) -> MemorySite {
        MemorySite {
            point: Self::point(function, operation),
            access,
            storage: Optional::from(storage),
            value_type: TypeId(0),
        }
    }

    /// Create one counter site.
    pub(crate) const fn counter(function: u32, operation: u32, counter: u32) -> CounterSite {
        CounterSite {
            name: Optional::none(),
            point: Self::point(function, operation),
            counter: CounterId(counter),
        }
    }

    /// Create one word-sized sample site.
    pub(crate) const fn sample(function: u32, operation: u32, sampler: u32) -> SampleSite {
        SampleSite {
            name: Optional::none(),
            point: Self::point(function, operation),
            sampler: SamplerId(sampler),
            value_type: TypeId(0),
        }
    }

    /// Create one direct returning call site.
    pub(crate) const fn call(function: u32, operation: u32, resume: u32, target: u32) -> CallSite {
        CallSite {
            point: Self::point(function, operation),
            resume: Optional::some(Self::point(function, resume)),
            unwind: Optional::none(),
            mode: CallMode::Return,
            dispatch: CallDispatch::Direct,
            space: Optional::none(),
            target: Optional::some(FunctionId(target)),
            dispatch_type: Optional::none(),
            signature: SignatureId(0),
            slot: Optional::none(),
        }
    }

    /// Create one direct tail call site.
    pub(crate) const fn tail_call(function: u32, operation: u32, target: u32) -> CallSite {
        CallSite {
            point: Self::point(function, operation),
            resume: Optional::none(),
            unwind: Optional::none(),
            mode: CallMode::Tail,
            dispatch: CallDispatch::Direct,
            space: Optional::none(),
            target: Optional::some(FunctionId(target)),
            dispatch_type: Optional::none(),
            signature: SignatureId(0),
            slot: Optional::none(),
        }
    }

    /// Create one runtime breakpoint.
    pub(crate) const fn breakpoint(function: u32, operation: u32, breakpoint: u64) -> StopPoint {
        let point = Self::point(function, operation);
        let reason = StopReason::Breakpoint {
            breakpoint_id: BreakpointId::new(breakpoint),
            point,
        };

        StopPoint::new(point, reason)
    }

    /// Create one point watchpoint.
    pub(crate) const fn watchpoint(
        function: u32,
        operation: u32,
        watchpoint: u64,
        access: MemoryAccess,
    ) -> MemoryStop {
        let point = Self::point(function, operation);

        MemoryStop::new(
            WatchpointId::new(watchpoint),
            access,
            MemoryTarget::Point(point),
        )
    }

    /// Create empty test Program tables.
    pub(crate) fn new() -> Self {
        Self {
            bindings: HashMap::new(),
            signatures: Vec::new(),
            default_signature: None,
            globals: Vec::new(),
            sites: SiteTableBuilder::new(),
            virtual_tables: Vec::new(),
            dynamic_tables: Vec::new(),
            dynamic_table_ids: HashMap::new(),
            drops: Vec::new(),
            layouts: Vec::new(),
            frames: Vec::new(),
        }
    }

    /// Create test Program tables for untyped word execution.
    pub(crate) fn words() -> Self {
        Self::new().default_signature([], 0)
    }

    /// Set the signature used by functions without explicit metadata.
    fn default_signature(mut self, parameters: impl IntoIterator<Item = u32>, result: u32) -> Self {
        self.default_signature = Some(Signature {
            parameters: parameters.into_iter().map(TypeId).collect(),
            result: TypeId(result),
        });

        self
    }

    /// Attach one runtime binding to a bytecode function.
    pub(crate) fn binding(mut self, function: u32, binding: &str) -> Self {
        self.bindings.insert(function, binding.to_string());

        self
    }

    /// Set one function's Program signature.
    pub(crate) fn signature(
        mut self,
        function: u32,
        parameters: impl IntoIterator<Item = u32>,
        result: u32,
    ) -> Self {
        let index = function as usize;
        self.signatures.resize_with(index + 1, || None);
        self.signatures[index] = Some(Signature {
            parameters: parameters.into_iter().map(TypeId).collect(),
            result: TypeId(result),
        });

        self
    }

    /// Append one mutable worker-local global.
    pub(crate) fn local_global(mut self) -> Self {
        self.globals.push(program::GlobalLocation::LocalStatic);

        self
    }

    /// Append one mutable runtime-shared global.
    pub(crate) fn shared_global(mut self) -> Self {
        self.globals.push(program::GlobalLocation::SharedStatic);

        self
    }

    /// Set allocation sites.
    pub(crate) fn allocations(mut self, sites: impl IntoIterator<Item = AllocationSite>) -> Self {
        self.sites = self.sites.allocations(sites);

        self
    }

    /// Set memory sites.
    pub(crate) fn memory(mut self, sites: impl IntoIterator<Item = MemorySite>) -> Self {
        self.sites = self.sites.memory(sites);

        self
    }

    /// Set profile counter sites.
    pub(crate) fn counters(mut self, sites: impl IntoIterator<Item = CounterSite>) -> Self {
        self.sites = self.sites.counters(sites);

        self
    }

    /// Set profile sample sites.
    pub(crate) fn samples(mut self, sites: impl IntoIterator<Item = SampleSite>) -> Self {
        self.sites = self.sites.samples(sites);

        self
    }

    /// Set function call sites.
    pub(crate) fn calls(mut self, sites: impl IntoIterator<Item = CallSite>) -> Self {
        self.sites = self.sites.calls(sites);

        self
    }

    /// Set control-flow edge sites.
    pub(crate) fn edges(mut self, sites: impl IntoIterator<Item = EdgeSite>) -> Self {
        self.sites = self.sites.edges(sites);

        self
    }

    /// Append one virtual table in dense runtime id order.
    pub(crate) fn virtual_table(mut self, ty: u32, methods: impl IntoIterator<Item = u32>) -> Self {
        let methods = methods.into_iter().map(FunctionId);
        let table = VirtualTableBuilder::new(TypeId(ty)).methods(methods);
        self.virtual_tables.push(table);

        self
    }

    /// Append one dynamic table in dense runtime id order.
    pub(crate) fn dynamic_table(
        mut self,
        concrete: u32,
        constraint: u32,
        entries: impl IntoIterator<Item = DynamicEntry>,
    ) -> Self {
        let table = self.dynamic_tables.len() as u32;
        self.dynamic_table_ids.insert(
            (bytecode::TypeId(concrete), bytecode::TypeId(constraint)),
            table,
        );
        self.dynamic_tables
            .push(DynamicTableBuilder::new(TypeId(concrete), TypeId(constraint)).entries(entries));

        self
    }

    /// Attach one destructor function to an object-local type and storage.
    pub(crate) fn destructor(mut self, ty: u32, storage: Storage, function: u32) -> Self {
        self.drops.push((TypeId(ty), storage, FunctionId(function)));

        self
    }

    /// Append one canonical frame state and its physical register map.
    pub(crate) fn frame(
        mut self,
        function: u32,
        operation: u32,
        values: impl IntoIterator<Item = (bytecode::RegisterSpan, u32)>,
    ) -> Self {
        let values = values
            .into_iter()
            .map(|(registers, ty)| (registers, TypeId(ty)))
            .collect();
        self.frames.push(TestFrame {
            function,
            operation,
            values,
        });

        self
    }

    /// Set one reference layout and its exact trace map.
    pub(crate) fn reference(
        mut self,
        ty: u32,
        pointee: u32,
        kind: Reference,
        storage: Storage,
    ) -> Self {
        let reference = ReferenceLayout::new(TypeId(pointee), kind, Access::Mutable);
        let shape = LayoutShapeBuilder::Reference(reference);
        let trace = Self::reference_trace(kind, storage);
        self.insert_layout(TestLayout {
            ty: TypeId(ty),
            shape,
            byte_len: Word::BYTE_LEN as u32,
            alignment: Word::BYTE_LEN as u32,
            trace,
        });

        self
    }

    /// Return the exact trace map for one reference representation.
    fn reference_trace(kind: Reference, storage: Storage) -> TraceMap {
        match (kind, storage) {
            (Reference::Managed(Space::Local), Storage::Heap(Space::Local)) => TraceMap::Fixed {
                local_offsets: Box::new([0]),
                shared_offsets: Box::new([]),
                frame_offsets: Box::new([]),
                borrow_offsets: Box::default(),
            },
            (Reference::Managed(Space::Shared), Storage::Heap(Space::Shared)) => TraceMap::Fixed {
                local_offsets: Box::new([]),
                shared_offsets: Box::new([0]),
                frame_offsets: Box::new([]),
                borrow_offsets: Box::default(),
            },
            (Reference::Managed(_) | Reference::Unique | Reference::Borrowed, Storage::Frame) => {
                TraceMap::Fixed {
                    local_offsets: Box::new([]),
                    shared_offsets: Box::new([]),
                    frame_offsets: Box::new([0]),
                    borrow_offsets: Box::new([]),
                }
            }
            _ => TraceMap::empty(),
        }
    }

    /// Set one object layout with a virtual dispatch word.
    pub(crate) fn virtual_object(mut self, ty: u32, byte_len: u32, dispatch: u32) -> Self {
        let object = ObjectLayoutBuilder::new([]).dispatch_offset(dispatch);
        self.insert_layout(TestLayout {
            ty: TypeId(ty),
            shape: LayoutShapeBuilder::Object(object),
            byte_len,
            alignment: Word::BYTE_LEN as u32,
            trace: TraceMap::empty(),
        });

        self
    }

    /// Set one exact Program layout for a bytecode type.
    pub(crate) fn layout(
        mut self,
        ty: u32,
        shape: LayoutShapeBuilder,
        byte_len: u32,
        alignment: u32,
    ) -> Self {
        self.insert_layout(TestLayout {
            ty: TypeId(ty),
            shape,
            byte_len,
            alignment,
            trace: TraceMap::empty(),
        });

        self
    }

    /// Insert one concrete test layout or require its existing definition to match.
    fn insert_layout(&mut self, layout: TestLayout) {
        if let Some(current) = self.layouts.iter().find(|current| current.ty == layout.ty) {
            assert_eq!(
                current, &layout,
                "test type layout must have one definition"
            );
        } else {
            self.layouts.push(layout);
        }
    }
}
