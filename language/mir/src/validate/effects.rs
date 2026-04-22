use crate::{
    ArgumentAttribute, CallBehavior, EffectClass, Function, Instruction, LocalNodeId,
    MemoryAccessKind, MemoryAccessMetadata, MemoryEffect, NodeType, PointerAttribute,
    ReturnBehavior, SuspendBehavior, Type, UnwindBehavior,
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
        let return_type =
            self.require_type_reference(function.return_type, anchor, "function return type")?;
        self.ensure_node_type(NodeType::Type, return_type.id, anchor)?;

        for parameter in &function.parameters {
            let parameter_type =
                self.require_type_reference(parameter.ty, anchor, "function parameter type")?;
            self.ensure_node_type(NodeType::Type, parameter_type.id, anchor)?;
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
        self.validate_memory_effect_invariants(Some(&function.memory_effect), anchor)?;
        self.validate_call_behavior_invariants(Some(&function.call_behavior), anchor)?;

        // pointer attributes
        self.validate_pointer_attributes_invariants(
            "function return attributes",
            &function.return_attribute,
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
            let environment =
                self.require_type_reference(environment, anchor, "function environment type")?;
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

    /// Validate call metadata invariants.
    pub(super) fn validate_call_metadata(
        &self,
        instruction_id: LocalNodeId<Instruction>,
        memory_effect: Option<&MemoryEffect>,
        call_behavior: Option<&CallBehavior>,
        argument_attributes: &[ArgumentAttribute],
        return_attribute: &PointerAttribute,
        argument_count: usize,
    ) -> ValidateResult<()> {
        let anchor = ValidateAnchor::node(instruction_id);
        if !argument_attributes.is_empty() && argument_attributes.len() != argument_count {
            return Err(ValidateError::MetadataInvariantViolation {
                message: format!(
                    "call argument attribute count mismatch expected {argument_count} got {}",
                    argument_attributes.len()
                ),
                anchor,
            });
        }

        self.validate_memory_effect_invariants(memory_effect, anchor)?;
        self.validate_call_behavior_invariants(call_behavior, anchor)?;
        self.validate_pointer_attributes_invariants(
            "call return attributes",
            return_attribute,
            anchor,
        )?;

        for argument in argument_attributes {
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

        if !effect.reads && !effect.writes && !effect.regions.is_empty() {
            return Err(ValidateError::MetadataInvariantViolation {
                message: "memory effect has no reads/writes but non empty regions".to_string(),
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

        if behavior.return_behavior == ReturnBehavior::WillReturn
            && behavior.unwind == UnwindBehavior::MayUnwind
        {
            return Err(ValidateError::MetadataInvariantViolation {
                message: "will_return requires cannot_unwind".to_string(),
                anchor,
            });
        }

        if behavior.effect_class == EffectClass::Pure
            && behavior.suspend == SuspendBehavior::MaySuspend
        {
            return Err(ValidateError::MetadataInvariantViolation {
                message: "pure effect cannot suspend".to_string(),
                anchor,
            });
        }

        if behavior.effect_class == EffectClass::Pure
            && behavior.unwind == UnwindBehavior::MayUnwind
        {
            return Err(ValidateError::MetadataInvariantViolation {
                message: "pure effect cannot unwind".to_string(),
                anchor,
            });
        }

        Ok(())
    }

    /// Validate pointer attribute invariants.
    pub(super) fn validate_pointer_attributes_invariants(
        &self,
        label: &str,
        attributes: &PointerAttribute,
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
        if access.is_load_invariant && access.kind != MemoryAccessKind::Read {
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
