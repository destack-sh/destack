use destack_core::FxIndexMap;

use crate::build::FunctionBuilder;
use crate::{Block, BlockParameter, Instruction, LocalNodeId, Terminator, Value, terminator_remap};

#[allow(clippy::too_many_arguments)]
impl<'a> FunctionBuilder<'a> {
    /// Replace all uses of a value with another value in the current function.
    pub(crate) fn replace_value(&mut self, from: Value, to: Value) {
        // skip no op replacements
        if from == to {
            return;
        }

        // update variable definitions
        for value in self.variable_definitions.values_mut() {
            Self::replace_plain_value(value, from, to);
        }

        // update incomplete phis
        for phis in self.incomplete_phis.values_mut() {
            for (_variable, phi_value) in phis {
                Self::replace_plain_value(phi_value, from, to);
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
        let terminator_id = self.tree.get(block).terminator;
        let block_data = self.tree.get_mut(block);
        Self::replace_values_in_parameters(&mut block_data.parameters, from, to);

        let mut terminator = self.tree.get(terminator_id).clone();
        self.replace_value_in_terminator(&mut terminator, from, to);
        self.tree.set(terminator_id, terminator);
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
                Instruction::Error => {}
                Instruction::Const { .. }
                | Instruction::LocalGet { .. }
                | Instruction::LocalAddr { .. }
                | Instruction::GlobalAddr { .. }
                | Instruction::FunctionAddr { .. }
                | Instruction::NewZeroed { .. }
                | Instruction::NewUninit { .. } => {}
                Instruction::Copy {
                    value: environment, ..
                }
                | Instruction::FunctionBind { environment, .. } => {
                    Self::replace_value_in_slot(environment, from, to);
                }
                Instruction::FunctionEnvironment { function, .. } => {
                    Self::replace_value_in_slot(function, from, to);
                }
                Instruction::FunctionEnvironmentCurrent { .. } => {}
                Instruction::ContextCurrent { .. } => {}
                Instruction::ContextReplace { context, .. } => {
                    Self::replace_value_in_slot(context, from, to);
                }
                Instruction::ContextBind {
                    context,
                    variable,
                    value,
                    ..
                } => {
                    Self::replace_value_in_slot(context, from, to);
                    Self::replace_value_in_slot(variable, from, to);
                    Self::replace_value_in_slot(value, from, to);
                }
                Instruction::ContextGet {
                    context,
                    variable,
                    default,
                    ..
                } => {
                    Self::replace_value_in_slot(context, from, to);
                    Self::replace_value_in_slot(variable, from, to);
                    Self::replace_value_in_slot(default, from, to);
                }
                Instruction::Poll | Instruction::Breakpoint => {}
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
                | Instruction::SliceLength {
                    slice: argument, ..
                }
                | Instruction::DynamicBind {
                    payload: argument, ..
                }
                | Instruction::DynamicPayload {
                    dynamic: argument, ..
                }
                | Instruction::DynamicType {
                    dynamic: argument, ..
                }
                | Instruction::Release { value: argument }
                | Instruction::AtomicLoad {
                    pointer: argument, ..
                } => {
                    Self::replace_value_in_slot(argument, from, to);
                }
                Instruction::DynamicRead { dynamic, .. } => {
                    Self::replace_value_in_slot(dynamic, from, to);
                }
                Instruction::DynamicFind { dynamic, key, .. } => {
                    Self::replace_value_in_slot(dynamic, from, to);
                    Self::replace_value_in_slot(key, from, to);
                }
                Instruction::BarrierWrite {
                    object,
                    offset,
                    byte_len,
                } => {
                    Self::replace_value_in_slot(object, from, to);
                    Self::replace_value_in_slot(offset, from, to);
                    Self::replace_value_in_slot(byte_len, from, to);
                }
                Instruction::Call { call, .. } => {
                    call.callee = call
                        .callee
                        .map_values(|value| if value == from { to } else { value });
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
                Instruction::FieldGet { aggregate, .. } => {
                    Self::replace_value_in_slot(aggregate, from, to);
                }
                Instruction::ElementGet { aggregate, .. } => {
                    Self::replace_value_in_slot(aggregate, from, to);
                }
                Instruction::FieldSet {
                    aggregate, value, ..
                }
                | Instruction::ElementSet {
                    aggregate, value, ..
                } => {
                    Self::replace_value_in_slot(aggregate, from, to);
                    Self::replace_value_in_slot(value, from, to);
                }
                Instruction::FieldAddr { aggregate, .. } => {
                    Self::replace_value_in_slot(aggregate, from, to);
                }
                Instruction::VariantNew { payload, .. } => {
                    if let Some(payload) = payload {
                        Self::replace_value_in_slot(payload, from, to);
                    }
                }
                Instruction::VariantTag { variant, .. }
                | Instruction::VariantTagLoad { variant, .. }
                | Instruction::VariantPayload { variant, .. }
                | Instruction::VariantPayloadAddr { variant, .. } => {
                    Self::replace_value_in_slot(variant, from, to);
                }
                Instruction::ElementAddr { base, index, .. } => {
                    Self::replace_value_in_slot(base, from, to);
                    Self::replace_value_in_slot(index, from, to);
                }
                Instruction::SliceView {
                    source,
                    start,
                    length,
                    ..
                } => {
                    Self::replace_value_in_slot(source, from, to);
                    Self::replace_value_in_slot(start, from, to);
                    Self::replace_value_in_slot(length, from, to);
                }
                Instruction::NewComplete { value, .. } => {
                    Self::replace_value_in_slot(value, from, to);
                }
                Instruction::Drop { value } => {
                    Self::replace_value_in_slot(value, from, to);
                }
                Instruction::NewSliceZeroed { length, .. }
                | Instruction::NewSliceUninit { length, .. } => {
                    Self::replace_value_in_slot(length, from, to);
                }
                Instruction::AtomicStore { pointer, value, .. }
                | Instruction::Store { pointer, value } => {
                    Self::replace_value_in_slot(pointer, from, to);
                    Self::replace_value_in_slot(value, from, to);
                }
                Instruction::AtomicCompareExchange {
                    pointer,
                    expected,
                    new_value,
                    ..
                } => {
                    Self::replace_value_in_slot(pointer, from, to);
                    Self::replace_value_in_slot(expected, from, to);
                    Self::replace_value_in_slot(new_value, from, to);
                }
                Instruction::AtomicRmw { pointer, value, .. } => {
                    Self::replace_value_in_slot(pointer, from, to);
                    Self::replace_value_in_slot(value, from, to);
                }
                Instruction::AtomicFence { .. } => {}
                Instruction::Assume { condition } => {
                    Self::replace_value_in_slot(condition, from, to);
                }
                Instruction::ProfileIncrement { .. } => {}
                Instruction::ProfileSample { value, .. } => {
                    Self::replace_value_in_slot(value, from, to);
                }

                // arguments stored externally
                Instruction::Aggregate { .. } | Instruction::Intrinsic { .. } => {}
            }
            argument_slice
        };

        // update external arguments
        if let Some(argument_slice) = argument_slice {
            let start = argument_slice.start as usize;
            let end = start + argument_slice.count as usize;
            Self::replace_values_in_slice(&mut self.tree.values[start..end], from, to);
        }
    }

    /// Replace a value in a terminator.
    fn replace_value_in_terminator(&mut self, terminator: &mut Terminator, from: Value, to: Value) {
        let block_map = FxIndexMap::default();
        let substitutions = FxIndexMap::from_iter([(from, to)]);

        terminator_remap(self.tree, terminator, &block_map, &substitutions);
    }

    /// Replace a value in a slot.
    fn replace_plain_value(value: &mut Value, from: Value, to: Value) {
        if *value == from {
            *value = to;
        }
    }

    /// Replace a value in a recoverable slot.
    fn replace_value_in_slot(value: &mut Value, from: Value, to: Value) {
        if *value == from {
            *value = to;
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
    fn replace_values_in_parameters(parameters: &mut [BlockParameter], from: Value, to: Value) {
        // update each parameter value
        for parameter in parameters {
            Self::replace_value_in_slot(&mut parameter.value, from, to);
        }
    }
}
