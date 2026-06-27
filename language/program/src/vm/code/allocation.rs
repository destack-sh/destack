use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use super::{AllocationPlanId, Edge, SliceProjectionId};

/// Branching allocation consumed by fallible heap allocation instructions.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct AllocationBranch {
    /// The destination frame offset.
    pub destination: u32,
    /// The allocation plan.
    pub allocation: AllocationPlanId,
    /// The success edge.
    pub success: Edge,
    /// The failure edge.
    pub failure: Edge,
}

/// Branching slice allocation consumed by fallible slice allocation instructions.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct SliceAllocationBranch {
    /// The destination frame offset.
    pub destination: u32,
    /// The length cell frame offset.
    pub length: u32,
    /// The backing element allocation plan.
    pub element: AllocationPlanId,
    /// The slice descriptor projection.
    pub access: SliceProjectionId,
    /// The success edge.
    pub success: Edge,
    /// The failure edge.
    pub failure: Edge,
}
