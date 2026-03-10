use destack_core::StringId;
use indexmap::{IndexMap, IndexSet};

use crate::build::Variable;
use crate::{
    AllocSize, AllocationMode, Block, CallBehavior, CallSite, DevirtualizationMetadata,
    ExecutionModel, ExecutionStage, Function, Instruction, Lifetime, Linkage, LocalNodeId,
    MemoryEffect, NodeTree, PointerAttributes, Type, TypedValue, Value,
};

mod aggregate;
mod calls;
mod control;
mod memory;
mod ssa;
mod values;

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
/// block parameters (phi-functions) when a variable is used. Key features:
/// - **Local Value Numbering**: If a variable is defined in the current block, return that value.
/// - **Global Value Numbering**: Otherwise, recursively look up the value from predecessors.
/// - **Block Parameters**: Created at join points where different predecessors have different values.
/// - **Trivial phi Removal**: If all predecessors have the same value, no block parameter is needed.
/// - **Incomplete CFGs**: Blocks can be used before all predecessors are known (unsealed blocks).
#[derive(Debug)]
pub struct FunctionBuilder<'a> {
    // meta
    /// The node tree this function is being built in.
    tree: &'a mut NodeTree,
    /// The id of the function being built.
    function_id: LocalNodeId<Function>,
    /// Whether to verify this function when finishing.
    verify: bool,
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
        verify: bool,
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
            memory_effects: MemoryEffect::unknown(),
            call_behavior: CallBehavior::unknown(),
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
            verify,
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
    pub fn from_declared(
        tree: &'a mut NodeTree,
        function_id: LocalNodeId<Function>,
        verify: bool,
    ) -> Self {
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
            verify,
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
    pub fn set_parameter_name(&mut self, index: usize, name: destack_core::StringId) {
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
        function.memory_effects = effects;
    }

    /// Set behavioral effects for the function.
    pub fn set_call_behavior(&mut self, behavior: CallBehavior) {
        // update the function call behavior
        let function = self.tree.get_mut(self.function_id);
        function.call_behavior = behavior;
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

    /// Record that `from_block` is a predecessor of `to_block`.
    fn add_predecessor(&mut self, from_block: LocalNodeId<Block>, to_block: LocalNodeId<Block>) {
        self.predecessors
            .entry(to_block)
            .or_default()
            .push(from_block);
    }

    /// Record dispatch metadata when a callsite carries real devirtualization facts.
    fn insert_dispatch_callsite_metadata(
        &mut self,
        callsite: CallSite,
        metadata: DevirtualizationMetadata,
    ) {
        if metadata.is_empty() {
            return;
        }

        self.tree
            .dispatch_table
            .insert_callsite_metadata(callsite, metadata);
    }

    /// Insert an instruction into the current block.
    fn insert_instruction(&mut self, instruction: Instruction) -> LocalNodeId<Instruction> {
        let instruction_id = self.tree.insert(instruction);
        let block_id = self.current_block();
        let block = self.tree.get_mut(block_id);
        block.instructions.push(instruction_id);
        instruction_id
    }
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

        // keep the verify flag alive in release builds
        #[cfg(not(any(test, debug_assertions)))]
        let _ = self.verify;

        // always validate in debug and test builds
        #[cfg(any(test, debug_assertions))]
        {
            if self.verify {
                let validator = crate::validate::Validator::new(self.tree);
                if let Err(error) = validator.validate_function(self.function_id) {
                    panic!("mir validation failed: {error}");
                }
            }
        }

        self.function_id
    }
}
