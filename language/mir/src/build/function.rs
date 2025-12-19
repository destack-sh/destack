use indexmap::{IndexMap, IndexSet};

use crate::{
    AllocationMode, BinaryOperator, Block, Constant, Function, Global, Instruction, Linkage, Local,
    LocalNodeId, Mutability, NodeTree, Ownership, Terminator, Type, TypedValue, UnaryOperator,
    Value,
};

use super::Variable;

/// Builder for constructing a single MIR function with automatic SSA construction.
///
/// Implements the algorithm from
///  - "Simple and Efficient Construction of Static Single Assignment Form" (Braun et al., 2013)
///    <https://c9x.me/compile/bib/braun13cc.pdf>
///  - Cranelift (<https://github.com/bytecodealliance/wasmtime/tree/main/cranelift>)
///
/// # Usage
///
/// 1. Create blocks with `create_block()`.
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
///
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

impl<'a> FunctionBuilder<'a> {
    /// Create a new function builder.
    /// NOTE: the entry block is NOT created automatically (call `create_block()` first).
    pub fn new(
        tree: &'a mut NodeTree,
        name: destack_base::StringId,
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

        // blank function (entry will be set in finish())
        let function = Function {
            name,
            parameters,
            return_type,
            linkage: Linkage::Local,
            allocation_mode: AllocationMode::Any,
            coroutine: None,
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

    /// Get the function id being built.
    pub fn function_id(&self) -> LocalNodeId<Function> {
        self.function_id
    }

    /// Allocate a new SSA value.
    fn allocate_value(&mut self) -> Value {
        let value = Value::new(self.next_value_id);
        self.next_value_id += 1;
        value
    }

    /// Create a new variable for SSA construction.
    ///
    /// Variables represent mutable bindings from the source language. During SSA
    /// construction, they are mapped to SSA values, with block parameters inserted
    /// at join points as needed.
    pub fn create_variable(&mut self, ty: LocalNodeId<Type>) -> Variable {
        let variable = Variable::new(self.next_variable_id);
        self.next_variable_id += 1;
        self.variable_types.insert(variable, ty);
        variable
    }

    // block management

    /// Create a new basic block.
    pub fn create_block(&mut self) -> LocalNodeId<Block> {
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
            self.variable_definitions
                .insert((block, variable), replacement);
            replacement
        } else {
            // phi is needed: add jump arguments from predecessors
            for (predecessor, predecessor_value) in operand_values {
                self.add_phi_argument(predecessor, block, predecessor_value);
            }
            phi_value
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
        self.insert_instruction(Instruction::Const {
            destination,
            value: constant,
        });
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
        destination
    }

    // instruction builders: binary operations

    /// Insert a binary operation.
    fn binary(&mut self, operator: BinaryOperator, left_value: Value, right_value: Value) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::Binary {
            destination,
            operator,
            left: left_value,
            right: right_value,
        });
        destination
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

    /// Insert a unary operation.
    fn unary(&mut self, operator: UnaryOperator, argument_value: Value) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::Unary {
            destination,
            operator,
            argument: argument_value,
        });
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
    pub fn create_local(
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

    /// Load from a local variable.
    pub fn local_get(&mut self, local: LocalNodeId<Local>) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::LocalGet { destination, local });
        destination
    }

    /// Store to a local variable.
    pub fn local_set(&mut self, local: LocalNodeId<Local>, value: Value) {
        self.insert_instruction(Instruction::LocalSet { local, value });
    }

    /// Get the address of a mutable global variable.
    pub fn global_addr(&mut self, global: LocalNodeId<Global>) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::GlobalAddr {
            destination,
            global,
        });
        destination
    }

    /// Load the value of an immutable global constant.
    pub fn global_const(&mut self, global: LocalNodeId<Global>) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::GlobalConst {
            destination,
            global,
        });
        destination
    }

    /// Load from a pointer.
    pub fn load(&mut self, pointer_value: Value) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::Load {
            destination,
            pointer: pointer_value,
        });
        destination
    }

    /// Store to a pointer.
    pub fn store(&mut self, pointer_value: Value, value: Value) {
        self.insert_instruction(Instruction::Store {
            pointer: pointer_value,
            value,
        });
    }

    // instruction builders: allocation

    /// Allocate a managed (runtime-tracked) struct.
    /// Returns a `ManagedReference<T>`.
    pub fn managed_alloc(&mut self, layout: LocalNodeId<Type>) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::ManagedAlloc {
            destination,
            layout,
        });
        destination
    }

    /// Allocate a managed array.
    /// Returns a `ManagedReference<[T]>`.
    pub fn managed_alloc_array(&mut self, element: LocalNodeId<Type>, length: Value) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::ManagedAllocArray {
            destination,
            element,
            length,
        });
        destination
    }

    /// Allocate raw memory on the heap.
    /// Returns a `RawPointer<T>`. Caller must free with `raw.free`.
    pub fn raw_alloc(&mut self, layout: LocalNodeId<Type>) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::RawAlloc {
            destination,
            layout,
        });
        destination
    }

    /// Free raw heap memory previously allocated with `raw.alloc`.
    pub fn raw_free(&mut self, pointer: Value) {
        self.insert_instruction(Instruction::RawFree { pointer });
    }

    /// Allocate on the stack (lives until function returns).
    /// Returns a `RawPointer<T>`.
    pub fn stack_alloc(&mut self, layout: LocalNodeId<Type>) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::StackAlloc {
            destination,
            layout,
        });
        destination
    }

    // instruction builders: function calls

    /// Call a function.
    pub fn call(
        &mut self,
        function: LocalNodeId<Function>,
        argument_values: Vec<Value>,
    ) -> Option<Value> {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::Call {
            destination: Some(destination),
            function,
            arguments: argument_values,
        });
        Some(destination)
    }

    /// Call a function with no return value.
    pub fn call_void(&mut self, function: LocalNodeId<Function>, argument_values: Vec<Value>) {
        self.insert_instruction(Instruction::Call {
            destination: None,
            function,
            arguments: argument_values,
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

    /// Insert an instruction into the current block.
    fn insert_instruction(&mut self, instruction: Instruction) {
        let instruction_id = self.tree.insert(instruction);
        let block = self.current_block();
        let block_data = self.tree.get_mut(block);
        block_data.instructions.push(instruction_id);
    }

    /// Finish building the function.
    ///
    /// Seals all remaining unsealed blocks and returns the function id.
    pub fn finish(mut self) -> LocalNodeId<Function> {
        // seal any remaining unsealed blocks
        self.seal_all_blocks();

        // set entry block to the first created block
        let entry_block = self.blocks[0];

        // update function
        let function = self.tree.get_mut(self.function_id);
        function.entry = Some(entry_block);
        function.blocks = self.blocks;
        function.next_value_id = self.next_value_id;

        self.function_id
    }
}
