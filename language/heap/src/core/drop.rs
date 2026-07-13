use std::num::NonZeroU32;

use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::{GcCollector, HeapReference, SharedHeapReference};

/// Dense identity for one allocation drop function.
#[repr(transparent)]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Reflect,
)]
pub struct DropId(NonZeroU32);

impl DropId {
    /// Create one drop identity from its dense index.
    pub const fn from_index(index: u32) -> Self {
        match NonZeroU32::new(index + 1) {
            Some(raw) => Self(raw),
            None => unreachable!(),
        }
    }

    /// Return the zero-based drop table index.
    pub const fn index(self) -> usize {
        self.0.get() as usize - 1
    }
}

const _: () = assert!(std::mem::size_of::<DropId>() == std::mem::size_of::<u32>());
const _: () = assert!(std::mem::size_of::<Option<DropId>>() == std::mem::size_of::<u32>());

/// The number of adjacent values covered by one allocation drop.
#[repr(u32)]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Reflect,
)]
pub enum DropCardinality {
    /// Drop exactly one value.
    One,
    /// Drop every value in one exact-sized repeated allocation.
    Repeated,
}

/// Destruction required by one managed allocation.
#[repr(C)]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Reflect,
)]
pub struct DropPlan {
    /// Drop function identity.
    pub drop: DropId,
    /// The number of adjacent values covered by this plan.
    pub cardinality: DropCardinality,
}

impl DropPlan {
    /// Create one single-value drop plan.
    pub const fn one(drop: DropId) -> Self {
        Self {
            drop,
            cardinality: DropCardinality::One,
        }
    }

    /// Return this plan for a repeated allocation.
    pub const fn repeated(mut self) -> Self {
        self.cardinality = DropCardinality::Repeated;

        self
    }
}

const _: () = assert!(std::mem::size_of::<DropPlan>() == 2 * std::mem::size_of::<u32>());
const _: () = assert!(std::mem::size_of::<Option<DropPlan>>() == 2 * std::mem::size_of::<u32>());

/// Heap allocation selected for one drop function.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum DropReference {
    /// Worker-local heap allocation.
    Local(HeapReference),
    /// Runtime-shared heap allocation.
    Shared(SharedHeapReference),
}

/// One unreachable allocation that must run Drop before reclamation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct GcDrop {
    /// Collector that discovered the allocation.
    pub collector: GcCollector,
    /// Whether this increment also started the collection cycle.
    pub is_start: bool,
    /// Allocation selected for Drop.
    pub reference: DropReference,
    /// The exact byte length occupied by values presented to Drop.
    pub byte_len: usize,
    /// Destruction required by the allocation.
    pub plan: DropPlan,
    /// Requested work budget in bytes.
    pub budget_bytes: usize,
    /// Actual charged work in bytes.
    pub work_bytes: usize,
}
