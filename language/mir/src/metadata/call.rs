use serde::{Deserialize, Serialize};

use super::{AllocSize, CallArgumentMetadata, CallBehavior, MemoryEffect, PointerAttributes};

/// Effects and attributes for a callsite.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct CallEffects {
    /// The memory effects for this callsite.
    pub memory_effects: Option<MemoryEffect>,
    /// Behavioral effects for this callsite.
    pub behavior: Option<CallBehavior>,
    /// Allocation size metadata for allocator-like calls.
    pub alloc_size: Option<AllocSize>,
    /// Argument metadata for pointer related effects.
    pub argument_metadata: Vec<CallArgumentMetadata>,
    /// Pointer attributes for the return value.
    pub return_attributes: PointerAttributes,
}

impl CallEffects {
    /// Set the memory effects for this callsite.
    pub fn with_memory_effects(mut self, effects: MemoryEffect) -> Self {
        self.memory_effects = Some(effects);
        self
    }

    /// Set the argument metadata for this callsite.
    pub fn with_argument_metadata(mut self, metadata: Vec<CallArgumentMetadata>) -> Self {
        self.argument_metadata = metadata;
        self
    }

    /// Set the call behavior for this callsite.
    pub fn with_behavior(mut self, behavior: CallBehavior) -> Self {
        self.behavior = Some(behavior);
        self
    }

    /// Set the allocation size metadata for this callsite.
    pub fn with_alloc_size(mut self, alloc_size: AllocSize) -> Self {
        self.alloc_size = Some(alloc_size);
        self
    }

    /// Set the return pointer attributes for this callsite.
    pub fn with_return_attributes(mut self, attributes: PointerAttributes) -> Self {
        self.return_attributes = attributes;
        self
    }
}
