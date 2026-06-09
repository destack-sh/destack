use destack_core::{StringId, StringPool};
use indexmap::{IndexMap, IndexSet};

use crate::build::{BuildError, BuildResult, Variable};
use crate::{
    AllocationMode, AllocationSize, Block, Function, FunctionBehavior, Instruction, Linkage,
    LocalNodeId, MemoryEffect, Parameter, Place, Projection, Tree, Type, TypeReference, Value,
    ValueReference, finalize_function_names,
};

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
    /// The tree this function is being built in.
    pub(super) tree: &'a mut Tree,
    /// The string pool used for generated MIR names.
    pub(super) strings: &'a mut StringPool,
    /// The id of the function being built.
    pub(super) function_id: LocalNodeId<Function>,
    /// Current block we're inserting into.
    pub(super) current_block: Option<LocalNodeId<Block>>,

    // ssa construction state
    /// Next SSA value id to allocate.
    pub(super) next_value_id: u32,
    /// Next variable id to allocate.
    pub(super) next_variable_id: u32,
    /// Variable definitions: (block, variable) → value.
    /// Tracks the SSA value of each variable at the end of each block.
    pub(super) variable_definitions: IndexMap<(LocalNodeId<Block>, Variable), Value>,
    /// Sealed blocks (all predecessors known).
    pub(super) sealed_blocks: IndexSet<LocalNodeId<Block>>,
    /// Block predecessors: block → list of predecessor blocks.
    pub(super) predecessors: IndexMap<LocalNodeId<Block>, Vec<LocalNodeId<Block>>>,
    /// Incomplete block parameters that need resolution when the block is sealed.
    /// Maps block → list of (variable, parameter value) pairs.
    pub(super) incomplete_phis: IndexMap<LocalNodeId<Block>, Vec<(Variable, Value)>>,
    /// Variable types (needed for creating block parameters).
    pub(super) variable_types: IndexMap<Variable, LocalNodeId<Type>>,
    /// Blocks in order of creation.
    pub(super) blocks: Vec<LocalNodeId<Block>>,
}

// allow builder helpers with many parameters
#[allow(clippy::too_many_arguments)]
impl<'a> FunctionBuilder<'a> {
    /// Create a new function builder.
    pub fn new(
        tree: &'a mut Tree,
        strings: &'a mut StringPool,
        name: StringId,
        parameter_types: &[LocalNodeId<Type>],
        return_type: LocalNodeId<Type>,
    ) -> Self {
        // create parameter values
        let mut next_value_id = 0u32;
        let parameters: Vec<Parameter> = parameter_types
            .iter()
            .map(|&ty| {
                let value = Value::new(next_value_id);
                next_value_id += 1;
                Parameter {
                    value: ValueReference::Value(value),
                    ty: TypeReference::from(ty),
                }
            })
            .collect();
        let (_, value_types) = Function::parameter_state(&parameters);

        // blank function (entry will be set in finish())
        let function = Function {
            name,
            parameters,
            lifetimes: Vec::new(),
            parameter_names: vec![None; parameter_types.len()],
            value_names: vec![None; next_value_id as usize],
            value_types,
            value_places: vec![None; next_value_id as usize],
            return_type: TypeReference::from(return_type),
            borrow_obligations: Vec::new(),
            linkage: Linkage::Local,
            allocation: AllocationMode::Any,
            suspension: None,
            environment: None,
            locals: Vec::new(),
            blocks: Vec::new(),
            entry: None,
            next_value_id,
        };
        let function_id = tree.insert(function);

        Self {
            tree,
            strings,
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
    pub fn from_declared(
        tree: &'a mut Tree,
        strings: &'a mut StringPool,
        function_id: LocalNodeId<Function>,
    ) -> BuildResult<Self> {
        // validate the declared function is still empty
        let next_value_id = {
            let function = tree.get(function_id);
            if function.entry.is_some() || !function.blocks.is_empty() {
                return Err(BuildError::FunctionAlreadyHasBody {
                    function: function_id,
                });
            }

            function.next_value_id
        };

        Ok(Self {
            tree,
            strings,
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
        })
    }

    /// Set a debug parameter name on the function signature.
    pub fn set_parameter_name(&mut self, index: usize, name: destack_core::StringId) {
        // record parameter names for diagnostics
        let function = self.tree.get_mut(self.function_id);
        if let Some(slot) = function.parameter_names.get_mut(index) {
            *slot = Some(name);
        }
    }

    /// Set the memory effect for the function.
    pub fn set_memory_effect(&mut self, effect: MemoryEffect) {
        self.tree
            .metadata
            .functions
            .function_mut(self.function_id)
            .memory = effect;
    }

    /// Set behavioral effects for the function.
    pub fn set_function_behavior(&mut self, behavior: FunctionBehavior) {
        self.tree
            .metadata
            .functions
            .function_mut(self.function_id)
            .behavior = behavior;
    }

    /// Set allocation size metadata for the function.
    pub fn set_allocation_size(&mut self, allocation_size: AllocationSize) {
        self.tree
            .metadata
            .functions
            .function_mut(self.function_id)
            .allocation_size = Some(allocation_size);
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

    /// Get a reference to the underlying tree.
    pub fn tree(&self) -> &Tree {
        self.tree
    }

    /// Get a mutable reference to the underlying tree.
    pub fn tree_mut(&mut self) -> &mut Tree {
        self.tree
    }

    /// Allocate a new SSA value.
    pub(super) fn allocate_value(&mut self) -> Value {
        let value = Value::new(self.next_value_id);
        self.next_value_id += 1;
        value
    }

    /// Record the type for a value produced by an instruction.
    pub(super) fn define_value(&mut self, value: Value, ty: LocalNodeId<Type>) {
        let function = self.tree.get_mut(self.function_id);
        function.set_value_type(value, ty);
    }

    /// Record the type and root place for an SSA value.
    pub(super) fn define_value_with_place(
        &mut self,
        value: Value,
        ty: LocalNodeId<Type>,
        place: Place,
    ) {
        self.define_value(value, ty);
        self.define_place(value, place);
    }

    /// Record the type and projected place for an SSA value.
    pub(super) fn define_value_from_projection(
        &mut self,
        value: Value,
        ty: LocalNodeId<Type>,
        base: Value,
        projection: Projection,
    ) {
        self.define_value(value, ty);
        self.define_projection(value, base, projection);
    }

    /// Record the type and copied place for an SSA value.
    pub(super) fn define_value_from_place(
        &mut self,
        value: Value,
        ty: LocalNodeId<Type>,
        source: Value,
    ) {
        self.define_value(value, ty);
        self.propagate_place(value, source);
    }

    /// Record the place for an SSA value.
    pub(super) fn define_place(&mut self, value: Value, place: Place) {
        let function = self.tree.get_mut(self.function_id);
        function.set_value_place(value, place)
    }

    /// Record a projected place for an SSA value.
    pub(super) fn define_projection(&mut self, value: Value, base: Value, projection: Projection) {
        let function = self.tree.get_mut(self.function_id);
        function.set_projected_place(value, base, projection)
    }

    /// Copy a place from one SSA value to another.
    pub(super) fn propagate_place(&mut self, value: Value, source: Value) {
        let function = self.tree.get_mut(self.function_id);
        function.copy_value_place(value, source)
    }

    /// Get the type of an existing SSA value.
    pub(super) fn value_type(&self, value: Value) -> Option<LocalNodeId<Type>> {
        let function = self.tree.get(self.function_id);
        function.value_type(value)
    }

    /// Require the type of an SSA value.
    pub(super) fn expect_value_type(&self, value: Value, context: &str) -> LocalNodeId<Type> {
        let result = self
            .value_type(value)
            .ok_or_else(|| BuildError::MissingValueType {
                value,
                context: context.to_string(),
            });

        self.expect_build(result)
    }

    /// Unwrap one builder result for an infallible builder operation.
    pub(super) fn expect_build<T>(&self, result: BuildResult<T>) -> T {
        result.unwrap_or_else(|error| unreachable!("{error}"))
    }

    /// Record that `from_block` is a predecessor of `to_block`.
    pub(super) fn add_predecessor(
        &mut self,
        from_block: LocalNodeId<Block>,
        to_block: LocalNodeId<Block>,
    ) {
        self.predecessors
            .entry(to_block)
            .or_default()
            .push(from_block);
    }

    /// Insert an instruction into the current block.
    pub(super) fn insert_instruction(
        &mut self,
        instruction: Instruction,
    ) -> LocalNodeId<Instruction> {
        let instruction_id = self.tree.insert(instruction);
        let block_id = self.current_block();
        let block = self.tree.get_mut(block_id);
        block.instructions.push(instruction_id);
        instruction_id
    }
    pub fn finish(mut self) -> BuildResult<LocalNodeId<Function>> {
        // seal any remaining unsealed blocks
        self.seal_all_blocks();

        // set entry block to the first created block
        let entry_block = self
            .blocks
            .first()
            .copied()
            .ok_or(BuildError::MissingEntryBlock {
                function: self.function_id,
            })?;

        // capture function parameters for entry block checks
        let parameters = {
            let function = self.tree.get(self.function_id);
            function.parameters.clone()
        };

        // ensure entry block parameters match function parameters
        let is_entry_mismatch = {
            let block = self.tree.get_mut(entry_block);
            // populate entry block parameters when missing
            if block.parameters.is_empty() {
                block.parameters = parameters.clone();
            }

            block.parameters != parameters
        };
        if is_entry_mismatch {
            return Err(BuildError::EntryParameterMismatch);
        }

        // update function
        let function = self.tree.get_mut(self.function_id);
        function.entry = Some(entry_block);
        function.blocks = self.blocks;
        function.next_value_id = self.next_value_id;

        // finalize generated names before formatting
        finalize_function_names(self.tree, self.strings, self.function_id);

        Ok(self.function_id)
    }
}
