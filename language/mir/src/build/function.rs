use destack_base::StringId;
use indexmap::{IndexMap, IndexSet};

use crate::{
    AddressSpace, AllocSize, AllocationMode, AtomicScope, BinaryOperator, Block, CallBehavior,
    CallEffects, CastOperator, CheckConstraint, CheckTarget, Constant, ExecutionModel,
    ExecutionStage, Function, Global, Instruction, Intrinsic, Lifetime, Linkage, Local,
    LocalNodeId, MemoryEffect, MemoryOrdering, MemoryScope, MemorySemantics, Mutability, NodeTree,
    Ownership, PointerAttributes, ReferenceKind, TensorConvertMode,
    TensorConvolutionDimensionNumbers, TensorConvolutionWindow, TensorDotDimensionNumbers,
    TensorGatherDimensionNumbers, TensorReduceOperator, TensorScatterDimensionNumbers,
    TensorScatterMode, Terminator, Type, TypedValue, UnaryOperator, Value, VectorConvertMode,
    VectorReduceOperator,
};

use super::Variable;

/// Builder for constructing a single MIR function with automatic SSA construction.
/// Implements the algorithm from
///  - "Simple and Efficient Construction of Static Single Assignment Form" (Braun et al., 2013)
///    <https://c9x.me/compile/bib/braun13cc.pdf>
///  - Cranelift (<https://github.com/bytecodealliance/wasmtime/tree/main/cranelift>)
///
/// # Usage
///
/// 1. Create blocks with `block()`.
/// 2. Switch to a block with `switch_to_block()`.
/// 3. Add instructions (which return SSA values).
/// 4. Use `define_variable()` / `use_variable()` for mutable bindings.
/// 5. Seal blocks when all predecessors are known with `seal_block()`.
/// 6. Call `finish()` to get the completed function.
///
/// # SSA Construction
///
/// The algorithm works by tracking variable definitions per-block and lazily constructing
/// block parameters (φ-functions) when a variable is used. Key features:
/// - **Local Value Numbering**: If a variable is defined in the current block, return that value.
/// - **Global Value Numbering**: Otherwise, recursively look up the value from predecessors.
/// - **Block Parameters**: Created at join points where different predecessors have different values.
/// - **Trivial φ Removal**: If all predecessors have the same value, no block parameter is needed.
/// - **Incomplete CFGs**: Blocks can be used before all predecessors are known (unsealed blocks).
#[derive(Debug)]
pub struct FunctionBuilder<'a> {
    // meta
    /// The node tree this function is being built in.
    tree: &'a mut NodeTree,
    /// The id of the function being built.
    function_id: LocalNodeId<Function>,
    /// Current block we're inserting into.
    current_block: Option<LocalNodeId<Block>>,

    // ssa construction state
    /// Next SSA value id to allocate.
    next_value_id: u32,
    /// Next variable id to allocate.
    next_variable_id: u32,
    /// Variable definitions: (block, variable) → value.
    /// Tracks the SSA value of each variable at the end of each block.
    variable_definitions: IndexMap<(LocalNodeId<Block>, Variable), Value>,
    /// Sealed blocks (all predecessors known).
    sealed_blocks: IndexSet<LocalNodeId<Block>>,
    /// Block predecessors: block → list of predecessor blocks.
    predecessors: IndexMap<LocalNodeId<Block>, Vec<LocalNodeId<Block>>>,
    /// Incomplete block parameters that need resolution when the block is sealed.
    /// Maps block → list of (variable, parameter value) pairs.
    incomplete_phis: IndexMap<LocalNodeId<Block>, Vec<(Variable, Value)>>,
    /// Variable types (needed for creating block parameters).
    variable_types: IndexMap<Variable, LocalNodeId<Type>>,
    /// Blocks in order of creation.
    blocks: Vec<LocalNodeId<Block>>,
}

// allow builder helpers with many parameters
#[allow(clippy::too_many_arguments)]
impl<'a> FunctionBuilder<'a> {
    /// Create a new function builder.
    pub fn new(
        tree: &'a mut NodeTree,
        name: StringId,
        parameter_types: &[LocalNodeId<Type>],
        return_type: LocalNodeId<Type>,
    ) -> Self {
        // create parameter values
        let mut next_value_id = 0u32;
        let parameters: Vec<TypedValue> = parameter_types
            .iter()
            .map(|&ty| {
                let value = Value::new(next_value_id);
                next_value_id += 1;
                TypedValue::new(value, ty)
            })
            .collect();
        let value_types = parameters.iter().map(|param| param.ty).collect();

        // blank function (entry will be set in finish())
        let function = Function {
            name,
            parameters,
            parameter_names: vec![None; parameter_types.len()],
            value_types,
            return_type,
            return_lifetime: Lifetime::Inferred,
            memory_effects: None,
            call_behavior: None,
            alloc_size: None,
            parameter_attributes: vec![PointerAttributes::default(); parameter_types.len()],
            return_attributes: PointerAttributes::default(),
            linkage: Linkage::Local,
            allocation: AllocationMode::Any,
            coroutine: None,
            execution_model: None,
            execution_stage: None,
            workgroup_size: None,
            closure_env_type: None,
            locals: Vec::new(),
            blocks: Vec::new(),
            entry: None,
            next_value_id,
        };
        let function_id = tree.insert(function);

        Self {
            tree,
            function_id,
            current_block: None,
            next_value_id,
            next_variable_id: 0,
            variable_definitions: IndexMap::new(),
            sealed_blocks: IndexSet::new(),
            predecessors: IndexMap::new(),
            incomplete_phis: IndexMap::new(),
            variable_types: IndexMap::new(),
            blocks: Vec::new(),
        }
    }

    /// Create a function builder for an existing declared function.
    /// (Must not have a body yet).
    pub fn from_declared(tree: &'a mut NodeTree, function_id: LocalNodeId<Function>) -> Self {
        // validate the declared function is still empty
        let next_value_id = {
            let function = tree.get(function_id);
            assert!(
                function.entry.is_none() && function.blocks.is_empty(),
                "function already has a body"
            );

            function.next_value_id
        };

        Self {
            tree,
            function_id,
            current_block: None,
            next_value_id,
            next_variable_id: 0,
            variable_definitions: IndexMap::new(),
            sealed_blocks: IndexSet::new(),
            predecessors: IndexMap::new(),
            incomplete_phis: IndexMap::new(),
            variable_types: IndexMap::new(),
            blocks: Vec::new(),
        }
    }

    /// Set a debug parameter name on the function signature.
    pub fn set_parameter_name(&mut self, index: usize, name: destack_base::StringId) {
        // record parameter names for diagnostics
        let function = self.tree.get_mut(self.function_id);
        if let Some(slot) = function.parameter_names.get_mut(index) {
            *slot = Some(name);
        }
    }

    /// Set memory effects for the function.
    pub fn set_memory_effects(&mut self, effects: MemoryEffect) {
        // update the function memory effects
        let function = self.tree.get_mut(self.function_id);
        function.memory_effects = Some(effects);
    }

    /// Set behavioral effects for the function.
    pub fn set_call_behavior(&mut self, behavior: CallBehavior) {
        // update the function call behavior
        let function = self.tree.get_mut(self.function_id);
        function.call_behavior = Some(behavior);
    }

    /// Set allocation size metadata for the function.
    pub fn set_alloc_size(&mut self, alloc_size: AllocSize) {
        // update the allocation size metadata
        let function = self.tree.get_mut(self.function_id);
        function.alloc_size = Some(alloc_size);
    }

    /// Set pointer attributes for all parameters.
    pub fn set_parameter_attributes(&mut self, attributes: Vec<PointerAttributes>) {
        // validate the parameter count
        let parameter_count = self.tree.get(self.function_id).parameters.len();
        assert_eq!(
            parameter_count,
            attributes.len(),
            "parameter attribute count does not match parameters"
        );

        // update parameter attributes
        let function = self.tree.get_mut(self.function_id);
        function.parameter_attributes = attributes;
    }

    /// Set pointer attributes for a single parameter.
    pub fn set_parameter_attribute(&mut self, index: usize, attributes: PointerAttributes) {
        // access the parameter attributes
        let function = self.tree.get_mut(self.function_id);
        let parameter_attributes = &mut function.parameter_attributes;

        // update the requested parameter
        let Some(target) = parameter_attributes.get_mut(index) else {
            panic!("parameter index out of range");
        };
        *target = attributes;
    }

    /// Set pointer attributes for the return value.
    pub fn set_return_attributes(&mut self, attributes: PointerAttributes) {
        // update the return attributes
        let function = self.tree.get_mut(self.function_id);
        function.return_attributes = attributes;
    }

    /// Set the execution model for this function.
    pub fn set_execution_model(&mut self, model: ExecutionModel) {
        // update the execution model
        let function = self.tree.get_mut(self.function_id);
        function.execution_model = Some(model);
    }

    /// Set the execution stage for this function.
    pub fn set_execution_stage(&mut self, stage: ExecutionStage) {
        // update the execution stage
        let function = self.tree.get_mut(self.function_id);
        function.execution_stage = Some(stage);
    }

    /// Set the workgroup size for this function.
    pub fn set_workgroup_size(&mut self, size: [u32; 3]) {
        // update the workgroup size
        let function = self.tree.get_mut(self.function_id);
        function.workgroup_size = Some(size);
    }

    /// Set allocation mode for this function.
    pub fn set_allocation_mode(&mut self, allocation: AllocationMode) {
        // update allocation mode
        let function = self.tree.get_mut(self.function_id);
        function.allocation = allocation;
    }

    /// Get the function id being built.
    pub fn function_id(&self) -> LocalNodeId<Function> {
        self.function_id
    }

    /// Set the return lifetime for this function.
    pub fn set_return_lifetime(&mut self, lifetime: Lifetime) {
        let function = self.tree.get_mut(self.function_id);
        function.return_lifetime = lifetime;
    }

    /// Get a reference to the underlying node tree.
    pub fn tree(&self) -> &NodeTree {
        self.tree
    }

    /// Get a mutable reference to the underlying node tree.
    pub fn tree_mut(&mut self) -> &mut NodeTree {
        self.tree
    }

    /// Allocate a new SSA value.
    fn allocate_value(&mut self) -> Value {
        let value = Value::new(self.next_value_id);
        self.next_value_id += 1;
        value
    }

    /// Record the type for a value produced by an instruction.
    fn define_value(&mut self, value: Value, ty: LocalNodeId<Type>) {
        let function = self.tree.get_mut(self.function_id);
        function.set_value_type(value, ty);
    }

    /// Get the type of an existing SSA value.
    fn value_type(&self, value: Value) -> Option<LocalNodeId<Type>> {
        let function = self.tree.get(self.function_id);
        function.value_type(value)
    }

    /// Require the type of an SSA value.
    fn value_type_or_panic(&self, value: Value, context: &str) -> LocalNodeId<Type> {
        self.value_type(value)
            .unwrap_or_else(|| panic!("missing value type for {context}"))
    }

    /// Create a new variable for SSA construction.
    ///
    /// Variables represent mutable bindings from the source language. During SSA
    /// construction, they are mapped to SSA values, with block parameters inserted
    /// at join points as needed.
    pub fn variable(&mut self, ty: LocalNodeId<Type>) -> Variable {
        let variable = Variable::new(self.next_variable_id);
        self.next_variable_id += 1;
        self.variable_types.insert(variable, ty);
        variable
    }

    // block management

    /// Create a new basic block.
    pub fn block(&mut self) -> LocalNodeId<Block> {
        let block = self.tree.insert(Block::new());
        self.blocks.push(block);
        self.predecessors.insert(block, Vec::new());
        block
    }

    /// Switch to inserting instructions into the given block.
    pub fn switch_to_block(&mut self, block: LocalNodeId<Block>) {
        self.current_block = Some(block);
    }

    /// Get the current block.
    ///
    /// # Panics
    ///
    /// Panics if no current block is set.
    pub fn current_block(&self) -> LocalNodeId<Block> {
        self.current_block.expect("no current block set")
    }

    /// Get a function parameter value.
    pub fn function_parameter(&self, index: usize) -> Value {
        let function = self.tree.get(self.function_id);
        function.parameters[index].value
    }

    /// Add a block parameter and return its value.
    pub fn add_block_parameter(
        &mut self,
        block: LocalNodeId<Block>,
        ty: LocalNodeId<Type>,
    ) -> Value {
        let value = self.allocate_value();
        let block_data = self.tree.get_mut(block);
        block_data.parameters.push(TypedValue::new(value, ty));
        self.define_value(value, ty);
        value
    }

    /// Seal a block, indicating all predecessors are now known.
    ///
    /// This triggers resolution of any incomplete φ-functions (block parameters)
    /// that were created when variables were used before the block was sealed.
    pub fn seal_block(&mut self, block: LocalNodeId<Block>) {
        if self.sealed_blocks.contains(&block) {
            return;
        }

        // resolve incomplete phis
        if let Some(incomplete) = self.incomplete_phis.shift_remove(&block) {
            for (variable, phi_value) in incomplete {
                self.add_phi_operands(variable, phi_value, block);
            }
        }

        self.sealed_blocks.insert(block);
    }

    /// Seal all blocks.
    pub fn seal_all_blocks(&mut self) {
        let blocks: Vec<_> = self.blocks.clone();
        for block in blocks {
            self.seal_block(block);
        }
    }

    // ssa construction

    /// Define a variable's value in the current block (Algorithm 1: writeVariable).
    pub fn define_variable(&mut self, variable: Variable, value: Value) {
        let block = self.current_block();
        self.variable_definitions.insert((block, variable), value);
    }

    /// Use a variable, returning its SSA value (Algorithm 1: readVariable).
    ///
    /// This implements the SSA construction algorithm, looking up the value in the
    /// current block or predecessors, potentially creating block parameters.
    pub fn use_variable(&mut self, variable: Variable) -> Value {
        let block = self.current_block();
        self.read_variable(variable, block)
    }

    /// Read a variable's value at a given block (local value numbering + global lookup).
    fn read_variable(&mut self, variable: Variable, block: LocalNodeId<Block>) -> Value {
        // local value numbering: check if defined in this block
        if let Some(&value) = self.variable_definitions.get(&(block, variable)) {
            return value;
        }

        // global value numbering: look in predecessors
        self.read_variable_recursive(variable, block)
    }

    /// Recursively read a variable from predecessors (Algorithm 2: readVariableRecursive).
    fn read_variable_recursive(&mut self, variable: Variable, block: LocalNodeId<Block>) -> Value {
        let value = if !self.sealed_blocks.contains(&block) {
            // block not sealed yet: create an incomplete phi (block parameter)
            // (that will be resolved when the block is sealed)
            let ty = self.variable_types[&variable];
            let phi_value = self.add_block_parameter(block, ty);
            self.incomplete_phis
                .entry(block)
                .or_default()
                .push((variable, phi_value));
            phi_value
        } else {
            let predecessors = self.predecessors.get(&block).cloned().unwrap_or_default();

            if predecessors.is_empty() {
                // no predecessors: compiler bug! (all variables should be defined before use)
                panic!(
                    "use of undefined variable {variable} in block {block:?} with no predecessors \
                     (this indicates a bug in the frontend - all variables must be defined before use)",
                );
            } else if predecessors.len() == 1 {
                // single predecessor: no phi needed, just recurse
                self.read_variable(variable, predecessors[0])
            } else {
                // multiple predecessors: may need a phi (block parameter)
                let ty = self.variable_types[&variable];
                let phi_value = self.add_block_parameter(block, ty);

                // record the definition BEFORE recursing to break cycles
                self.variable_definitions
                    .insert((block, variable), phi_value);

                // add operands from predecessors
                self.add_phi_operands(variable, phi_value, block)
            }
        };

        self.variable_definitions.insert((block, variable), value);
        value
    }

    /// Add operands to a phi (block parameter) from all predecessors.
    ///
    /// Also performs trivial phi removal: if all operands are the same value
    /// (or the phi itself), the phi is unnecessary and we return that value instead.
    fn add_phi_operands(
        &mut self,
        variable: Variable,
        phi_value: Value,
        block: LocalNodeId<Block>,
    ) -> Value {
        let predecessors = self.predecessors.get(&block).cloned().unwrap_or_default();

        // collect values from all predecessors
        let mut operand_values = Vec::with_capacity(predecessors.len());
        for &predecessor in &predecessors {
            let predecessor_value = self.read_variable(variable, predecessor);
            operand_values.push((predecessor, predecessor_value));
        }

        // try to remove trivial phi: if all operands are the same (ignoring the phi itself)
        let trivial_value = self.try_remove_trivial_phi(phi_value, &operand_values);

        if let Some(replacement) = trivial_value {
            // phi is trivial: remove the block parameter and use the single value
            self.remove_block_parameter(block, phi_value);
            self.replace_value(phi_value, replacement);

            // only update the definition if it still points to the phi we're removing
            // (a later define_variable may have overwritten it with a new value)
            if self.variable_definitions.get(&(block, variable)) == Some(&phi_value) {
                self.variable_definitions
                    .insert((block, variable), replacement);
            }
            replacement
        } else {
            // phi is needed: add jump arguments from predecessors
            for (predecessor, predecessor_value) in operand_values {
                self.add_phi_argument(predecessor, block, predecessor_value);
            }
            phi_value
        }
    }

    /// Replace all uses of a value with another value in the current function.
    fn replace_value(&mut self, from: Value, to: Value) {
        // skip no op replacements
        if from == to {
            return;
        }

        // update variable definitions
        for value in self.variable_definitions.values_mut() {
            Self::replace_value_in_slot(value, from, to);
        }

        // update incomplete phis
        for phis in self.incomplete_phis.values_mut() {
            for (_variable, phi_value) in phis {
                Self::replace_value_in_slot(phi_value, from, to);
            }
        }

        // update blocks
        let blocks = self.blocks.clone();
        for block in blocks {
            self.replace_value_in_block(block, from, to);
        }
    }

    /// Replace a value in a block.
    fn replace_value_in_block(&mut self, block: LocalNodeId<Block>, from: Value, to: Value) {
        // update instructions
        let instruction_ids = self.tree.get(block).instructions.clone();
        for instruction_id in instruction_ids {
            self.replace_value_in_instruction(instruction_id, from, to);
        }

        // update parameters and terminator
        let block_data = self.tree.get_mut(block);
        Self::replace_values_in_parameters(&mut block_data.parameters, from, to);
        Self::replace_value_in_terminator(&mut block_data.terminator, from, to);
    }

    /// Replace a value in an instruction.
    fn replace_value_in_instruction(
        &mut self,
        instruction_id: LocalNodeId<Instruction>,
        from: Value,
        to: Value,
    ) {
        // update inline operands
        let argument_slice = {
            let instruction = self.tree.get_mut(instruction_id);
            let argument_slice = instruction.argument_slice();
            match instruction {
                Instruction::Const { .. }
                | Instruction::LocalGet { .. }
                | Instruction::LocalAddr { .. }
                | Instruction::GlobalAddr { .. }
                | Instruction::GlobalConst { .. }
                | Instruction::FunctionAddr { .. }
                | Instruction::FunctionEnv { .. }
                | Instruction::ManagedAlloc { .. }
                | Instruction::RawAlloc { .. }
                | Instruction::StackAlloc { .. } => {}
                Instruction::Binary { left, right, .. } => {
                    Self::replace_value_in_slot(left, from, to);
                    Self::replace_value_in_slot(right, from, to);
                }
                Instruction::Unary { argument, .. }
                | Instruction::Cast { argument, .. }
                | Instruction::VectorSplat {
                    value: argument, ..
                }
                | Instruction::VectorReduce {
                    vector: argument, ..
                }
                | Instruction::VectorConvert {
                    vector: argument, ..
                }
                | Instruction::TensorReshape {
                    tensor: argument, ..
                }
                | Instruction::TensorBroadcast {
                    tensor: argument, ..
                }
                | Instruction::TensorTranspose {
                    tensor: argument, ..
                }
                | Instruction::TensorCast {
                    tensor: argument, ..
                }
                | Instruction::TensorSlice {
                    tensor: argument, ..
                }
                | Instruction::TensorConvert {
                    tensor: argument, ..
                }
                | Instruction::RawFree { pointer: argument }
                | Instruction::RawDrop { value: argument }
                | Instruction::StackDrop { value: argument } => {
                    Self::replace_value_in_slot(argument, from, to);
                }
                Instruction::CallIndirect { callee, env, .. } => {
                    Self::replace_value_in_slot(callee, from, to);
                    if let Some(env) = env {
                        Self::replace_value_in_slot(env, from, to);
                    }
                }
                Instruction::VectorExtract { vector, index, .. } => {
                    Self::replace_value_in_slot(vector, from, to);
                    Self::replace_value_in_slot(index, from, to);
                }
                Instruction::VectorInsert {
                    vector,
                    index,
                    value,
                    ..
                } => {
                    Self::replace_value_in_slot(vector, from, to);
                    Self::replace_value_in_slot(index, from, to);
                    Self::replace_value_in_slot(value, from, to);
                }
                Instruction::VectorShuffle { left, right, .. } => {
                    Self::replace_value_in_slot(left, from, to);
                    Self::replace_value_in_slot(right, from, to);
                }
                Instruction::VectorCompare { left, right, .. } => {
                    Self::replace_value_in_slot(left, from, to);
                    Self::replace_value_in_slot(right, from, to);
                }
                Instruction::TensorLoad { view, .. } => {
                    Self::replace_value_in_slot(view, from, to);
                }
                Instruction::TensorView { view, .. } => {
                    Self::replace_value_in_slot(view, from, to);
                }
                Instruction::TensorStore { view, value, .. } => {
                    Self::replace_value_in_slot(view, from, to);
                    Self::replace_value_in_slot(value, from, to);
                }
                Instruction::TensorFill { view, value } => {
                    Self::replace_value_in_slot(view, from, to);
                    Self::replace_value_in_slot(value, from, to);
                }
                Instruction::TensorCopy { target, source } => {
                    Self::replace_value_in_slot(target, from, to);
                    Self::replace_value_in_slot(source, from, to);
                }
                Instruction::TensorPad { tensor, value, .. } => {
                    Self::replace_value_in_slot(tensor, from, to);
                    Self::replace_value_in_slot(value, from, to);
                }
                Instruction::TensorReduce {
                    tensor, initial, ..
                } => {
                    Self::replace_value_in_slot(tensor, from, to);
                    Self::replace_value_in_slot(initial, from, to);
                }
                Instruction::TensorDot { left, right, .. } => {
                    Self::replace_value_in_slot(left, from, to);
                    Self::replace_value_in_slot(right, from, to);
                }
                Instruction::TensorConvolution { input, kernel, .. } => {
                    Self::replace_value_in_slot(input, from, to);
                    Self::replace_value_in_slot(kernel, from, to);
                }
                Instruction::TensorGather {
                    operand, indices, ..
                } => {
                    Self::replace_value_in_slot(operand, from, to);
                    Self::replace_value_in_slot(indices, from, to);
                }
                Instruction::TensorScatter {
                    operand,
                    indices,
                    updates,
                    ..
                } => {
                    Self::replace_value_in_slot(operand, from, to);
                    Self::replace_value_in_slot(indices, from, to);
                    Self::replace_value_in_slot(updates, from, to);
                }
                Instruction::TensorCompare { left, right, .. } => {
                    Self::replace_value_in_slot(left, from, to);
                    Self::replace_value_in_slot(right, from, to);
                }
                Instruction::CallVirtual { receiver, .. }
                | Instruction::CallInterface { receiver, .. } => {
                    Self::replace_value_in_slot(receiver, from, to);
                }
                Instruction::Select {
                    condition,
                    then_value,
                    else_value,
                    ..
                } => {
                    Self::replace_value_in_slot(condition, from, to);
                    Self::replace_value_in_slot(then_value, from, to);
                    Self::replace_value_in_slot(else_value, from, to);
                }
                Instruction::Load { pointer, .. } => {
                    Self::replace_value_in_slot(pointer, from, to);
                }
                Instruction::LocalSet { value, .. } => {
                    Self::replace_value_in_slot(value, from, to);
                }
                Instruction::Store { pointer, value } => {
                    Self::replace_value_in_slot(pointer, from, to);
                    Self::replace_value_in_slot(value, from, to);
                }
                Instruction::FieldGet { aggregate, .. }
                | Instruction::FieldAddr { aggregate, .. } => {
                    Self::replace_value_in_slot(aggregate, from, to);
                }
                Instruction::FieldSet {
                    aggregate, value, ..
                } => {
                    Self::replace_value_in_slot(aggregate, from, to);
                    Self::replace_value_in_slot(value, from, to);
                }
                Instruction::ElementGet { array, index, .. }
                | Instruction::ElementAddr { array, index, .. } => {
                    Self::replace_value_in_slot(array, from, to);
                    Self::replace_value_in_slot(index, from, to);
                }
                Instruction::ElementSet {
                    array,
                    index,
                    value,
                    ..
                } => {
                    Self::replace_value_in_slot(array, from, to);
                    Self::replace_value_in_slot(index, from, to);
                    Self::replace_value_in_slot(value, from, to);
                }
                Instruction::ManagedAllocArray { length, .. } => {
                    Self::replace_value_in_slot(length, from, to);
                }
                Instruction::Assume { condition } => {
                    Self::replace_value_in_slot(condition, from, to);
                }
                // arguments stored externally
                Instruction::Struct { .. }
                | Instruction::Tuple { .. }
                | Instruction::Array { .. }
                | Instruction::Call { .. }
                | Instruction::TensorConcat { .. }
                | Instruction::Intrinsic { .. } => {}
            }
            argument_slice
        };

        // update external arguments
        if let Some(argument_slice) = argument_slice {
            let start = argument_slice.start as usize;
            let end = start + argument_slice.count as usize;
            Self::replace_values_in_slice(
                &mut self.tree.instruction_arguments[start..end],
                from,
                to,
            );
        }
    }

    /// Replace a value in a terminator.
    fn replace_value_in_terminator(terminator: &mut Terminator, from: Value, to: Value) {
        // update terminator operands
        match terminator {
            Terminator::Return { value } => {
                if let Some(value) = value {
                    Self::replace_value_in_slot(value, from, to);
                }
            }
            Terminator::Jump { arguments, .. } => {
                Self::replace_values_in_slice(arguments, from, to);
            }
            Terminator::Branch {
                condition,
                then_arguments,
                else_arguments,
                ..
            } => {
                Self::replace_value_in_slot(condition, from, to);
                Self::replace_values_in_slice(then_arguments, from, to);
                Self::replace_values_in_slice(else_arguments, from, to);
            }
            Terminator::Check {
                condition,
                constraint,
                success,
                failure,
            } => {
                Self::replace_value_in_slot(condition, from, to);
                Self::replace_values_in_check_kind(constraint, from, to);
                Self::replace_values_in_slice(&mut success.arguments, from, to);
                Self::replace_values_in_slice(&mut failure.arguments, from, to);
            }
            Terminator::Switch {
                value,
                default_arguments,
                cases,
                ..
            } => {
                Self::replace_value_in_slot(value, from, to);
                Self::replace_values_in_slice(default_arguments, from, to);
                for case in cases {
                    Self::replace_values_in_slice(&mut case.arguments, from, to);
                }
            }
            Terminator::Yield {
                value,
                resume_arguments,
                ..
            } => {
                Self::replace_value_in_slot(value, from, to);
                Self::replace_values_in_slice(resume_arguments, from, to);
            }
            Terminator::Unreachable => {}
            Terminator::TailCall { arguments, .. } => {
                Self::replace_values_in_slice(arguments, from, to);
            }
            Terminator::TailCallIndirect {
                callee,
                env,
                arguments,
                ..
            } => {
                Self::replace_value_in_slot(callee, from, to);
                if let Some(env) = env {
                    Self::replace_value_in_slot(env, from, to);
                }
                Self::replace_values_in_slice(arguments, from, to);
            }
            Terminator::TailCallVirtual {
                receiver,
                arguments,
                ..
            }
            | Terminator::TailCallInterface {
                receiver,
                arguments,
                ..
            } => {
                Self::replace_value_in_slot(receiver, from, to);
                Self::replace_values_in_slice(arguments, from, to);
            }
        }
    }

    /// Replace a value in a slot.
    fn replace_value_in_slot(value: &mut Value, from: Value, to: Value) {
        // update matching values
        if *value == from {
            *value = to;
        }
    }

    /// Replace values referenced by a check kind.
    fn replace_values_in_check_kind(kind: &mut CheckConstraint, from: Value, to: Value) {
        // update values stored in the check kind
        match kind {
            CheckConstraint::Bounds {
                index,
                length,
                collection,
                ..
            } => {
                Self::replace_value_in_slot(index, from, to);
                Self::replace_value_in_slot(length, from, to);
                Self::replace_value_in_slot(collection, from, to);
            }
            CheckConstraint::Null { value } => {
                Self::replace_value_in_slot(value, from, to);
            }
            CheckConstraint::DivZero { divisor } => {
                Self::replace_value_in_slot(divisor, from, to);
            }
            CheckConstraint::ShiftRange { value, .. } => {
                Self::replace_value_in_slot(value, from, to);
            }
            CheckConstraint::Narrow { value, .. } => {
                Self::replace_value_in_slot(value, from, to);
            }
            CheckConstraint::Overflow { left, right, .. } => {
                Self::replace_value_in_slot(left, from, to);
                Self::replace_value_in_slot(right, from, to);
            }
            CheckConstraint::Type { value, .. } => {
                Self::replace_value_in_slot(value, from, to);
            }
            CheckConstraint::Union { value, .. } => {
                Self::replace_value_in_slot(value, from, to);
            }
            CheckConstraint::Vtable { receiver, .. } => {
                Self::replace_value_in_slot(receiver, from, to);
            }
            CheckConstraint::Itab { receiver, .. } => {
                Self::replace_value_in_slot(receiver, from, to);
            }
        }
    }

    /// Replace values in a slice.
    fn replace_values_in_slice(values: &mut [Value], from: Value, to: Value) {
        // update each value
        for value in values {
            Self::replace_value_in_slot(value, from, to);
        }
    }

    /// Replace values in a parameter list.
    fn replace_values_in_parameters(parameters: &mut [TypedValue], from: Value, to: Value) {
        // update each parameter value
        for parameter in parameters {
            Self::replace_value_in_slot(&mut parameter.value, from, to);
        }
    }

    /// Try to remove a trivial phi: returns Some(value) if all operands are the same
    /// (excluding references to the phi itself), or None if the phi is necessary.
    fn try_remove_trivial_phi(
        &self,
        phi_value: Value,
        operands: &[(LocalNodeId<Block>, Value)],
    ) -> Option<Value> {
        let mut same: Option<Value> = None;

        for &(_predecessor, operand) in operands {
            // skip the phi itself (self-references in loops)
            if operand == phi_value {
                continue;
            }

            // check if all non-phi operands are the same
            match same {
                None => same = Some(operand),
                Some(existing) if existing != operand => return None, // different values
                Some(_) => {}                                         // same value, continue
            }
        }

        // if same is None, all operands were self-references (shouldn't happen in valid code)
        // if same is Some(v), all operands are v (or the phi itself)
        same
    }

    /// Remove a block parameter (phi) by value.
    fn remove_block_parameter(&mut self, block: LocalNodeId<Block>, value: Value) {
        let block_data = self.tree.get_mut(block);
        if let Some(position) = block_data
            .parameters
            .iter()
            .position(|param| param.value == value)
        {
            block_data.parameters.remove(position);
        }
    }

    /// Add a value as an argument to jumps from `from_block` to `to_block`.
    fn add_phi_argument(
        &mut self,
        from_block: LocalNodeId<Block>,
        to_block: LocalNodeId<Block>,
        value: Value,
    ) {
        let block_data = self.tree.get_mut(from_block);
        match &mut block_data.terminator {
            Terminator::Jump { target, arguments } if *target == to_block => {
                arguments.push(value);
            }
            Terminator::Branch {
                then_target,
                then_arguments,
                else_target,
                else_arguments,
                ..
            } => {
                if *then_target == to_block {
                    then_arguments.push(value);
                }
                if *else_target == to_block {
                    else_arguments.push(value);
                }
            }
            Terminator::Check {
                success, failure, ..
            } => {
                if success.target == to_block {
                    success.arguments.push(value);
                }
                if failure.target == to_block {
                    failure.arguments.push(value);
                }
            }
            Terminator::Switch {
                default,
                default_arguments,
                cases,
                ..
            } => {
                if *default == to_block {
                    default_arguments.push(value);
                }
                for case in cases {
                    if case.target == to_block {
                        case.arguments.push(value);
                    }
                }
            }
            _ => {
                // terminator doesn't jump to this block
            }
        }
    }

    /// Record that `from_block` is a predecessor of `to_block`.
    fn add_predecessor(&mut self, from_block: LocalNodeId<Block>, to_block: LocalNodeId<Block>) {
        self.predecessors
            .entry(to_block)
            .or_default()
            .push(from_block);
    }

    // instruction builders: constants

    /// Insert an integer constant.
    pub fn iconst(&mut self, value: i64, width: u8, signed: bool) -> Value {
        let destination = self.allocate_value();
        let constant = if signed {
            Constant::Int {
                value,
                width,
                is_signed: true,
            }
        } else {
            Constant::UInt {
                value: value as u64,
                width,
            }
        };
        let ty = Type::Int {
            width: width.into(),
            is_signed: signed,
        };
        let ty_id = self.tree.insert_type(ty);
        self.insert_instruction(Instruction::Const {
            destination,
            value: constant,
        });
        self.define_value(destination, ty_id);
        destination
    }

    /// Insert a 32-bit signed integer constant.
    pub fn iconst_i32(&mut self, value: i32) -> Value {
        self.iconst(value as i64, 32, true)
    }

    /// Insert a 64-bit signed integer constant.
    pub fn iconst_i64(&mut self, value: i64) -> Value {
        self.iconst(value, 64, true)
    }

    /// Insert a boolean constant.
    pub fn bconst(&mut self, value: bool) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::Const {
            destination,
            value: Constant::Boolean { value },
        });
        let ty_id = self.tree.insert_type(Type::Boolean);
        self.define_value(destination, ty_id);
        destination
    }

    /// Insert a floating point constant.
    pub fn fconst(&mut self, value: f64, width: u8) -> Value {
        let destination = self.allocate_value();
        let bits = if width == 32 {
            f32::to_bits(value as f32) as u64
        } else {
            value.to_bits()
        };
        self.insert_instruction(Instruction::Const {
            destination,
            value: Constant::Float { bits, width },
        });
        let ty_id = self.tree.insert_type(Type::Float {
            width: width.into(),
        });
        self.define_value(destination, ty_id);
        destination
    }

    // instruction builders: binary operations

    /// Insert a binary operation.
    fn binary(&mut self, operator: BinaryOperator, left_value: Value, right_value: Value) -> Value {
        let destination = self.allocate_value();
        let left_type_id = self.value_type_or_panic(left_value, "binary left");
        let right_type_id = self.value_type_or_panic(right_value, "binary right");
        let left_type = self.tree.get(left_type_id);
        let right_type = self.tree.get(right_type_id);
        if left_type != right_type {
            panic!("binary operator expects matching operand types");
        }
        self.insert_instruction(Instruction::Binary {
            destination,
            operator,
            left: left_value,
            right: right_value,
        });
        if operator.is_comparison() {
            let bool_type = self.tree.insert_type(Type::Boolean);
            self.define_value(destination, bool_type);
        } else {
            self.define_value(destination, left_type_id);
        }
        destination
    }

    /// Insert a binary operation with an explicit operator.
    pub fn binary_op(
        &mut self,
        operator: BinaryOperator,
        left_value: Value,
        right_value: Value,
    ) -> Value {
        self.binary(operator, left_value, right_value)
    }

    /// Integer addition.
    pub fn iadd(&mut self, left_value: Value, right_value: Value) -> Value {
        self.binary(BinaryOperator::Add, left_value, right_value)
    }

    /// Integer subtraction.
    pub fn isub(&mut self, left_value: Value, right_value: Value) -> Value {
        self.binary(BinaryOperator::Subtract, left_value, right_value)
    }

    /// Integer multiplication.
    pub fn imul(&mut self, left_value: Value, right_value: Value) -> Value {
        self.binary(BinaryOperator::Multiply, left_value, right_value)
    }

    /// Signed integer division.
    pub fn sdiv(&mut self, left_value: Value, right_value: Value) -> Value {
        self.binary(BinaryOperator::SignedDivide, left_value, right_value)
    }

    /// Unsigned integer division.
    pub fn udiv(&mut self, left_value: Value, right_value: Value) -> Value {
        self.binary(BinaryOperator::UnsignedDivide, left_value, right_value)
    }

    /// Bitwise AND.
    pub fn band(&mut self, left_value: Value, right_value: Value) -> Value {
        self.binary(BinaryOperator::And, left_value, right_value)
    }

    /// Bitwise OR.
    pub fn bor(&mut self, left_value: Value, right_value: Value) -> Value {
        self.binary(BinaryOperator::Or, left_value, right_value)
    }

    /// Bitwise XOR.
    pub fn bxor(&mut self, left_value: Value, right_value: Value) -> Value {
        self.binary(BinaryOperator::Xor, left_value, right_value)
    }

    /// Integer comparison: equal.
    pub fn icmp_eq(&mut self, left_value: Value, right_value: Value) -> Value {
        self.binary(BinaryOperator::Equal, left_value, right_value)
    }

    /// Integer comparison: not equal.
    pub fn icmp_ne(&mut self, left_value: Value, right_value: Value) -> Value {
        self.binary(BinaryOperator::NotEqual, left_value, right_value)
    }

    /// Signed integer comparison: less than.
    pub fn icmp_slt(&mut self, left_value: Value, right_value: Value) -> Value {
        self.binary(BinaryOperator::SignedLessThan, left_value, right_value)
    }

    /// Signed integer comparison: less than or equal.
    pub fn icmp_sle(&mut self, left_value: Value, right_value: Value) -> Value {
        self.binary(BinaryOperator::SignedLessEqual, left_value, right_value)
    }

    /// Signed integer comparison: greater than.
    pub fn icmp_sgt(&mut self, left_value: Value, right_value: Value) -> Value {
        self.binary(BinaryOperator::SignedGreaterThan, left_value, right_value)
    }

    /// Signed integer comparison: greater than or equal.
    pub fn icmp_sge(&mut self, left_value: Value, right_value: Value) -> Value {
        self.binary(BinaryOperator::SignedGreaterEqual, left_value, right_value)
    }

    // instruction builders: unary operations

    /// Insert a unary operation with an explicit operator.
    pub fn unary_op(&mut self, operator: UnaryOperator, argument_value: Value) -> Value {
        self.unary(operator, argument_value)
    }

    /// Insert a unary operation.
    fn unary(&mut self, operator: UnaryOperator, argument_value: Value) -> Value {
        let destination = self.allocate_value();
        let argument_type = self.value_type_or_panic(argument_value, "unary argument");
        self.insert_instruction(Instruction::Unary {
            destination,
            operator,
            argument: argument_value,
        });
        self.define_value(destination, argument_type);
        destination
    }

    /// Integer negation.
    pub fn ineg(&mut self, argument_value: Value) -> Value {
        self.unary(UnaryOperator::Negate, argument_value)
    }

    /// Bitwise NOT.
    pub fn bnot(&mut self, argument_value: Value) -> Value {
        self.unary(UnaryOperator::Not, argument_value)
    }

    // instruction builders: memory

    /// Create a local variable (stack slot).
    pub fn local(
        &mut self,
        ty: LocalNodeId<Type>,
        mutability: Mutability,
    ) -> LocalNodeId<Local> {
        let local = self
            .tree
            .insert(Local::new(ty, mutability, Ownership::Owned));
        let function = self.tree.get_mut(self.function_id);
        function.locals.push(local);
        local
    }

    /// Create a reference type for inline instruction typing.
    pub fn type_reference(
        &mut self,
        kind: ReferenceKind,
        pointee: LocalNodeId<Type>,
        mutability: Mutability,
        address_space: AddressSpace,
        is_nullable: bool,
    ) -> LocalNodeId<Type> {
        self.tree.insert_type(Type::Reference {
            kind,
            address_space,
            mutability,
            pointee,
            is_nullable,
        })
    }

    /// Load from a local variable.
    pub fn local_get(&mut self, local: LocalNodeId<Local>) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::LocalGet { destination, local });
        let local_ty = self.tree.get(local).ty;
        self.define_value(destination, local_ty);
        destination
    }

    /// Get the address of a local variable.
    pub fn local_addr(
        &mut self,
        local: LocalNodeId<Local>,
        result_type: LocalNodeId<Type>,
    ) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::LocalAddr {
            destination,
            local,
            result_type,
        });
        self.define_value(destination, result_type);
        destination
    }

    /// Store to a local variable.
    pub fn local_set(&mut self, local: LocalNodeId<Local>, value: Value) {
        self.insert_instruction(Instruction::LocalSet { local, value });
    }

    /// Get the address of a mutable global variable.
    pub fn global_addr(
        &mut self,
        global: LocalNodeId<Global>,
        result_type: LocalNodeId<Type>,
    ) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::GlobalAddr {
            destination,
            global,
            result_type,
        });
        self.define_value(destination, result_type);
        destination
    }

    /// Load the value of an immutable global constant.
    pub fn global_const(&mut self, global: LocalNodeId<Global>) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::GlobalConst {
            destination,
            global,
        });
        let global_ty = self.tree.get(global).ty;
        self.define_value(destination, global_ty);
        destination
    }

    /// Load from a pointer.
    pub fn load(&mut self, pointer_value: Value, result_type: LocalNodeId<Type>) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::Load {
            destination,
            pointer: pointer_value,
            result_type,
        });
        self.define_value(destination, result_type);
        destination
    }

    /// Store to a pointer.
    pub fn store(&mut self, pointer_value: Value, value: Value) {
        self.insert_instruction(Instruction::Store {
            pointer: pointer_value,
            value,
        });
    }

    /// Resolve a field type for a struct or tuple aggregate.
    fn field_type_for_aggregate(
        &self,
        aggregate_type: LocalNodeId<Type>,
        index: u32,
    ) -> LocalNodeId<Type> {
        let aggregate = self.tree.get(aggregate_type);
        match aggregate {
            Type::Struct { fields, .. } => fields
                .get(index as usize)
                .map(|field_id| self.tree.get(*field_id).ty)
                .unwrap_or_else(|| panic!("field index out of bounds")),
            Type::Tuple { elements, .. } => elements
                .get(index as usize)
                .copied()
                .unwrap_or_else(|| panic!("field index out of bounds")),
            _ => panic!("field access expects struct or tuple"),
        }
    }

    /// Resolve the element type for an array aggregate.
    fn element_type_for_array(&self, array_type: LocalNodeId<Type>) -> LocalNodeId<Type> {
        let array = self.tree.get(array_type);
        match array {
            Type::Array { element, .. } => *element,
            _ => panic!("element access expects array type"),
        }
    }

    /// Resolve the element type for a vector type.
    fn element_type_for_vector(&self, vector_type: LocalNodeId<Type>) -> LocalNodeId<Type> {
        let vector = self.tree.get(vector_type);
        match vector {
            Type::Vector { element, .. } => *element,
            _ => panic!("vector access expects vector type"),
        }
    }

    /// Resolve the element type for a tensor reference.
    fn element_type_for_tensor_reference(
        &self,
        reference_type: LocalNodeId<Type>,
    ) -> LocalNodeId<Type> {
        let reference_type = self.tree.get(reference_type);
        match reference_type {
            Type::TensorReference { element, .. } => *element,
            _ => panic!("tensor access expects tensor reference type"),
        }
    }

    /// Convert a list length into u16 for instruction metadata.
    fn to_u16_count(&self, count: usize, context: &str) -> u16 {
        u16::try_from(count).unwrap_or_else(|_| panic!("{context} is too large"))
    }

    /// Resolve the return type for a function signature.
    fn signature_result_type(&self, signature: LocalNodeId<Type>) -> LocalNodeId<Type> {
        let signature_type = self.tree.get(signature);
        match signature_type {
            Type::FunctionPointer { result, .. } => *result,
            _ => panic!("call expects function pointer signature"),
        }
    }

    // instruction builders: aggregates

    /// Extract a field from a struct or tuple.
    pub fn field_get(&mut self, aggregate: Value, index: u32) -> Value {
        let destination = self.allocate_value();
        let aggregate_type = self.value_type_or_panic(aggregate, "field.get aggregate");
        let field_type = self.field_type_for_aggregate(aggregate_type, index);
        self.insert_instruction(Instruction::FieldGet {
            destination,
            aggregate,
            index,
        });
        self.define_value(destination, field_type);
        destination
    }

    /// Get the address of a field from a struct or tuple.
    pub fn field_addr(
        &mut self,
        aggregate: Value,
        index: u32,
        result_type: LocalNodeId<Type>,
    ) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::FieldAddr {
            destination,
            aggregate,
            index,
            result_type,
        });
        self.define_value(destination, result_type);
        destination
    }

    /// Insert a value into a struct or tuple field.
    pub fn field_set(&mut self, aggregate: Value, index: u32, value: Value) -> Value {
        let destination = self.allocate_value();
        let aggregate_type = self.value_type_or_panic(aggregate, "field.set aggregate");
        self.insert_instruction(Instruction::FieldSet {
            destination,
            aggregate,
            index,
            value,
        });
        self.define_value(destination, aggregate_type);
        destination
    }

    /// Extract an element from an array.
    pub fn element_get(&mut self, array: Value, index: Value) -> Value {
        let destination = self.allocate_value();
        let array_type = self.value_type_or_panic(array, "element.get array");
        let element_type = self.element_type_for_array(array_type);
        self.insert_instruction(Instruction::ElementGet {
            destination,
            array,
            index,
        });
        self.define_value(destination, element_type);
        destination
    }

    /// Get the address of an element from an array.
    pub fn element_addr(
        &mut self,
        array: Value,
        index: Value,
        result_type: LocalNodeId<Type>,
    ) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::ElementAddr {
            destination,
            array,
            index,
            result_type,
        });
        self.define_value(destination, result_type);
        destination
    }

    /// Insert a value into an array element.
    pub fn element_set(&mut self, array: Value, index: Value, value: Value) -> Value {
        let destination = self.allocate_value();
        let array_type = self.value_type_or_panic(array, "element.set array");
        self.insert_instruction(Instruction::ElementSet {
            destination,
            array,
            index,
            value,
        });
        self.define_value(destination, array_type);
        destination
    }

    /// Construct a struct from field values.
    ///
    /// Fields must be provided in layout order.
    pub fn struct_(&mut self, ty: LocalNodeId<Type>, field_values: Vec<Value>) -> Value {
        let destination = self.allocate_value();
        let fields = self.tree.add_arguments(&field_values);
        self.insert_instruction(Instruction::Struct {
            destination,
            ty,
            fields,
        });
        self.define_value(destination, ty);
        destination
    }

    /// Construct a tuple from element values.
    ///
    /// Elements must be provided in order.
    pub fn tuple(&mut self, ty: LocalNodeId<Type>, element_values: Vec<Value>) -> Value {
        let destination = self.allocate_value();
        let elements = self.tree.add_arguments(&element_values);
        self.insert_instruction(Instruction::Tuple {
            destination,
            ty,
            elements,
        });
        self.define_value(destination, ty);
        destination
    }

    /// Construct an array from element values.
    ///
    /// Elements must be provided in index order.
    pub fn array(&mut self, ty: LocalNodeId<Type>, element_values: Vec<Value>) -> Value {
        let destination = self.allocate_value();
        let elements = self.tree.add_arguments(&element_values);
        self.insert_instruction(Instruction::Array {
            destination,
            ty,
            elements,
        });
        self.define_value(destination, ty);
        destination
    }

    // instruction builders: vector operations

    /// Broadcast a scalar to all vector lanes.
    pub fn vector_splat(&mut self, vector_type: LocalNodeId<Type>, value: Value) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::VectorSplat { destination, value });
        self.define_value(destination, vector_type);
        destination
    }

    /// Extract a lane from a vector.
    pub fn vector_extract(&mut self, vector: Value, index: Value) -> Value {
        let destination = self.allocate_value();
        let vector_type = self.value_type_or_panic(vector, "vector.extract vector");
        let element_type = self.element_type_for_vector(vector_type);
        self.insert_instruction(Instruction::VectorExtract {
            destination,
            vector,
            index,
        });
        self.define_value(destination, element_type);
        destination
    }

    /// Insert a lane into a vector.
    pub fn vector_insert(&mut self, vector: Value, index: Value, value: Value) -> Value {
        let destination = self.allocate_value();
        let vector_type = self.value_type_or_panic(vector, "vector.insert vector");
        self.insert_instruction(Instruction::VectorInsert {
            destination,
            vector,
            index,
            value,
        });
        self.define_value(destination, vector_type);
        destination
    }

    /// Shuffle vector lanes with a constant mask.
    pub fn vector_shuffle(
        &mut self,
        vector_type: LocalNodeId<Type>,
        left: Value,
        right: Value,
        mask: Vec<u32>,
    ) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::VectorShuffle {
            destination,
            left,
            right,
            mask,
        });
        self.define_value(destination, vector_type);
        destination
    }

    /// Reduce a vector to a scalar.
    pub fn vector_reduce(&mut self, operator: VectorReduceOperator, vector: Value) -> Value {
        let destination = self.allocate_value();
        let vector_type = self.value_type_or_panic(vector, "vector.reduce vector");
        let element_type = self.element_type_for_vector(vector_type);
        self.insert_instruction(Instruction::VectorReduce {
            destination,
            operator,
            vector,
        });
        self.define_value(destination, element_type);
        destination
    }

    /// Compare two vectors elementwise.
    pub fn vector_compare(
        &mut self,
        result_type: LocalNodeId<Type>,
        operator: BinaryOperator,
        left: Value,
        right: Value,
    ) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::VectorCompare {
            destination,
            operator,
            left,
            right,
        });
        self.define_value(destination, result_type);
        destination
    }

    /// Convert vector element types with an explicit mode.
    pub fn vector_convert(
        &mut self,
        result_type: LocalNodeId<Type>,
        mode: VectorConvertMode,
        vector: Value,
    ) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::VectorConvert {
            destination,
            mode,
            vector,
        });
        self.define_value(destination, result_type);
        destination
    }

    // instruction builders: tensor operations

    /// Load a tensor element from a tensor reference.
    pub fn tensor_load(&mut self, view: Value, indices: Vec<Value>) -> Value {
        let destination = self.allocate_value();
        let view_type = self.value_type_or_panic(view, "tensor.load view");
        let element_type = self.element_type_for_tensor_reference(view_type);
        let indices = self.tree.add_arguments(&indices);
        self.insert_instruction(Instruction::TensorLoad {
            destination,
            view,
            indices,
        });
        self.define_value(destination, element_type);
        destination
    }

    /// Store a tensor element into a tensor reference.
    pub fn tensor_store(&mut self, view: Value, indices: Vec<Value>, value: Value) {
        let indices = self.tree.add_arguments(&indices);
        self.insert_instruction(Instruction::TensorStore {
            view,
            indices,
            value,
        });
    }

    /// Fill a tensor reference with a scalar value.
    pub fn tensor_fill(&mut self, view: Value, value: Value) {
        self.insert_instruction(Instruction::TensorFill { view, value });
    }

    /// Copy elements from a source tensor reference into a destination tensor reference.
    pub fn tensor_copy(&mut self, target: Value, source: Value) {
        self.insert_instruction(Instruction::TensorCopy { target, source });
    }

    /// Reshape a tensor value into a new shape.
    pub fn tensor_reshape(
        &mut self,
        result_type: LocalNodeId<Type>,
        tensor: Value,
        shape_values: Vec<Value>,
    ) -> Value {
        let destination = self.allocate_value();
        let shape = self.tree.add_arguments(&shape_values);
        self.insert_instruction(Instruction::TensorReshape {
            destination,
            tensor,
            shape,
        });
        self.define_value(destination, result_type);
        destination
    }

    /// Broadcast a tensor into a larger shape.
    pub fn tensor_broadcast(
        &mut self,
        result_type: LocalNodeId<Type>,
        tensor: Value,
        dimensions: Vec<u32>,
    ) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::TensorBroadcast {
            destination,
            tensor,
            dimensions,
        });
        self.define_value(destination, result_type);
        destination
    }

    /// Permute tensor dimensions.
    pub fn tensor_transpose(
        &mut self,
        result_type: LocalNodeId<Type>,
        tensor: Value,
        permutation: Vec<u32>,
    ) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::TensorTranspose {
            destination,
            tensor,
            permutation,
        });
        self.define_value(destination, result_type);
        destination
    }

    /// Refine a tensor type without changing its contents.
    pub fn tensor_cast(&mut self, result_type: LocalNodeId<Type>, tensor: Value) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::TensorCast {
            destination,
            tensor,
        });
        self.define_value(destination, result_type);
        destination
    }

    /// Create a view into a tensor reference.
    pub fn tensor_view(
        &mut self,
        result_type: LocalNodeId<Type>,
        view: Value,
        offsets: Vec<Value>,
        sizes: Vec<Value>,
        strides: Vec<Value>,
    ) -> Value {
        let destination = self.allocate_value();
        let offsets_count = self.to_u16_count(offsets.len(), "offsets count");
        let sizes_count = self.to_u16_count(sizes.len(), "sizes count");
        let strides_count = self.to_u16_count(strides.len(), "strides count");
        let mut values = Vec::with_capacity(offsets.len() + sizes.len() + strides.len());
        values.extend_from_slice(&offsets);
        values.extend_from_slice(&sizes);
        values.extend_from_slice(&strides);
        let arguments = self.tree.add_arguments(&values);
        self.insert_instruction(Instruction::TensorView {
            destination,
            view,
            arguments,
            offsets_count,
            sizes_count,
            strides_count,
        });
        self.define_value(destination, result_type);
        destination
    }

    /// Slice a tensor by offsets, sizes, and strides.
    pub fn tensor_slice(
        &mut self,
        result_type: LocalNodeId<Type>,
        tensor: Value,
        offsets: Vec<Value>,
        sizes: Vec<Value>,
        strides: Vec<Value>,
    ) -> Value {
        let destination = self.allocate_value();
        let offsets_count = self.to_u16_count(offsets.len(), "offsets count");
        let sizes_count = self.to_u16_count(sizes.len(), "sizes count");
        let strides_count = self.to_u16_count(strides.len(), "strides count");
        let mut values = Vec::with_capacity(offsets.len() + sizes.len() + strides.len());
        values.extend_from_slice(&offsets);
        values.extend_from_slice(&sizes);
        values.extend_from_slice(&strides);
        let arguments = self.tree.add_arguments(&values);
        self.insert_instruction(Instruction::TensorSlice {
            destination,
            tensor,
            arguments,
            offsets_count,
            sizes_count,
            strides_count,
        });
        self.define_value(destination, result_type);
        destination
    }

    /// Pad a tensor with low, high, and interior padding.
    pub fn tensor_pad(
        &mut self,
        result_type: LocalNodeId<Type>,
        tensor: Value,
        value: Value,
        low: Vec<Value>,
        high: Vec<Value>,
        interior: Vec<Value>,
    ) -> Value {
        let destination = self.allocate_value();
        let low_count = self.to_u16_count(low.len(), "low padding count");
        let high_count = self.to_u16_count(high.len(), "high padding count");
        let interior_count = self.to_u16_count(interior.len(), "interior padding count");
        let mut values = Vec::with_capacity(low.len() + high.len() + interior.len());
        values.extend_from_slice(&low);
        values.extend_from_slice(&high);
        values.extend_from_slice(&interior);
        let arguments = self.tree.add_arguments(&values);
        self.insert_instruction(Instruction::TensorPad {
            destination,
            tensor,
            arguments,
            low_count,
            high_count,
            interior_count,
            value,
        });
        self.define_value(destination, result_type);
        destination
    }

    /// Concatenate tensors along a dimension.
    pub fn tensor_concat(
        &mut self,
        result_type: LocalNodeId<Type>,
        tensors: Vec<Value>,
        axis: u32,
    ) -> Value {
        let destination = self.allocate_value();
        let tensors = self.tree.add_arguments(&tensors);
        self.insert_instruction(Instruction::TensorConcat {
            destination,
            tensors,
            axis,
        });
        self.define_value(destination, result_type);
        destination
    }

    /// Compare two tensors elementwise.
    pub fn tensor_compare(
        &mut self,
        result_type: LocalNodeId<Type>,
        operator: BinaryOperator,
        left: Value,
        right: Value,
    ) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::TensorCompare {
            destination,
            operator,
            left,
            right,
        });
        self.define_value(destination, result_type);
        destination
    }

    /// Reduce a tensor along axes with a fixed operator.
    pub fn tensor_reduce(
        &mut self,
        result_type: LocalNodeId<Type>,
        operator: TensorReduceOperator,
        tensor: Value,
        initial: Value,
        axes: Vec<u32>,
    ) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::TensorReduce {
            destination,
            operator,
            tensor,
            initial,
            axes,
        });
        self.define_value(destination, result_type);
        destination
    }

    /// Compute a tensor dot product.
    pub fn tensor_dot(
        &mut self,
        result_type: LocalNodeId<Type>,
        left: Value,
        right: Value,
        dimensions: TensorDotDimensionNumbers,
    ) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::TensorDot {
            destination,
            left,
            right,
            dimensions,
        });
        self.define_value(destination, result_type);
        destination
    }

    /// Perform a tensor convolution.
    pub fn tensor_convolution(
        &mut self,
        result_type: LocalNodeId<Type>,
        input: Value,
        kernel: Value,
        dimensions: TensorConvolutionDimensionNumbers,
        window: TensorConvolutionWindow,
        feature_group_count: u32,
        batch_group_count: u32,
    ) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::TensorConvolution {
            destination,
            input,
            kernel,
            dimensions,
            window,
            feature_group_count,
            batch_group_count,
        });
        self.define_value(destination, result_type);
        destination
    }

    /// Gather slices from a tensor based on indices.
    pub fn tensor_gather(
        &mut self,
        result_type: LocalNodeId<Type>,
        operand: Value,
        indices: Value,
        dimensions: TensorGatherDimensionNumbers,
        slice_sizes: Vec<u32>,
    ) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::TensorGather {
            destination,
            operand,
            indices,
            dimensions,
            slice_sizes,
        });
        self.define_value(destination, result_type);
        destination
    }

    /// Scatter updates into a tensor based on indices.
    pub fn tensor_scatter(
        &mut self,
        result_type: LocalNodeId<Type>,
        operand: Value,
        indices: Value,
        updates: Value,
        dimensions: TensorScatterDimensionNumbers,
        mode: TensorScatterMode,
    ) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::TensorScatter {
            destination,
            operand,
            indices,
            updates,
            dimensions,
            mode,
        });
        self.define_value(destination, result_type);
        destination
    }

    /// Convert a tensor element type.
    pub fn tensor_convert(
        &mut self,
        result_type: LocalNodeId<Type>,
        mode: TensorConvertMode,
        tensor: Value,
    ) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::TensorConvert {
            destination,
            mode,
            tensor,
        });
        self.define_value(destination, result_type);
        destination
    }

    // instruction builders: allocation

    /// Allocate a managed (runtime-tracked) struct.
    /// Returns a `ref<managed T>`.
    pub fn managed_alloc(
        &mut self,
        layout: LocalNodeId<Type>,
        result_type: LocalNodeId<Type>,
    ) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::ManagedAlloc {
            destination,
            layout,
            result_type,
        });
        self.define_value(destination, result_type);
        destination
    }

    /// Allocate a managed array.
    /// Returns a `ref<managed [T]>`.
    pub fn managed_alloc_array(
        &mut self,
        element: LocalNodeId<Type>,
        length: Value,
        result_type: LocalNodeId<Type>,
    ) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::ManagedAllocArray {
            destination,
            element,
            length,
            result_type,
        });
        self.define_value(destination, result_type);
        destination
    }

    /// Allocate raw memory on the heap.
    /// Returns a `ref<raw T>`. Caller must free with `raw.free`.
    pub fn raw_alloc(
        &mut self,
        layout: LocalNodeId<Type>,
        result_type: LocalNodeId<Type>,
    ) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::RawAlloc {
            destination,
            layout,
            result_type,
        });
        self.define_value(destination, result_type);
        destination
    }

    /// Free raw heap memory previously allocated with `raw.alloc`.
    pub fn raw_free(&mut self, pointer: Value) {
        self.insert_instruction(Instruction::RawFree { pointer });
    }

    /// Allocate on the stack (lives until function returns).
    /// Returns a `ref<raw T>`.
    pub fn stack_alloc(
        &mut self,
        layout: LocalNodeId<Type>,
        result_type: LocalNodeId<Type>,
    ) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::StackAlloc {
            destination,
            layout,
            result_type,
        });
        self.define_value(destination, result_type);
        destination
    }

    // instruction builders: assumptions

    /// Assume a condition is true (UB if false).
    pub fn assume(&mut self, condition: Value) {
        self.insert_instruction(Instruction::Assume { condition });
    }

    // instruction builders: function calls

    /// Call a function.
    pub fn call(
        &mut self,
        function: LocalNodeId<Function>,
        signature: LocalNodeId<Type>,
        argument_values: Vec<Value>,
    ) -> Option<Value> {
        let destination = self.allocate_value();
        let result_type = self.signature_result_type(signature);
        let arguments = self.tree.add_arguments(&argument_values);
        self.insert_instruction(Instruction::Call {
            destination: Some(destination),
            function,
            arguments,
            signature,
            effects: None,
        });
        self.define_value(destination, result_type);
        Some(destination)
    }

    /// Call a function with no return value.
    pub fn call_void(
        &mut self,
        function: LocalNodeId<Function>,
        signature: LocalNodeId<Type>,
        argument_values: Vec<Value>,
    ) {
        let arguments = self.tree.add_arguments(&argument_values);
        self.insert_instruction(Instruction::Call {
            destination: None,
            function,
            arguments,
            signature,
            effects: None,
        });
    }

    /// Call a virtual method through a vtable slot.
    pub fn call_virtual(
        &mut self,
        receiver: Value,
        declaring_type: LocalNodeId<Type>,
        slot_id: u32,
        declared_target: Option<LocalNodeId<Function>>,
        signature: LocalNodeId<Type>,
        argument_values: Vec<Value>,
    ) -> Option<Value> {
        let destination = self.allocate_value();
        let result_type = self.signature_result_type(signature);
        let arguments = self.tree.add_arguments(&argument_values);
        self.insert_instruction(Instruction::CallVirtual {
            destination: Some(destination),
            receiver,
            arguments,
            declaring_type,
            slot_id,
            declared_target,
            signature,
            effects: None,
        });
        self.define_value(destination, result_type);
        Some(destination)
    }

    /// Call a virtual method with no return value.
    pub fn call_virtual_void(
        &mut self,
        receiver: Value,
        declaring_type: LocalNodeId<Type>,
        slot_id: u32,
        declared_target: Option<LocalNodeId<Function>>,
        signature: LocalNodeId<Type>,
        argument_values: Vec<Value>,
    ) {
        let arguments = self.tree.add_arguments(&argument_values);
        self.insert_instruction(Instruction::CallVirtual {
            destination: None,
            receiver,
            arguments,
            declaring_type,
            slot_id,
            declared_target,
            signature,
            effects: None,
        });
    }

    /// Call an interface method through an itab slot.
    pub fn call_interface(
        &mut self,
        receiver: Value,
        declaring_type: LocalNodeId<Type>,
        slot_id: u32,
        declared_target: Option<LocalNodeId<Function>>,
        signature: LocalNodeId<Type>,
        argument_values: Vec<Value>,
    ) -> Option<Value> {
        let destination = self.allocate_value();
        let result_type = self.signature_result_type(signature);
        let arguments = self.tree.add_arguments(&argument_values);
        self.insert_instruction(Instruction::CallInterface {
            destination: Some(destination),
            receiver,
            arguments,
            declaring_type,
            slot_id,
            declared_target,
            signature,
            effects: None,
        });
        self.define_value(destination, result_type);
        Some(destination)
    }

    /// Call an interface method with no return value.
    pub fn call_interface_void(
        &mut self,
        receiver: Value,
        declaring_type: LocalNodeId<Type>,
        slot_id: u32,
        declared_target: Option<LocalNodeId<Function>>,
        signature: LocalNodeId<Type>,
        argument_values: Vec<Value>,
    ) {
        let arguments = self.tree.add_arguments(&argument_values);
        self.insert_instruction(Instruction::CallInterface {
            destination: None,
            receiver,
            arguments,
            declaring_type,
            slot_id,
            declared_target,
            signature,
            effects: None,
        });
    }

    /// Call a function with explicit effects metadata.
    pub fn call_with_effects(
        &mut self,
        function: LocalNodeId<Function>,
        signature: LocalNodeId<Type>,
        argument_values: Vec<Value>,
        effects: CallEffects,
    ) -> Option<Value> {
        let destination = self.allocate_value();
        let result_type = self.signature_result_type(signature);
        let arguments = self.tree.add_arguments(&argument_values);
        self.insert_instruction(Instruction::Call {
            destination: Some(destination),
            function,
            arguments,
            signature,
            effects: Some(effects),
        });
        self.define_value(destination, result_type);
        Some(destination)
    }

    /// Call a function with no return value and explicit effects metadata.
    pub fn call_void_with_effects(
        &mut self,
        function: LocalNodeId<Function>,
        signature: LocalNodeId<Type>,
        argument_values: Vec<Value>,
        effects: CallEffects,
    ) {
        let arguments = self.tree.add_arguments(&argument_values);
        self.insert_instruction(Instruction::Call {
            destination: None,
            function,
            arguments,
            signature,
            effects: Some(effects),
        });
    }

    /// Load a function pointer value for a function.
    pub fn function_addr(
        &mut self,
        function: LocalNodeId<Function>,
        signature: LocalNodeId<Type>,
    ) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::FunctionAddr {
            destination,
            function,
        });
        self.define_value(destination, signature);
        destination
    }

    /// Load the closure environment pointer for the current function.
    pub fn function_env(&mut self, env_type: LocalNodeId<Type>) -> Value {
        // record the closure env type on the function metadata
        {
            let function = self.tree.get_mut(self.function_id);
            match function.closure_env_type {
                Some(existing) if existing != env_type => {
                    panic!("mismatched closure env types for function.env");
                }
                Some(_) => {}
                None => {
                    function.closure_env_type = Some(env_type);
                }
            }
        }

        let destination = self.allocate_value();
        self.insert_instruction(Instruction::FunctionEnv { destination });
        self.define_value(destination, env_type);
        destination
    }

    /// Call through a function pointer with an explicit signature type.
    pub fn call_indirect(
        &mut self,
        callee: Value,
        env: Option<Value>,
        signature: LocalNodeId<Type>,
        args: Vec<Value>,
    ) -> Value {
        let destination = self.allocate_value();
        let result_type = self.signature_result_type(signature);
        let arguments = self.tree.add_arguments(&args);
        self.insert_instruction(Instruction::CallIndirect {
            destination: Some(destination),
            callee,
            env,
            arguments,
            signature,
            effects: None,
        });
        self.define_value(destination, result_type);
        destination
    }

    /// Call through a function pointer with no return value.
    pub fn call_indirect_void(
        &mut self,
        callee: Value,
        env: Option<Value>,
        signature: LocalNodeId<Type>,
        args: Vec<Value>,
    ) {
        let arguments = self.tree.add_arguments(&args);
        self.insert_instruction(Instruction::CallIndirect {
            destination: None,
            callee,
            env,
            arguments,
            signature,
            effects: None,
        });
    }

    // instruction builders: casts

    /// Cast a value to a different type.
    pub fn cast(
        &mut self,
        operator: CastOperator,
        argument: Value,
        to_type: LocalNodeId<Type>,
    ) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::Cast {
            destination,
            operator,
            argument,
            to_type,
        });
        self.define_value(destination, to_type);
        destination
    }

    /// Bitcast (reinterpret bits, same size).
    pub fn bitcast(&mut self, argument: Value, to_type: LocalNodeId<Type>) -> Value {
        self.cast(CastOperator::Bitcast, argument, to_type)
    }

    /// Truncate integer to smaller width.
    pub fn trunc(&mut self, argument: Value, to_type: LocalNodeId<Type>) -> Value {
        self.cast(CastOperator::Truncate, argument, to_type)
    }

    /// Zero-extend integer to larger width.
    pub fn zext(&mut self, argument: Value, to_type: LocalNodeId<Type>) -> Value {
        self.cast(CastOperator::ZeroExtend, argument, to_type)
    }

    /// Sign-extend integer to larger width.
    pub fn sext(&mut self, argument: Value, to_type: LocalNodeId<Type>) -> Value {
        self.cast(CastOperator::SignExtend, argument, to_type)
    }

    // instruction builders: selection

    /// Select between two values based on a boolean condition.
    ///
    /// Returns `then_value` if `condition` is true, `else_value` otherwise.
    /// Both values must have the same type.
    pub fn select(&mut self, condition: Value, then_value: Value, else_value: Value) -> Value {
        let destination = self.allocate_value();
        let then_type = self.value_type_or_panic(then_value, "select then");
        let else_type = self.value_type_or_panic(else_value, "select else");
        let then_ty = self.tree.get(then_type);
        let else_ty = self.tree.get(else_type);
        if then_ty != else_ty {
            panic!("select expects matching value types");
        }
        self.insert_instruction(Instruction::Select {
            destination,
            condition,
            then_value,
            else_value,
        });
        self.define_value(destination, then_type);
        destination
    }

    // instruction builders: intrinsics

    /// Call an intrinsic that returns a value.
    pub fn intrinsic(
        &mut self,
        intrinsic: Intrinsic,
        result_type: LocalNodeId<Type>,
        args: Vec<Value>,
    ) -> Value {
        let destination = self.allocate_value();
        let arguments = self.tree.add_arguments(&args);
        self.insert_instruction(Instruction::Intrinsic {
            destination: Some(destination),
            intrinsic,
            arguments,
            ordering: None,
            scope: None,
            memory_scope: None,
            semantics: None,
        });
        self.define_value(destination, result_type);
        destination
    }

    /// Call an intrinsic with no return value.
    pub fn intrinsic_void(&mut self, intrinsic: Intrinsic, args: Vec<Value>) {
        let arguments = self.tree.add_arguments(&args);
        self.insert_instruction(Instruction::Intrinsic {
            destination: None,
            intrinsic,
            arguments,
            ordering: None,
            scope: None,
            memory_scope: None,
            semantics: None,
        });
    }

    /// Call an atomic intrinsic that returns a value.
    pub fn atomic_intrinsic(
        &mut self,
        intrinsic: Intrinsic,
        args: Vec<Value>,
        ordering: MemoryOrdering,
        scope: AtomicScope,
        memory_scope: MemoryScope,
        semantics: MemorySemantics,
        result_type: LocalNodeId<Type>,
    ) -> Value {
        let destination = self.allocate_value();
        let arguments = self.tree.add_arguments(&args);
        self.insert_instruction(Instruction::Intrinsic {
            destination: Some(destination),
            intrinsic,
            arguments,
            ordering: Some(ordering),
            scope: Some(scope),
            memory_scope: Some(memory_scope),
            semantics: Some(semantics),
        });
        self.define_value(destination, result_type);
        destination
    }

    /// Call an atomic intrinsic with no return value.
    pub fn atomic_intrinsic_void(
        &mut self,
        intrinsic: Intrinsic,
        args: Vec<Value>,
        ordering: MemoryOrdering,
        scope: AtomicScope,
        memory_scope: MemoryScope,
        semantics: MemorySemantics,
    ) {
        let arguments = self.tree.add_arguments(&args);
        self.insert_instruction(Instruction::Intrinsic {
            destination: None,
            intrinsic,
            arguments,
            ordering: Some(ordering),
            scope: Some(scope),
            memory_scope: Some(memory_scope),
            semantics: Some(semantics),
        });
    }

    // terminators

    /// Return from the function.
    pub fn return_(&mut self, return_value: Option<Value>) {
        let block = self.current_block();
        let block_data = self.tree.get_mut(block);
        block_data.terminator = Terminator::Return {
            value: return_value,
        };
    }

    /// Unconditional jump to another block.
    pub fn jump(&mut self, target_block: LocalNodeId<Block>) {
        let block = self.current_block();
        self.add_predecessor(block, target_block);
        let block_data = self.tree.get_mut(block);
        block_data.terminator = Terminator::Jump {
            target: target_block,
            arguments: Vec::new(),
        };
    }

    /// Conditional branch.
    pub fn branch(
        &mut self,
        condition_value: Value,
        then_block: LocalNodeId<Block>,
        else_block: LocalNodeId<Block>,
    ) {
        let block = self.current_block();
        self.add_predecessor(block, then_block);
        self.add_predecessor(block, else_block);
        let block_data = self.tree.get_mut(block);
        block_data.terminator = Terminator::Branch {
            condition: condition_value,
            then_target: then_block,
            then_arguments: Vec::new(),
            else_target: else_block,
            else_arguments: Vec::new(),
        };
    }

    /// Conditional check with explicit success and failure edges.
    pub fn check(
        &mut self,
        condition_value: Value,
        constraint: CheckConstraint,
        success_block: LocalNodeId<Block>,
        failure_block: LocalNodeId<Block>,
    ) {
        let block = self.current_block();
        self.add_predecessor(block, success_block);
        self.add_predecessor(block, failure_block);
        let block_data = self.tree.get_mut(block);
        block_data.terminator = Terminator::Check {
            condition: condition_value,
            constraint,
            success: CheckTarget {
                target: success_block,
                arguments: Vec::new(),
            },
            failure: CheckTarget {
                target: failure_block,
                arguments: Vec::new(),
            },
        };
    }

    /// Tail call to a function (does not return to this function).
    ///
    /// The callee's return value becomes this function's return value.
    pub fn tail_call(&mut self, function: LocalNodeId<Function>, argument_values: Vec<Value>) {
        let block = self.current_block();
        let block = self.tree.get_mut(block);
        block.terminator = Terminator::TailCall {
            function,
            arguments: argument_values,
        };
    }

    /// Tail call through a virtual dispatch slot.
    ///
    /// The callee's return value becomes this function's return value.
    pub fn tail_call_virtual(
        &mut self,
        receiver: Value,
        declaring_type: LocalNodeId<Type>,
        slot_id: u32,
        declared_target: Option<LocalNodeId<Function>>,
        signature: LocalNodeId<Type>,
        argument_values: Vec<Value>,
    ) {
        let block = self.current_block();
        let block = self.tree.get_mut(block);
        block.terminator = Terminator::TailCallVirtual {
            receiver,
            arguments: argument_values,
            declaring_type,
            slot_id,
            declared_target,
            signature,
        };
    }

    /// Tail call through an interface dispatch slot.
    ///
    /// The callee's return value becomes this function's return value.
    pub fn tail_call_interface(
        &mut self,
        receiver: Value,
        declaring_type: LocalNodeId<Type>,
        slot_id: u32,
        declared_target: Option<LocalNodeId<Function>>,
        signature: LocalNodeId<Type>,
        argument_values: Vec<Value>,
    ) {
        let block = self.current_block();
        let block = self.tree.get_mut(block);
        block.terminator = Terminator::TailCallInterface {
            receiver,
            arguments: argument_values,
            declaring_type,
            slot_id,
            declared_target,
            signature,
        };
    }

    /// Tail call through a function pointer (does not return to this function).
    ///
    /// The callee's return value becomes this function's return value.
    pub fn tail_call_indirect(
        &mut self,
        callee: Value,
        env: Option<Value>,
        signature: LocalNodeId<Type>,
        argument_values: Vec<Value>,
    ) {
        let block = self.current_block();
        let block = self.tree.get_mut(block);
        block.terminator = Terminator::TailCallIndirect {
            callee,
            env,
            arguments: argument_values,
            signature,
        };
    }

    /// Insert an instruction into the current block.
    fn insert_instruction(&mut self, instruction: Instruction) -> LocalNodeId<Instruction> {
        let instruction_id = self.tree.insert(instruction);
        let block_id = self.current_block();
        let block = self.tree.get_mut(block_id);
        block.instructions.push(instruction_id);
        instruction_id
    }

    /// Finish building the function.
    ///
    /// Seals all remaining unsealed blocks and returns the function id.
    pub fn finish(mut self) -> LocalNodeId<Function> {
        // seal any remaining unsealed blocks
        self.seal_all_blocks();

        // set entry block to the first created block
        let entry_block = self.blocks[0];

        // capture function parameters for entry block checks
        let parameters = {
            let function = self.tree.get(self.function_id);
            function.parameters.clone()
        };

        // ensure entry block parameters match function parameters
        {
            let block = self.tree.get_mut(entry_block);
            // populate entry block parameters when missing
            if block.parameters.is_empty() {
                block.parameters = parameters.clone();
            } else if block.parameters != parameters {
                panic!("entry block parameters must match function parameters");
            }
        }

        // update function
        let function = self.tree.get_mut(self.function_id);
        function.entry = Some(entry_block);
        function.blocks = self.blocks;
        function.next_value_id = self.next_value_id;

        // always verify in debug and test builds
        #[cfg(any(test, debug_assertions))]
        {
            use crate::VerifierOptions;

            let verifier =
                crate::verify::Verifier::new_with_options(self.tree, VerifierOptions::strict());
            if let Err(error) = verifier.verify_function(self.function_id) {
                panic!("mir verification failed: {error}");
            }
        }

        self.function_id
    }
}
