use std::collections::HashMap;

use crate::{Function, Instruction, LocalNodeId, Type, Value};

use super::{AllocSize, CallArgumentMetadata, CallBehavior, MemoryEffect, PointerAttributes};

/// Dispatch style for a callsite.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CallDispatchKind {
    /// Direct call to a known function symbol.
    Direct,
    /// Virtual dispatch through a vtable slot.
    Virtual { slot_id: u32 },
    /// Interface dispatch through an itab slot.
    Interface { slot_id: u32 },
    /// Indirect call through a function pointer.
    Indirect,
    /// Dynamic dispatch with multiple possible targets.
    Dynamic,
}

impl CallDispatchKind {
    /// Return true when this dispatch expects a receiver value.
    pub fn expects_receiver(self) -> bool {
        matches!(
            self,
            CallDispatchKind::Virtual { .. } | CallDispatchKind::Interface { .. }
        )
    }

    /// Return the slot id if this dispatch uses one.
    pub fn slot_id(self) -> Option<u32> {
        match self {
            CallDispatchKind::Virtual { slot_id } | CallDispatchKind::Interface { slot_id } => {
                Some(slot_id)
            }
            _ => None,
        }
    }
}

/// Metadata describing a callsite's dispatch semantics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallMetadata {
    /// Dispatch style for this callsite.
    pub dispatch: CallDispatchKind,
    /// The receiver value for virtual or interface dispatch.
    pub receiver: Option<Value>,
    /// The declared function target, when known.
    pub declared_target: Option<LocalNodeId<Function>>,
    /// The declared receiver type, when known.
    pub declaring_type: Option<LocalNodeId<Type>>,
    /// The function signature type.
    pub signature: LocalNodeId<Type>,
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

impl CallMetadata {
    /// Create metadata for a direct call.
    pub fn direct(target: LocalNodeId<Function>, signature: LocalNodeId<Type>) -> Self {
        Self {
            dispatch: CallDispatchKind::Direct,
            receiver: None,
            declared_target: Some(target),
            declaring_type: None,
            signature,
            memory_effects: None,
            behavior: None,
            alloc_size: None,
            argument_metadata: Vec::new(),
            return_attributes: PointerAttributes::default(),
        }
    }

    /// Create metadata for a virtual call.
    pub fn virtual_call(
        receiver: Value,
        declaring_type: LocalNodeId<Type>,
        slot_id: u32,
        signature: LocalNodeId<Type>,
        declared_target: Option<LocalNodeId<Function>>,
    ) -> Self {
        Self {
            dispatch: CallDispatchKind::Virtual { slot_id },
            receiver: Some(receiver),
            declared_target,
            declaring_type: Some(declaring_type),
            signature,
            memory_effects: None,
            behavior: None,
            alloc_size: None,
            argument_metadata: Vec::new(),
            return_attributes: PointerAttributes::default(),
        }
    }

    /// Create metadata for an interface call.
    pub fn interface_call(
        receiver: Value,
        declaring_type: LocalNodeId<Type>,
        slot_id: u32,
        signature: LocalNodeId<Type>,
        declared_target: Option<LocalNodeId<Function>>,
    ) -> Self {
        Self {
            dispatch: CallDispatchKind::Interface { slot_id },
            receiver: Some(receiver),
            declared_target,
            declaring_type: Some(declaring_type),
            signature,
            memory_effects: None,
            behavior: None,
            alloc_size: None,
            argument_metadata: Vec::new(),
            return_attributes: PointerAttributes::default(),
        }
    }

    /// Create metadata for an indirect call.
    pub fn indirect(signature: LocalNodeId<Type>) -> Self {
        Self {
            dispatch: CallDispatchKind::Indirect,
            receiver: None,
            declared_target: None,
            declaring_type: None,
            signature,
            memory_effects: None,
            behavior: None,
            alloc_size: None,
            argument_metadata: Vec::new(),
            return_attributes: PointerAttributes::default(),
        }
    }

    /// Create metadata for a dynamic call.
    pub fn dynamic(signature: LocalNodeId<Type>) -> Self {
        Self {
            dispatch: CallDispatchKind::Dynamic,
            receiver: None,
            declared_target: None,
            declaring_type: None,
            signature,
            memory_effects: None,
            behavior: None,
            alloc_size: None,
            argument_metadata: Vec::new(),
            return_attributes: PointerAttributes::default(),
        }
    }

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

    /// Return true when this callsite expects a receiver value.
    pub fn expects_receiver(&self) -> bool {
        self.dispatch.expects_receiver()
    }
}

/// Table of callsite metadata entries.
#[derive(Clone, Debug, Default)]
pub struct CallTable {
    /// Callsite metadata keyed by instruction id.
    pub call_metadata_by_instruction_id: HashMap<LocalNodeId<Instruction>, CallMetadata>,
}

impl CallTable {
    /// Create a new empty call table.
    pub fn new() -> Self {
        Self::default()
    }

    /// Return call metadata for an instruction id.
    pub fn call_metadata(&self, instruction: LocalNodeId<Instruction>) -> Option<&CallMetadata> {
        self.call_metadata_by_instruction_id.get(&instruction)
    }

    /// Return mutable call metadata for an instruction id.
    pub fn call_metadata_mut(
        &mut self,
        instruction: LocalNodeId<Instruction>,
    ) -> Option<&mut CallMetadata> {
        self.call_metadata_by_instruction_id.get_mut(&instruction)
    }

    /// Insert call metadata for an instruction id.
    pub fn insert_call_metadata(
        &mut self,
        instruction: LocalNodeId<Instruction>,
        metadata: CallMetadata,
    ) -> Option<CallMetadata> {
        self.call_metadata_by_instruction_id
            .insert(instruction, metadata)
    }

    /// Remove call metadata for an instruction id.
    pub fn remove_call_metadata(
        &mut self,
        instruction: LocalNodeId<Instruction>,
    ) -> Option<CallMetadata> {
        self.call_metadata_by_instruction_id.remove(&instruction)
    }
}
