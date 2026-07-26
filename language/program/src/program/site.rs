use destack_core::{Optional, SectionBuilder, SectionEntry, SectionImage, SectionSlice};
use destack_mir::Space;
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use super::{
    FrameStateId, FunctionId, LayoutId, ProgramPoint, SignatureId, TypeId, VirtualTableId,
};

/// Program sites used by debugging, probes, and observations.
#[repr(C)]
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry,
)]
pub struct SiteTable {
    /// Allocation sites sorted by program point.
    allocations: SectionSlice<AllocationSite>,
    /// Addressable memory operation sites sorted by program point.
    memory: SectionSlice<MemorySite>,
    /// Function call operation sites sorted by program point.
    calls: SectionSlice<CallSite>,
    /// Continuation resume operation sites sorted by program point.
    resumes: SectionSlice<ResumeSite>,
    /// Control-flow edge sites sorted by source and target point.
    edges: SectionSlice<EdgeSite>,
    /// Coroutine suspension sites sorted by program point.
    suspensions: SectionSlice<SuspensionSite>,
    /// Explicit counter sites sorted by program point.
    counters: SectionSlice<CounterSite>,
    /// Explicit sample sites sorted by program point.
    samples: SectionSlice<SampleSite>,
}

impl SiteTable {
    /// Return the allocation site at one program point.
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

    /// Return one allocation site by dense id.
    #[inline(always)]
    pub fn allocation_by_id<'a>(
        &self,
        sections: SectionImage<'a>,
        id: AllocationSiteId,
    ) -> Option<&'a AllocationSite> {
        self.allocations(sections).get(id.index())
    }

    /// Return all allocation sites.
    pub fn allocations<'a>(&self, sections: SectionImage<'a>) -> &'a [AllocationSite] {
        sections.entries(self.allocations)
    }

    /// Return the number of allocation sites.
    pub fn allocation_count(&self, sections: SectionImage<'_>) -> usize {
        self.allocations(sections).len()
    }

    /// Return memory sites at one program point.
    pub fn memory<'a>(&self, sections: SectionImage<'a>, point: ProgramPoint) -> &'a [MemorySite] {
        let memory = self.memory_sites(sections);
        let start = memory.partition_point(|site| site.point < point);
        let end = start + memory[start..].partition_point(|site| site.point == point);

        &memory[start..end]
    }

    /// Return all memory sites.
    pub fn memory_sites<'a>(&self, sections: SectionImage<'a>) -> &'a [MemorySite] {
        sections.entries(self.memory)
    }

    /// Return the call site at one program point.
    pub fn call<'a>(
        &self,
        sections: SectionImage<'a>,
        point: ProgramPoint,
    ) -> Option<(CallSiteId, &'a CallSite)> {
        let calls = self.calls(sections);
        let index = calls.binary_search_by_key(&point, |site| site.point).ok()?;

        Some((CallSiteId(index as u32), &calls[index]))
    }

    /// Return all call sites.
    pub fn calls<'a>(&self, sections: SectionImage<'a>) -> &'a [CallSite] {
        sections.entries(self.calls)
    }

    /// Return the number of call sites.
    pub fn call_count(&self, sections: SectionImage<'_>) -> usize {
        self.calls(sections).len()
    }

    /// Return the continuation resume site at one program point.
    pub fn resume<'a>(
        &self,
        sections: SectionImage<'a>,
        point: ProgramPoint,
    ) -> Option<(ResumeSiteId, &'a ResumeSite)> {
        let resumes = self.resumes(sections);
        let index = resumes
            .binary_search_by_key(&point, |site| site.point)
            .ok()?;

        Some((ResumeSiteId(index as u32), &resumes[index]))
    }

    /// Return all continuation resume sites.
    pub fn resumes<'a>(&self, sections: SectionImage<'a>) -> &'a [ResumeSite] {
        sections.entries(self.resumes)
    }

    /// Return the number of continuation resume sites.
    pub fn resume_count(&self, sections: SectionImage<'_>) -> usize {
        self.resumes(sections).len()
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

    /// Return all control-flow edge sites.
    pub fn edges<'a>(&self, sections: SectionImage<'a>) -> &'a [EdgeSite] {
        sections.entries(self.edges)
    }

    /// Return the number of control-flow edge sites.
    pub fn edge_count(&self, sections: SectionImage<'_>) -> usize {
        self.edges(sections).len()
    }

    /// Return the coroutine suspension site at one program point.
    pub fn suspension<'a>(
        &self,
        sections: SectionImage<'a>,
        point: ProgramPoint,
    ) -> Option<(SuspensionSiteId, &'a SuspensionSite)> {
        let suspensions = self.suspensions(sections);
        let index = suspensions
            .binary_search_by_key(&point, |site| site.point)
            .ok()?;

        Some((SuspensionSiteId(index as u32), &suspensions[index]))
    }

    /// Return all coroutine suspension sites.
    pub fn suspensions<'a>(&self, sections: SectionImage<'a>) -> &'a [SuspensionSite] {
        sections.entries(self.suspensions)
    }

    /// Return the number of coroutine suspension sites.
    pub fn suspension_count(&self, sections: SectionImage<'_>) -> usize {
        self.suspensions(sections).len()
    }

    /// Return the counter site at one program point.
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

    /// Return all counter sites.
    pub fn counters<'a>(&self, sections: SectionImage<'a>) -> &'a [CounterSite] {
        sections.entries(self.counters)
    }

    /// Return the number of dense counters.
    pub fn counter_count(&self, sections: SectionImage<'_>) -> usize {
        self.counters(sections)
            .iter()
            .map(|site| site.counter.index() + 1)
            .fold(0, usize::max)
    }

    /// Return the number of dense samplers.
    pub fn sampler_count(&self, sections: SectionImage<'_>) -> usize {
        self.samples(sections)
            .iter()
            .map(|site| site.sampler.index() + 1)
            .fold(0, usize::max)
    }

    /// Return the sample site at one program point.
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

    /// Return all sample sites.
    pub fn samples<'a>(&self, sections: SectionImage<'a>) -> &'a [SampleSite] {
        sections.entries(self.samples)
    }
}

/// Build-time program sites used by debugging, probes, and observations.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SiteTableBuilder {
    /// Allocation sites.
    allocations: Vec<AllocationSite>,
    /// Addressable memory operation sites.
    memory: Vec<MemorySite>,
    /// Function call operation sites.
    calls: Vec<CallSite>,
    /// Continuation resume operation sites.
    resumes: Vec<ResumeSite>,
    /// Control-flow edge sites.
    edges: Vec<EdgeSite>,
    /// Coroutine suspension sites.
    suspensions: Vec<SuspensionSite>,
    /// Explicit counter sites.
    counters: Vec<CounterSite>,
    /// Explicit sample sites.
    samples: Vec<SampleSite>,
}

impl SiteTableBuilder {
    /// Create an empty program site table builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set allocation sites.
    pub fn allocations(mut self, allocations: impl IntoIterator<Item = AllocationSite>) -> Self {
        self.allocations = allocations.into_iter().collect();

        self
    }

    /// Set addressable memory operation sites.
    pub fn memory(mut self, memory: impl IntoIterator<Item = MemorySite>) -> Self {
        self.memory = memory.into_iter().collect();

        self
    }

    /// Set function call operation sites.
    pub fn calls(mut self, calls: impl IntoIterator<Item = CallSite>) -> Self {
        self.calls = calls.into_iter().collect();

        self
    }

    /// Set continuation resume operation sites.
    pub fn resumes(mut self, resumes: impl IntoIterator<Item = ResumeSite>) -> Self {
        self.resumes = resumes.into_iter().collect();

        self
    }

    /// Set control flow edge sites.
    pub fn edges(mut self, edges: impl IntoIterator<Item = EdgeSite>) -> Self {
        self.edges = edges.into_iter().collect();

        self
    }

    /// Set coroutine suspension sites.
    pub fn suspensions(mut self, suspensions: impl IntoIterator<Item = SuspensionSite>) -> Self {
        self.suspensions = suspensions.into_iter().collect();

        self
    }

    /// Set explicit counter sites.
    pub fn counters(mut self, counters: impl IntoIterator<Item = CounterSite>) -> Self {
        self.counters = counters.into_iter().collect();

        self
    }

    /// Set explicit sample sites.
    pub fn samples(mut self, samples: impl IntoIterator<Item = SampleSite>) -> Self {
        self.samples = samples.into_iter().collect();

        self
    }

    /// Build this site table into final program sections.
    pub(crate) fn build(mut self, sections: &mut SectionBuilder) -> SiteTable {
        self.allocations.sort_unstable_by_key(|site| site.point);
        self.memory.sort_unstable_by_key(|site| site.point);
        self.calls.sort_unstable_by_key(|site| site.point);
        self.resumes.sort_unstable_by_key(|site| site.point);
        self.edges
            .sort_unstable_by_key(|site| (site.source, site.target));
        self.edges.dedup_by_key(|site| (site.source, site.target));
        self.suspensions.sort_unstable_by_key(|site| site.point);
        self.counters.sort_unstable_by_key(|site| site.point);
        self.samples.sort_unstable_by_key(|site| site.point);

        SiteTable {
            allocations: sections.insert(self.allocations),
            memory: sections.insert(self.memory),
            calls: sections.insert(self.calls),
            resumes: sections.insert(self.resumes),
            edges: sections.insert(self.edges),
            suspensions: sections.insert(self.suspensions),
            counters: sections.insert(self.counters),
            samples: sections.insert(self.samples),
        }
    }
}

/// Dense allocation site identifier within one program.
#[repr(transparent)]
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
    Reflect,
    SectionEntry,
)]
pub struct AllocationSiteId(pub u32);

/// Dense call site identifier within one program.
#[repr(transparent)]
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
    Reflect,
    SectionEntry,
)]
pub struct CallSiteId(pub u32);

/// Dense continuation resume site identifier within one program.
#[repr(transparent)]
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
    Reflect,
    SectionEntry,
)]
pub struct ResumeSiteId(pub u32);

/// Dense control-flow edge site identifier within one program.
#[repr(transparent)]
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
    Reflect,
    SectionEntry,
)]
pub struct EdgeSiteId(pub u32);

/// Dense coroutine suspension site identifier within one program.
#[repr(transparent)]
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
    Reflect,
    SectionEntry,
)]
pub struct SuspensionSiteId(pub u32);

/// Dense program profile counter identifier within one program.
#[repr(transparent)]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, SectionEntry,
)]
pub struct CounterId(pub u32);

/// Dense program profile sampler identifier within one program.
#[repr(transparent)]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, SectionEntry,
)]
pub struct SamplerId(pub u32);

/// One allocation site.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct AllocationSite {
    /// The program point that performs the allocation.
    pub point: ProgramPoint,
    /// The storage space receiving the allocated storage.
    pub space: Space,
    /// The type produced by the allocation expression.
    pub result_type: TypeId,
    /// The type used for the allocated storage.
    pub storage_type: TypeId,
    /// The physical layout used for the allocated storage.
    pub layout: LayoutId,
    /// The virtual table written into the allocation when present.
    pub virtual_table: Optional<VirtualTableId>,
}

/// Addressable memory operation at one program point.
#[repr(C)]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, SectionEntry,
)]
pub struct MemorySite {
    /// The program point that performs the access.
    pub point: ProgramPoint,
    /// The memory operation performed at the site.
    pub access: MemoryAccess,
    /// The storage space accessed by the operation.
    pub space: Space,
    /// The loaded or stored value type.
    pub value_type: TypeId,
}

/// Memory access operation.
#[repr(u32)]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, SectionEntry,
)]
pub enum MemoryAccess {
    /// Read addressable storage.
    Read,
    /// Write addressable storage.
    Write,
    /// Read and write addressable storage.
    ReadWrite,
}

/// Function call operation at one program point.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct CallSite {
    /// The program point that performs the call.
    pub point: ProgramPoint,
    /// The program point entered after normal completion when the call returns.
    pub resume: Optional<ProgramPoint>,
    /// The program point entered during panic unwinding when present.
    pub unwind: Optional<ProgramPoint>,
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
    /// The callable signature.
    pub signature: SignatureId,
    /// The dispatch slot for virtual and dynamic calls.
    pub slot: Optional<u32>,
}

/// Continuation resume operation at one program point.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct ResumeSite {
    /// The program point that resumes the continuation.
    pub point: ProgramPoint,
    /// The program point entered when the continuation yields.
    pub yielded: ProgramPoint,
    /// The program point entered when the continuation returns.
    pub returned: ProgramPoint,
    /// The program point entered during panic unwinding.
    pub unwind: Optional<ProgramPoint>,
}

/// Control-flow edge between program points.
#[repr(C)]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, SectionEntry,
)]
pub struct EdgeSite {
    /// The program point producing the transfer.
    pub source: ProgramPoint,
    /// The program point entered after the transfer.
    pub target: ProgramPoint,
}

/// Coroutine suspension operation at one program point.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct SuspensionSite {
    /// The program point that suspends the coroutine.
    pub point: ProgramPoint,
    /// The program point entered by normal resumption.
    pub resume: ProgramPoint,
    /// The program point entered during cancellation when present.
    pub cancel: Optional<ProgramPoint>,
    /// The program point entered during panic unwinding.
    pub unwind: Optional<ProgramPoint>,
    /// The canonical frame state captured at this site.
    pub frame_state: FrameStateId,
    /// The suspension operation.
    pub operation: Suspension,
    /// The value passed to the coroutine owner.
    pub value_type: TypeId,
    /// The value received when execution resumes.
    pub resume_type: TypeId,
}

/// Coroutine suspension operation.
#[repr(u32)]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, SectionEntry,
)]
pub enum Suspension {
    /// Wait for an asynchronous value to fulfill.
    Await,
    /// Yield one value to a generator owner.
    Yield,
}

/// Explicit counter operation at one program point.
#[repr(C)]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, SectionEntry,
)]
pub struct CounterSite {
    /// The program point that increments the counter.
    pub point: ProgramPoint,
    /// The counter incremented at this site.
    pub counter: CounterId,
}

/// Explicit sample operation at one program point.
#[repr(C)]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, SectionEntry,
)]
pub struct SampleSite {
    /// The program point that records the sample.
    pub point: ProgramPoint,
    /// The sampler receiving values at this site.
    pub sampler: SamplerId,
    /// The sampled value type.
    pub value_type: TypeId,
}

/// Call return mode.
#[repr(u32)]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, SectionEntry,
)]
pub enum CallMode {
    /// Return to the caller after the callee finishes.
    Return,
    /// Replace the current frame with the callee frame.
    Tail,
}

/// Call dispatch mechanism.
#[repr(u32)]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, SectionEntry,
)]
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

impl ResumeSiteId {
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

impl SuspensionSiteId {
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

impl SamplerId {
    /// Return this sampler id as a dense array index.
    pub const fn index(self) -> usize {
        self.0 as usize
    }
}
