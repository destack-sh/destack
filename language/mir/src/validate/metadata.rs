use crate::{
    CallEffects, Function, Instruction, LocalNodeId, MemoryAccessKind, MemoryAccessMetadata,
    MemoryEffect, NodeType, Repeatability, Type, Value,
};

use super::{ValidateAnchor, ValidateError, ValidateResult, Validator};

#[allow(clippy::type_complexity)]
impl<'a> Validator<'a> {
    /// Validate metadata tables after function checks.
    pub(super) fn validate_metadata_tables(&self) -> ValidateResult<()> {
        // validate memory metadata entries
        for (&instruction_id, accesses) in &self.tree.memory_table.memory_accesses_by_instruction_id
        {
            // ensure the instruction id resolves
            self.ensure_node_type(
                NodeType::Instruction,
                instruction_id.id,
                ValidateAnchor::node(instruction_id),
            )?;

            // ensure the instruction can carry memory metadata
            let instruction = self.tree.get(instruction_id);
            if !matches!(
                instruction,
                Instruction::Load { .. }
                    | Instruction::Store { .. }
                    | Instruction::Intrinsic { .. }
            ) {
                return Err(ValidateError::MetadataInvariantViolation {
                    message: "memory metadata attached to non memory instruction".to_string(),
                    anchor: ValidateAnchor::node(instruction_id),
                });
            }

            // validate memory access entries
            for access in accesses {
                self.validate_memory_access_invariants(
                    access,
                    ValidateAnchor::node(instruction_id),
                )?;
            }
        }

        Ok(())
    }

    /// Validate function metadata invariants.
    pub(super) fn validate_function_metadata(
        &self,
        function_id: LocalNodeId<Function>,
        function: &Function,
    ) -> ValidateResult<()> {
        // ensure parameter metadata aligns with parameters
        if function.parameter_attributes.len() != function.parameters.len() {
            return Err(ValidateError::MetadataInvariantViolation {
                message: format!(
                    "parameter attributes length mismatch expected {} got {}",
                    function.parameters.len(),
                    function.parameter_attributes.len()
                ),
                anchor: ValidateAnchor::node(function_id),
            });
        }

        // validate memory effects
        self.validate_memory_effect_invariants(
            function.memory_effects.as_ref(),
            ValidateAnchor::node(function_id),
        )?;

        // validate call behavior
        self.validate_call_behavior_invariants(
            function.call_behavior.as_ref(),
            ValidateAnchor::node(function_id),
        )?;

        // validate return attributes
        self.validate_pointer_attributes_invariants(
            "function return attributes",
            &function.return_attributes,
            ValidateAnchor::node(function_id),
        )?;

        // validate parameter attributes
        for attributes in &function.parameter_attributes {
            self.validate_pointer_attributes_invariants(
                "function parameter attributes",
                attributes,
                ValidateAnchor::node(function_id),
            )?;
        }

        // validate closure env type when present
        if let Some(env_type) = function.closure_env_type {
            let env_type = self.tree.get(env_type);
            if !matches!(env_type, Type::Reference { .. }) {
                return Err(ValidateError::MetadataInvariantViolation {
                    message: "closure_env type must be a reference".to_string(),
                    anchor: ValidateAnchor::node(function_id),
                });
            }
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
        // resolve the signature type
        let Type::FunctionPointer { parameters, result } = self.tree.get(signature) else {
            return Err(ValidateError::MetadataInvariantViolation {
                message: "call signature is not a function type".to_string(),
                anchor: ValidateAnchor::node(instruction_id),
            });
        };

        // reject mismatched argument counts
        if argument_count != parameters.len() {
            return Err(ValidateError::CallArgumentCountMismatch {
                expected: parameters.len(),
                got: argument_count,
                anchor: ValidateAnchor::node(instruction_id),
            });
        }

        let returns_void = matches!(self.tree.get(*result), Type::Void);

        // reject return values from void callees
        if returns_void && destination.is_some() {
            return Err(ValidateError::CallReturnValueNotAllowedForVoid {
                anchor: ValidateAnchor::node(instruction_id),
            });
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
        // resolve the signature type
        let Type::FunctionPointer { parameters, result } = self.tree.get(signature) else {
            return Err(ValidateError::MetadataInvariantViolation {
                message: "call signature is not a function type".to_string(),
                anchor,
            });
        };

        let function = self.tree.get(function_id);

        // reject mismatched parameter counts
        if parameters.len() != function.parameters.len() {
            return Err(ValidateError::MetadataInvariantViolation {
                message: "call signature does not match callee".to_string(),
                anchor,
            });
        }

        // validate parameter types
        for (parameter, signature_type) in function.parameters.iter().zip(parameters.iter()) {
            if parameter.ty != *signature_type {
                return Err(ValidateError::MetadataInvariantViolation {
                    message: "call signature does not match callee".to_string(),
                    anchor,
                });
            }
        }

        // validate return type
        if function.return_type != *result {
            return Err(ValidateError::MetadataInvariantViolation {
                message: "call signature does not match callee".to_string(),
                anchor,
            });
        }

        Ok(())
    }

    /// Validate call effects invariants.
    pub(super) fn validate_call_effects(
        &self,
        instruction_id: LocalNodeId<Instruction>,
        effects: Option<&CallEffects>,
        argument_count: usize,
    ) -> ValidateResult<()> {
        // skip empty effects
        let Some(effects) = effects else {
            return Ok(());
        };

        let anchor = ValidateAnchor::node(instruction_id);

        // validate argument metadata length
        if !effects.argument_metadata.is_empty()
            && effects.argument_metadata.len() != argument_count
        {
            return Err(ValidateError::MetadataInvariantViolation {
                message: format!(
                    "call effects argument count mismatch expected {argument_count} got {}",
                    effects.argument_metadata.len()
                ),
                anchor,
            });
        }

        // validate memory effects
        self.validate_memory_effect_invariants(effects.memory_effects.as_ref(), anchor)?;

        // validate call behavior
        self.validate_call_behavior_invariants(effects.behavior.as_ref(), anchor)?;

        // validate return attributes
        self.validate_pointer_attributes_invariants(
            "call return attributes",
            &effects.return_attributes,
            anchor,
        )?;

        // validate argument attributes
        for argument in &effects.argument_metadata {
            self.validate_pointer_attributes_invariants(
                "call argument attributes",
                &argument.attributes,
                anchor,
            )?;
        }

        Ok(())
    }

    /// Validate memory effect invariants.
    pub(super) fn validate_memory_effect_invariants(
        &self,
        effect: Option<&MemoryEffect>,
        anchor: ValidateAnchor,
    ) -> ValidateResult<()> {
        // skip empty effects
        let Some(effect) = effect else {
            return Ok(());
        };

        // require locations for memory effects
        if !effect.reads && !effect.writes && !effect.locations.is_empty() {
            return Err(ValidateError::MetadataInvariantViolation {
                message: "memory effect has no reads/writes but non empty locations".to_string(),
                anchor,
            });
        }

        // validate argument memory constraints
        if effect.argmemonly
            && !(effect
                .locations
                .contains(crate::MemoryLocationSet::ARGUMENTS)
                || effect.locations.is_empty())
        {
            return Err(ValidateError::MetadataInvariantViolation {
                message: "argmemonly set without argument locations".to_string(),
                anchor,
            });
        }

        // validate inaccessible memory constraints
        if effect.inaccessible_mem_only
            && !(effect
                .locations
                .contains(crate::MemoryLocationSet::INACCESSIBLE)
                || effect.locations.is_empty())
        {
            return Err(ValidateError::MetadataInvariantViolation {
                message: "inaccessibleMemOnly set without inaccessible locations".to_string(),
                anchor,
            });
        }

        Ok(())
    }

    /// Validate call behavior invariants.
    pub(super) fn validate_call_behavior_invariants(
        &self,
        behavior: Option<&crate::CallBehavior>,
        anchor: ValidateAnchor,
    ) -> ValidateResult<()> {
        // skip empty behaviors
        let Some(behavior) = behavior else {
            return Ok(());
        };

        // reject incompatible return flags
        if behavior.noreturn && behavior.will_return {
            return Err(ValidateError::MetadataInvariantViolation {
                message: "noreturn implies will_return is false".to_string(),
                anchor,
            });
        }

        // reject pure operations that may suspend
        if behavior.repeatability == Repeatability::Pure && behavior.may_suspend {
            return Err(ValidateError::MetadataInvariantViolation {
                message: "pure effect cannot suspend".to_string(),
                anchor,
            });
        }

        // reject replay barriers on repeatable operations
        if behavior.no_replay && behavior.repeatability != Repeatability::NonRepeatable {
            return Err(ValidateError::MetadataInvariantViolation {
                message: "no_replay requires non_repeatable effect".to_string(),
                anchor,
            });
        }

        // validate allocation location metadata
        if !behavior.allocates && behavior.alloc_locations.is_some() {
            return Err(ValidateError::MetadataInvariantViolation {
                message: "alloc locations set without allocates".to_string(),
                anchor,
            });
        }

        // validate allocation address space metadata
        if !behavior.allocates && behavior.alloc_address_spaces.is_some() {
            return Err(ValidateError::MetadataInvariantViolation {
                message: "alloc address spaces set without allocates".to_string(),
                anchor,
            });
        }

        // validate free location metadata
        if !behavior.frees && behavior.free_locations.is_some() {
            return Err(ValidateError::MetadataInvariantViolation {
                message: "free locations set without frees".to_string(),
                anchor,
            });
        }

        // validate free address space metadata
        if !behavior.frees && behavior.free_address_spaces.is_some() {
            return Err(ValidateError::MetadataInvariantViolation {
                message: "free address spaces set without frees".to_string(),
                anchor,
            });
        }

        Ok(())
    }

    /// Validate pointer attribute invariants.
    pub(super) fn validate_pointer_attributes_invariants(
        &self,
        label: &str,
        attributes: &crate::PointerAttributes,
        anchor: ValidateAnchor,
    ) -> ValidateResult<()> {
        // reject conflicting access flags
        if attributes.readonly && attributes.writeonly {
            return Err(ValidateError::MetadataInvariantViolation {
                message: format!("{label} cannot be both readonly and writeonly"),
                anchor,
            });
        }

        Ok(())
    }

    /// Validate memory access invariants.
    pub(super) fn validate_memory_access_invariants(
        &self,
        access: &MemoryAccessMetadata,
        anchor: ValidateAnchor,
    ) -> ValidateResult<()> {
        // reject invariant writes
        if access.is_invariant && access.kind != MemoryAccessKind::Read {
            return Err(ValidateError::MetadataInvariantViolation {
                message: "invariant access must be read".to_string(),
                anchor,
            });
        }

        // reject atomic ordering on non atomic accesses
        if access.ordering.is_some()
            && !matches!(
                access.kind,
                MemoryAccessKind::Read
                    | MemoryAccessKind::Write
                    | MemoryAccessKind::ReadWrite
                    | MemoryAccessKind::ReadModifyWrite
                    | MemoryAccessKind::Fence
            )
        {
            return Err(ValidateError::MetadataInvariantViolation {
                message: "atomic ordering set for non atomic access".to_string(),
                anchor,
            });
        }

        Ok(())
    }
}
