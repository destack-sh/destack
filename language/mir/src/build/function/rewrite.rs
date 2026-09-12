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
            instruction.map_uses(|value| if value == from { to } else { value });

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

    /// Replace values in a slice.
    fn replace_values_in_slice(values: &mut [Value], from: Value, to: Value) {
        // update each value
        for value in values {
            Self::replace_plain_value(value, from, to);
        }
    }

    /// Replace values in a parameter list.
    fn replace_values_in_parameters(parameters: &mut [BlockParameter], from: Value, to: Value) {
        // update each parameter value
        for parameter in parameters {
            Self::replace_plain_value(&mut parameter.value, from, to);
        }
    }
}
