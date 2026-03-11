use std::collections::{HashMap, HashSet};

use crate::{Block, Function, Local, LocalNodeId, NodeTree, NodeType, Type, Value};

use super::{ValidateAnchor, ValidateError, ValidateResult};

/// Validates MIR invariants for a tree or function.
#[derive(Debug)]
pub struct Validator<'a> {
    /// The MIR tree to validate.
    pub(super) tree: &'a NodeTree,
}

#[allow(clippy::type_complexity)]
impl<'a> Validator<'a> {
    /// Create a new validator for a tree.
    pub fn new(tree: &'a NodeTree) -> Self {
        Self { tree }
    }

    /// Validate a full MIR module.
    pub fn validate(&self) -> ValidateResult<()> {
        // collect function ids
        let function_ids = self.collect_function_ids();

        // validate each function
        for function_id in function_ids {
            self.validate_function(function_id)?;
        }

        // validate module metadata tables
        self.validate_metadata_tables()?;

        Ok(())
    }

    /// Validate all functions in a MIR tree.
    pub fn validate_tree(&self) -> ValidateResult<()> {
        self.validate()
    }

    /// Validate a full MIR module.
    pub fn validate_module(&self) -> ValidateResult<()> {
        self.validate()
    }

    /// Validate a single MIR function.
    pub fn validate_function(&self, function_id: LocalNodeId<Function>) -> ValidateResult<()> {
        // collect function state
        let function = self.tree.get(function_id);

        // skip imports with no body
        if function.blocks.is_empty() {
            // reject imports with entries
            if function.entry.is_some() {
                return Err(ValidateError::FunctionHasEntryWithoutBlocks {
                    anchor: ValidateAnchor::node(function_id),
                });
            }

            return Ok(());
        }

        // collect block ids and order
        let (block_ids, block_order) = self.collect_block_info(function)?;

        // collect entry block
        let entry = function.entry.ok_or(ValidateError::MissingEntryBlock {
            anchor: ValidateAnchor::node(function_id),
        })?;

        // collect locals
        let locals = self.collect_local_info(function, function_id)?;

        // ensure entry is part of the block list
        if !block_ids.contains(&entry) {
            return Err(ValidateError::EntryBlockNotInFunction {
                anchor: ValidateAnchor::node(function_id),
            });
        }

        // validate entry parameters
        self.validate_entry_block(function, entry)?;

        // validate function metadata
        self.validate_function_metadata(function_id, function)?;

        // collect defined values and ensure uniqueness
        let defined_values = self.collect_defined_values(function_id, function, entry)?;

        // validate type entries for all defined values
        self.validate_value_types(function_id, function, &defined_values)?;

        // validate instructions and terminators
        self.validate_blocks(function, &locals, &block_ids, &block_order, &defined_values)?;

        Ok(())
    }

    /// Collect function ids from the tree.
    fn collect_function_ids(&self) -> Vec<LocalNodeId<Function>> {
        // collect function ids
        let mut function_ids = Vec::new();

        for (global_id, node_type) in self.tree.node_type_by_node_id.iter().enumerate() {
            // skip non function nodes
            if *node_type != NodeType::Function {
                continue;
            }

            function_ids.push(LocalNodeId::<Function>::new(global_id as u32));
        }

        function_ids
    }

    /// Collect block ids and their order for a function.
    fn collect_block_info(
        &self,
        function: &Function,
    ) -> ValidateResult<(
        HashSet<LocalNodeId<Block>>,
        HashMap<LocalNodeId<Block>, usize>,
    )> {
        // collect unique block ids
        let mut block_ids = HashSet::new();
        let mut block_order = HashMap::new();

        for (index, &block_id) in function.blocks.iter().enumerate() {
            // ensure the block id resolves
            self.ensure_node_type(NodeType::Block, block_id.id, ValidateAnchor::node(block_id))?;

            // reject duplicate blocks
            if !block_ids.insert(block_id) {
                return Err(ValidateError::DuplicateBlockId {
                    block_id,
                    anchor: ValidateAnchor::node(block_id),
                });
            }

            // record block order
            block_order.insert(block_id, index);
        }

        Ok((block_ids, block_order))
    }

    /// Collect locals and ensure uniqueness.
    fn collect_local_info(
        &self,
        function: &Function,
        function_id: LocalNodeId<Function>,
    ) -> ValidateResult<HashSet<LocalNodeId<Local>>> {
        // track unique locals
        let mut locals = HashSet::new();

        for &local_id in &function.locals {
            // ensure the local id resolves
            self.ensure_node_type(
                NodeType::Local,
                local_id.id,
                ValidateAnchor::node(function_id),
            )?;

            // reject duplicate locals
            if !locals.insert(local_id) {
                return Err(ValidateError::DuplicateLocalId {
                    local_id,
                    anchor: ValidateAnchor::node(function_id),
                });
            }
        }

        Ok(locals)
    }

    /// Validate entry block parameters against function parameters.
    fn validate_entry_block(
        &self,
        function: &Function,
        entry: LocalNodeId<Block>,
    ) -> ValidateResult<()> {
        // check entry parameter shape
        let entry_block = self.tree.get(entry);
        if entry_block.parameters.len() != function.parameters.len() {
            return Err(ValidateError::EntryBlockParameterMismatch {
                anchor: ValidateAnchor::node(entry),
            });
        }

        // check entry parameters match function parameters
        for (param, function_param) in entry_block
            .parameters
            .iter()
            .zip(function.parameters.iter())
        {
            // reject parameter value mismatches
            if param.value != function_param.value {
                return Err(ValidateError::EntryBlockParameterMismatch {
                    anchor: ValidateAnchor::node(entry),
                });
            }
        }

        Ok(())
    }

    /// Collect defined values and ensure uniqueness.
    fn collect_defined_values(
        &self,
        function_id: LocalNodeId<Function>,
        function: &Function,
        entry: LocalNodeId<Block>,
    ) -> ValidateResult<HashSet<Value>> {
        // track unique definitions
        let mut defined = HashSet::new();

        // record function parameters
        for param in &function.parameters {
            // reject duplicate parameter values
            if !defined.insert(param.value) {
                return Err(ValidateError::DuplicateValueDefinition {
                    value: param.value,
                    anchor: ValidateAnchor::node(function_id),
                });
            }
        }

        // record block parameters and instruction destinations
        for &block_id in &function.blocks {
            // load block definitions
            let block = self.tree.get(block_id);

            // skip entry parameters already verified
            if block_id != entry {
                // record block parameters
                for param in &block.parameters {
                    // reject duplicate block parameters
                    if !defined.insert(param.value) {
                        return Err(ValidateError::DuplicateValueDefinition {
                            value: param.value,
                            anchor: ValidateAnchor::node(block_id),
                        });
                    }
                }
            }

            // record instruction destinations
            for &instruction_id in &block.instructions {
                // load instruction definition
                let instruction = self.tree.get(instruction_id);

                // reject duplicate destinations
                if let Some(destination) = instruction.destination()
                    && !defined.insert(destination)
                {
                    return Err(ValidateError::DuplicateValueDefinition {
                        value: destination,
                        anchor: ValidateAnchor::node(instruction_id),
                    });
                }
            }
        }

        Ok(defined)
    }

    /// Ensure every defined value has a type entry.
    fn validate_value_types(
        &self,
        function_id: LocalNodeId<Function>,
        function: &Function,
        defined_values: &HashSet<Value>,
    ) -> ValidateResult<()> {
        for value in defined_values {
            if function.value_type(*value).is_none() {
                return Err(ValidateError::MissingValueType {
                    value: *value,
                    anchor: ValidateAnchor::node(function_id),
                });
            }
        }

        Ok(())
    }

    /// Validate instruction and terminator invariants across blocks.
    fn validate_blocks(
        &self,
        function: &Function,
        locals: &HashSet<LocalNodeId<Local>>,
        block_ids: &HashSet<LocalNodeId<Block>>,
        block_order: &HashMap<LocalNodeId<Block>, usize>,
        defined_values: &HashSet<Value>,
    ) -> ValidateResult<()> {
        // track instruction uniqueness
        let mut seen_instructions = HashSet::new();

        // validate per block invariants
        for &block_id in &function.blocks {
            // load block state
            let block = self.tree.get(block_id);

            // validate instruction uses
            for &instruction_id in &block.instructions {
                // ensure instruction ids resolve
                self.ensure_node_type(
                    NodeType::Instruction,
                    instruction_id.id,
                    ValidateAnchor::node(block_id),
                )?;

                // reject duplicate instruction ids
                if !seen_instructions.insert(instruction_id) {
                    return Err(ValidateError::DuplicateInstructionId {
                        instruction_id,
                        anchor: ValidateAnchor::node(block_id),
                    });
                }

                // validate instruction invariants
                let instruction = self.tree.get(instruction_id);
                self.validate_instruction(
                    function,
                    instruction_id,
                    instruction,
                    locals,
                    defined_values,
                )?;
            }

            // validate terminator uses and successors
            self.validate_terminator(
                function,
                block_id,
                &block.terminator,
                block_ids,
                block_order,
                defined_values,
            )?;
        }

        Ok(())
    }

    /// Return a short type kind name.
    pub(super) fn type_kind(&self, ty: &Type) -> &'static str {
        // map types to short labels
        match ty {
            Type::Void => "void",
            Type::Boolean => "bool",
            Type::Int { .. } => "int",
            Type::Isize => "isize",
            Type::Usize => "usize",
            Type::Float { .. } => "float",
            Type::TypeDescriptor => "type_descriptor",
            Type::TypeId => "type_id",
            Type::Reference { .. } => "ref",
            Type::Array { .. } => "array",
            Type::Tuple { .. } => "tuple",
            Type::Struct { .. } => "struct",
            Type::Newtype { .. } => "newtype",
            Type::Vector { .. } => "vector",
            Type::Tensor { .. } => "tensor",
            Type::TensorReference { .. } => "tensor_ref",
            Type::FunctionPointer { .. } => "fn",
            Type::FunctionValue { .. } => "fnvalue",
        }
    }

    /// Format a block label using function order.
    pub(super) fn block_label(
        &self,
        block_id: LocalNodeId<Block>,
        block_order: &HashMap<LocalNodeId<Block>, usize>,
    ) -> String {
        // format block labels from the function order
        if let Some(index) = block_order.get(&block_id) {
            return format!("block{index}");
        }

        format!("block{}", block_id.id)
    }
}
