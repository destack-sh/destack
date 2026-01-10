use std::collections::HashMap;

use crate::{Function, Instruction, LocalNodeId, Type, Value};

/// Metadata table for MIR types and callsites.
///
/// Contains additional information about types that lowering populates
/// from DIR type info. Currently tracks drop functions for types
/// that implement Drop, plus callsite metadata.
#[derive(Clone, Debug, Default)]
pub struct MetadataTable {
    /// Drop function for each type that implements Drop.
    /// Maps type id → drop function id.
    pub drop_function_by_type_id: HashMap<LocalNodeId<Type>, LocalNodeId<Function>>,
    /// Callsite metadata keyed by instruction id.
    pub call_metadata_by_instruction_id: HashMap<LocalNodeId<Instruction>, CallMetadata>,
}

impl MetadataTable {
    /// Create a new empty metadata table.
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
    /// The receiver value for virtual/interface dispatch.
    pub receiver: Option<Value>,
    /// The declared function target, when known.
    pub declared_target: Option<LocalNodeId<Function>>,
    /// The declared receiver type, when known.
    pub declaring_type: Option<LocalNodeId<Type>>,
    /// The function signature type.
    pub signature: LocalNodeId<Type>,
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
        }
    }

    /// Return true when this callsite expects a receiver value.
    pub fn expects_receiver(&self) -> bool {
        self.dispatch.expects_receiver()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Direct call metadata carries a declared target.
    #[test]
    fn test_call_metadata_direct_target() {
        let target = LocalNodeId::<Function>::new(1);
        let signature = LocalNodeId::<Type>::new(2);
        let meta = CallMetadata::direct(target, signature);

        assert_eq!(meta.declared_target, Some(target));
        assert!(!meta.expects_receiver());
    }

    /// Virtual call metadata carries receiver and slot information.
    #[test]
    fn test_call_metadata_virtual_slot() {
        let receiver = Value::new(3);
        let declaring_type = LocalNodeId::<Type>::new(4);
        let signature = LocalNodeId::<Type>::new(5);
        let meta = CallMetadata::virtual_call(receiver, declaring_type, 7, signature, None);

        assert_eq!(meta.receiver, Some(receiver));
        assert_eq!(meta.dispatch.slot_id(), Some(7));
        assert!(meta.expects_receiver());
    }

    /// Metadata table stores call metadata entries.
    #[test]
    fn test_metadata_table_call_metadata_roundtrip() {
        let mut table = MetadataTable::new();
        let instruction = LocalNodeId::<Instruction>::new(8);
        let target = LocalNodeId::<Function>::new(9);
        let signature = LocalNodeId::<Type>::new(10);
        let metadata = CallMetadata::direct(target, signature);

        assert!(table.call_metadata(instruction).is_none());

        table.insert_call_metadata(instruction, metadata.clone());

        assert_eq!(table.call_metadata(instruction), Some(&metadata));

        table.remove_call_metadata(instruction);

        assert!(table.call_metadata(instruction).is_none());
    }
}
