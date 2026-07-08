use destack_core::{Optional, SectionEntry, SectionImage, SectionPacker, SectionSlice};
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use super::{AddressSpace, FunctionId, LayoutId, ProgramPoint, TypeId};

/// Executable program sites used by debugging, probes, and observations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct SiteTable {
    /// Heap allocation operation sites sorted by program point.
    allocations: SectionSlice<AllocationSite>,
    /// Addressable memory operation sites sorted by program point.
    memory: SectionSlice<MemorySite>,
    /// Function call operation sites sorted by program point.
    calls: SectionSlice<CallSite>,
}

impl SiteTable {
    /// Pack executable site rows.
    pub fn pack(
        sections: &mut SectionPacker,
        mut allocations: Vec<AllocationSite>,
        mut memory: Vec<MemorySite>,
        mut calls: Vec<CallSite>,
    ) -> Self {
        allocations.sort_unstable_by_key(|site| site.point);
        memory.sort_unstable_by_key(|site| site.point);
        calls.sort_unstable_by_key(|site| site.point);

        let allocations = sections.insert(allocations);
        let memory = sections.insert(memory);
        let calls = sections.insert(calls);

        Self {
            allocations,
            memory,
            calls,
        }
    }

    /// Return the allocation site at one executable point.
    pub fn allocation<'a>(
        &self,
        sections: SectionImage<'a>,
        point: ProgramPoint,
    ) -> Option<&'a AllocationSite> {
        let allocations = self.allocations(sections);
        let index = allocations
            .binary_search_by_key(&point, |site| site.point)
            .ok()?;

        Some(&allocations[index])
    }

    /// Return all allocation site rows.
    pub fn allocations<'a>(&self, sections: SectionImage<'a>) -> &'a [AllocationSite] {
        sections.entries(self.allocations)
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
    ) -> Option<&'a CallSite> {
        let calls = self.calls(sections);
        let index = calls.binary_search_by_key(&point, |site| site.point).ok()?;

        Some(&calls[index])
    }

    /// Return all call site rows.
    pub fn calls<'a>(&self, sections: SectionImage<'a>) -> &'a [CallSite] {
        sections.entries(self.calls)
    }
}

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
    /// The address space receiving the allocated storage.
    pub address_space: AddressSpace,
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
    /// The address space accessed by the operation.
    pub address_space: AddressSpace,
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
    /// The receiver address space for virtual and dynamic calls.
    pub address_space: Optional<AddressSpace>,
    /// The resolved callee for direct calls.
    pub target: Optional<FunctionId>,
    /// The type that defines the virtual or dynamic dispatch slot.
    pub dispatch_type: Optional<TypeId>,
    /// The program signature type for this call.
    pub signature_type: TypeId,
    /// The dispatch slot for virtual and dynamic calls.
    pub slot: Optional<u32>,
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

// SAFETY: site rows are fixed-width program section entries.
unsafe impl SectionEntry for AllocationSite {}
unsafe impl SectionEntry for AllocationOperation {}
unsafe impl SectionEntry for AllocationInitialization {}
unsafe impl SectionEntry for MemorySite {}
unsafe impl SectionEntry for MemoryAccess {}
unsafe impl SectionEntry for CallSite {}
unsafe impl SectionEntry for CallMode {}
unsafe impl SectionEntry for CallDispatch {}
