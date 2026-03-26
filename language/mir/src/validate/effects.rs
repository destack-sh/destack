use crate::{
    CallBehavior, CallEffects, Function, Instruction, LocalNodeId, MemoryAccessKind,
    MemoryAccessMetadata, MemoryEffect, NodeType, PointerAttributes, Repeatability, Type,
    UnwindBehavior,
};

use super::{ValidateAnchor, ValidateError, ValidateResult, Validator};

#[allow(clippy::type_complexity)]
impl<'a> Validator<'a> {
    /// Validate function metadata invariants.
    pub(super) fn validate_function_metadata(
        &self,
        function_id: LocalNodeId<Function>,
        function: &Function,
    ) -> ValidateResult<()> {
        let anchor = ValidateAnchor::node(function_id);

        // type references
        self.ensure_node_type(NodeType::Type, function.return_type.id, anchor)?;

        for parameter in &function.parameters {
            self.ensure_node_type(NodeType::Type, parameter.ty.id, anchor)?;
        }

        // metadata shape
        if function.parameter_attributes.len() != function.parameters.len() {
            return Err(ValidateError::MetadataInvariantViolation {
                message: format!(
                    "parameter attributes length mismatch expected {} got {}",
                    function.parameters.len(),
                    function.parameter_attributes.len()
                ),
                anchor,
            });
        }

        // memory and call behavior
        self.validate_memory_effect_invariants(Some(&function.memory_effects), anchor)?;
        self.validate_call_behavior_invariants(Some(&function.call_behavior), anchor)?;

        // pointer attributes
        self.validate_pointer_attributes_invariants(
            "function return attributes",
            &function.return_attributes,
            anchor,
        )?;

        for attributes in &function.parameter_attributes {
            self.validate_pointer_attributes_invariants(
                "function parameter attributes",
                attributes,
                anchor,
            )?;
        }

        // function environment
        if let Some(environment) = function.environment {
            self.ensure_node_type(NodeType::Type, environment.id, anchor)?;

            let environment_type = self.tree.get(environment);
            if !matches!(environment_type, Type::Reference { .. }) {
                return Err(ValidateError::MetadataInvariantViolation {
                    message: "function environment type must be a reference".to_string(),
                    anchor,
                });
            }
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
        let Some(effects) = effects else {
            return Ok(());
        };

        let anchor = ValidateAnchor::node(instruction_id);
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

        self.validate_memory_effect_invariants(effects.memory_effects.as_ref(), anchor)?;
        self.validate_call_behavior_invariants(effects.behavior.as_ref(), anchor)?;
        self.validate_pointer_attributes_invariants(
            "call return attributes",
            &effects.return_attributes,
            anchor,
        )?;

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
        let Some(effect) = effect else {
            return Ok(());
        };

        if !effect.reads && !effect.writes && !effect.locations.is_empty() {
            return Err(ValidateError::MetadataInvariantViolation {
                message: "memory effect has no reads/writes but non empty locations".to_string(),
                anchor,
            });
        }

        if effect.argmemonly && effect.inaccessible_mem_only {
            return Err(ValidateError::MetadataInvariantViolation {
                message: "memory effect cannot be both argmemonly and inaccessibleMemOnly"
                    .to_string(),
                anchor,
            });
        }

        Ok(())
    }

    /// Validate call behavior invariants.
    pub(super) fn validate_call_behavior_invariants(
        &self,
        behavior: Option<&CallBehavior>,
        anchor: ValidateAnchor,
    ) -> ValidateResult<()> {
        let Some(behavior) = behavior else {
            return Ok(());
        };

        if behavior.noreturn && behavior.will_return {
            return Err(ValidateError::MetadataInvariantViolation {
                message: "noreturn implies will_return is false".to_string(),
                anchor,
            });
        }

        if behavior.will_return && behavior.unwind_behavior == UnwindBehavior::MayUnwind {
            return Err(ValidateError::MetadataInvariantViolation {
                message: "will_return requires cannot_unwind".to_string(),
                anchor,
            });
        }

        if behavior.repeatability == Repeatability::Pure && behavior.may_suspend {
            return Err(ValidateError::MetadataInvariantViolation {
                message: "pure effect cannot suspend".to_string(),
                anchor,
            });
        }

        if behavior.repeatability == Repeatability::Pure
            && behavior.unwind_behavior == UnwindBehavior::MayUnwind
        {
            return Err(ValidateError::MetadataInvariantViolation {
                message: "pure effect cannot unwind".to_string(),
                anchor,
            });
        }

        if !behavior.allocates && behavior.alloc_locations.is_some() {
            return Err(ValidateError::MetadataInvariantViolation {
                message: "alloc locations set without allocates".to_string(),
                anchor,
            });
        }

        if !behavior.allocates && behavior.alloc_address_spaces.is_some() {
            return Err(ValidateError::MetadataInvariantViolation {
                message: "alloc address spaces set without allocates".to_string(),
                anchor,
            });
        }

        if !behavior.frees && behavior.free_locations.is_some() {
            return Err(ValidateError::MetadataInvariantViolation {
                message: "free locations set without frees".to_string(),
                anchor,
            });
        }

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
        attributes: &PointerAttributes,
        anchor: ValidateAnchor,
    ) -> ValidateResult<()> {
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
        if access.is_invariant && access.kind != MemoryAccessKind::Read {
            return Err(ValidateError::MetadataInvariantViolation {
                message: "invariant access must be read".to_string(),
                anchor,
            });
        }

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
