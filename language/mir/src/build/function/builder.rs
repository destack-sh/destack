use destack_source::{ModuleId, Span};
use indexmap::{IndexMap, IndexSet};

use crate::build::{BuildError, BuildResult, FunctionHeader, Variable};
use crate::{
    AllocationMode, Block, EffectTable, Function, FunctionBehavior, FunctionBody,
    FunctionParameter, Instruction, Linkage, Local, LocalNodeId, MemoryEffect, Node, Terminator,
    Tree, TreeMut, TypeId, Value,
};

/// The builder for one MIR function, constructing SSA as it goes.
#[derive(Debug)]
pub struct FunctionBuilder<'a> {
    // meta
    /// The module the function belongs to.
    pub(super) module: ModuleId,
    /// The tree this function is being built in.
    pub(super) tree: &'a mut Tree,
    /// The effect table for the call and function metadata this builder emits.
    pub(super) effects: &'a mut EffectTable,
    /// The pointer width in bits.
    pub(super) pointer_bits: u16,
    /// The source assigned to the emitted nodes.
    pub(super) source: Option<(u32, Span)>,
    /// The id of the function being built.
    pub(super) function_id: LocalNodeId<Function>,
    /// The block currently receiving instructions.
    pub(super) current_block: Option<LocalNodeId<Block>>,
    /// The locals built for this function body.
    pub(super) locals: Vec<LocalNodeId<Local>>,
    /// The SSA value types, keyed by value id.
    pub(super) value_types: Vec<Option<TypeId>>,

    // ssa construction state
    /// The next SSA value id to allocate.
    pub(super) next_value_id: u32,
    /// The next variable id to allocate.
    pub(super) next_variable_id: u32,
    /// The value each block and variable pair defines.
    pub(super) variable_definitions: IndexMap<(LocalNodeId<Block>, Variable), Value>,
    /// The sealed blocks, whose predecessors are all known.
    pub(super) sealed_blocks: IndexSet<LocalNodeId<Block>>,
    /// The predecessor blocks of each block.
    pub(super) predecessors: IndexMap<LocalNodeId<Block>, Vec<LocalNodeId<Block>>>,
    /// The incomplete block parameters awaiting their block's sealing.
    pub(super) incomplete_phis: IndexMap<LocalNodeId<Block>, Vec<(Variable, Value)>>,
    /// The declared type of each variable.
    pub(super) variable_types: IndexMap<Variable, TypeId>,
    /// The blocks in creation order.
    pub(super) blocks: Vec<LocalNodeId<Block>>,
}

// allow the builder methods with many parameters
#[allow(clippy::too_many_arguments)]
impl<'a> FunctionBuilder<'a> {
    /// Create one function builder.
    pub fn new(
        tree: &'a mut Tree,
        effects: &'a mut EffectTable,
        pointer_bits: u16,
        header: FunctionHeader,
    ) -> Self {
        let FunctionHeader {
            name,
            generics,
            arguments,
            symbol,
            lifetimes,
            parameters,
            result,
            kind,
        } = header;
        let module = symbol.declaring_module();

        // create parameter values
        let parameters = FunctionHeader::parameters_from_types(parameters);
        let (next_value_id, value_types) = Function::parameter_state(&parameters);

        // insert a signature-only function until finish commits the body
        let function = Function {
            name,
            generics,
            arguments,
            template: None,
            symbol,
            kind,
            linkage: Linkage::Local,
            allocation: AllocationMode::Any,
            parameters,
            lifetimes,
            return_type: result,
            environment: None,
            binding: None,
            body: None,
        };
        let function_id = tree.insert(function);

        Self {
            module,
            tree,
            effects,
            pointer_bits,
            source: None,
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

    /// Create one function builder over an existing declared function.
    pub fn from_declared(
        tree: &'a mut Tree,
        effects: &'a mut EffectTable,
        pointer_bits: u16,
        function_id: LocalNodeId<Function>,
    ) -> BuildResult<Self> {
        // require the declared function to be empty
        let (next_value_id, value_types) = {
            let function = tree.get(function_id);
            if function.body().is_some() {
                return Err(BuildError::FunctionAlreadyHasBody {
                    function: function_id,
                });
            }

            Function::parameter_state(&function.parameters)
        };
        let module = tree.get(function_id).symbol.declaring_module();

        Ok(Self {
            module,
            tree,
            effects,
            pointer_bits,
            source: None,
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

    /// Set the memory effect of the function.
    pub fn set_memory_effect(&mut self, effect: MemoryEffect) {
        self.effects.upsert_function(self.function_id).memory = effect;
    }

    /// Set the behavioral effects of the function.
    pub fn set_function_behavior(&mut self, behavior: FunctionBehavior) {
        self.effects.upsert_function(self.function_id).behavior = behavior;
    }

    /// Set the allocation mode of the function.
    pub fn set_allocation_mode(&mut self, allocation: AllocationMode) {
        let function = self.tree.get_mut(self.function_id);
        function.allocation = allocation;
    }

    /// Return the id of the function being built.
    pub fn function_id(&self) -> LocalNodeId<Function> {
        self.function_id
    }

    /// Return the tree.
    pub fn tree(&self) -> &Tree {
        self.tree
    }

    /// Replace the source assigned to the emitted nodes.
    pub fn replace_source(&mut self, source: Option<(u32, Span)>) -> Option<(u32, Span)> {
        std::mem::replace(&mut self.source, source)
    }

    /// Return the source node and span anchoring the instructions inserted now.
    pub fn source(&self) -> Option<(u32, Span)> {
        self.source
    }

    /// Return the target pointer width in bits.
    pub fn pointer_bits(&self) -> u16 {
        self.pointer_bits
    }

    /// Return the pointer width in bytes.
    pub fn pointer_bytes(&self) -> u8 {
        (self.pointer_bits / 8) as u8
    }

    /// Return the mutable effect table.
    pub fn effects_mut(&mut self) -> &mut EffectTable {
        self.effects
    }

    /// Return the mutable tree and effect table together.
    pub fn tree_and_effects_mut(&mut self) -> (&mut Tree, &mut EffectTable) {
        (self.tree, self.effects)
    }

    /// Return the mutable tree.
    pub fn tree_mut(&mut self) -> &mut Tree {
        self.tree
    }

    /// Allocate one new SSA value.
    pub(super) fn allocate_value(&mut self) -> Value {
        let value = Value::new(self.next_value_id);
        self.next_value_id += 1;

        value
    }

    /// Record the type of one value an instruction produces.
    pub(super) fn define_value(&mut self, value: Value, ty: TypeId) {
        let index = self.resize_value_slots(value);

        // keep the recorded type, requiring every definition to agree
        if let Some(existing) = self.value_types[index] {
            if existing != ty {
                unreachable!("a value {value:?} with mismatched types {existing:?} and {ty:?}");
            }

            return;
        }

        self.value_types[index] = Some(ty);
    }

    /// Return the type of one existing SSA value.
    pub fn value_type(&self, value: Value) -> Option<TypeId> {
        self.value_types.get(value.0 as usize).copied().flatten()
    }

    /// Require the type of one SSA value.
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

    /// Resize the SSA side tables for one value.
    fn resize_value_slots(&mut self, value: Value) -> usize {
        let index = value.0 as usize;
        let value_count = index + 1;

        if self.value_types.len() < value_count {
            self.value_types.resize(value_count, None);
        }

        index
    }

    /// Record one block as a predecessor of another.
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

    /// Insert one instruction into the current block.
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

    /// Insert one node with the active source location.
    pub(super) fn insert<T>(&mut self, node: T) -> LocalNodeId<T>
    where
        T: Node,
        Tree: TreeMut<T>,
    {
        match self.source {
            Some((source_node, span)) => {
                let id = self.tree.insert_from(node, source_node);
                self.tree.set_span(id, span);

                id
            }
            None => self.tree.insert(node),
        }
    }

    /// Anchor one block's terminator at the active source location.
    pub(super) fn stamp_terminator(&mut self, id: LocalNodeId<Terminator>) {
        if let Some((source_node, span)) = self.source {
            self.tree.set_source(id.id, source_node);
            self.tree.set_span(id, span);
        }
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

        // capture function parameters for entry block checks
        let parameters = {
            let function = self.tree.get(self.function_id);
            function
                .parameters
                .iter()
                .map(FunctionParameter::block_parameter)
                .collect::<Vec<_>>()
        };

        // require the entry block parameters to match the function parameters
        let is_entry_mismatch = {
            let block = self.tree.get_mut(entry_block);
            if block.parameters.is_empty() {
                block.parameters = parameters.clone();
            }

            block.parameters != parameters
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
