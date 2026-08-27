use destack_source::{ProvenanceBuilder, ProvenanceId, ProvenanceJournal};
use indexmap::{IndexMap, IndexSet};

use crate::build::{BuildError, BuildResult, FunctionHeader, Variable};
use crate::{
    AllocationMode, Block, EffectTable, Function, FunctionBehavior, FunctionBody, Instruction,
    Linkage, Local, LocalNodeId, MemoryEffect, Node, Tree, TreeMut, TypeId, Value,
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
    /// The provenance table being extended by this function.
    pub(super) provenance: &'a mut ProvenanceBuilder,
    /// Effect table for call and function metadata emitted by this builder.
    pub(super) effects: &'a mut EffectTable,
    /// Pointer width in bits.
    pub(super) pointer_bits: u16,
    /// The input provenance assigned to emitted nodes.
    pub(super) source: ProvenanceId,
    /// The transform recorded for emitted provenance.
    pub(super) transform: &'static str,
    /// The id of the function being built.
    pub(super) function_id: LocalNodeId<Function>,
    /// Current block we're inserting into.
    pub(super) current_block: Option<LocalNodeId<Block>>,
    /// Locals built for this function body.
    pub(super) locals: Vec<LocalNodeId<Local>>,
    /// SSA value types keyed by value id.
    pub(super) value_types: Vec<Option<TypeId>>,

    // ssa construction state
    /// Next SSA value id to allocate.
    pub(super) next_value_id: u32,
    /// Next variable id to allocate.
    pub(super) next_variable_id: u32,
    /// Variable definitions: (block, variable) → value.
    pub(super) variable_definitions: IndexMap<(LocalNodeId<Block>, Variable), Value>,
    /// Sealed blocks (all predecessors known).
    pub(super) sealed_blocks: IndexSet<LocalNodeId<Block>>,
    /// Block predecessors: block → list of predecessor blocks.
    pub(super) predecessors: IndexMap<LocalNodeId<Block>, Vec<LocalNodeId<Block>>>,
    /// Incomplete block parameters that need resolution when the block is sealed.
    pub(super) incomplete_phis: IndexMap<LocalNodeId<Block>, Vec<(Variable, Value)>>,
    /// Variable types (needed for creating block parameters).
    pub(super) variable_types: IndexMap<Variable, TypeId>,
    /// Blocks in order of creation.
    pub(super) blocks: Vec<LocalNodeId<Block>>,
}

// allow builder helpers with many parameters
#[allow(clippy::too_many_arguments)]
impl<'a> FunctionBuilder<'a> {
    /// Create a new function builder.
    pub fn new(
        tree: &'a mut Tree,
        provenance: &'a mut ProvenanceBuilder,
        effects: &'a mut EffectTable,
        pointer_bits: u16,
        transform: &'static str,
        header: FunctionHeader,
        source: ProvenanceId,
    ) -> Self {
        let FunctionHeader {
            name,
            arguments,
            symbol,
            lifetimes,
            parameters,
            result,
        } = header;

        let (next_value_id, value_types) = Function::parameter_state(&parameters);

        // insert a signature-only function until finish commits the body
        let function = Function {
            name,
            arguments,
            symbol,
            linkage: Linkage::Local,
            allocation: AllocationMode::Any,
            parameters,
            lifetimes,
            return_type: result,
            environment: None,
            binding: None,
            body: None,
        };
        let function_id = tree.insert(function, source);

        Self {
            tree,
            provenance,
            effects,
            pointer_bits,
            source,
            transform,
            function_id,
            current_block: None,
            locals: Vec::new(),
            value_types,
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
        provenance: &'a mut ProvenanceBuilder,
        effects: &'a mut EffectTable,
        pointer_bits: u16,
        transform: &'static str,
        function_id: LocalNodeId<Function>,
    ) -> BuildResult<Self> {
        // validate the declared function is still empty
        let (next_value_id, value_types) = {
            let function = tree.get(function_id);
            if function.body().is_some() {
                return Err(BuildError::FunctionAlreadyHasBody {
                    function: function_id,
                });
            }

            Function::parameter_state(&function.parameters)
        };

        let source = tree.provenance(function_id.id);

        Ok(Self {
            tree,
            provenance,
            effects,
            pointer_bits,
            source,
            transform,
            function_id,
            current_block: None,
            locals: Vec::new(),
            value_types,
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

    /// Set the memory effect for the function.
    pub fn set_memory_effect(&mut self, effect: MemoryEffect) {
        self.effects.upsert_function(self.function_id).memory = effect;
    }

    /// Set behavioral effects for the function.
    pub fn set_function_behavior(&mut self, behavior: FunctionBehavior) {
        self.effects.upsert_function(self.function_id).behavior = behavior;
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

    /// Replace the input provenance assigned to emitted nodes.
    pub fn replace_source(&mut self, source: ProvenanceId) -> ProvenanceId {
        std::mem::replace(&mut self.source, source)
    }

    /// Return the target pointer width in bits.
    pub fn pointer_bits(&self) -> u16 {
        self.pointer_bits
    }

    /// Return the pointer width in bytes.
    pub fn pointer_bytes(&self) -> u8 {
        (self.pointer_bits / 8) as u8
    }

    /// Get a mutable reference to the underlying tree.
    pub fn tree_mut(&mut self) -> &mut Tree {
        self.tree
    }

    /// Borrow the mutable tree and active provenance journal independently.
    pub fn split_mut(&mut self) -> (&mut Tree, ProvenanceJournal<'_>) {
        let provenance = self.provenance.record(self.transform);

        (self.tree, provenance)
    }

    /// Allocate a new SSA value.
    pub(super) fn allocate_value(&mut self) -> Value {
        let value = Value::new(self.next_value_id);
        self.next_value_id += 1;
        value
    }

    /// Record the type for a value produced by an instruction.
    pub(super) fn define_value(&mut self, value: Value, ty: TypeId) {
        let index = self.resize_value_slots(value);

        if let Some(existing) = self.value_types[index] {
            if existing != ty {
                unreachable!("value {value:?} has mismatched types {existing:?} and {ty:?}");
            }

            return;
        }

        self.value_types[index] = Some(ty);
    }

    /// Get the type of an existing SSA value.
    pub fn value_type(&self, value: Value) -> Option<TypeId> {
        self.value_types.get(value.0 as usize).copied().flatten()
    }

    /// Require the type of an SSA value.
    pub(super) fn expect_value_type(&self, value: Value, context: &str) -> TypeId {
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

    /// Resize SSA side tables for one value.
    fn resize_value_slots(&mut self, value: Value) -> usize {
        let index = value.0 as usize;
        let value_count = index + 1;

        if self.value_types.len() < value_count {
            self.value_types.resize(value_count, None);
        }

        index
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
        let instruction_id = self.insert(instruction);
        let block_id = self.current_block();
        let block = self.tree.get_mut(block_id);
        block.instructions.push(instruction_id);
        instruction_id
    }

    /// Insert one node derived from the active provenance.
    pub(super) fn insert<T>(&mut self, node: T) -> LocalNodeId<T>
    where
        T: Node,
        Tree: TreeMut<T>,
    {
        let provenance = self.provenance.record(self.transform).derive(self.source);

        self.tree.insert(node, provenance)
    }

    /// Finish the function body and return its function id.
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

        // populate an empty entry from the function parameters
        let is_entry_mismatch = if self.tree.get(entry_block).parameters.is_empty() {
            let parameters = {
                let function = self.tree.get(self.function_id);
                let mut provenance = self.provenance.record(self.transform);
                function
                    .parameters
                    .iter()
                    .map(|parameter| parameter.block_parameter(&mut provenance))
                    .collect::<Vec<_>>()
            };
            self.tree.get_mut(entry_block).parameters = parameters;

            false
        }
        // otherwise require matching values and types
        else {
            let function = self.tree.get(self.function_id);
            let block = self.tree.get(entry_block);
            block.parameters.len() != function.parameters.len()
                || block
                    .parameters
                    .iter()
                    .zip(&function.parameters)
                    .any(|(actual, expected)| actual.typed_value() != expected.typed_value())
        };
        if is_entry_mismatch {
            return Err(BuildError::EntryParameterMismatch);
        }

        // commit the complete body
        let body = FunctionBody::new(
            entry_block,
            self.blocks,
            self.locals,
            self.value_types,
            self.next_value_id,
            self.tree,
        );
        let function = self.tree.get_mut(self.function_id);
        function.set_body(body);

        Ok(self.function_id)
    }
}
