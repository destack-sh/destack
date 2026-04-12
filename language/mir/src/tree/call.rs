use serde::{Deserialize, Serialize};

use crate::{
    AllocationSize, ArgumentAttribute, CallBehavior, MemoryEffect, PointerAttribute, TypeReference,
};

/// Shared call facts for one call-like instruction or terminator.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Call<A> {
    /// The call arguments.
    pub arguments: A,
    /// The signature type for the callee.
    pub signature: TypeReference,
    /// Optional memory effect summary for this call.
    pub memory_effect: Option<MemoryEffect>,
    /// Optional behavioral summary for this call.
    pub behavior: Option<CallBehavior>,
    /// Optional allocation size metadata for allocator-like callees.
    pub allocation_size: Option<AllocationSize>,
    /// Per-argument attributes for this call.
    pub argument_attributes: Vec<ArgumentAttribute>,
    /// Pointer attribute for the return value.
    pub return_attribute: PointerAttribute,
}

impl<A> Call<A> {
    /// Create one call payload with default per-call facts.
    pub fn new(arguments: A, signature: TypeReference) -> Self {
        Self {
            arguments,
            signature,
            memory_effect: None,
            behavior: None,
            allocation_size: None,
            argument_attributes: Vec::new(),
            return_attribute: PointerAttribute::default(),
        }
    }
}
