use std::collections::HashMap;

use destack_bytecode as bytecode;
use destack_core::Optional;
use destack_mir::{
    Access, Nullability, ReferenceKind, Space, Storage, TensorFormat, TensorViewFormat, TraceMap,
};
use destack_program as program;
use destack_program::{
    AllocationSite, BreakpointId, ContinuationSite, CounterId, CounterSite, DynamicEntry,
    DynamicTableBuilder, FunctionId, InstructionStop, LayoutId, LayoutShapeBuilder, MemoryAccess,
    MemorySite, MemoryStop, MemoryTarget, ObjectLayoutBuilder, ProgramPoint, ReferenceFlags,
    ReferenceLayout, SampleSite, SamplerId, ScalarFormat, Signature, SiteTableBuilder, StopReason,
    Suspension, SuspensionSite, TensorDimension, TensorLayoutBuilder, TensorViewLayoutBuilder,
    TypeId, VirtualTableBuilder, WatchpointId, Word,
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
    /// Hidden environment types keyed by bytecode function id.
    pub(super) environments: HashMap<u32, TypeId>,
    /// Coroutine behavior keyed by bytecode function id.
    pub(super) coroutines: HashMap<u32, program::CoroutineKind>,
    /// Program global locations in dense global id order.
    pub(super) globals: Vec<program::GlobalLocation>,
    /// Program sites under test.
    pub(super) sites: SiteTableBuilder,
    /// Suspension sites awaiting canonical frame state assignment.
    pub(super) suspensions: Vec<TestSuspension>,
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

/// One suspension site before its canonical frame state is linked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TestSuspension {
    /// The suspension operation point.
    point: ProgramPoint,
    /// The normal resumption point.
    resume: ProgramPoint,
    /// The cancellation cleanup point when present.
    cancel: Optional<ProgramPoint>,
    /// The generator completion point when present.
    complete: Optional<ProgramPoint>,
    /// The panic unwind point when present.
    unwind: Optional<ProgramPoint>,
    /// The suspension operation.
    operation: Suspension,
    /// The suspended value type.
    value_type: TypeId,
    /// The normal resumption value type.
    resume_type: TypeId,
    /// The generator completion value type when present.
    complete_type: Optional<TypeId>,
}

impl TestSuspension {
    /// Return the suspension operation point.
    pub(super) const fn point(self) -> ProgramPoint {
        self.point
    }

    /// Link this suspension to its canonical frame state.
    pub(super) fn link(self, frame_state: program::FrameStateId) -> SuspensionSite {
        SuspensionSite {
            point: self.point,
            resume: self.resume,
            cancel: self.cancel,
            complete: self.complete,
            unwind: self.unwind,
            frame_state,
            operation: self.operation,
            value_type: self.value_type,
            resume_type: self.resume_type,
            complete_type: self.complete_type,
        }
    }
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

    /// Create one dynamically sized tensor allocation site.
    pub(crate) const fn tensor_allocation(
        function: u32,
        operation: u32,
        space: Space,
        result_type: u32,
    ) -> AllocationSite {
        AllocationSite {
            point: Self::point(function, operation),
            space,
            result_type: TypeId(result_type),
            storage_type: TypeId(result_type),
            layout: LayoutId::new(result_type + 1),
            virtual_table: Optional::none(),
        }
    }

    /// Create one word-sized memory site.
    pub(crate) const fn memory_site(
        function: u32,
        operation: u32,
        access: MemoryAccess,
        storage: Storage,
    ) -> MemorySite {
        MemorySite {
            point: Self::point(function, operation),
            access,
            storage,
            value_type: TypeId(0),
        }
    }

    /// Create one counter site.
    pub(crate) const fn counter(function: u32, operation: u32, counter: u32) -> CounterSite {
        CounterSite {
            point: Self::point(function, operation),
            counter: CounterId(counter),
        }
    }

    /// Create one word-sized sample site.
    pub(crate) const fn sample(function: u32, operation: u32, sampler: u32) -> SampleSite {
        SampleSite {
            point: Self::point(function, operation),
            sampler: SamplerId(sampler),
            value_type: TypeId(0),
        }
    }

    /// Create one await suspension site.
    pub(crate) const fn await_site(
        function: u32,
        operation: u32,
        resume: u32,
        cancel: u32,
        unwind: u32,
        value_type: u32,
        resume_type: u32,
    ) -> TestSuspension {
        TestSuspension {
            point: Self::point(function, operation),
            resume: Self::point(function, resume),
            cancel: Optional::some(Self::point(function, cancel)),
            complete: Optional::none(),
            unwind: Optional::some(Self::point(function, unwind)),
            operation: Suspension::Await,
            value_type: TypeId(value_type),
            resume_type: TypeId(resume_type),
            complete_type: Optional::none(),
        }
    }

    /// Create one yield suspension site.
    pub(crate) const fn yield_site(
        function: u32,
        operation: u32,
        resume: u32,
        complete: u32,
        unwind: u32,
        value_type: u32,
        resume_type: u32,
        complete_type: u32,
    ) -> TestSuspension {
        TestSuspension {
            point: Self::point(function, operation),
            resume: Self::point(function, resume),
            cancel: Optional::none(),
            complete: Optional::some(Self::point(function, complete)),
            unwind: Optional::some(Self::point(function, unwind)),
            operation: Suspension::Yield,
            value_type: TypeId(value_type),
            resume_type: TypeId(resume_type),
            complete_type: Optional::some(TypeId(complete_type)),
        }
    }

    /// Create one continuation control site.
    pub(crate) const fn continuation_site(
        function: u32,
        operation: u32,
        yielded: u32,
        returned: u32,
        unwind: Option<u32>,
    ) -> ContinuationSite {
        ContinuationSite {
            point: Self::point(function, operation),
            yielded: Self::point(function, yielded),
            returned: Self::point(function, returned),
            unwind: match unwind {
                Some(operation) => Optional::some(Self::point(function, operation)),
                None => Optional::none(),
            },
        }
    }

    /// Create one runtime breakpoint.
    pub(crate) const fn breakpoint(
        function: u32,
        operation: u32,
        breakpoint: u64,
    ) -> InstructionStop {
        let point = Self::point(function, operation);
        let reason = StopReason::Breakpoint {
            breakpoint_id: BreakpointId::new(breakpoint),
            point,
        };

        InstructionStop::new(point, reason)
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
            environments: HashMap::new(),
            coroutines: HashMap::new(),
            globals: Vec::new(),
            sites: SiteTableBuilder::new(),
            suspensions: Vec::new(),
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

    /// Set one function's hidden environment type.
    pub(crate) fn environment(mut self, function: u32, ty: u32) -> Self {
        self.environments.insert(function, TypeId(ty));

        self
    }

    /// Set one function's coroutine behavior.
    pub(crate) fn coroutine(mut self, function: u32, coroutine: program::CoroutineKind) -> Self {
        self.coroutines.insert(function, coroutine);

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

    /// Set coroutine suspension sites.
    pub(crate) fn suspensions(mut self, sites: impl IntoIterator<Item = TestSuspension>) -> Self {
        self.suspensions = sites.into_iter().collect();

        self
    }

    /// Set continuation control sites.
    pub(crate) fn continuations(
        mut self,
        sites: impl IntoIterator<Item = ContinuationSite>,
    ) -> Self {
        self.sites = self.sites.continuations(sites);

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
        kind: ReferenceKind,
        storage: Storage,
    ) -> Self {
        let flags = ReferenceFlags::new(kind, storage, Access::Mutable, Nullability::None);
        let shape = LayoutShapeBuilder::Reference(ReferenceLayout {
            pointee: TypeId(pointee),
            flags,
        });
        let trace = match (kind, storage) {
            (ReferenceKind::Managed, Storage::Heap(Space::Local)) => TraceMap::Fixed {
                local_offsets: Box::new([0]),
                shared_offsets: Box::new([]),
                frame_offsets: Box::new([]),
            },
            (ReferenceKind::Managed, Storage::Heap(Space::Shared)) => TraceMap::Fixed {
                local_offsets: Box::new([]),
                shared_offsets: Box::new([0]),
                frame_offsets: Box::new([]),
            },
            (
                ReferenceKind::Managed | ReferenceKind::Unique | ReferenceKind::Borrowed,
                Storage::Frame,
            ) => TraceMap::Fixed {
                local_offsets: Box::new([]),
                shared_offsets: Box::new([]),
                frame_offsets: Box::new([0]),
            },
            _ => TraceMap::empty(),
        };
        self.insert_layout(TestLayout {
            ty: TypeId(ty),
            shape,
            byte_len: Word::BYTE_LEN as u32,
            alignment: Word::BYTE_LEN as u32,
            trace,
        });

        self
    }

    /// Set one dense owning tensor layout.
    pub(crate) fn tensor(
        mut self,
        ty: u32,
        element: u32,
        scalar: ScalarFormat,
        space: Space,
        dimensions: impl IntoIterator<Item = u64>,
    ) -> Self {
        let scalar_byte_len = scalar.byte_len();
        let dimensions = dimensions.into_iter().map(TensorDimension::fixed);
        let tensor = TensorLayoutBuilder::new(
            space,
            TypeId(element),
            TensorFormat::dense_row_major(),
            dimensions,
        );
        self.insert_layout(TestLayout {
            ty: TypeId(ty),
            shape: LayoutShapeBuilder::Tensor(tensor),
            byte_len: Word::BYTE_LEN as u32,
            alignment: Word::BYTE_LEN as u32,
            trace: TraceMap::empty(),
        });
        self.insert_layout(TestLayout {
            ty: TypeId(element),
            shape: LayoutShapeBuilder::Scalar(scalar),
            byte_len: scalar_byte_len as u32,
            alignment: scalar_byte_len as u32,
            trace: TraceMap::empty(),
        });

        self
    }

    /// Set one dense borrowed tensor-view layout.
    pub(crate) fn tensor_view(
        mut self,
        ty: u32,
        element: u32,
        scalar: ScalarFormat,
        space: Space,
        dimensions: impl IntoIterator<Item = u64>,
    ) -> Self {
        let scalar_byte_len = scalar.byte_len();
        let dimensions = dimensions
            .into_iter()
            .map(TensorDimension::fixed)
            .collect::<Vec<_>>();
        let rank = dimensions.len() as u16;
        let reference = ReferenceLayout {
            pointee: TypeId(element),
            flags: ReferenceFlags::new(
                ReferenceKind::Borrowed,
                Storage::Heap(space),
                Access::Mutable,
                Nullability::None,
            ),
        };
        let tensor = TensorViewLayoutBuilder::new(
            reference,
            TypeId(element),
            TensorViewFormat::dense_row_major(),
            dimensions,
        );
        let word_count = bytecode::ValueType::tensor_view_word_count(rank)
            .expect("test tensor view rank should fit one register range");
        self.insert_layout(TestLayout {
            ty: TypeId(ty),
            shape: LayoutShapeBuilder::TensorView(tensor),
            byte_len: u32::from(word_count) * Word::BYTE_LEN as u32,
            alignment: Word::BYTE_LEN as u32,
            trace: TraceMap::empty(),
        });
        self.insert_layout(TestLayout {
            ty: TypeId(element),
            shape: LayoutShapeBuilder::Scalar(scalar),
            byte_len: scalar_byte_len as u32,
            alignment: scalar_byte_len as u32,
            trace: TraceMap::empty(),
        });

        self
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
