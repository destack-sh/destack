use std::collections::{HashMap, HashSet};

use crate::{Block, Function, Local, LocalNodeId, NodeType, Type, Value, ValueReference};

use super::{ValidateAnchor, ValidateError, ValidateResult, Validator};

#[allow(clippy::type_complexity)]
impl<'tree> Validator<'tree> {
    /// Validate a single MIR function.
    pub fn validate_function(&self, function_id: LocalNodeId<Function>) -> ValidateResult<()> {
        let function = self.tree.get(function_id);

        // imports
        if function.blocks.is_empty() {
            if function.entry.is_some() {
                return Err(ValidateError::FunctionHasEntryWithoutBlocks {
                    anchor: ValidateAnchor::node(function_id),
                });
            }

            self.validate_function_metadata(function_id, function)?;

            return Ok(());
        }

        // entry and declarations
        let entry = function.entry.ok_or(ValidateError::MissingEntryBlock {
            anchor: ValidateAnchor::node(function_id),
        })?;
        let (block_ids, block_order, defined_values) =
            self.collect_function_blocks(function_id, function, entry)?;
        let locals = self.collect_function_locals(function_id, function)?;

        self.validate_function_entry(function, entry, &block_ids)?;

        // function metadata
        self.validate_function_metadata(function_id, function)?;
        self.validate_value_types(function_id, function, &defined_values)?;

        // block bodies
        for &block_id in &function.blocks {
            let block = self.tree.get(block_id);
            let terminator_id = block.terminator;
            let terminator = self.tree.get(block.terminator);

            for &instruction_id in &block.instructions {
                let instruction = self.tree.get(instruction_id);
                self.validate_instruction(
                    function,
                    instruction_id,
                    instruction,
                    &locals,
                    &defined_values,
                )?;
            }

            self.validate_terminator(
                function,
                block_id,
                terminator_id,
                terminator,
                &block_ids,
                &block_order,
                &defined_values,
            )?;
        }

        self.validate_names(function_id, function)?;

        Ok(())
    }

    /// Validate block and SSA value names for one function.
    fn validate_names(
        &self,
        function_id: LocalNodeId<Function>,
        function: &Function,
    ) -> ValidateResult<()> {
        let entry = function.entry;
        let mut block_names = HashSet::new();
        let mut value_names = HashSet::new();

        // block names
        for &block_id in &function.blocks {
            let block = self.tree.get(block_id);
            let Some(name_id) = block.name else {
                return Err(ValidateError::MissingBlockName {
                    block_id,
                    anchor: ValidateAnchor::node(block_id),
                });
            };

            if !block_names.insert(name_id) {
                return Err(ValidateError::DuplicateBlockName {
                    anchor: ValidateAnchor::node(block_id),
                });
            }
        }

        // parameter names
        for parameter in &function.parameters {
            let ValueReference::Value(value) = parameter.value else {
                continue;
            };

            let Some(name_id) = function.value_name(value) else {
                return Err(ValidateError::MissingValueName {
                    value,
                    anchor: ValidateAnchor::node(function_id),
                });
            };

            if !value_names.insert(name_id) {
                return Err(ValidateError::DuplicateValueName {
                    anchor: ValidateAnchor::node(function_id),
                });
            }
        }

        // block parameter and instruction names
        for &block_id in &function.blocks {
            let block = self.tree.get(block_id);

            if Some(block_id) != entry {
                for parameter in &block.parameters {
                    let ValueReference::Value(value) = parameter.value else {
                        continue;
                    };

                    let Some(name_id) = function.value_name(value) else {
                        return Err(ValidateError::MissingValueName {
                            value,
                            anchor: ValidateAnchor::node(block_id),
                        });
                    };

                    if !value_names.insert(name_id) {
                        return Err(ValidateError::DuplicateValueName {
                            anchor: ValidateAnchor::node(block_id),
                        });
                    }
                }
            }

            for &instruction_id in &block.instructions {
                let instruction = self.tree.get(instruction_id);
                let Some(destination) = instruction.destination() else {
                    continue;
                };
                let ValueReference::Value(destination) = destination else {
                    continue;
                };

                let Some(name_id) = function.value_name(destination) else {
                    return Err(ValidateError::MissingValueName {
                        value: destination,
                        anchor: ValidateAnchor::node(instruction_id),
                    });
                };

                if !value_names.insert(name_id) {
                    return Err(ValidateError::DuplicateValueName {
                        anchor: ValidateAnchor::node(instruction_id),
                    });
                }
            }
        }

        Ok(())
    }

    /// Collect block ids, block order, and value definitions for one function.
    fn collect_function_blocks(
        &self,
        function_id: LocalNodeId<Function>,
        function: &Function,
        entry: LocalNodeId<Block>,
    ) -> ValidateResult<(
        HashSet<LocalNodeId<Block>>,
        HashMap<LocalNodeId<Block>, usize>,
        HashSet<Value>,
    )> {
        // parameters
        let mut defined_values = HashSet::new();
        for parameter in &function.parameters {
            let ValueReference::Value(value) = parameter.value else {
                continue;
            };

            if !defined_values.insert(value) {
                return Err(ValidateError::DuplicateValueDefinition {
                    value,
                    anchor: ValidateAnchor::node(function_id),
                });
            }
        }

        // blocks and instructions
        let mut block_ids = HashSet::new();
        let mut block_order = HashMap::new();
        let mut seen_instructions = HashSet::new();
        let mut seen_terminators = HashSet::new();

        for (index, &block_id) in function.blocks.iter().enumerate() {
            self.ensure_node_type(NodeType::Block, block_id.id, ValidateAnchor::node(block_id))?;

            if !block_ids.insert(block_id) {
                return Err(ValidateError::DuplicateBlockId {
                    block_id,
                    anchor: ValidateAnchor::node(block_id),
                });
            }

            block_order.insert(block_id, index);

            let block = self.tree.get(block_id);
            self.ensure_node_type(
                NodeType::Terminator,
                block.terminator.id,
                ValidateAnchor::node(block_id),
            )?;

            if !seen_terminators.insert(block.terminator) {
                return Err(ValidateError::DuplicateTerminatorId {
                    terminator_id: block.terminator,
                    anchor: ValidateAnchor::node(block_id),
                });
            }

            // block parameters
            if block_id != entry {
                for parameter in &block.parameters {
                    let ValueReference::Value(value) = parameter.value else {
                        continue;
                    };

                    if !defined_values.insert(value) {
                        return Err(ValidateError::DuplicateValueDefinition {
                            value,
                            anchor: ValidateAnchor::node(block_id),
                        });
                    }
                }
            }

            // instruction destinations
            for &instruction_id in &block.instructions {
                self.ensure_node_type(
                    NodeType::Instruction,
                    instruction_id.id,
                    ValidateAnchor::node(block_id),
                )?;

                if !seen_instructions.insert(instruction_id) {
                    return Err(ValidateError::DuplicateInstructionId {
                        instruction_id,
                        anchor: ValidateAnchor::node(block_id),
                    });
                }

                let instruction = self.tree.get(instruction_id);
                if let Some(ValueReference::Value(destination)) = instruction.destination()
                    && !defined_values.insert(destination)
                {
                    return Err(ValidateError::DuplicateValueDefinition {
                        value: destination,
                        anchor: ValidateAnchor::node(instruction_id),
                    });
                }
            }
        }

        Ok((block_ids, block_order, defined_values))
    }

    /// Collect locals for one function and reject duplicates.
    fn collect_function_locals(
        &self,
        function_id: LocalNodeId<Function>,
        function: &Function,
    ) -> ValidateResult<HashSet<LocalNodeId<Local>>> {
        let mut locals = HashSet::new();

        for &local_id in &function.locals {
            self.ensure_node_type(
                NodeType::Local,
                local_id.id,
                ValidateAnchor::node(function_id),
            )?;

            if !locals.insert(local_id) {
                return Err(ValidateError::DuplicateLocalId {
                    local_id,
                    anchor: ValidateAnchor::node(function_id),
                });
            }
        }

        Ok(locals)
    }

    /// Validate the entry block contract for one function.
    fn validate_function_entry(
        &self,
        function: &Function,
        entry: LocalNodeId<Block>,
        block_ids: &HashSet<LocalNodeId<Block>>,
    ) -> ValidateResult<()> {
        // ownership
        if !block_ids.contains(&entry) {
            return Err(ValidateError::EntryBlockNotInFunction {
                anchor: ValidateAnchor::node(entry),
            });
        }

        // parameters
        let entry_block = self.tree.get(entry);
        if entry_block.parameters.len() != function.parameters.len() {
            return Err(ValidateError::EntryBlockParameterMismatch {
                anchor: ValidateAnchor::node(entry),
            });
        }

        for (parameter, function_parameter) in
            entry_block.parameters.iter().zip(&function.parameters)
        {
            if parameter.value != function_parameter.value {
                return Err(ValidateError::EntryBlockParameterMismatch {
                    anchor: ValidateAnchor::node(entry),
                });
            }
        }

        Ok(())
    }

    /// Validate that every defined SSA value has a concrete value type.
    fn validate_value_types(
        &self,
        function_id: LocalNodeId<Function>,
        function: &Function,
        defined_values: &HashSet<Value>,
    ) -> ValidateResult<()> {
        for &value in defined_values {
            let Some(value_type) = function.value_type(value) else {
                return Err(ValidateError::MissingValueType {
                    value,
                    anchor: ValidateAnchor::node(function_id),
                });
            };

            if matches!(self.tree.get(value_type), Type::Atomic { .. }) {
                return Err(ValidateError::MetadataInvariantViolation {
                    message: "atomic storage type cannot be used as a value type".to_string(),
                    anchor: ValidateAnchor::node(function_id),
                });
            }
        }

        Ok(())
    }
}
