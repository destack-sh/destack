use crate::build::{FunctionBuilder, Variable};
use crate::{
    Block, CheckConstraint, Instruction, LocalNodeId, Terminator, Type, TypedValue, Value,
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
                Instruction::VectorSelect {
                    mask,
                    then_value,
                    else_value,
                    ..
                } => {
                    Self::replace_value_in_slot(mask, from, to);
                    Self::replace_value_in_slot(then_value, from, to);
                    Self::replace_value_in_slot(else_value, from, to);
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
                Instruction::TensorSelect {
                    mask,
                    then_value,
                    else_value,
                    ..
                } => {
                    Self::replace_value_in_slot(mask, from, to);
                    Self::replace_value_in_slot(then_value, from, to);
                    Self::replace_value_in_slot(else_value, from, to);
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
            Terminator::Call {
                arguments,
                normal_arguments,
                unwind_arguments,
                ..
            } => {
                Self::replace_values_in_slice(arguments, from, to);
                Self::replace_values_in_slice(normal_arguments, from, to);
                Self::replace_values_in_slice(unwind_arguments, from, to);
            }
            Terminator::CallIndirect {
                callee,
                env,
                arguments,
                normal_arguments,
                unwind_arguments,
                ..
            } => {
                Self::replace_value_in_slot(callee, from, to);
                if let Some(env) = env {
                    Self::replace_value_in_slot(env, from, to);
                }

                Self::replace_values_in_slice(arguments, from, to);
                Self::replace_values_in_slice(normal_arguments, from, to);
                Self::replace_values_in_slice(unwind_arguments, from, to);
            }
            Terminator::CallVirtual {
                receiver,
                arguments,
                normal_arguments,
                unwind_arguments,
                ..
            }
            | Terminator::CallInterface {
                receiver,
                arguments,
                normal_arguments,
                unwind_arguments,
                ..
            } => {
                Self::replace_value_in_slot(receiver, from, to);
                Self::replace_values_in_slice(arguments, from, to);
                Self::replace_values_in_slice(normal_arguments, from, to);
                Self::replace_values_in_slice(unwind_arguments, from, to);
            }
            Terminator::Throw { value } => {
                Self::replace_value_in_slot(value, from, to);
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
            CheckConstraint::ReceiverType { receiver, .. } => {
                Self::replace_value_in_slot(receiver, from, to);
            }
            CheckConstraint::Implements { receiver, .. } => {
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
}
