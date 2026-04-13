use crate::{
    Function, FunctionReference, Instruction, InterfaceDispatchEntry, InterfaceSlotId, ItabEntry,
    LocalNodeId, NodeType, Terminator, Type, TypeReference, ValueReference, VtableEntry,
    VtableSlotId,
};

use super::{ValidateAnchor, ValidateError, ValidateResult, Validator};

#[allow(clippy::type_complexity)]
impl<'a> Validator<'a> {
    /// Validate dispatch call facts metadata table invariants.
    pub(super) fn validate_dispatch(&self) -> ValidateResult<()> {
        // dynamic call instructions
        for (instruction_id, instruction) in self.tree.iter_nodes::<Instruction>() {
            match instruction {
                Instruction::CallVirtual {
                    declared_target,
                    call,
                    ..
                }
                | Instruction::CallInterface {
                    declared_target,
                    call,
                    ..
                } => {
                    self.validate_dispatch_call_fact_target(
                        ValidateAnchor::node(instruction_id),
                        call.signature,
                        *declared_target,
                    )?;
                }
                _ => {}
            }
        }

        // dynamic call terminators
        for (block_id, block) in self.tree.iter_nodes::<crate::Block>() {
            let terminator = self.tree.get(block.terminator);

            match terminator {
                Terminator::InvokeVirtual {
                    declared_target,
                    call,
                    ..
                }
                | Terminator::InvokeInterface {
                    declared_target,
                    call,
                    ..
                }
                | Terminator::TailCallVirtual {
                    declared_target,
                    call,
                    ..
                }
                | Terminator::TailCallInterface {
                    declared_target,
                    call,
                    ..
                } => {
                    self.validate_dispatch_call_fact_target(
                        ValidateAnchor::node(block_id),
                        call.signature,
                        *declared_target,
                    )?;
                }
                _ => {}
            }
        }

        Ok(())
    }

    /// Validate dispatch call fact targets metadata.
    fn validate_dispatch_call_fact_target(
        &self,
        anchor: ValidateAnchor,
        signature: TypeReference,
        declared_target: Option<FunctionReference>,
    ) -> ValidateResult<()> {
        if let Some(declared_target) = declared_target {
            let declared_target =
                self.require_function_reference(declared_target, anchor, "declared call target")?;
            self.validate_call_signature_matches_function(anchor, signature, declared_target)?;
        }

        Ok(())
    }

    /// Validate call signature invariants.
    pub(super) fn validate_call_signature(
        &self,
        instruction_id: LocalNodeId<Instruction>,
        destination: Option<ValueReference>,
        argument_count: usize,
        signature: TypeReference,
    ) -> ValidateResult<()> {
        let anchor = ValidateAnchor::node(instruction_id);

        // signature type
        let signature = self.require_type_reference(signature, anchor, "call signature")?;
        self.ensure_node_type(NodeType::Type, signature.id, anchor)?;

        let signature = match self.tree.get(signature) {
            Type::FunctionPointer { .. } => signature,
            Type::Closure { signature, .. } => {
                self.require_type_reference(*signature, anchor, "callable signature")?
            }
            _ => {
                return Err(ValidateError::MetadataInvariantViolation {
                    message: "call signature is not a function type".to_string(),
                    anchor,
                });
            }
        };
        self.ensure_node_type(NodeType::Type, signature.id, anchor)?;

        let Type::FunctionPointer { parameters, result } = self.tree.get(signature) else {
            return Err(ValidateError::MetadataInvariantViolation {
                message: "call signature is not a function type".to_string(),
                anchor,
            });
        };

        // result type
        let result = self.require_type_reference(*result, anchor, "call result type")?;
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
        let returns_void = matches!(self.tree.get(result), Type::Void);
        if returns_void && destination.is_some() {
            return Err(ValidateError::CallReturnValueNotAllowedForVoid { anchor });
        }

        Ok(())
    }

    /// Validate call signatures against a concrete function.
    pub(super) fn validate_call_signature_matches_function(
        &self,
        anchor: ValidateAnchor,
        signature: TypeReference,
        function_id: LocalNodeId<Function>,
    ) -> ValidateResult<()> {
        // node kinds
        let signature = self.require_type_reference(signature, anchor, "call signature")?;
        self.ensure_node_type(NodeType::Type, signature.id, anchor)?;
        self.ensure_node_type(NodeType::Function, function_id.id, anchor)?;

        let Type::FunctionPointer { parameters, result } = self.tree.get(signature) else {
            return Err(ValidateError::MetadataInvariantViolation {
                message: "call signature is not a function type".to_string(),
                anchor,
            });
        };

        // function signature shape
        let result = self.require_type_reference(*result, anchor, "call result type")?;
        self.ensure_node_type(NodeType::Type, result.id, anchor)?;
        for &parameter in parameters {
            let parameter =
                self.require_type_reference(parameter, anchor, "call parameter type")?;
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
        if function.return_type != result.into() {
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
        let Some(vtable_id) = self.tree.metadata.dispatch.vtable_id(declaring_type) else {
            return Ok(());
        };

        // vtable entry
        let Some(vtable) = self.tree.metadata.dispatch.vtables.get(vtable_id.index()) else {
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
            .metadata
            .dispatch
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
        for (_itab_id, itab) in self.tree.metadata.dispatch.iter_itabs() {
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
                if expected_declared_method != slot_declared_method {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "interface dispatch slot maps inconsistent declared methods"
                            .to_string(),
                        anchor,
                    });
                }
            } else {
                declared_method = Some(slot_declared_method);
            }
        }

        if !found_itab {
            return Ok(());
        }

        Ok(())
    }

    /// Reject direct calls to functions that require a hidden environment.
    pub(super) fn validate_direct_call_environment(
        &self,
        anchor: ValidateAnchor,
        function_id: LocalNodeId<Function>,
    ) -> ValidateResult<()> {
        let function = self.tree.get(function_id);
        if function.environment.is_some() {
            return Err(ValidateError::MetadataInvariantViolation {
                message: "direct call cannot target a function with an environment".to_string(),
                anchor,
            });
        }

        Ok(())
    }
}
