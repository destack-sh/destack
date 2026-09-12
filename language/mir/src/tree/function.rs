use destack_core::StringId;
use destack_serde::Reflect;
use destack_source::ModuleId;
use serde::{Deserialize, Serialize};

use crate::{
    Binding, Block, FunctionId, FunctionParameter, GenericArgument, GenericParameter, Instruction,
    LifetimeParameter, Linkage, Local, LocalNodeId, Node, NodeType, Symbol, Tree, Type, TypeId,
    Value,
};

/// One MIR function declaration or definition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct Function {
    /// The function's declared name.
    pub name: StringId,
    /// The generic parameters a template takes.
    pub generics: Vec<GenericParameter>,
    /// The generic arguments a specialization applies to its template.
    pub arguments: Vec<GenericArgument>,
    /// The template a specialization applies at its arguments, in this tree.
    pub template: Option<FunctionId>,
    /// The function's persistent mangled symbol: its linkable identity.
    pub symbol: Symbol,
    /// Linkage (local, export, or import).
    pub linkage: Linkage,
    /// Memory allocation restrictions for this function.
    pub allocation: AllocationMode,
    /// Function parameters as typed SSA slots.
    pub parameters: Vec<FunctionParameter>,
    /// Lifetime parameters in function-local slot order.
    pub lifetimes: Vec<LifetimeParameter>,
    /// The return type.
    pub return_type: TypeId,
    /// The hidden environment type for this function when present.
    pub environment: Option<TypeId>,
    /// Runtime binding declaration when this function has a binding identity.
    pub binding: Option<Box<Binding>>,
    /// The executable function body when this function is defined.
    pub body: Option<FunctionBody>,
}

impl Node for Function {
    const TYPE: NodeType = NodeType::Function;
}

/// The executable body of one MIR function.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct FunctionBody {
    /// The entry block where execution starts.
    entry: LocalNodeId<Block>,
    /// The function blocks in layout order.
    blocks: Vec<LocalNodeId<Block>>,
    /// The function locals in slot order.
    locals: Vec<LocalNodeId<Local>>,
    /// SSA value types keyed by value id.
    value_types: Vec<Option<LocalNodeId<Type>>>,
    /// Counter for allocating unique SSA value ids.
    next_value_id: u32,
    /// Instruction locations keyed by instruction id.
    instruction_index: InstructionIndex,
}

/// Memory allocation restrictions for a function.
///
/// This allows marking functions as managed-allocation-free or heap-free.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Reflect)]
pub enum AllocationMode {
    /// No restrictions on allocation.
    #[default]
    Any,
    /// Managed allocation forbidden.
    /// Unique allocation and function-local storage are still allowed.
    NoManaged,
    /// No heap allocation.
    /// Only function-local storage is allowed.
    NoHeap,
}

impl AllocationMode {
    /// Return the MIR text representation.
    pub fn to_str(self) -> &'static str {
        match self {
            AllocationMode::Any => "any",
            AllocationMode::NoManaged => "noManaged",
            AllocationMode::NoHeap => "noHeap",
        }
    }
}

/// Location of one instruction in a MIR function body.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct InstructionLocation {
    /// The block that owns the instruction.
    pub block: LocalNodeId<Block>,
    /// The instruction index in the block.
    pub index: u32,
}

/// Function-local instruction location index.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, Reflect)]
struct InstructionIndex {
    /// Instruction locations sorted by instruction id.
    locations: Vec<(LocalNodeId<Instruction>, InstructionLocation)>,
}

impl InstructionIndex {
    /// Build an instruction index for one function body.
    fn build(blocks: &[LocalNodeId<Block>], tree: &Tree) -> Self {
        let mut index = Self::default();
        index.rebuild(blocks, tree);
        index
    }

    /// Rebuild the whole instruction index.
    fn rebuild(&mut self, blocks: &[LocalNodeId<Block>], tree: &Tree) {
        self.locations.clear();

        for &block_id in blocks {
            let block = tree.get(block_id);
            for (index, &instruction) in block.instructions.iter().enumerate() {
                self.locations.push((
                    instruction,
                    InstructionLocation {
                        block: block_id,
                        index: index as u32,
                    },
                ));
            }
        }

        self.sort();
    }

    /// Sort locations by instruction id and reject duplicate ownership.
    fn sort(&mut self) {
        self.locations
            .sort_unstable_by_key(|(instruction, _)| instruction.id);

        // reject duplicate instruction ownership
        if self
            .locations
            .windows(2)
            .any(|locations| locations[0].0 == locations[1].0)
        {
            unreachable!("instruction belongs to multiple blocks");
        }
    }

    /// Return one instruction location.
    fn location(&self, instruction: LocalNodeId<Instruction>) -> Option<InstructionLocation> {
        let index = self
            .locations
            .binary_search_by_key(&instruction.id, |(instruction, _)| instruction.id)
            .ok()?;

        Some(self.locations[index].1)
    }

    /// Return the block that owns one instruction.
    fn block(&self, instruction: LocalNodeId<Instruction>) -> Option<LocalNodeId<Block>> {
        self.location(instruction).map(|location| location.block)
    }

    /// Return one instruction's index in its owning block.
    fn position(&self, instruction: LocalNodeId<Instruction>) -> Option<usize> {
        self.location(instruction)
            .map(|location| location.index as usize)
    }

    /// Replace all instruction locations for one block.
    fn replace_block(
        &mut self,
        block: LocalNodeId<Block>,
        instructions: &[LocalNodeId<Instruction>],
    ) {
        self.locations
            .retain(|(_, location)| location.block != block);

        // append replacement locations before restoring lookup order
        self.locations
            .extend(
                instructions
                    .iter()
                    .copied()
                    .enumerate()
                    .map(|(index, instruction)| {
                        (
                            instruction,
                            InstructionLocation {
                                block,
                                index: index as u32,
                            },
                        )
                    }),
            );
        self.sort();
    }
}

impl FunctionBody {
    /// Create an empty function body.
    fn empty(
        entry: LocalNodeId<Block>,
        value_types: Vec<Option<LocalNodeId<Type>>>,
        next_value_id: u32,
    ) -> Self {
        Self {
            entry,
            blocks: Vec::new(),
            locals: Vec::new(),
            value_types,
            next_value_id,
            instruction_index: InstructionIndex::default(),
        }
    }

    /// Create a complete function body.
    pub fn new(
        entry: LocalNodeId<Block>,
        blocks: Vec<LocalNodeId<Block>>,
        locals: Vec<LocalNodeId<Local>>,
        value_types: Vec<Option<LocalNodeId<Type>>>,
        next_value_id: u32,
        tree: &Tree,
    ) -> Self {
        let instruction_index = InstructionIndex::build(&blocks, tree);

        Self {
            entry,
            blocks,
            locals,
            value_types,
            next_value_id,
            instruction_index,
        }
    }

    /// Return the entry block.
    pub fn entry(&self) -> LocalNodeId<Block> {
        self.entry
    }

    /// Set the entry block.
    pub fn set_entry(&mut self, entry: LocalNodeId<Block>) {
        self.entry = entry;
    }

    /// Return the function blocks in layout order.
    pub fn blocks(&self) -> &[LocalNodeId<Block>] {
        &self.blocks
    }

    /// Return one function block by layout index.
    pub fn block(&self, index: usize) -> LocalNodeId<Block> {
        self.blocks
            .get(index)
            .copied()
            .unwrap_or_else(|| unreachable!("missing block at layout index {index}"))
    }

    /// Replace the function blocks in layout order.
    pub fn replace_blocks(&mut self, blocks: Vec<LocalNodeId<Block>>, tree: &Tree) {
        self.blocks = blocks;
        self.rebuild_instruction_index(tree);
    }

    /// Append one block to the body.
    pub fn add_block(&mut self, block: LocalNodeId<Block>, tree: &Tree) {
        self.blocks.push(block);
        self.rebuild_instruction_index(tree);
    }

    /// Insert one block after a predecessor, or append it when the predecessor is absent.
    pub fn insert_block_after(
        &mut self,
        predecessor: LocalNodeId<Block>,
        block: LocalNodeId<Block>,
        tree: &Tree,
    ) {
        if let Some(index) = self.blocks.iter().position(|id| *id == predecessor) {
            self.blocks.insert(index + 1, block);
        } else {
            self.blocks.push(block);
        }

        self.rebuild_instruction_index(tree);
    }

    /// Return the function locals in slot order.
    pub fn locals(&self) -> &[LocalNodeId<Local>] {
        &self.locals
    }

    /// Return one function local by slot index.
    pub fn local(&self, index: usize) -> LocalNodeId<Local> {
        self.locals
            .get(index)
            .copied()
            .unwrap_or_else(|| unreachable!("missing local at slot index {index}"))
    }

    /// Replace the function locals in slot order.
    pub fn replace_locals(&mut self, locals: Vec<LocalNodeId<Local>>) {
        self.locals = locals;
    }

    /// Append one local to the body.
    pub fn add_local(&mut self, local: LocalNodeId<Local>) {
        self.locals.push(local);
    }

    /// Return the dense value table capacity for this body.
    pub fn value_capacity(&self) -> usize {
        self.next_value_id as usize
    }

    /// Return the type for one SSA value.
    pub fn value_type(&self, value: Value) -> Option<LocalNodeId<Type>> {
        self.value_types.get(value.0 as usize).copied().flatten()
    }

    /// Return all value type slots.
    pub fn value_types(&self) -> &[Option<LocalNodeId<Type>>] {
        &self.value_types
    }

    /// Return one instruction location.
    pub fn instruction_location(
        &self,
        instruction: LocalNodeId<Instruction>,
    ) -> Option<InstructionLocation> {
        self.instruction_index.location(instruction)
    }

    /// Return the block that owns one instruction.
    pub fn instruction_block(
        &self,
        instruction: LocalNodeId<Instruction>,
    ) -> Option<LocalNodeId<Block>> {
        self.instruction_index.block(instruction)
    }

    /// Return one instruction's index in its owning block.
    pub fn instruction_position(&self, instruction: LocalNodeId<Instruction>) -> Option<usize> {
        self.instruction_index.position(instruction)
    }

    /// Rebuild the instruction location index.
    pub fn rebuild_instruction_index(&mut self, tree: &Tree) {
        self.instruction_index.rebuild(&self.blocks, tree);
    }

    /// Replace one block's instruction locations.
    fn replace_block_instructions(
        &mut self,
        block: LocalNodeId<Block>,
        instructions: &[LocalNodeId<Instruction>],
    ) {
        self.instruction_index.replace_block(block, instructions);
    }

    /// Replace the SSA value type table.
    pub fn replace_value_types(&mut self, value_types: Vec<Option<LocalNodeId<Type>>>) {
        self.value_types = value_types;
        self.next_value_id = self.next_value_id.max(self.value_types.len() as u32);
    }

    /// Return the expected type for one SSA value.
    pub fn expect_value_type(&self, value: Value) -> LocalNodeId<Type> {
        match self.value_type(value) {
            Some(ty) => ty,
            None => unreachable!("missing type for value {value:?}"),
        }
    }

    /// Record the type for one SSA value.
    pub fn set_value_type(&mut self, value: Value, ty: LocalNodeId<Type>) {
        let index = self.resize_value_slots(value);

        if let Some(existing) = self.value_types[index] {
            if existing != ty {
                unreachable!("value {value:?} has mismatched types {existing:?} and {ty:?}");
            }

            return;
        }

        self.value_types[index] = Some(ty);
    }

    /// Allocate a new SSA value.
    pub fn next_value(&mut self) -> Value {
        let value = Value::new(self.next_value_id);
        self.next_value_id += 1;
        value
    }

    /// Allocate a new SSA value and record its type.
    pub fn next_typed_value(&mut self, ty: LocalNodeId<Type>) -> Value {
        let value = self.next_value();
        self.set_value_type(value, ty);
        value
    }

    /// Allocate a new SSA value with the same type as an existing value.
    pub fn next_typed_value_like(&mut self, source: Value) -> Value {
        let ty = self.expect_value_type(source);
        self.next_typed_value(ty)
    }

    /// Recompute the next SSA value id from live body values.
    pub fn recompute_next_value_id(&mut self, parameters: &[FunctionParameter], tree: &Tree) {
        let mut next_value_id = 0;

        // scan function parameters
        for parameter in parameters {
            next_value_id = next_value_id.max(parameter.value.0 + 1);
        }

        // scan block parameters and instruction destinations
        for &block_id in &self.blocks {
            let block = tree.get(block_id);
            for parameter in &block.parameters {
                next_value_id = next_value_id.max(parameter.value.0 + 1);
            }

            for &instruction_id in &block.instructions {
                let Some(value) = tree.get(instruction_id).destination() else {
                    continue;
                };

                next_value_id = next_value_id.max(value.0 + 1);
            }
        }

        let typed_value_count = self.value_types.len() as u32;
        self.next_value_id = self.next_value_id.max(next_value_id).max(typed_value_count);
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
}

impl Function {
    /// Return the explicit function signature type.
    pub fn signature(&self) -> Type {
        let parameters = self
            .parameters
            .iter()
            .map(FunctionParameter::signature_parameter)
            .collect();

        Type::FunctionSignature {
            lifetimes: self.lifetimes.clone(),
            parameters,
            result: self.return_type,
        }
    }

    /// Return the dense value table capacity for this function.
    pub fn value_capacity(&self) -> usize {
        self.body
            .as_ref()
            .map(FunctionBody::value_capacity)
            .unwrap_or(0)
    }

    /// Build parameter-derived SSA tables.
    pub(crate) fn parameter_state(
        parameters: &[FunctionParameter],
    ) -> (u32, Vec<Option<LocalNodeId<Type>>>) {
        // derive the next value id from parameters
        let next_value_id = parameters
            .iter()
            .map(|parameter| parameter.value.0 + 1)
            .max()
            .unwrap_or(0);

        // initialize the type table with the next value id
        let mut value_types = vec![None; next_value_id as usize];

        // record parameter types by value id
        for parameter in parameters {
            let index = parameter.value.0 as usize;
            let slot = &mut value_types[index];

            // idempotent duplicates are okay
            if slot.is_some_and(|existing| existing != parameter.ty) {
                let value = parameter.value;
                unreachable!("value {value:?} has mismatched parameter types");
            }

            *slot = Some(parameter.ty);
        }

        (next_value_id, value_types)
    }

    /// Create one function from its signature.
    fn with_signature(
        module: ModuleId,
        name: StringId,
        lifetimes: Vec<LifetimeParameter>,
        parameters: Vec<FunctionParameter>,
        return_type: TypeId,
        linkage: Linkage,
        body: Option<FunctionBody>,
    ) -> Self {
        // build the function signature
        Self {
            name,
            generics: Vec::new(),
            arguments: Vec::new(),
            template: None,
            symbol: Symbol::named(module, name),
            linkage,
            allocation: AllocationMode::Any,
            parameters,
            lifetimes,
            return_type,
            environment: None,
            binding: None,
            body,
        }
    }

    /// Create a local function declaration without a body.
    pub fn declare(
        module: ModuleId,
        name: StringId,
        lifetimes: Vec<LifetimeParameter>,
        parameters: Vec<FunctionParameter>,
        return_type: TypeId,
    ) -> Self {
        Self::with_signature(
            module,
            name,
            lifetimes,
            parameters,
            return_type,
            Linkage::Local,
            None,
        )
    }

    /// Create a defined local function with an empty body.
    pub fn define(
        module: ModuleId,
        name: StringId,
        lifetimes: Vec<LifetimeParameter>,
        parameters: Vec<FunctionParameter>,
        return_type: TypeId,
        entry: LocalNodeId<Block>,
    ) -> Self {
        let (next_value_id, value_types) = Self::parameter_state(&parameters);
        let body = FunctionBody::empty(entry, value_types, next_value_id);

        Self::with_signature(
            module,
            name,
            lifetimes,
            parameters,
            return_type,
            Linkage::Local,
            Some(body),
        )
    }

    /// Create an imported function declaration (no body).
    pub fn import(
        module: ModuleId,
        name: StringId,
        lifetimes: Vec<LifetimeParameter>,
        parameters: Vec<FunctionParameter>,
        return_type: TypeId,
    ) -> Self {
        Self::with_signature(
            module,
            name,
            lifetimes,
            parameters,
            return_type,
            Linkage::Import,
            None,
        )
    }

    /// Return the type for an SSA value.
    pub fn value_type(&self, value: Value) -> Option<LocalNodeId<Type>> {
        self.body.as_ref().and_then(|body| body.value_type(value))
    }

    /// Return the expected type for one SSA value.
    pub fn expect_value_type(&self, value: Value) -> LocalNodeId<Type> {
        match self.value_type(value) {
            Some(ty) => ty,
            None => unreachable!("missing type for value {value:?}"),
        }
    }

    /// Return whether one value can substitute another value.
    pub fn can_substitute(&self, destination: Value, replacement: Value) -> bool {
        self.expect_value_type(destination) == self.expect_value_type(replacement)
    }

    /// Set the linkage and return self (builder pattern).
    pub fn with_linkage(mut self, linkage: Linkage) -> Self {
        self.linkage = linkage;
        self
    }

    /// Set the persistent symbol and return self.
    pub fn with_symbol(mut self, symbol: Symbol) -> Self {
        self.symbol = symbol;

        self
    }

    /// Set the generic arguments this specialization applies and return self.
    pub fn with_arguments(mut self, arguments: Vec<GenericArgument>) -> Self {
        self.arguments = arguments;

        self
    }

    /// Set the generic parameters this template takes and return self.
    pub fn with_generics(mut self, generics: Vec<GenericParameter>) -> Self {
        self.generics = generics;

        self
    }

    /// Return whether this function is a template.
    pub fn is_polymorphic(&self) -> bool {
        !self.generics.is_empty()
    }

    /// Set the runtime binding declaration and return self.
    pub fn with_binding(mut self, binding: Binding) -> Self {
        self.binding = Some(Box::new(binding));

        self
    }

    /// Return the runtime binding name when one is present.
    pub fn binding_name(&self) -> Option<StringId> {
        self.binding.as_ref().map(|binding| binding.name)
    }

    /// Check if this function is imported (defined elsewhere).
    pub fn is_import(&self) -> bool {
        self.linkage.is_import()
    }

    /// Return whether this function has an executable body.
    pub fn is_defined(&self) -> bool {
        self.body.is_some()
    }

    /// Return the executable body when present.
    pub fn body(&self) -> Option<&FunctionBody> {
        self.body.as_ref()
    }

    /// Return the mutable executable body when present.
    pub(crate) fn body_mut(&mut self) -> Option<&mut FunctionBody> {
        self.body.as_mut()
    }

    /// Replace the executable body.
    pub fn set_body(&mut self, body: FunctionBody) {
        self.body = Some(body);
    }

    /// Remove the executable body.
    pub fn clear_body(&mut self) {
        self.body = None;
    }

    /// Return the entry block when this function has a body.
    pub fn entry(&self) -> Option<LocalNodeId<Block>> {
        self.body.as_ref().map(FunctionBody::entry)
    }

    /// Return the function blocks in layout order.
    pub fn blocks(&self) -> &[LocalNodeId<Block>] {
        self.body.as_ref().map(FunctionBody::blocks).unwrap_or(&[])
    }

    /// Return one function block by layout index.
    pub fn block(&self, index: usize) -> LocalNodeId<Block> {
        let Some(body) = self.body.as_ref() else {
            unreachable!("cannot read block {index} from a function without a body");
        };

        body.block(index)
    }

    /// Return the function locals in slot order.
    pub fn locals(&self) -> &[LocalNodeId<Local>] {
        self.body.as_ref().map(FunctionBody::locals).unwrap_or(&[])
    }

    /// Return one function local by slot index.
    pub fn local(&self, index: usize) -> LocalNodeId<Local> {
        let Some(body) = self.body.as_ref() else {
            unreachable!("cannot read local {index} from a function without a body");
        };

        body.local(index)
    }

    /// Return the value type table when this function has a body.
    pub fn value_types(&self) -> &[Option<LocalNodeId<Type>>] {
        self.body
            .as_ref()
            .map(FunctionBody::value_types)
            .unwrap_or(&[])
    }

    /// Return one instruction location when this function has a body.
    pub fn instruction_location(
        &self,
        instruction: LocalNodeId<Instruction>,
    ) -> Option<InstructionLocation> {
        self.body
            .as_ref()
            .and_then(|body| body.instruction_location(instruction))
    }

    /// Return the block that owns one instruction.
    pub fn instruction_block(
        &self,
        instruction: LocalNodeId<Instruction>,
    ) -> Option<LocalNodeId<Block>> {
        self.body
            .as_ref()
            .and_then(|body| body.instruction_block(instruction))
    }

    /// Return one instruction's index in its owning block.
    pub fn instruction_position(&self, instruction: LocalNodeId<Instruction>) -> Option<usize> {
        self.body
            .as_ref()
            .and_then(|body| body.instruction_position(instruction))
    }

    /// Set the entry block.
    pub fn set_entry(&mut self, entry: LocalNodeId<Block>) {
        let Some(body) = self.body.as_mut() else {
            unreachable!("cannot set entry on a function without a body");
        };

        body.set_entry(entry);
    }

    /// Replace the function blocks in layout order.
    pub fn replace_blocks(&mut self, blocks: Vec<LocalNodeId<Block>>, tree: &Tree) {
        let Some(body) = self.body.as_mut() else {
            unreachable!("cannot replace blocks on a function without a body");
        };

        body.replace_blocks(blocks, tree);
    }

    /// Retain function blocks that satisfy one predicate.
    pub fn retain_blocks(
        &mut self,
        mut retain: impl FnMut(LocalNodeId<Block>) -> bool,
        tree: &Tree,
    ) {
        let Some(body) = self.body.as_mut() else {
            unreachable!("cannot retain blocks on a function without a body");
        };

        body.blocks.retain(|block| retain(*block));
        body.rebuild_instruction_index(tree);
    }

    /// Insert one block after another block, or append it when the predecessor is absent.
    pub fn insert_block_after(
        &mut self,
        predecessor: LocalNodeId<Block>,
        block: LocalNodeId<Block>,
        tree: &Tree,
    ) {
        let Some(body) = self.body.as_mut() else {
            unreachable!("cannot insert block into a function without a body");
        };

        body.insert_block_after(predecessor, block, tree);
    }

    /// Replace one block's instruction list.
    pub fn replace_block_instructions(
        &mut self,
        block: LocalNodeId<Block>,
        instructions: Vec<LocalNodeId<Instruction>>,
        tree: &mut Tree,
    ) {
        self.replace_block_instruction_index(block, &instructions);
        tree.get_mut(block).instructions = instructions;
    }

    /// Replace one block's instruction index entries.
    pub(crate) fn replace_block_instruction_index(
        &mut self,
        block: LocalNodeId<Block>,
        instructions: &[LocalNodeId<Instruction>],
    ) {
        let Some(body) = self.body.as_mut() else {
            unreachable!("cannot replace instructions on a function without a body");
        };

        body.replace_block_instructions(block, instructions);
    }

    /// Rebuild the function body's instruction index.
    pub fn rebuild_instruction_index(&mut self, tree: &Tree) {
        let Some(body) = self.body.as_mut() else {
            unreachable!("cannot rebuild instruction index on a function without a body");
        };

        body.rebuild_instruction_index(tree);
    }

    /// Replace the function locals in slot order.
    pub fn replace_locals(&mut self, locals: Vec<LocalNodeId<Local>>) {
        let Some(body) = self.body.as_mut() else {
            unreachable!("cannot replace locals on a function without a body");
        };

        body.replace_locals(locals);
    }

    /// Retain function locals that satisfy one predicate.
    pub fn retain_locals(&mut self, mut retain: impl FnMut(LocalNodeId<Local>) -> bool) {
        let Some(body) = self.body.as_mut() else {
            unreachable!("cannot retain locals on a function without a body");
        };

        body.locals.retain(|local| retain(*local));
    }

    /// Replace the SSA value type table.
    pub fn replace_value_types(&mut self, value_types: Vec<Option<LocalNodeId<Type>>>) {
        let Some(body) = self.body.as_mut() else {
            unreachable!("cannot replace value types on a function without a body");
        };

        body.replace_value_types(value_types);
    }

    /// Allocate a new SSA value.
    pub fn next_value(&mut self) -> Value {
        let Some(body) = self.body.as_mut() else {
            unreachable!("cannot allocate SSA value in a function without a body");
        };

        body.next_value()
    }

    /// Allocate a new SSA value and record its type.
    pub fn next_typed_value(&mut self, ty: LocalNodeId<Type>) -> Value {
        let Some(body) = self.body.as_mut() else {
            unreachable!("cannot allocate typed SSA value in a function without a body");
        };

        body.next_typed_value(ty)
    }

    /// Allocate a new SSA value with the same type as an existing value.
    pub fn next_typed_value_like(&mut self, source: Value) -> Value {
        let Some(body) = self.body.as_mut() else {
            unreachable!("cannot allocate typed SSA value in a function without a body");
        };

        body.next_typed_value_like(source)
    }

    /// Recompute the next SSA value id from live function values.
    pub fn recompute_next_value_id(&mut self, tree: &Tree) {
        let parameters = &self.parameters;
        let Some(body) = self.body.as_mut() else {
            unreachable!("cannot recompute SSA values in a function without a body");
        };

        body.recompute_next_value_id(parameters, tree);
    }

    /// Append one local to the function body.
    pub fn add_local(&mut self, local: LocalNodeId<Local>) {
        let Some(body) = self.body.as_mut() else {
            unreachable!("cannot append local to a function without a body");
        };

        body.add_local(local);
    }

    /// Append one block to the function body.
    pub fn add_block(&mut self, block: LocalNodeId<Block>, tree: &Tree) {
        let Some(body) = self.body.as_mut() else {
            unreachable!("cannot append block to a function without a body");
        };

        body.add_block(block, tree);
    }
}
