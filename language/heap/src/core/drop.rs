use destack_core::SectionEntry;
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::{
    GcCollector, HeapError, HeapReference, HeapRepresentationError, HeapResult, SharedHeapReference,
};

/// Dense identity for one allocation drop function.
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
pub struct DropId(u32);

impl DropId {
    /// Create one drop identity from its dense index.
    pub const fn from_index(index: u32) -> Self {
        Self(index)
    }

    /// Return the zero-based drop table index.
    pub const fn index(self) -> usize {
        self.0 as usize
    }
}

const _: () = assert!(std::mem::size_of::<DropId>() == std::mem::size_of::<u32>());
/// The number of adjacent values covered by one allocation drop.
#[repr(u32)]
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
pub enum DropCardinality {
    /// Drop exactly one value.
    One,
    /// Drop every value in one exact-sized repeated allocation.
    Repeated,
}

/// Destruction required by one managed allocation.
#[repr(C)]
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
pub struct DropPlan {
    /// Drop function identity.
    pub drop: DropId,
    /// The exact byte stride between adjacent values.
    pub stride: u32,
    /// The number of adjacent values covered by this plan.
    pub cardinality: DropCardinality,
}

impl DropPlan {
    /// Create one single-value drop plan.
    pub fn one(drop: DropId, stride: usize) -> HeapResult<Self> {
        // encode the runtime stride in the compact allocation plan
        let stride = u32::try_from(stride).map_err(|_| {
            HeapError::representation(HeapRepresentationError::InvalidDropLayout {
                byte_len: stride,
                stride,
            })
        })?;

        // reject the only cardinality that cannot name a value
        if stride == 0 {
            return Err(HeapError::representation(
                HeapRepresentationError::InvalidDropLayout {
                    byte_len: 0,
                    stride: 0,
                },
            ));
        }

        Ok(Self {
            drop,
            stride,
            cardinality: DropCardinality::One,
        })
    }

    /// Return this plan for a repeated allocation.
    pub const fn repeated(mut self) -> Self {
        self.cardinality = DropCardinality::Repeated;

        self
    }

    /// Return the exact byte stride between adjacent values.
    pub const fn stride(self) -> usize {
        self.stride as usize
    }

    /// Return the number of values this plan covers in one allocation.
    pub fn value_count(self, byte_len: usize) -> HeapResult<usize> {
        let stride = self.stride();
        let count = match self.cardinality {
            DropCardinality::One if stride > 0 && byte_len >= stride => 1,
            DropCardinality::Repeated
                if stride > 0 && byte_len >= stride && byte_len.is_multiple_of(stride) =>
            {
                byte_len / stride
            }
            _ => {
                return Err(HeapError::representation(
                    HeapRepresentationError::InvalidDropLayout { byte_len, stride },
                ));
            }
        };

        Ok(count)
    }
}

/// The outcome of releasing one uniquely owned allocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Release {
    /// The allocation was freed.
    Freed,
    /// Heap storage retained the allocation for the collector.
    Retained,
    /// The caller destroys the allocation's values through this plan, then frees it.
    Destroy {
        /// The destruction plan.
        plan: DropPlan,
        /// The allocation byte length selecting the value count.
        byte_len: usize,
    },
}

const _: () = assert!(std::mem::size_of::<DropPlan>() == 3 * std::mem::size_of::<u32>());
/// Heap value selected for one drop function.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum DropReference {
    /// Worker-local heap allocation.
    Local(HeapReference),
    /// Runtime-shared heap allocation.
    Shared(SharedHeapReference),
}

impl DropReference {
    /// Add one byte offset to this reference.
    fn offset(self, byte_offset: usize) -> Self {
        match self {
            Self::Local(reference) => {
                Self::Local(HeapReference::new(reference.offset() + byte_offset))
            }
            Self::Shared(reference) => {
                Self::Shared(SharedHeapReference::new(reference.offset() + byte_offset))
            }
        }
    }
}

/// One unreachable value that must run Drop before reclamation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct GcDrop {
    /// Collector that discovered the value.
    pub collector: GcCollector,
    /// Whether this increment also started the collection cycle.
    pub is_start: bool,
    /// Value selected for Drop.
    pub reference: DropReference,
    /// Drop function identity.
    pub drop: DropId,
    /// Requested work budget in bytes.
    pub budget_bytes: usize,
    /// Actual charged work in bytes.
    pub work_bytes: usize,
}

/// Incremental Drop progress for one unreachable allocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DropCursor {
    /// Collector that discovered the allocation.
    collector: GcCollector,
    /// Allocation base reference.
    base: DropReference,
    /// Drop function identity.
    drop: DropId,
    /// The exact byte stride between adjacent values.
    stride: usize,
    /// The number of values still requiring Drop.
    remaining: usize,
    /// Whether one value is currently claimed by the runtime.
    is_claimed: bool,
    /// Work charged by the next claim.
    work_bytes: usize,
}

impl DropCursor {
    /// Create incremental Drop progress for one allocation.
    pub(crate) fn new(
        collector: GcCollector,
        base: DropReference,
        byte_len: usize,
        plan: DropPlan,
        work_bytes: usize,
    ) -> HeapResult<Self> {
        // derive exact value cardinality from the allocation shape
        let stride = plan.stride();
        let remaining = plan.value_count(byte_len)?;

        Ok(Self {
            collector,
            base,
            drop: plan.drop,
            stride,
            remaining,
            is_claimed: false,
            work_bytes: work_bytes + stride,
        })
    }

    /// Claim the next value in reverse acquisition order.
    pub(crate) fn claim(&mut self, budget_bytes: usize) -> HeapResult<GcDrop> {
        // reject overlapping or exhausted claims
        if self.is_claimed || self.remaining == 0 {
            return Err(HeapError::internal("invalid Drop cursor claim"));
        }

        // select the last value that has not completed Drop
        let byte_offset = (self.remaining - 1) * self.stride;
        let drop = GcDrop {
            collector: self.collector,
            is_start: false,
            reference: self.base.offset(byte_offset),
            drop: self.drop,
            budget_bytes,
            work_bytes: self.work_bytes,
        };

        // retain the claim until the runtime completes the destructor
        self.is_claimed = true;
        self.work_bytes = self.stride;

        Ok(drop)
    }

    /// Complete the claimed value.
    pub(crate) fn complete(&mut self, reference: DropReference) -> HeapResult<()> {
        // require one active claim
        if !self.is_claimed {
            return Err(HeapError::internal("Drop cursor has no claimed value"));
        }

        // require completion of the exact value that was claimed
        let expected_offset = (self.remaining - 1) * self.stride;
        if self.base.offset(expected_offset) != reference {
            return Err(HeapError::internal("Drop cursor claim mismatch"));
        }

        // advance to the preceding value
        self.remaining -= 1;
        self.is_claimed = false;

        Ok(())
    }

    /// Return whether one value is currently claimed by the runtime.
    pub(crate) const fn is_claimed(self) -> bool {
        self.is_claimed
    }

    /// Return whether every value completed Drop.
    pub(crate) const fn is_complete(self) -> bool {
        self.remaining == 0
    }
}
