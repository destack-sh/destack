use destack_core::{Optional, SectionEntry, SectionImage, SectionPacker, SectionSlice};
use destack_mir::Space;
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use super::{FrameStateId, FunctionId, LayoutId, ProgramPoint, TypeId};

/// Executable program sites used by debugging, probes, and observations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct SiteTable {
    /// Heap allocation operation sites sorted by program point.
    allocations: SectionSlice<AllocationSite>,
    /// Addressable memory operation sites sorted by program point.
    memory: SectionSlice<MemorySite>,
    /// Function call operation sites sorted by program point.
    calls: SectionSlice<CallSite>,
    /// Control-flow edge sites sorted by source and target point.
    edges: SectionSlice<EdgeSite>,
    /// Continuation capture sites sorted by captured frame state.
    continuations: SectionSlice<ContinuationSite>,
    /// Explicit counter sites sorted by program point.
    counters: SectionSlice<CounterSite>,
    /// Explicit sample sites sorted by program point.
    samples: SectionSlice<SampleSite>,
}

impl SiteTable {
    /// Pack executable site rows.
    pub fn pack(
        sections: &mut SectionPacker,
        mut allocations: Vec<AllocationSite>,
        mut memory: Vec<MemorySite>,
        mut calls: Vec<CallSite>,
        mut edges: Vec<EdgeSite>,
        mut continuations: Vec<ContinuationSite>,
        mut counters: Vec<CounterSite>,
        mut samples: Vec<SampleSite>,
    ) -> Self {
        allocations.sort_unstable_by_key(|site| site.point);
        memory.sort_unstable_by_key(|site| site.point);
        calls.sort_unstable_by_key(|site| site.point);
        edges.sort_unstable_by_key(|site| (site.source, site.target));
        edges.dedup_by_key(|site| (site.source, site.target));
        continuations.sort_unstable_by_key(|site| site.frame_state);
        counters.sort_unstable_by_key(|site| site.point);
        samples.sort_unstable_by_key(|site| site.point);

        let allocations = sections.insert(allocations);
        let memory = sections.insert(memory);
        let calls = sections.insert(calls);
        let edges = sections.insert(edges);
        let continuations = sections.insert(continuations);
        let counters = sections.insert(counters);
        let samples = sections.insert(samples);

        Self {
            allocations,
            memory,
            calls,
            edges,
            continuations,
            counters,
            samples,
        }
    }

    /// Return the allocation site at one executable point.
    pub fn allocation<'a>(
        &self,
        sections: SectionImage<'a>,
        point: ProgramPoint,
    ) -> Option<(AllocationSiteId, &'a AllocationSite)> {
        let allocations = self.allocations(sections);
        let index = allocations
            .binary_search_by_key(&point, |site| site.point)
            .ok()?;

        Some((AllocationSiteId(index as u32), &allocations[index]))
    }

    /// Return all allocation site rows.
    pub fn allocations<'a>(&self, sections: SectionImage<'a>) -> &'a [AllocationSite] {
        sections.entries(self.allocations)
    }

    /// Return the number of allocation site rows.
    pub fn allocation_count(&self, sections: SectionImage<'_>) -> usize {
        self.allocations(sections).len()
    }

    /// Return memory sites at one executable point.
    pub fn memory<'a>(&self, sections: SectionImage<'a>, point: ProgramPoint) -> &'a [MemorySite] {
        let memory = self.memory_sites(sections);
        let start = memory.partition_point(|site| site.point < point);
        let end = start + memory[start..].partition_point(|site| site.point == point);

        &memory[start..end]
    }

    /// Return all memory site rows.
    pub fn memory_sites<'a>(&self, sections: SectionImage<'a>) -> &'a [MemorySite] {
        sections.entries(self.memory)
    }

    /// Return the call site at one executable point.
    pub fn call<'a>(
        &self,
        sections: SectionImage<'a>,
        point: ProgramPoint,
    ) -> Option<(CallSiteId, &'a CallSite)> {
        let calls = self.calls(sections);
        let index = calls.binary_search_by_key(&point, |site| site.point).ok()?;

        Some((CallSiteId(index as u32), &calls[index]))
    }

    /// Return all call site rows.
    pub fn calls<'a>(&self, sections: SectionImage<'a>) -> &'a [CallSite] {
        sections.entries(self.calls)
    }

    /// Return the number of call site rows.
    pub fn call_count(&self, sections: SectionImage<'_>) -> usize {
        self.calls(sections).len()
    }

    /// Return the edge site for one observed transfer.
    pub fn edge<'a>(
        &self,
        sections: SectionImage<'a>,
        source: ProgramPoint,
        target: ProgramPoint,
    ) -> Option<(EdgeSiteId, &'a EdgeSite)> {
        let edges = self.edges(sections);
        let index = edges
            .binary_search_by_key(&(source, target), |site| (site.source, site.target))
            .ok()?;

        Some((EdgeSiteId(index as u32), &edges[index]))
    }

    /// Return all control-flow edge site rows.
    pub fn edges<'a>(&self, sections: SectionImage<'a>) -> &'a [EdgeSite] {
        sections.entries(self.edges)
    }

    /// Return the number of control-flow edge site rows.
    pub fn edge_count(&self, sections: SectionImage<'_>) -> usize {
        self.edges(sections).len()
    }

    /// Return all continuation site rows.
    pub fn continuations<'a>(&self, sections: SectionImage<'a>) -> &'a [ContinuationSite] {
        sections.entries(self.continuations)
    }

    /// Return the continuation site for one captured frame state.
    pub fn continuation_state<'a>(
        &self,
        sections: SectionImage<'a>,
        frame_state: FrameStateId,
    ) -> Option<(ContinuationSiteId, &'a ContinuationSite)> {
        let continuations = self.continuations(sections);
        let index = continuations
            .binary_search_by_key(&frame_state, |site| site.frame_state)
            .ok()?;

        Some((ContinuationSiteId(index as u32), &continuations[index]))
    }

    /// Return the number of continuation site rows.
    pub fn continuation_count(&self, sections: SectionImage<'_>) -> usize {
        self.continuations(sections).len()
    }

    /// Return the counter site at one executable point.
    pub fn counter<'a>(
        &self,
        sections: SectionImage<'a>,
        point: ProgramPoint,
    ) -> Option<&'a CounterSite> {
        let counters = self.counters(sections);
        let index = counters
            .binary_search_by_key(&point, |site| site.point)
            .ok()?;

        Some(&counters[index])
    }

    /// Return all counter site rows.
    pub fn counters<'a>(&self, sections: SectionImage<'a>) -> &'a [CounterSite] {
        sections.entries(self.counters)
    }

    /// Return the number of dense profile counters.
    pub fn profile_counter_count(&self, sections: SectionImage<'_>) -> usize {
        let counters = self
            .counters(sections)
            .iter()
            .map(|site| site.counter.index() + 1);
        let samples = self
            .samples(sections)
            .iter()
            .map(|site| site.counter.index() + 1);

        counters.chain(samples).max().unwrap_or(0)
    }

    /// Return the sample site at one executable point.
    pub fn sample<'a>(
        &self,
        sections: SectionImage<'a>,
        point: ProgramPoint,
    ) -> Option<&'a SampleSite> {
        let samples = self.samples(sections);
        let index = samples
            .binary_search_by_key(&point, |site| site.point)
            .ok()?;

        Some(&samples[index])
    }

    /// Return all sample site rows.
    pub fn samples<'a>(&self, sections: SectionImage<'a>) -> &'a [SampleSite] {
        sections.entries(self.samples)
    }
}

/// Dense allocation site identifier within one program.
#[repr(transparent)]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Reflect,
)]
pub struct AllocationSiteId(pub u32);

/// Dense call site identifier within one program.
#[repr(transparent)]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Reflect,
)]
pub struct CallSiteId(pub u32);

/// Dense control-flow edge site identifier within one program.
#[repr(transparent)]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Reflect,
)]
pub struct EdgeSiteId(pub u32);

/// Dense continuation site identifier within one program.
#[repr(transparent)]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Reflect,
)]
pub struct ContinuationSiteId(pub u32);

/// Dense executable profile counter identifier within one program.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct CounterId(pub u32);

/// Executable heap allocation operation.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct AllocationSite {
    /// The executable point that performs the allocation.
    pub point: ProgramPoint,
    /// The allocation operation family.
    pub operation: AllocationOperation,
    /// The byte initialization mode.
    pub initialization: AllocationInitialization,
    /// The storage space receiving the allocated storage.
    pub space: Space,
    /// The type produced by the allocation expression.
    pub result_type: TypeId,
    /// The type used for the allocated storage.
    pub storage_type: TypeId,
    /// The layout used for the allocated storage.
    pub storage_layout: LayoutId,
}

/// Executable allocation operation family.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum AllocationOperation {
    /// Allocate one typed object.
    Object,
    /// Allocate repeated typed storage.
    Slice,
}

/// Allocation byte initialization mode.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum AllocationInitialization {
    /// Initialize the allocated bytes to zero.
    Zeroed,
    /// Leave the allocated bytes uninitialized.
    Uninit,
}

/// Executable addressable memory operation.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct MemorySite {
    /// The executable point that performs the access.
    pub point: ProgramPoint,
    /// The memory operation performed at the site.
    pub access: MemoryAccess,
    /// The storage space accessed by the operation.
    pub space: Space,
    /// The loaded or stored value type.
    pub value_type: TypeId,
}

/// Executable memory access operation.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum MemoryAccess {
    /// Read addressable storage.
    Read,
    /// Write addressable storage.
    Write,
    /// Read and write addressable storage.
    ReadWrite,
}

/// Executable function call operation.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct CallSite {
    /// The executable point that performs the call.
    pub point: ProgramPoint,
    /// Whether the call returns to the caller frame.
    pub mode: CallMode,
    /// The dispatch mechanism used by the call.
    pub dispatch: CallDispatch,
    /// The receiver storage space for virtual and dynamic calls.
    pub space: Optional<Space>,
    /// The resolved callee for direct calls.
    pub target: Optional<FunctionId>,
    /// The type that defines the virtual or dynamic dispatch slot.
    pub dispatch_type: Optional<TypeId>,
    /// The program signature type for this call.
    pub signature_type: TypeId,
    /// The dispatch slot for virtual and dynamic calls.
    pub slot: Optional<u32>,
}

/// Executable control-flow edge.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct EdgeSite {
    /// The executable point producing the transfer.
    pub source: ProgramPoint,
    /// The executable point entered after the transfer.
    pub target: ProgramPoint,
}

/// Executable continuation capture point.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct ContinuationSite {
    /// The executable point that captures the continuation.
    pub point: ProgramPoint,
    /// The executable point entered by normal resume.
    pub resume: ProgramPoint,
    /// The executable point entered by panic unwinding.
    pub unwind: Optional<ProgramPoint>,
    /// The captured frame state.
    pub frame_state: FrameStateId,
    /// The type yielded to the coroutine owner.
    pub yielded_type: TypeId,
}

/// Executable explicit counter operation.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct CounterSite {
    /// The executable point that increments the counter.
    pub point: ProgramPoint,
    /// The counter incremented at this site.
    pub counter: CounterId,
}

/// Executable explicit sample operation.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct SampleSite {
    /// The executable point that records the sample.
    pub point: ProgramPoint,
    /// The counter receiving samples at this site.
    pub counter: CounterId,
    /// The sampled value type.
    pub value_type: TypeId,
}

/// Executable call continuation mode.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum CallMode {
    /// Return to the caller after the callee finishes.
    Return,
    /// Replace the current frame with the callee frame.
    Tail,
}

/// Executable call dispatch mechanism.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum CallDispatch {
    /// Call a known function.
    Direct,
    /// Call through a concrete type dispatch table.
    Virtual,
    /// Call through a dynamic dispatch table.
    Dynamic,
    /// Call through a function value.
    Indirect,
}

impl MemoryAccess {
    /// Return whether this access selector matches one concrete memory access.
    pub const fn selects(self, access: Self) -> bool {
        match (self, access) {
            (Self::ReadWrite, _) => true,
            (Self::Read, Self::Read | Self::ReadWrite) => true,
            (Self::Write, Self::Write | Self::ReadWrite) => true,
            (Self::Read, Self::Write) | (Self::Write, Self::Read) => false,
        }
    }
}

impl AllocationSiteId {
    /// Return this site id as a dense array index.
    pub const fn index(self) -> usize {
        self.0 as usize
    }
}

impl CallSiteId {
    /// Return this site id as a dense array index.
    pub const fn index(self) -> usize {
        self.0 as usize
    }
}

impl EdgeSiteId {
    /// Return this site id as a dense array index.
    pub const fn index(self) -> usize {
        self.0 as usize
    }
}

impl ContinuationSiteId {
    /// Return this site id as a dense array index.
    pub const fn index(self) -> usize {
        self.0 as usize
    }
}

impl CounterId {
    /// Return this counter id as a dense array index.
    pub const fn index(self) -> usize {
        self.0 as usize
    }
}

// SAFETY: site rows are fixed-width program section entries.
unsafe impl SectionEntry for AllocationSite {}
unsafe impl SectionEntry for AllocationSiteId {}
unsafe impl SectionEntry for AllocationOperation {}
unsafe impl SectionEntry for AllocationInitialization {}
unsafe impl SectionEntry for MemorySite {}
unsafe impl SectionEntry for MemoryAccess {}
unsafe impl SectionEntry for CallSite {}
unsafe impl SectionEntry for CallSiteId {}
unsafe impl SectionEntry for CallMode {}
unsafe impl SectionEntry for CallDispatch {}
unsafe impl SectionEntry for EdgeSite {}
unsafe impl SectionEntry for EdgeSiteId {}
unsafe impl SectionEntry for ContinuationSite {}
unsafe impl SectionEntry for ContinuationSiteId {}
unsafe impl SectionEntry for CounterSite {}
unsafe impl SectionEntry for SampleSite {}
unsafe impl SectionEntry for CounterId {}
