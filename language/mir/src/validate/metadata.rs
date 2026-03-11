use crate::{Instruction, NodeType};

use super::{ValidateAnchor, ValidateError, ValidateResult, Validator};

#[allow(clippy::type_complexity)]
impl<'a> Validator<'a> {
    /// Validate module metadata tables after function checks.
    pub(super) fn validate_metadata_tables(&self) -> ValidateResult<()> {
        self.validate_data_layout_metadata()?;
        self.validate_memory_metadata_tables()?;
        self.validate_type_legality()?;
        self.validate_type_dispatch_metadata()?;
        self.validate_layout_metadata_consistency()?;
        self.validate_dispatch_call_facts()?;

        Ok(())
    }

    /// Validate memory metadata tables.
    fn validate_memory_metadata_tables(&self) -> ValidateResult<()> {
        for (&instruction_id, accesses) in &self.tree.memory_table.memory_accesses_by_instruction_id
        {
            self.ensure_node_type(
                NodeType::Instruction,
                instruction_id.id,
                ValidateAnchor::node(instruction_id),
            )?;

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

            for access in accesses {
                self.validate_memory_access_invariants(
                    access,
                    ValidateAnchor::node(instruction_id),
                )?;
            }
        }

        Ok(())
    }
}
