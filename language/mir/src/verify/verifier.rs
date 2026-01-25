use std::collections::{HashMap, HashSet};

use crate::{
    ArgumentSlice, Block, CallEffects, Function, Instruction, Local, LocalNodeId, MemoryAccessKind,
    MemoryAccessMetadata, MemoryEffect, Mutability, NodeTree, NodeType, ReferenceKind, SwitchCase,
    TensorDimension, Terminator, Type, Value,
};

use super::{VerifyAnchor, VerifyError, VerifyResult};

/// Configuration for MIR verification.
#[derive(Debug, Clone, Copy)]
pub struct VerifierOptions {
    /// Validate argument slices against the shared buffer.
    pub verify_argument_slices: bool,
    /// Validate local references against the function's locals.
    pub verify_local_references: bool,
    /// Validate node ids against their expected node types.
    pub verify_node_types: bool,
    /// Validate aggregate constructor argument counts.
    pub verify_type_shapes: bool,
    /// Validate metadata table invariants.
    pub verify_metadata: bool,
    /// Validate instruction ids are unique within a function.
    pub verify_instruction_uniqueness: bool,
}

impl VerifierOptions {
    /// Basic validation for structural MIR correctness.
    pub const fn basic() -> Self {
        Self {
            verify_argument_slices: true,
            verify_local_references: true,
            verify_node_types: true,
            verify_type_shapes: true,
            verify_metadata: false,
            verify_instruction_uniqueness: true,
        }
    }

    /// Strict validation including metadata invariants.
    pub const fn strict() -> Self {
        Self {
            verify_metadata: true,
            ..Self::basic()
        }
    }
}

/// Verifies MIR invariants for a tree or function.
#[derive(Debug)]
pub struct Verifier<'a> {
    /// The MIR tree to verify.
    tree: &'a NodeTree,
    /// Verifier options.
    options: VerifierOptions,
}

#[allow(clippy::type_complexity)]
impl<'a> Verifier<'a> {
    /// Create a new verifier for a tree.
    pub fn new(tree: &'a NodeTree) -> Self {
        Self {
            tree,
            options: VerifierOptions::basic(),
        }
    }

    /// Create a new verifier with explicit options.
    pub fn new_with_options(tree: &'a NodeTree, options: VerifierOptions) -> Self {
        Self { tree, options }
    }

    /// Verify all functions in a MIR tree.
    pub fn verify_tree(&self) -> VerifyResult<()> {
        self.verify_module()
    }

    /// Verify a full MIR module.
    pub fn verify_module(&self) -> VerifyResult<()> {
        // collect function ids
        let function_ids = self.collect_function_ids();

        // verify each function
        for function_id in function_ids {
            self.verify_function(function_id)?;
        }

        // verify module metadata tables
        if self.options.verify_metadata {
            self.verify_metadata_tables()?;
        }

        Ok(())
    }

    /// Verify a single MIR function.
    pub fn verify_function(&self, function_id: LocalNodeId<Function>) -> VerifyResult<()> {
        // collect function state
        let function = self.tree.get(function_id);

        // skip imports with no body
        if function.blocks.is_empty() {
            // reject imports with entries
            if function.entry.is_some() {
                return Err(VerifyError::FunctionHasEntryWithoutBlocks {
                    anchor: VerifyAnchor::node(function_id),
                });
            }

            return Ok(());
        }

        // collect block ids and order
        let (block_ids, block_order) = self.collect_block_info(function)?;

        // collect entry block
        let entry = function.entry.ok_or(VerifyError::MissingEntryBlock {
            anchor: VerifyAnchor::node(function_id),
        })?;

        // collect locals
        let locals = self.collect_local_info(function, function_id)?;

        // ensure entry is part of the block list
        if !block_ids.contains(&entry) {
            return Err(VerifyError::EntryBlockNotInFunction {
                anchor: VerifyAnchor::node(function_id),
            });
        }

        // verify entry parameters
        self.verify_entry_block(function, entry)?;

        // verify function metadata
        if self.options.verify_metadata {
            self.verify_function_metadata(function_id, function)?;
        }

        // collect defined values and ensure uniqueness
        let defined_values = self.collect_defined_values(function_id, function, entry)?;

        // verify type entries for all defined values
        self.verify_value_types(function_id, function, &defined_values)?;

        // verify instructions and terminators
        self.verify_blocks(function, &locals, &block_ids, &block_order, &defined_values)?;

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
    ) -> VerifyResult<(
        HashSet<LocalNodeId<Block>>,
        HashMap<LocalNodeId<Block>, usize>,
    )> {
        // collect unique block ids
        let mut block_ids = HashSet::new();
        let mut block_order = HashMap::new();

        for (index, &block_id) in function.blocks.iter().enumerate() {
            // ensure the block id resolves
            self.ensure_node_type(NodeType::Block, block_id.id, VerifyAnchor::node(block_id))?;

            // reject duplicate blocks
            if !block_ids.insert(block_id) {
                return Err(VerifyError::DuplicateBlockId {
                    block_id,
                    anchor: VerifyAnchor::node(block_id),
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
    ) -> VerifyResult<HashSet<LocalNodeId<Local>>> {
        // track unique locals
        let mut locals = HashSet::new();

        for &local_id in &function.locals {
            // ensure the local id resolves
            self.ensure_node_type(
                NodeType::Local,
                local_id.id,
                VerifyAnchor::node(function_id),
            )?;

            // reject duplicate locals
            if !locals.insert(local_id) {
                return Err(VerifyError::DuplicateLocalId {
                    local_id,
                    anchor: VerifyAnchor::node(function_id),
                });
            }
        }

        Ok(locals)
    }

    /// Verify entry block parameters against function parameters.
    fn verify_entry_block(
        &self,
        function: &Function,
        entry: LocalNodeId<Block>,
    ) -> VerifyResult<()> {
        // check entry parameter shape
        let entry_block = self.tree.get(entry);
        if entry_block.parameters.len() != function.parameters.len() {
            return Err(VerifyError::EntryBlockParameterMismatch {
                anchor: VerifyAnchor::node(entry),
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
                return Err(VerifyError::EntryBlockParameterMismatch {
                    anchor: VerifyAnchor::node(entry),
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
    ) -> VerifyResult<HashSet<Value>> {
        // track unique definitions
        let mut defined = HashSet::new();

        // record function parameters
        for param in &function.parameters {
            // reject duplicate parameter values
            if !defined.insert(param.value) {
                return Err(VerifyError::DuplicateValueDefinition {
                    value: param.value,
                    anchor: VerifyAnchor::node(function_id),
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
                        return Err(VerifyError::DuplicateValueDefinition {
                            value: param.value,
                            anchor: VerifyAnchor::node(block_id),
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
                    return Err(VerifyError::DuplicateValueDefinition {
                        value: destination,
                        anchor: VerifyAnchor::node(instruction_id),
                    });
                }
            }
        }

        Ok(defined)
    }

    /// Ensure every defined value has a type entry.
    fn verify_value_types(
        &self,
        function_id: LocalNodeId<Function>,
        function: &Function,
        defined_values: &HashSet<Value>,
    ) -> VerifyResult<()> {
        for value in defined_values {
            if function.value_type(*value).is_none() {
                return Err(VerifyError::MissingValueType {
                    value: *value,
                    anchor: VerifyAnchor::node(function_id),
                });
            }
        }

        Ok(())
    }

    /// Verify instruction and terminator invariants across blocks.
    fn verify_blocks(
        &self,
        function: &Function,
        locals: &HashSet<LocalNodeId<Local>>,
        block_ids: &HashSet<LocalNodeId<Block>>,
        block_order: &HashMap<LocalNodeId<Block>, usize>,
        defined_values: &HashSet<Value>,
    ) -> VerifyResult<()> {
        // track instruction uniqueness
        let mut seen_instructions = HashSet::new();

        // verify per block invariants
        for &block_id in &function.blocks {
            // load block state
            let block = self.tree.get(block_id);

            // verify instruction uses
            for &instruction_id in &block.instructions {
                // ensure instruction ids resolve
                self.ensure_node_type(
                    NodeType::Instruction,
                    instruction_id.id,
                    VerifyAnchor::node(block_id),
                )?;

                // reject duplicate instruction ids
                if self.options.verify_instruction_uniqueness
                    && !seen_instructions.insert(instruction_id)
                {
                    return Err(VerifyError::DuplicateInstructionId {
                        instruction_id,
                        anchor: VerifyAnchor::node(block_id),
                    });
                }

                // verify instruction invariants
                let instruction = self.tree.get(instruction_id);
                self.verify_instruction(
                    function,
                    instruction_id,
                    instruction,
                    locals,
                    defined_values,
                )?;
            }

            // verify terminator uses and successors
            self.verify_terminator(
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

    /// Verify an instruction uses only defined values.
    fn verify_instruction(
        &self,
        function: &Function,
        instruction_id: LocalNodeId<Instruction>,
        instruction: &Instruction,
        locals: &HashSet<LocalNodeId<Local>>,
        defined_values: &HashSet<Value>,
    ) -> VerifyResult<()> {
        // check inline uses
        for value in instruction.uses() {
            self.ensure_defined(value, VerifyAnchor::node(instruction_id), defined_values)?;
        }

        // check external argument uses
        if let Some(slice) = instruction.argument_slice() {
            // validate the slice bounds
            self.verify_argument_slice(slice, instruction_id)?;

            // ensure slice arguments are defined
            let arguments = self.tree.get_arguments(slice);
            for &value in arguments {
                self.ensure_defined(value, VerifyAnchor::node(instruction_id), defined_values)?;
            }

            // validate call signatures and effects
            match instruction {
                Instruction::Call {
                    destination,
                    function,
                    signature,
                    effects,
                    ..
                } => {
                    self.verify_call_signature(
                        instruction_id,
                        *destination,
                        arguments.len(),
                        *signature,
                    )?;
                    self.verify_call_signature_matches_function(
                        VerifyAnchor::node(instruction_id),
                        *signature,
                        *function,
                    )?;
                    self.verify_call_effects(instruction_id, effects.as_ref(), arguments.len())?;
                }
                Instruction::CallVirtual {
                    destination,
                    signature,
                    effects,
                    declared_target,
                    ..
                } => {
                    self.verify_call_signature(
                        instruction_id,
                        *destination,
                        arguments.len(),
                        *signature,
                    )?;
                    if let Some(target) = declared_target {
                        self.verify_call_signature_matches_function(
                            VerifyAnchor::node(instruction_id),
                            *signature,
                            *target,
                        )?;
                    }
                    self.verify_call_effects(instruction_id, effects.as_ref(), arguments.len())?;
                }
                Instruction::CallInterface {
                    destination,
                    signature,
                    effects,
                    declared_target,
                    ..
                } => {
                    self.verify_call_signature(
                        instruction_id,
                        *destination,
                        arguments.len(),
                        *signature,
                    )?;
                    if let Some(target) = declared_target {
                        self.verify_call_signature_matches_function(
                            VerifyAnchor::node(instruction_id),
                            *signature,
                            *target,
                        )?;
                    }
                    self.verify_call_effects(instruction_id, effects.as_ref(), arguments.len())?;
                }
                Instruction::CallIndirect {
                    destination,
                    signature,
                    effects,
                    ..
                } => {
                    self.verify_call_signature(
                        instruction_id,
                        *destination,
                        arguments.len(),
                        *signature,
                    )?;
                    self.verify_call_effects(instruction_id, effects.as_ref(), arguments.len())?;
                }
                _ => {}
            }
        }

        // validate instruction node references
        self.verify_instruction_nodes(instruction, instruction_id)?;

        // validate local references
        self.verify_local_reference(instruction, locals, instruction_id)?;

        // validate instruction shape invariants
        self.verify_instruction_shapes(instruction, instruction_id)?;

        // validate vector and tensor instruction semantics
        self.verify_vector_tensor_ops(function, instruction, instruction_id)?;

        // validate inline types
        self.verify_instruction_inline_types(function, instruction, instruction_id)?;

        Ok(())
    }

    /// Resolve a value type for verification.
    fn value_type_or_error(
        &self,
        function: &Function,
        value: Value,
        anchor: VerifyAnchor,
        label: &'static str,
    ) -> VerifyResult<LocalNodeId<Type>> {
        // look up the value type
        let value_type =
            function
                .value_type(value)
                .ok_or_else(|| VerifyError::MetadataInvariantViolation {
                    message: format!("missing value type for {label}"),
                    anchor,
                })?;

        Ok(value_type)
    }

    /// Check whether two tensor shapes are compatible for casts.
    fn tensor_shapes_compatible(
        &self,
        left: &[TensorDimension],
        right: &[TensorDimension],
    ) -> bool {
        // reject mismatched ranks
        if left.len() != right.len() {
            return false;
        }

        // allow dynamic to match any dimension
        left.iter()
            .zip(right)
            .all(|(left_dim, right_dim)| match (left_dim, right_dim) {
                (TensorDimension::Static(left_size), TensorDimension::Static(right_size)) => {
                    left_size == right_size
                }
                _ => true,
            })
    }

    /// Verify vector and tensor instruction invariants.
    fn verify_vector_tensor_ops(
        &self,
        function: &Function,
        instruction: &Instruction,
        instruction_id: LocalNodeId<Instruction>,
    ) -> VerifyResult<()> {
        // prepare anchor for error reporting
        let anchor = VerifyAnchor::node(instruction_id);

        match instruction {
            Instruction::VectorCompare {
                destination,
                operator,
                left,
                right,
            } => {
                // reject non comparison operators
                if !operator.is_comparison() {
                    return Err(VerifyError::MetadataInvariantViolation {
                        message: "vector.compare requires a comparison operator".to_string(),
                        anchor,
                    });
                }

                // resolve operand types
                let left_type_id =
                    self.value_type_or_error(function, *left, anchor, "vector.compare left")?;
                let right_type_id =
                    self.value_type_or_error(function, *right, anchor, "vector.compare right")?;

                // resolve operand vector layouts
                let left_type = self.tree.get(left_type_id);
                let right_type = self.tree.get(right_type_id);
                let (left_element, left_lanes) = match left_type {
                    Type::Vector { element, lanes, .. } => (element, lanes),
                    other => {
                        return Err(VerifyError::MetadataInvariantViolation {
                            message: format!(
                                "vector.compare expects vector operands, found {}",
                                self.type_kind(other)
                            ),
                            anchor,
                        });
                    }
                };
                let (right_element, right_lanes) = match right_type {
                    Type::Vector { element, lanes, .. } => (element, lanes),
                    other => {
                        return Err(VerifyError::MetadataInvariantViolation {
                            message: format!(
                                "vector.compare expects vector operands, found {}",
                                self.type_kind(other)
                            ),
                            anchor,
                        });
                    }
                };

                // reject mismatched vector operands
                if left_element != right_element || left_lanes != right_lanes {
                    return Err(VerifyError::MetadataInvariantViolation {
                        message: "vector.compare expects matching vector operand types".to_string(),
                        anchor,
                    });
                }

                // resolve destination type
                let destination_type_id = self.value_type_or_error(
                    function,
                    *destination,
                    anchor,
                    "vector.compare result",
                )?;
                let destination_type = self.tree.get(destination_type_id);
                let (result_element, result_lanes) = match destination_type {
                    Type::Vector { element, lanes, .. } => (element, lanes),
                    other => {
                        return Err(VerifyError::MetadataInvariantViolation {
                            message: format!(
                                "vector.compare result must be a vector, found {}",
                                self.type_kind(other)
                            ),
                            anchor,
                        });
                    }
                };

                // reject mismatched lane counts
                if *result_lanes != *left_lanes {
                    return Err(VerifyError::MetadataInvariantViolation {
                        message: "vector.compare result must match lane count".to_string(),
                        anchor,
                    });
                }

                // reject non boolean result element types
                let result_element_type = self.tree.get(*result_element);
                if !matches!(result_element_type, Type::Boolean) {
                    return Err(VerifyError::MetadataInvariantViolation {
                        message: "vector.compare result element type must be bool".to_string(),
                        anchor,
                    });
                }
            }
            Instruction::VectorConvert {
                destination,
                vector,
                ..
            } => {
                // resolve operand types
                let source_type_id =
                    self.value_type_or_error(function, *vector, anchor, "vector.convert source")?;
                let destination_type_id = self.value_type_or_error(
                    function,
                    *destination,
                    anchor,
                    "vector.convert result",
                )?;

                // resolve vector types
                let source_type = self.tree.get(source_type_id);
                let destination_type = self.tree.get(destination_type_id);
                let source_lanes = match source_type {
                    Type::Vector { lanes, .. } => lanes,
                    other => {
                        return Err(VerifyError::MetadataInvariantViolation {
                            message: format!(
                                "vector.convert expects vector source, found {}",
                                self.type_kind(other)
                            ),
                            anchor,
                        });
                    }
                };
                let destination_lanes = match destination_type {
                    Type::Vector { lanes, .. } => lanes,
                    other => {
                        return Err(VerifyError::MetadataInvariantViolation {
                            message: format!(
                                "vector.convert expects vector result, found {}",
                                self.type_kind(other)
                            ),
                            anchor,
                        });
                    }
                };

                // reject mismatched lane counts
                if source_lanes != destination_lanes {
                    return Err(VerifyError::MetadataInvariantViolation {
                        message: "vector.convert must preserve lane count".to_string(),
                        anchor,
                    });
                }
            }
            Instruction::TensorCompare {
                destination,
                operator,
                left,
                right,
            } => {
                // reject non comparison operators
                if !operator.is_comparison() {
                    return Err(VerifyError::MetadataInvariantViolation {
                        message: "tensor.compare requires a comparison operator".to_string(),
                        anchor,
                    });
                }

                // resolve operand types
                let left_type_id =
                    self.value_type_or_error(function, *left, anchor, "tensor.compare left")?;
                let right_type_id =
                    self.value_type_or_error(function, *right, anchor, "tensor.compare right")?;

                // resolve operand tensor layouts
                let left_type = self.tree.get(left_type_id);
                let right_type = self.tree.get(right_type_id);
                let (left_element, left_shape, left_layout) = match left_type {
                    Type::Tensor {
                        element,
                        shape,
                        layout,
                        ..
                    } => (element, shape, layout),
                    other => {
                        return Err(VerifyError::MetadataInvariantViolation {
                            message: format!(
                                "tensor.compare expects tensor operands, found {}",
                                self.type_kind(other)
                            ),
                            anchor,
                        });
                    }
                };
                let (right_element, right_shape, right_layout) = match right_type {
                    Type::Tensor {
                        element,
                        shape,
                        layout,
                        ..
                    } => (element, shape, layout),
                    other => {
                        return Err(VerifyError::MetadataInvariantViolation {
                            message: format!(
                                "tensor.compare expects tensor operands, found {}",
                                self.type_kind(other)
                            ),
                            anchor,
                        });
                    }
                };

                // reject mismatched tensor operands
                if left_element != right_element
                    || left_shape != right_shape
                    || left_layout != right_layout
                {
                    return Err(VerifyError::MetadataInvariantViolation {
                        message: "tensor.compare expects matching tensor operand types".to_string(),
                        anchor,
                    });
                }

                // resolve destination type
                let destination_type_id = self.value_type_or_error(
                    function,
                    *destination,
                    anchor,
                    "tensor.compare result",
                )?;
                let destination_type = self.tree.get(destination_type_id);
                let (result_element, result_shape, result_layout) = match destination_type {
                    Type::Tensor {
                        element,
                        shape,
                        layout,
                        ..
                    } => (element, shape, layout),
                    other => {
                        return Err(VerifyError::MetadataInvariantViolation {
                            message: format!(
                                "tensor.compare result must be a tensor, found {}",
                                self.type_kind(other)
                            ),
                            anchor,
                        });
                    }
                };

                // reject mismatched shapes or layouts
                if result_shape != left_shape || result_layout != left_layout {
                    return Err(VerifyError::MetadataInvariantViolation {
                        message: "tensor.compare result must match tensor shape and layout"
                            .to_string(),
                        anchor,
                    });
                }

                // reject non boolean result element types
                let result_element_type = self.tree.get(*result_element);
                if !matches!(result_element_type, Type::Boolean) {
                    return Err(VerifyError::MetadataInvariantViolation {
                        message: "tensor.compare result element type must be bool".to_string(),
                        anchor,
                    });
                }
            }
            Instruction::TensorConvert {
                destination,
                tensor,
                ..
            } => {
                // resolve operand types
                let source_type_id =
                    self.value_type_or_error(function, *tensor, anchor, "tensor.convert source")?;
                let destination_type_id = self.value_type_or_error(
                    function,
                    *destination,
                    anchor,
                    "tensor.convert result",
                )?;

                // resolve tensor types
                let source_type = self.tree.get(source_type_id);
                let destination_type = self.tree.get(destination_type_id);
                let (source_shape, source_layout) = match source_type {
                    Type::Tensor { shape, layout, .. } => (shape, layout),
                    other => {
                        return Err(VerifyError::MetadataInvariantViolation {
                            message: format!(
                                "tensor.convert expects tensor source, found {}",
                                self.type_kind(other)
                            ),
                            anchor,
                        });
                    }
                };
                let (destination_shape, destination_layout) = match destination_type {
                    Type::Tensor { shape, layout, .. } => (shape, layout),
                    other => {
                        return Err(VerifyError::MetadataInvariantViolation {
                            message: format!(
                                "tensor.convert expects tensor result, found {}",
                                self.type_kind(other)
                            ),
                            anchor,
                        });
                    }
                };

                // reject mismatched shapes or layouts
                if source_shape != destination_shape || source_layout != destination_layout {
                    return Err(VerifyError::MetadataInvariantViolation {
                        message: "tensor.convert must preserve shape and layout".to_string(),
                        anchor,
                    });
                }
            }
            Instruction::TensorCast {
                destination,
                tensor,
            } => {
                // resolve operand types
                let source_type_id =
                    self.value_type_or_error(function, *tensor, anchor, "tensor.cast source")?;
                let destination_type_id =
                    self.value_type_or_error(function, *destination, anchor, "tensor.cast result")?;

                // resolve tensor types
                let source_type = self.tree.get(source_type_id);
                let destination_type = self.tree.get(destination_type_id);
                let (source_element, source_shape, source_layout) = match source_type {
                    Type::Tensor {
                        element,
                        shape,
                        layout,
                        ..
                    } => (element, shape, layout),
                    other => {
                        return Err(VerifyError::MetadataInvariantViolation {
                            message: format!(
                                "tensor.cast expects tensor source, found {}",
                                self.type_kind(other)
                            ),
                            anchor,
                        });
                    }
                };
                let (destination_element, destination_shape, destination_layout) =
                    match destination_type {
                        Type::Tensor {
                            element,
                            shape,
                            layout,
                            ..
                        } => (element, shape, layout),
                        other => {
                            return Err(VerifyError::MetadataInvariantViolation {
                                message: format!(
                                    "tensor.cast expects tensor result, found {}",
                                    self.type_kind(other)
                                ),
                                anchor,
                            });
                        }
                    };

                // reject element or layout mismatches
                if source_element != destination_element
                    || source_layout != destination_layout
                    || !self.tensor_shapes_compatible(source_shape, destination_shape)
                {
                    return Err(VerifyError::MetadataInvariantViolation {
                        message: "tensor.cast requires compatible shapes and matching layout"
                            .to_string(),
                        anchor,
                    });
                }
            }
            Instruction::TensorView {
                destination,
                view,
                offsets_count,
                sizes_count,
                strides_count,
                ..
            } => {
                // resolve operand types
                let source_type_id =
                    self.value_type_or_error(function, *view, anchor, "tensor.view source")?;
                let destination_type_id =
                    self.value_type_or_error(function, *destination, anchor, "tensor.view result")?;

                // resolve view types
                let source_type = self.tree.get(source_type_id);
                let destination_type = self.tree.get(destination_type_id);
                let (
                    source_kind,
                    source_address_space,
                    source_mutability,
                    source_element,
                    source_shape,
                ) = match source_type {
                    Type::TensorReference {
                        kind,
                        address_space,
                        mutability,
                        element,
                        shape,
                        ..
                    } => (kind, address_space, mutability, element, shape),
                    other => {
                        return Err(VerifyError::MetadataInvariantViolation {
                            message: format!(
                                "tensor.view expects tensor reference source, found {}",
                                self.type_kind(other)
                            ),
                            anchor,
                        });
                    }
                };
                let (
                    destination_kind,
                    destination_address_space,
                    destination_mutability,
                    destination_element,
                    destination_shape,
                ) = match destination_type {
                    Type::TensorReference {
                        kind,
                        address_space,
                        mutability,
                        element,
                        shape,
                        ..
                    } => (kind, address_space, mutability, element, shape),
                    other => {
                        return Err(VerifyError::MetadataInvariantViolation {
                            message: format!(
                                "tensor.view expects tensor reference result, found {}",
                                self.type_kind(other)
                            ),
                            anchor,
                        });
                    }
                };

                // reject incompatible reference kinds
                if source_kind != destination_kind
                    || source_address_space != destination_address_space
                    || source_element != destination_element
                {
                    return Err(VerifyError::MetadataInvariantViolation {
                        message: "tensor.view requires matching reference kind, address space, and element type"
                            .to_string(),
                        anchor,
                    });
                }

                // reject mutability upgrades
                if matches!(source_mutability, Mutability::Immutable)
                    && matches!(destination_mutability, Mutability::Mutable)
                {
                    return Err(VerifyError::MetadataInvariantViolation {
                        message: "tensor.view cannot increase mutability".to_string(),
                        anchor,
                    });
                }

                // reject mismatched ranks
                if source_shape.len() != destination_shape.len() {
                    return Err(VerifyError::MetadataInvariantViolation {
                        message: "tensor.view requires matching ranks".to_string(),
                        anchor,
                    });
                }

                // reject mismatched view argument counts
                let expected = destination_shape.len();
                if *offsets_count as usize != expected
                    || *sizes_count as usize != expected
                    || *strides_count as usize != expected
                {
                    return Err(VerifyError::MetadataInvariantViolation {
                        message:
                            "tensor.view requires offsets, sizes, and strides for each dimension"
                                .to_string(),
                        anchor,
                    });
                }
            }
            _ => {}
        }

        Ok(())
    }

    /// Verify a terminator and its successor edges.
    fn verify_terminator(
        &self,
        function: &Function,
        block_id: LocalNodeId<Block>,
        terminator: &Terminator,
        block_ids: &HashSet<LocalNodeId<Block>>,
        block_order: &HashMap<LocalNodeId<Block>, usize>,
        defined_values: &HashSet<Value>,
    ) -> VerifyResult<()> {
        // check uses
        for value in terminator.uses() {
            self.ensure_defined(value, VerifyAnchor::node(block_id), defined_values)?;
        }

        // check return value presence
        if let Terminator::Return { value } = terminator {
            let returns_void = matches!(self.tree.get(function.return_type), Type::Void);

            // reject return values for void functions
            if returns_void && value.is_some() {
                return Err(VerifyError::ReturnValueNotAllowedForVoid {
                    anchor: VerifyAnchor::node(block_id),
                });
            }

            // reject missing return values for non void functions
            if !returns_void && value.is_none() {
                return Err(VerifyError::ReturnValueRequiredForNonVoid {
                    anchor: VerifyAnchor::node(block_id),
                });
            }
        }

        // check successors and argument counts
        match terminator {
            Terminator::Return { .. } | Terminator::Unreachable => {
                // no successors to validate
            }
            Terminator::Jump { target, arguments } => {
                // validate jump arguments
                self.verify_block_arguments(block_id, *target, arguments, block_ids, block_order)?;
            }
            Terminator::Branch {
                then_target,
                then_arguments,
                else_target,
                else_arguments,
                ..
            } => {
                // validate branch arguments
                self.verify_block_arguments(
                    block_id,
                    *then_target,
                    then_arguments,
                    block_ids,
                    block_order,
                )?;
                self.verify_block_arguments(
                    block_id,
                    *else_target,
                    else_arguments,
                    block_ids,
                    block_order,
                )?;
            }
            Terminator::Check {
                success, failure, ..
            } => {
                // validate check successors
                self.verify_block_arguments(
                    block_id,
                    success.target,
                    &success.arguments,
                    block_ids,
                    block_order,
                )?;
                self.verify_block_arguments(
                    block_id,
                    failure.target,
                    &failure.arguments,
                    block_ids,
                    block_order,
                )?;
            }
            Terminator::Switch {
                default,
                default_arguments,
                cases,
                ..
            } => {
                // validate switch successors
                self.verify_block_arguments(
                    block_id,
                    *default,
                    default_arguments,
                    block_ids,
                    block_order,
                )?;
                self.verify_switch_cases(block_id, cases, block_ids, block_order)?;
            }
            Terminator::Yield {
                resume,
                resume_arguments,
                ..
            } => {
                // validate yield resume arguments
                self.verify_resume_arguments(
                    block_id,
                    *resume,
                    resume_arguments,
                    block_ids,
                    block_order,
                )?;
            }
            Terminator::TailCall {
                function: callee_id,
                arguments,
            } => {
                // validate tail call signatures
                self.ensure_node_type(
                    NodeType::Function,
                    callee_id.id,
                    VerifyAnchor::node(block_id),
                )?;

                // load the callee signature
                let callee = self.tree.get(*callee_id);

                // reject mismatched argument counts
                if arguments.len() != callee.parameters.len() {
                    return Err(VerifyError::CallArgumentCountMismatch {
                        expected: callee.parameters.len(),
                        got: arguments.len(),
                        anchor: VerifyAnchor::node(block_id),
                    });
                }

                // reject return kind mismatches
                let caller_returns_void = matches!(self.tree.get(function.return_type), Type::Void);
                let callee_returns_void = matches!(self.tree.get(callee.return_type), Type::Void);
                if caller_returns_void != callee_returns_void {
                    return Err(VerifyError::TailCallReturnTypeMismatch {
                        anchor: VerifyAnchor::node(block_id),
                    });
                }
            }
            Terminator::TailCallIndirect {
                arguments,
                signature,
                ..
            } => {
                let Type::FunctionPointer { parameters, result } = self.tree.get(*signature) else {
                    return Err(VerifyError::MetadataInvariantViolation {
                        message: "tailcall.indirect signature is not a function type".to_string(),
                        anchor: VerifyAnchor::node(block_id),
                    });
                };

                // reject mismatched argument counts
                if arguments.len() != parameters.len() {
                    return Err(VerifyError::CallArgumentCountMismatch {
                        expected: parameters.len(),
                        got: arguments.len(),
                        anchor: VerifyAnchor::node(block_id),
                    });
                }

                // reject return kind mismatches
                let caller_returns_void = matches!(self.tree.get(function.return_type), Type::Void);
                let callee_returns_void = matches!(self.tree.get(*result), Type::Void);
                if caller_returns_void != callee_returns_void {
                    return Err(VerifyError::TailCallReturnTypeMismatch {
                        anchor: VerifyAnchor::node(block_id),
                    });
                }
            }
            Terminator::TailCallVirtual {
                arguments,
                declaring_type,
                declared_target,
                signature,
                ..
            }
            | Terminator::TailCallInterface {
                arguments,
                declaring_type,
                declared_target,
                signature,
                ..
            } => {
                self.ensure_node_type(
                    NodeType::Type,
                    declaring_type.id,
                    VerifyAnchor::node(block_id),
                )?;

                if let Some(target) = declared_target {
                    self.ensure_node_type(
                        NodeType::Function,
                        target.id,
                        VerifyAnchor::node(block_id),
                    )?;
                    self.verify_call_signature_matches_function(
                        VerifyAnchor::node(block_id),
                        *signature,
                        *target,
                    )?;
                }

                let Type::FunctionPointer { parameters, result } = self.tree.get(*signature) else {
                    return Err(VerifyError::MetadataInvariantViolation {
                        message: "tailcall signature is not a function type".to_string(),
                        anchor: VerifyAnchor::node(block_id),
                    });
                };

                // reject mismatched argument counts
                if arguments.len() != parameters.len() {
                    return Err(VerifyError::CallArgumentCountMismatch {
                        expected: parameters.len(),
                        got: arguments.len(),
                        anchor: VerifyAnchor::node(block_id),
                    });
                }

                // reject return kind mismatches
                let caller_returns_void = matches!(self.tree.get(function.return_type), Type::Void);
                let callee_returns_void = matches!(self.tree.get(*result), Type::Void);
                if caller_returns_void != callee_returns_void {
                    return Err(VerifyError::TailCallReturnTypeMismatch {
                        anchor: VerifyAnchor::node(block_id),
                    });
                }
            }
        }

        Ok(())
    }

    /// Verify block argument counts for a target.
    fn verify_block_arguments(
        &self,
        source_block: LocalNodeId<Block>,
        target: LocalNodeId<Block>,
        arguments: &[Value],
        block_ids: &HashSet<LocalNodeId<Block>>,
        block_order: &HashMap<LocalNodeId<Block>, usize>,
    ) -> VerifyResult<()> {
        // ensure target exists
        if !block_ids.contains(&target) {
            return Err(VerifyError::UnknownBlockTarget {
                block_id: target,
                anchor: VerifyAnchor::node(source_block),
            });
        }

        // ensure argument count matches parameters
        let block = self.tree.get(target);
        if arguments.len() != block.parameters.len() {
            return Err(VerifyError::BlockArgumentCountMismatch {
                block_label: self.block_label(target, block_order),
                expected: block.parameters.len(),
                got: arguments.len(),
                anchor: VerifyAnchor::node(source_block),
            });
        }

        Ok(())
    }

    /// Verify resume arguments for a yield terminator.
    fn verify_resume_arguments(
        &self,
        source_block: LocalNodeId<Block>,
        resume: LocalNodeId<Block>,
        resume_arguments: &[Value],
        block_ids: &HashSet<LocalNodeId<Block>>,
        block_order: &HashMap<LocalNodeId<Block>, usize>,
    ) -> VerifyResult<()> {
        // ensure resume target exists
        if !block_ids.contains(&resume) {
            return Err(VerifyError::UnknownBlockTarget {
                block_id: resume,
                anchor: VerifyAnchor::node(source_block),
            });
        }

        // ensure resume arguments align with the resume block parameters
        let block = self.tree.get(resume);
        if resume_arguments.len() + 1 != block.parameters.len() {
            return Err(VerifyError::ResumeArgumentCountMismatch {
                block_label: self.block_label(resume, block_order),
                expected: block.parameters.len().saturating_sub(1),
                got: resume_arguments.len(),
                anchor: VerifyAnchor::node(source_block),
            });
        }

        Ok(())
    }

    /// Verify switch case values and arguments.
    fn verify_switch_cases(
        &self,
        source_block: LocalNodeId<Block>,
        cases: &[SwitchCase],
        block_ids: &HashSet<LocalNodeId<Block>>,
        block_order: &HashMap<LocalNodeId<Block>, usize>,
    ) -> VerifyResult<()> {
        // track case values
        let mut seen = HashSet::new();

        for case in cases {
            // reject duplicate case values
            if !seen.insert(case.value) {
                return Err(VerifyError::DuplicateSwitchCaseValue {
                    value: case.value,
                    anchor: VerifyAnchor::node(source_block),
                });
            }

            // validate case arguments
            self.verify_block_arguments(
                source_block,
                case.target,
                &case.arguments,
                block_ids,
                block_order,
            )?;
        }

        Ok(())
    }

    /// Ensure a value is defined within the function.
    fn ensure_defined(
        &self,
        value: Value,
        anchor: VerifyAnchor,
        defined_values: &HashSet<Value>,
    ) -> VerifyResult<()> {
        // ensure the value was defined
        if !defined_values.contains(&value) {
            return Err(VerifyError::UseOfUndefinedValue { value, anchor });
        }

        Ok(())
    }

    /// Ensure a node id points at the expected node type.
    fn ensure_node_type(
        &self,
        expected: NodeType,
        node_id: u32,
        anchor: VerifyAnchor,
    ) -> VerifyResult<()> {
        // skip node type checks when disabled
        if !self.options.verify_node_types {
            return Ok(());
        }

        // resolve the node type
        let found = self
            .tree
            .node_type_by_node_id
            .get(node_id as usize)
            .copied();

        // reject mismatched node types
        if found != Some(expected) {
            return Err(VerifyError::InvalidNodeReference {
                expected,
                found,
                node_id,
                anchor,
            });
        }

        Ok(())
    }

    /// Ensure an argument slice is within the argument buffer.
    fn verify_argument_slice(
        &self,
        slice: ArgumentSlice,
        instruction_id: LocalNodeId<Instruction>,
    ) -> VerifyResult<()> {
        // skip argument slice checks when disabled
        if !self.options.verify_argument_slices {
            return Ok(());
        }

        // compute slice bounds
        let start = slice.start as usize;
        let end = start + slice.count as usize;
        let len = self.tree.instruction_arguments.len();

        // reject out of bounds slices
        if end > len {
            return Err(VerifyError::ArgumentSliceOutOfBounds {
                start: slice.start,
                count: slice.count,
                len,
                anchor: VerifyAnchor::node(instruction_id),
            });
        }

        Ok(())
    }

    /// Verify local references are defined within the function.
    fn verify_local_reference(
        &self,
        instruction: &Instruction,
        locals: &HashSet<LocalNodeId<Local>>,
        instruction_id: LocalNodeId<Instruction>,
    ) -> VerifyResult<()> {
        // skip local reference checks when disabled
        if !self.options.verify_local_references {
            return Ok(());
        }

        // validate local references
        match instruction {
            Instruction::LocalGet { local, .. }
            | Instruction::LocalAddr { local, .. }
            | Instruction::LocalSet { local, .. } => {
                // reject locals not owned by the function
                if !locals.contains(local) {
                    return Err(VerifyError::LocalReferenceNotInFunction {
                        local_id: *local,
                        anchor: VerifyAnchor::node(instruction_id),
                    });
                }
            }
            _ => {}
        }

        Ok(())
    }

    /// Verify instruction node references.
    fn verify_instruction_nodes(
        &self,
        instruction: &Instruction,
        instruction_id: LocalNodeId<Instruction>,
    ) -> VerifyResult<()> {
        // skip node checks when disabled
        if !self.options.verify_node_types {
            return Ok(());
        }

        // prepare anchor for node checks
        let anchor = VerifyAnchor::node(instruction_id);

        // validate instruction node references
        match instruction {
            Instruction::LocalGet { local, .. } | Instruction::LocalSet { local, .. } => {
                // ensure local ids resolve
                self.ensure_node_type(NodeType::Local, local.id, anchor)?;
            }
            Instruction::LocalAddr {
                local, result_type, ..
            } => {
                // ensure local ids resolve
                self.ensure_node_type(NodeType::Local, local.id, anchor)?;
                self.ensure_node_type(NodeType::Type, result_type.id, anchor)?;
            }
            Instruction::GlobalAddr { global, .. } | Instruction::GlobalConst { global, .. } => {
                // ensure global ids resolve
                self.ensure_node_type(NodeType::Global, global.id, anchor)?;
            }
            Instruction::FunctionAddr { function, .. } => {
                // ensure function ids resolve
                self.ensure_node_type(NodeType::Function, function.id, anchor)?;
            }
            Instruction::Call { function, .. } => {
                // ensure function ids resolve
                self.ensure_node_type(NodeType::Function, function.id, anchor)?;
            }
            Instruction::CallVirtual {
                declared_target, ..
            }
            | Instruction::CallInterface {
                declared_target, ..
            } => {
                // ensure declared targets resolve
                if let Some(target) = declared_target {
                    self.ensure_node_type(NodeType::Function, target.id, anchor)?;
                }
            }
            Instruction::Cast { to_type, .. }
            | Instruction::Struct { ty: to_type, .. }
            | Instruction::Tuple { ty: to_type, .. }
            | Instruction::Array { ty: to_type, .. }
            | Instruction::ManagedAlloc {
                layout: to_type, ..
            }
            | Instruction::ManagedAllocArray {
                element: to_type, ..
            }
            | Instruction::RawAlloc {
                layout: to_type, ..
            }
            | Instruction::StackAlloc {
                layout: to_type, ..
            } => {
                // ensure type ids resolve
                self.ensure_node_type(NodeType::Type, to_type.id, anchor)?;
            }
            _ => {}
        }

        Ok(())
    }

    /// Verify aggregate constructor shapes.
    fn verify_instruction_shapes(
        &self,
        instruction: &Instruction,
        instruction_id: LocalNodeId<Instruction>,
    ) -> VerifyResult<()> {
        // skip type shape checks when disabled
        if !self.options.verify_type_shapes {
            return Ok(());
        }

        // prepare anchor for shape validation
        let anchor = VerifyAnchor::node(instruction_id);

        // validate aggregate constructor shapes
        match instruction {
            Instruction::Struct { ty, fields, .. } => {
                // resolve struct layout
                let ty = self.tree.get(*ty);
                let expected = match ty {
                    Type::Struct { fields, .. } => fields.len(),
                    other => {
                        return Err(VerifyError::AggregateTypeMismatch {
                            expected: "struct",
                            found: self.type_kind(other),
                            anchor,
                        });
                    }
                };
                let got = fields.len();

                // reject mismatched field counts
                if expected != got {
                    return Err(VerifyError::AggregateArgumentCountMismatch {
                        expected,
                        got,
                        anchor,
                    });
                }
            }
            Instruction::Tuple { ty, elements, .. } => {
                // resolve tuple layout
                let ty = self.tree.get(*ty);
                let expected = match ty {
                    Type::Tuple { elements, .. } => elements.len(),
                    other => {
                        return Err(VerifyError::AggregateTypeMismatch {
                            expected: "tuple",
                            found: self.type_kind(other),
                            anchor,
                        });
                    }
                };
                let got = elements.len();

                // reject mismatched element counts
                if expected != got {
                    return Err(VerifyError::AggregateArgumentCountMismatch {
                        expected,
                        got,
                        anchor,
                    });
                }
            }
            Instruction::Array { ty, elements, .. } => {
                // resolve array layout
                let ty = self.tree.get(*ty);
                let expected = match ty {
                    Type::Array { length, .. } => usize::try_from(*length).unwrap_or(usize::MAX),
                    other => {
                        return Err(VerifyError::AggregateTypeMismatch {
                            expected: "array",
                            found: self.type_kind(other),
                            anchor,
                        });
                    }
                };
                let got = elements.len();

                // reject mismatched element counts
                if expected != got {
                    return Err(VerifyError::AggregateArgumentCountMismatch {
                        expected,
                        got,
                        anchor,
                    });
                }
            }
            Instruction::TensorSlice {
                arguments,
                offsets_count,
                sizes_count,
                strides_count,
                ..
            } => {
                // validate argument counts
                let expected =
                    *offsets_count as usize + *sizes_count as usize + *strides_count as usize;
                let got = arguments.len();
                if expected != got {
                    return Err(VerifyError::AggregateArgumentCountMismatch {
                        expected,
                        got,
                        anchor,
                    });
                }
            }
            Instruction::TensorView {
                arguments,
                offsets_count,
                sizes_count,
                strides_count,
                ..
            } => {
                // validate argument counts
                let expected =
                    *offsets_count as usize + *sizes_count as usize + *strides_count as usize;
                let got = arguments.len();
                if expected != got {
                    return Err(VerifyError::AggregateArgumentCountMismatch {
                        expected,
                        got,
                        anchor,
                    });
                }
            }
            Instruction::TensorPad {
                arguments,
                low_count,
                high_count,
                interior_count,
                ..
            } => {
                // validate argument counts
                let expected =
                    *low_count as usize + *high_count as usize + *interior_count as usize;
                let got = arguments.len();
                if expected != got {
                    return Err(VerifyError::AggregateArgumentCountMismatch {
                        expected,
                        got,
                        anchor,
                    });
                }
            }
            _ => {}
        }

        Ok(())
    }

    /// Verify inline result and signature types.
    fn verify_instruction_inline_types(
        &self,
        function: &Function,
        instruction: &Instruction,
        instruction_id: LocalNodeId<Instruction>,
    ) -> VerifyResult<()> {
        // prepare anchor for error reporting
        let anchor = VerifyAnchor::node(instruction_id);

        // validate pointer-producing result types
        match instruction {
            Instruction::GlobalAddr {
                global,
                result_type,
                ..
            } => {
                self.ensure_node_type(NodeType::Type, result_type.id, anchor)?;
                let global_decl = self.tree.get(*global);
                self.verify_reference_result_type(
                    *result_type,
                    Some(global_decl.ty),
                    Some(ReferenceKind::Raw),
                    Some(global_decl.mutability),
                    anchor,
                )?;
            }
            Instruction::FunctionAddr {
                destination,
                function: target,
            } => {
                let destination_type_id = self.value_type_or_error(
                    function,
                    *destination,
                    anchor,
                    "function.addr result",
                )?;
                let destination_type = self.tree.get(destination_type_id);
                let Type::FunctionPointer { parameters, result } = destination_type else {
                    return Err(VerifyError::MetadataInvariantViolation {
                        message: "function.addr result must be a function pointer".to_string(),
                        anchor,
                    });
                };

                let target_function = self.tree.get(*target);
                if target_function.parameters.len() != parameters.len() {
                    return Err(VerifyError::MetadataInvariantViolation {
                        message: "function.addr result parameter count mismatch".to_string(),
                        anchor,
                    });
                }

                for (parameter, signature_type) in target_function
                    .parameters
                    .iter()
                    .zip(parameters.iter())
                {
                    if parameter.ty != *signature_type {
                        return Err(VerifyError::MetadataInvariantViolation {
                            message: "function.addr result parameter types mismatch".to_string(),
                            anchor,
                        });
                    }
                }

                if target_function.return_type != *result {
                    return Err(VerifyError::MetadataInvariantViolation {
                        message: "function.addr result return type mismatch".to_string(),
                        anchor,
                    });
                }
            }
            Instruction::ManagedAlloc {
                layout,
                result_type,
                ..
            } => {
                self.ensure_node_type(NodeType::Type, result_type.id, anchor)?;
                self.verify_reference_result_type(
                    *result_type,
                    Some(*layout),
                    Some(ReferenceKind::Managed),
                    None,
                    anchor,
                )?;
            }
            Instruction::ManagedAllocArray {
                element,
                result_type,
                ..
            } => {
                self.ensure_node_type(NodeType::Type, result_type.id, anchor)?;
                self.verify_reference_result_type(
                    *result_type,
                    Some(*element),
                    Some(ReferenceKind::Managed),
                    None,
                    anchor,
                )?;
            }
            Instruction::RawAlloc {
                layout,
                result_type,
                ..
            } => {
                self.ensure_node_type(NodeType::Type, result_type.id, anchor)?;
                self.verify_reference_result_type(
                    *result_type,
                    Some(*layout),
                    Some(ReferenceKind::Raw),
                    None,
                    anchor,
                )?;
            }
            Instruction::StackAlloc {
                layout,
                result_type,
                ..
            } => {
                self.ensure_node_type(NodeType::Type, result_type.id, anchor)?;
                self.verify_reference_result_type(
                    *result_type,
                    Some(*layout),
                    Some(ReferenceKind::Raw),
                    None,
                    anchor,
                )?;
            }
            Instruction::FieldAddr { result_type, .. }
            | Instruction::ElementAddr { result_type, .. } => {
                self.ensure_node_type(NodeType::Type, result_type.id, anchor)?;
                self.verify_reference_result_type(*result_type, None, None, None, anchor)?;
            }
            Instruction::Call { signature, .. } | Instruction::CallIndirect { signature, .. } => {
                self.ensure_node_type(NodeType::Type, signature.id, anchor)?;
                let ty = self.tree.get(*signature);
                if !matches!(ty, Type::FunctionPointer { .. }) {
                    return Err(VerifyError::MetadataInvariantViolation {
                        message: "call signature is not a function type".to_string(),
                        anchor,
                    });
                }
            }
            Instruction::CallVirtual {
                declaring_type,
                signature,
                ..
            }
            | Instruction::CallInterface {
                declaring_type,
                signature,
                ..
            } => {
                self.ensure_node_type(NodeType::Type, declaring_type.id, anchor)?;
                self.ensure_node_type(NodeType::Type, signature.id, anchor)?;
                let ty = self.tree.get(*signature);
                if !matches!(ty, Type::FunctionPointer { .. }) {
                    return Err(VerifyError::MetadataInvariantViolation {
                        message: "call signature is not a function type".to_string(),
                        anchor,
                    });
                }
            }
            _ => {}
        }

        Ok(())
    }

    /// Verify a reference result type.
    fn verify_reference_result_type(
        &self,
        result_type: LocalNodeId<Type>,
        expected_pointee: Option<LocalNodeId<Type>>,
        expected_kind: Option<ReferenceKind>,
        expected_mutability: Option<Mutability>,
        anchor: VerifyAnchor,
    ) -> VerifyResult<()> {
        let Type::Reference {
            kind,
            mutability,
            pointee,
            ..
        } = self.tree.get(result_type)
        else {
            return Err(VerifyError::MetadataInvariantViolation {
                message: "pointer-producing instruction result type is not a reference".to_string(),
                anchor,
            });
        };

        if let Some(expected) = expected_pointee
            && *pointee != expected
        {
            return Err(VerifyError::MetadataInvariantViolation {
                message: "pointer-producing instruction result type mismatches pointee".to_string(),
                anchor,
            });
        }

        if let Some(expected) = expected_kind
            && *kind != expected
        {
            return Err(VerifyError::MetadataInvariantViolation {
                message: "pointer-producing instruction result type has wrong reference kind"
                    .to_string(),
                anchor,
            });
        }

        if let Some(expected) = expected_mutability
            && *mutability != expected
        {
            return Err(VerifyError::MetadataInvariantViolation {
                message: "pointer-producing instruction result type has wrong mutability"
                    .to_string(),
                anchor,
            });
        }

        Ok(())
    }

    /// Verify metadata tables after function checks.
    fn verify_metadata_tables(&self) -> VerifyResult<()> {
        // validate memory metadata entries
        for (&instruction_id, accesses) in &self.tree.memory_table.memory_accesses_by_instruction_id
        {
            // ensure the instruction id resolves
            self.ensure_node_type(
                NodeType::Instruction,
                instruction_id.id,
                VerifyAnchor::node(instruction_id),
            )?;

            // ensure the instruction can carry memory metadata
            let instruction = self.tree.get(instruction_id);
            if !matches!(
                instruction,
                Instruction::Load { .. }
                    | Instruction::Store { .. }
                    | Instruction::Intrinsic { .. }
            ) {
                return Err(VerifyError::MetadataInvariantViolation {
                    message: "memory metadata attached to non memory instruction".to_string(),
                    anchor: VerifyAnchor::node(instruction_id),
                });
            }

            // validate memory access entries
            for access in accesses {
                self.verify_memory_access_invariants(access, VerifyAnchor::node(instruction_id))?;
            }
        }

        Ok(())
    }

    /// Verify function metadata invariants.
    fn verify_function_metadata(
        &self,
        function_id: LocalNodeId<Function>,
        function: &Function,
    ) -> VerifyResult<()> {
        // ensure parameter metadata aligns with parameters
        if function.parameter_attributes.len() != function.parameters.len() {
            return Err(VerifyError::MetadataInvariantViolation {
                message: format!(
                    "parameter attributes length mismatch expected {} got {}",
                    function.parameters.len(),
                    function.parameter_attributes.len()
                ),
                anchor: VerifyAnchor::node(function_id),
            });
        }

        // validate memory effects
        self.verify_memory_effect_invariants(
            function.memory_effects.as_ref(),
            VerifyAnchor::node(function_id),
        )?;

        // validate call behavior
        self.verify_call_behavior_invariants(
            function.call_behavior.as_ref(),
            VerifyAnchor::node(function_id),
        )?;

        // validate return attributes
        self.verify_pointer_attributes_invariants(
            "function return attributes",
            &function.return_attributes,
            VerifyAnchor::node(function_id),
        )?;

        // validate parameter attributes
        for attributes in &function.parameter_attributes {
            self.verify_pointer_attributes_invariants(
                "function parameter attributes",
                attributes,
                VerifyAnchor::node(function_id),
            )?;
        }

        Ok(())
    }

    /// Verify call signature invariants.
    fn verify_call_signature(
        &self,
        instruction_id: LocalNodeId<Instruction>,
        destination: Option<Value>,
        argument_count: usize,
        signature: LocalNodeId<Type>,
    ) -> VerifyResult<()> {
        // resolve the signature type
        let Type::FunctionPointer { parameters, result } = self.tree.get(signature) else {
            return Err(VerifyError::MetadataInvariantViolation {
                message: "call signature is not a function type".to_string(),
                anchor: VerifyAnchor::node(instruction_id),
            });
        };

        // reject mismatched argument counts
        if argument_count != parameters.len() {
            return Err(VerifyError::CallArgumentCountMismatch {
                expected: parameters.len(),
                got: argument_count,
                anchor: VerifyAnchor::node(instruction_id),
            });
        }

        let returns_void = matches!(self.tree.get(*result), Type::Void);

        // reject return values from void callees
        if returns_void && destination.is_some() {
            return Err(VerifyError::CallReturnValueNotAllowedForVoid {
                anchor: VerifyAnchor::node(instruction_id),
            });
        }

        Ok(())
    }

    /// Verify call signatures against a concrete function.
    fn verify_call_signature_matches_function(
        &self,
        anchor: VerifyAnchor,
        signature: LocalNodeId<Type>,
        function_id: LocalNodeId<Function>,
    ) -> VerifyResult<()> {
        // resolve the signature type
        let Type::FunctionPointer { parameters, result } = self.tree.get(signature) else {
            return Err(VerifyError::MetadataInvariantViolation {
                message: "call signature is not a function type".to_string(),
                anchor,
            });
        };

        let function = self.tree.get(function_id);

        // reject mismatched parameter counts
        if parameters.len() != function.parameters.len() {
            return Err(VerifyError::MetadataInvariantViolation {
                message: "call signature does not match callee".to_string(),
                anchor,
            });
        }

        // validate parameter types
        for (parameter, signature_type) in function.parameters.iter().zip(parameters.iter()) {
            if parameter.ty != *signature_type {
                return Err(VerifyError::MetadataInvariantViolation {
                    message: "call signature does not match callee".to_string(),
                    anchor,
                });
            }
        }

        // validate return type
        if function.return_type != *result {
            return Err(VerifyError::MetadataInvariantViolation {
                message: "call signature does not match callee".to_string(),
                anchor,
            });
        }

        Ok(())
    }

    /// Verify call effects invariants.
    fn verify_call_effects(
        &self,
        instruction_id: LocalNodeId<Instruction>,
        effects: Option<&CallEffects>,
        argument_count: usize,
    ) -> VerifyResult<()> {
        // skip empty effects
        let Some(effects) = effects else {
            return Ok(());
        };

        let anchor = VerifyAnchor::node(instruction_id);

        // validate argument metadata length
        if !effects.argument_metadata.is_empty()
            && effects.argument_metadata.len() != argument_count
        {
            return Err(VerifyError::MetadataInvariantViolation {
                message: format!(
                    "call effects argument count mismatch expected {argument_count} got {}",
                    effects.argument_metadata.len()
                ),
                anchor,
            });
        }

        // validate memory effects
        self.verify_memory_effect_invariants(effects.memory_effects.as_ref(), anchor)?;

        // validate call behavior
        self.verify_call_behavior_invariants(effects.behavior.as_ref(), anchor)?;

        // validate return attributes
        self.verify_pointer_attributes_invariants(
            "call return attributes",
            &effects.return_attributes,
            anchor,
        )?;

        // validate argument attributes
        for argument in &effects.argument_metadata {
            self.verify_pointer_attributes_invariants(
                "call argument attributes",
                &argument.attributes,
                anchor,
            )?;
        }

        Ok(())
    }

    /// Verify memory effect invariants.
    fn verify_memory_effect_invariants(
        &self,
        effect: Option<&MemoryEffect>,
        anchor: VerifyAnchor,
    ) -> VerifyResult<()> {
        // skip empty effects
        let Some(effect) = effect else {
            return Ok(());
        };

        // require locations for memory effects
        if !effect.reads && !effect.writes && !effect.locations.is_empty() {
            return Err(VerifyError::MetadataInvariantViolation {
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
            return Err(VerifyError::MetadataInvariantViolation {
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
            return Err(VerifyError::MetadataInvariantViolation {
                message: "inaccessibleMemOnly set without inaccessible locations".to_string(),
                anchor,
            });
        }

        Ok(())
    }

    /// Verify call behavior invariants.
    fn verify_call_behavior_invariants(
        &self,
        behavior: Option<&crate::CallBehavior>,
        anchor: VerifyAnchor,
    ) -> VerifyResult<()> {
        // skip empty behaviors
        let Some(behavior) = behavior else {
            return Ok(());
        };

        // reject incompatible return flags
        if behavior.noreturn && behavior.will_return {
            return Err(VerifyError::MetadataInvariantViolation {
                message: "noreturn implies will_return is false".to_string(),
                anchor,
            });
        }

        // validate allocation location metadata
        if !behavior.allocates && behavior.alloc_locations.is_some() {
            return Err(VerifyError::MetadataInvariantViolation {
                message: "alloc locations set without allocates".to_string(),
                anchor,
            });
        }

        // validate allocation address space metadata
        if !behavior.allocates && behavior.alloc_address_spaces.is_some() {
            return Err(VerifyError::MetadataInvariantViolation {
                message: "alloc address spaces set without allocates".to_string(),
                anchor,
            });
        }

        // validate free location metadata
        if !behavior.frees && behavior.free_locations.is_some() {
            return Err(VerifyError::MetadataInvariantViolation {
                message: "free locations set without frees".to_string(),
                anchor,
            });
        }

        // validate free address space metadata
        if !behavior.frees && behavior.free_address_spaces.is_some() {
            return Err(VerifyError::MetadataInvariantViolation {
                message: "free address spaces set without frees".to_string(),
                anchor,
            });
        }

        Ok(())
    }

    /// Verify pointer attribute invariants.
    fn verify_pointer_attributes_invariants(
        &self,
        label: &str,
        attributes: &crate::PointerAttributes,
        anchor: VerifyAnchor,
    ) -> VerifyResult<()> {
        // reject conflicting access flags
        if attributes.readonly && attributes.writeonly {
            return Err(VerifyError::MetadataInvariantViolation {
                message: format!("{label} cannot be both readonly and writeonly"),
                anchor,
            });
        }

        Ok(())
    }

    /// Verify memory access invariants.
    fn verify_memory_access_invariants(
        &self,
        access: &MemoryAccessMetadata,
        anchor: VerifyAnchor,
    ) -> VerifyResult<()> {
        // reject invariant writes
        if access.is_invariant && access.kind != MemoryAccessKind::Read {
            return Err(VerifyError::MetadataInvariantViolation {
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
            return Err(VerifyError::MetadataInvariantViolation {
                message: "atomic ordering set for non atomic access".to_string(),
                anchor,
            });
        }

        Ok(())
    }

    /// Return a short type kind name.
    fn type_kind(&self, ty: &Type) -> &'static str {
        // map types to short labels
        match ty {
            Type::Void => "void",
            Type::Boolean => "bool",
            Type::Int { .. } => "int",
            Type::Isize => "isize",
            Type::Usize => "usize",
            Type::Float { .. } => "float",
            Type::Type => "type",
            Type::Reference { .. } => "ref",
            Type::Array { .. } => "array",
            Type::Tuple { .. } => "tuple",
            Type::Struct { .. } => "struct",
            Type::Vector { .. } => "vector",
            Type::Tensor { .. } => "tensor",
            Type::TensorReference { .. } => "tensor_ref",
            Type::FunctionPointer { .. } => "fn",
        }
    }

    /// Format a block label using function order.
    fn block_label(
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
