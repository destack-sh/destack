use crate::{
    CallSite, DevirtualizationMetadata, Function, Instruction, InterfaceDispatchEntry,
    InterfaceSlotId, ItabEntry, LocalNodeId, NodeType, Terminator, Type, Value, VtableEntry,
    VtableSlotId,
};

use super::{ValidateAnchor, ValidateError, ValidateResult, Validator};

#[allow(clippy::type_complexity)]
impl<'a> Validator<'a> {
    /// Validate dispatch call facts metadata table invariants.
    pub(super) fn validate_dispatch(&self) -> ValidateResult<()> {
        // callsite facts
        for (&callsite, facts) in &self.tree.dispatch_table.callsite_metadata {
            if facts.is_empty() {
                return Err(ValidateError::MetadataInvariantViolation {
                    message: "dispatch callsite metadata must carry at least one fact".to_string(),
                    anchor: match callsite {
                        CallSite::Instruction(instruction_id) => {
                            ValidateAnchor::node(instruction_id)
                        }
                        CallSite::Terminator(block_id) => ValidateAnchor::node(block_id),
                    },
                });
            }

            // callsite ownership and signature
            match callsite {
                CallSite::Instruction(instruction_id) => {
                    self.ensure_node_type(
                        NodeType::Instruction,
                        instruction_id.id,
                        ValidateAnchor::node(instruction_id),
                    )?;

                    let instruction = self.tree.get(instruction_id);
                    let signature = match instruction {
                        Instruction::CallVirtual { signature, .. }
                        | Instruction::CallInterface { signature, .. } => *signature,
                        _ => {
                            return Err(ValidateError::MetadataInvariantViolation {
                                message:
                                    "dispatch callsite metadata attached to non dispatch instruction"
                                        .to_string(),
                                anchor: ValidateAnchor::node(instruction_id),
                            });
                        }
                    };

                    self.validate_dispatch_call_fact_target(
                        ValidateAnchor::node(instruction_id),
                        signature,
                        facts,
                    )?;
                }
                CallSite::Terminator(block_id) => {
                    self.ensure_node_type(
                        NodeType::Block,
                        block_id.id,
                        ValidateAnchor::node(block_id),
                    )?;

                    let block = self.tree.get(block_id);
                    let signature = match &block.terminator {
                        Terminator::CallVirtual { signature, .. }
                        | Terminator::CallInterface { signature, .. }
                        | Terminator::TailCallVirtual { signature, .. }
                        | Terminator::TailCallInterface { signature, .. } => *signature,
                        _ => {
                            return Err(ValidateError::MetadataInvariantViolation {
                                message:
                                    "dispatch callsite metadata attached to non dispatch terminator"
                                        .to_string(),
                                anchor: ValidateAnchor::node(block_id),
                            });
                        }
                    };

                    self.validate_dispatch_call_fact_target(
                        ValidateAnchor::node(block_id),
                        signature,
                        facts,
                    )?;
                }
            }
        }

        Ok(())
    }

    /// Validate dispatch call fact targets metadata.
    fn validate_dispatch_call_fact_target(
        &self,
        anchor: ValidateAnchor,
        signature: LocalNodeId<Type>,
        facts: &DevirtualizationMetadata,
    ) -> ValidateResult<()> {
        // declared target
        if let Some(declared_target) = facts.declared_target {
            self.validate_call_signature_matches_function(anchor, signature, declared_target)?;
        }

        Ok(())
    }

    /// Validate call signature invariants.
    pub(super) fn validate_call_signature(
        &self,
        instruction_id: LocalNodeId<Instruction>,
        destination: Option<Value>,
        argument_count: usize,
        signature: LocalNodeId<Type>,
    ) -> ValidateResult<()> {
        let anchor = ValidateAnchor::node(instruction_id);

        // signature type
        self.ensure_node_type(NodeType::Type, signature.id, anchor)?;

        let Type::FunctionPointer { parameters, result } = self.tree.get(signature) else {
            return Err(ValidateError::MetadataInvariantViolation {
                message: "call signature is not a function type".to_string(),
                anchor,
            });
        };

        // result type
        self.ensure_node_type(NodeType::Type, result.id, anchor)?;

        // argument count
        if argument_count != parameters.len() {
            return Err(ValidateError::CallArgumentCountMismatch {
                expected: parameters.len(),
                got: argument_count,
                anchor,
            });
        }

        // void destination
        let returns_void = matches!(self.tree.get(*result), Type::Void);
        if returns_void && destination.is_some() {
            return Err(ValidateError::CallReturnValueNotAllowedForVoid { anchor });
        }

        Ok(())
    }

    /// Validate call signatures against a concrete function.
    pub(super) fn validate_call_signature_matches_function(
        &self,
        anchor: ValidateAnchor,
        signature: LocalNodeId<Type>,
        function_id: LocalNodeId<Function>,
    ) -> ValidateResult<()> {
        // node kinds
        self.ensure_node_type(NodeType::Type, signature.id, anchor)?;
        self.ensure_node_type(NodeType::Function, function_id.id, anchor)?;

        let Type::FunctionPointer { parameters, result } = self.tree.get(signature) else {
            return Err(ValidateError::MetadataInvariantViolation {
                message: "call signature is not a function type".to_string(),
                anchor,
            });
        };

        // function signature shape
        self.ensure_node_type(NodeType::Type, result.id, anchor)?;
        for &parameter in parameters {
            self.ensure_node_type(NodeType::Type, parameter.id, anchor)?;
        }

        let function = self.tree.get(function_id);

        // arity
        if parameters.len() != function.parameters.len() {
            return Err(ValidateError::MetadataInvariantViolation {
                message: "call signature does not match callee".to_string(),
                anchor,
            });
        }

        // parameters
        for (parameter, signature_type) in function.parameters.iter().zip(parameters.iter()) {
            if parameter.ty != *signature_type {
                return Err(ValidateError::MetadataInvariantViolation {
                    message: "call signature does not match callee".to_string(),
                    anchor,
                });
            }
        }

        // result
        if function.return_type != *result {
            return Err(ValidateError::MetadataInvariantViolation {
                message: "call signature does not match callee".to_string(),
                anchor,
            });
        }

        Ok(())
    }

    /// Validate a virtual dispatch slot when vtable metadata is available.
    pub(super) fn validate_virtual_dispatch_slot(
        &self,
        declaring_type: LocalNodeId<Type>,
        slot_id: VtableSlotId,
        anchor: ValidateAnchor,
    ) -> ValidateResult<()> {
        let Some(vtable_id) = self.tree.type_table.vtable_id(declaring_type) else {
            return Ok(());
        };

        // vtable entry
        let Some(vtable) = self.tree.dispatch_table.vtables.get(vtable_id.index()) else {
            return Err(ValidateError::MetadataInvariantViolation {
                message: "virtual dispatch references missing vtable metadata".to_string(),
                anchor,
            });
        };
        let Some(entry) = vtable.entries.get(slot_id.index()) else {
            return Err(ValidateError::MetadataInvariantViolation {
                message: "virtual dispatch slot out of bounds".to_string(),
                anchor,
            });
        };

        if !matches!(entry, VtableEntry::Method { .. }) {
            return Err(ValidateError::MetadataInvariantViolation {
                message: "virtual dispatch slot does not reference a method entry".to_string(),
                anchor,
            });
        }

        Ok(())
    }

    /// Validate an interface dispatch slot when itab metadata is available.
    pub(super) fn validate_interface_dispatch_slot(
        &self,
        declaring_interface: LocalNodeId<Type>,
        slot_id: InterfaceSlotId,
        anchor: ValidateAnchor,
    ) -> ValidateResult<()> {
        // interface shape
        if let Some(shape) = self
            .tree
            .dispatch_table
            .interface_dispatch_shape(declaring_interface)
        {
            let Some(entry) = shape.entries.get(slot_id.index()) else {
                return Err(ValidateError::MetadataInvariantViolation {
                    message: "interface dispatch slot out of bounds".to_string(),
                    anchor,
                });
            };

            if !matches!(entry, InterfaceDispatchEntry::Method { .. }) {
                return Err(ValidateError::MetadataInvariantViolation {
                    message: "interface dispatch slot does not reference a method entry"
                        .to_string(),
                    anchor,
                });
            }

            return Ok(());
        }

        // itab consistency
        let mut found_itab = false;
        let mut declared_method = None;
        for (_itab_id, itab) in self.tree.dispatch_table.iter_itabs() {
            if itab.interface != declaring_interface {
                continue;
            }
            found_itab = true;

            let Some(entry) = itab.entries.get(slot_id.index()) else {
                return Err(ValidateError::MetadataInvariantViolation {
                    message: "interface dispatch slot out of bounds".to_string(),
                    anchor,
                });
            };
            let ItabEntry::Method {
                declared_method: slot_declared_method,
                ..
            } = entry
            else {
                return Err(ValidateError::MetadataInvariantViolation {
                    message: "interface dispatch slot does not reference a method entry"
                        .to_string(),
                    anchor,
                });
            };

            if let Some(expected_declared_method) = declared_method {
                if expected_declared_method != *slot_declared_method {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "interface dispatch slot maps inconsistent declared methods"
                            .to_string(),
                        anchor,
                    });
                }
            } else {
                declared_method = Some(*slot_declared_method);
            }
        }

        if !found_itab {
            return Ok(());
        }

        Ok(())
    }
}
