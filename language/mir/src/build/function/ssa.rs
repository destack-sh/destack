use crate::build::{FunctionBuilder, Variable};
use crate::{
    Block, BlockReference, LocalNodeId, Parameter, Terminator, Type, Value, ValueReference,
};

#[allow(clippy::too_many_arguments)]
impl<'a> FunctionBuilder<'a> {
    pub fn variable(&mut self, ty: LocalNodeId<Type>) -> Variable {
        let variable = Variable::new(self.next_variable_id);
        self.next_variable_id += 1;
        self.variable_types.insert(variable, ty);
        variable
    }

    // block management

    /// Create a new basic block.
    pub fn block(&mut self) -> LocalNodeId<Block> {
        let terminator = self.tree.insert(Terminator::Unreachable);
        let block = self.tree.insert(Block::new(terminator));
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
        let ValueReference::Value(value) = function.parameters[index].value else {
            panic!("missing concrete function parameter value");
        };
        value
    }

    /// Add a block parameter and return its value.
    pub fn add_block_parameter(
        &mut self,
        block: LocalNodeId<Block>,
        ty: LocalNodeId<Type>,
    ) -> Value {
        let value = self.allocate_value();
        let block_data = self.tree.get_mut(block);
        block_data.parameters.push(Parameter {
            value: value.into(),
            ty: ty.into(),
        });
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
            .position(|param| param.value == ValueReference::Value(value))
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
        let terminator_id = self.tree.get(from_block).terminator;
        let terminator = self.tree.get_mut(terminator_id);

        match terminator {
            Terminator::Jump { target } if target.block == BlockReference::Block(to_block) => {
                target.arguments.push(value.into());
            }
            Terminator::Branch {
                then_target,
                else_target,
                ..
            } => {
                if then_target.block == BlockReference::Block(to_block) {
                    then_target.arguments.push(value.into());
                }
                if else_target.block == BlockReference::Block(to_block) {
                    else_target.arguments.push(value.into());
                }
            }
            Terminator::Check {
                success, failure, ..
            } => {
                if success.block == BlockReference::Block(to_block) {
                    success.arguments.push(value.into());
                }
                if failure.block == BlockReference::Block(to_block) {
                    failure.arguments.push(value.into());
                }
            }
            Terminator::Switch { default, cases, .. } => {
                if default.block == BlockReference::Block(to_block) {
                    default.arguments.push(value.into());
                }
                for case in cases {
                    if case.target.block == BlockReference::Block(to_block) {
                        case.target.arguments.push(value.into());
                    }
                }
            }
            Terminator::Invoke {
                normal_target,
                unwind_target,
                ..
            }
            | Terminator::InvokeIndirect {
                normal_target,
                unwind_target,
                ..
            }
            | Terminator::InvokeVirtual {
                normal_target,
                unwind_target,
                ..
            }
            | Terminator::InvokeInterface {
                normal_target,
                unwind_target,
                ..
            } => {
                if normal_target.block == BlockReference::Block(to_block) {
                    normal_target.arguments.push(value.into());
                }
                if unwind_target.block == BlockReference::Block(to_block) {
                    unwind_target.arguments.push(value.into());
                }
            }
            other => {
                panic!(
                    "missing phi predecessor edge from block {from_block:?} to block {to_block:?} \
                     for terminator {other:?}",
                );
            }
        }
    }
}
