use std::collections::HashMap;

use crate as mir;

use super::{Analysis, FunctionCache, Mutation};

/// Definitions and integer constants for one MIR function.
#[derive(Debug, Clone, Default)]
pub struct DefinitionTable {
    /// Definition site for each SSA value.
    definitions: Vec<Option<ValueDefinition>>,
    /// Integer constant value for each SSA value known to be constant.
    constants: Vec<Option<i64>>,
    /// Values written into each local.
    local_values: HashMap<mir::LocalId, Vec<mir::Value>>,
    /// Values passed to each block parameter.
    block_parameter_values: HashMap<mir::Value, Vec<mir::Value>>,
}

/// Definition site for one SSA value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ValueDefinition {
    /// Function parameter definition.
    FunctionParameter(usize),
    /// Block parameter definition.
    BlockParameter {
        /// The block that owns the parameter.
        block: mir::LocalNodeId<mir::Block>,
        /// The parameter index inside the block.
        index: usize,
    },
    /// Instruction result definition.
    Instruction {
        /// The block that owns the instruction.
        block: mir::LocalNodeId<mir::Block>,
        /// The instruction id.
        instruction: mir::LocalNodeId<mir::Instruction>,
    },
}

impl ValueDefinition {
    /// Return the defining block when this value is block local.
    pub fn block(self) -> Option<mir::LocalNodeId<mir::Block>> {
        match self {
            Self::FunctionParameter(_) => None,
            Self::BlockParameter { block, .. } | Self::Instruction { block, .. } => Some(block),
        }
    }

    /// Return the defining instruction when this value is an instruction result.
    pub fn instruction(self) -> Option<mir::LocalNodeId<mir::Instruction>> {
        match self {
            Self::Instruction { instruction, .. } => Some(instruction),
            _ => None,
        }
    }
}

impl DefinitionTable {
    /// Build value definitions for one function.
    pub fn build(function: &mir::Function, tree: &mir::Tree) -> Self {
        let value_count = function.value_capacity();
        let mut definitions = vec![None; value_count];
        let mut constants = vec![None; value_count];
        let mut local_values = HashMap::new();

        // record function parameter definitions
        for (index, parameter) in function.parameters.iter().enumerate() {
            definitions[parameter.value.0 as usize] =
                Some(ValueDefinition::FunctionParameter(index));
        }

        // record block parameter and instruction definitions
        for &block_id in function.blocks() {
            let block = tree.get(block_id);

            for (index, parameter) in block.parameters.iter().enumerate() {
                definitions[parameter.value.0 as usize] = Some(ValueDefinition::BlockParameter {
                    block: block_id,
                    index,
                });
            }

            for &instruction_id in &block.instructions {
                let instruction = tree.get(instruction_id);
                if let Some(destination) = instruction.destination() {
                    definitions[destination.0 as usize] = Some(ValueDefinition::Instruction {
                        block: block_id,
                        instruction: instruction_id,
                    });
                }

                if let mir::Instruction::Const { destination, value } = instruction
                    && let mir::Constant::Int { value, .. } = value
                    && let Ok(value) = i64::try_from(*value)
                {
                    constants[destination.0 as usize] = Some(value);
                }

                if let mir::Instruction::LocalSet { local, value } = instruction {
                    local_values
                        .entry(*local)
                        .or_insert_with(Vec::new)
                        .push(*value);
                }
            }
        }

        let block_parameter_values = Self::build_block_parameter_values(function, tree);

        Self {
            definitions,
            constants,
            local_values,
            block_parameter_values,
        }
    }

    /// Return the definition site for one value.
    pub fn definition(&self, value: impl Into<mir::Value>) -> Option<ValueDefinition> {
        let value = value.into();

        self.definitions.get(value.0 as usize).copied().flatten()
    }

    /// Return the defining instruction for one value.
    pub fn instruction(
        &self,
        value: impl Into<mir::Value>,
    ) -> Option<mir::LocalNodeId<mir::Instruction>> {
        self.definition(value)
            .and_then(ValueDefinition::instruction)
    }

    /// Return the defining block for one value.
    pub fn block(&self, value: impl Into<mir::Value>) -> Option<mir::LocalNodeId<mir::Block>> {
        self.definition(value).and_then(ValueDefinition::block)
    }

    /// Return the integer constant for one value.
    pub fn int_constant(&self, value: impl Into<mir::Value>) -> Option<i64> {
        let value = value.into();

        self.constants.get(value.0 as usize).copied().flatten()
    }

    /// Return the raw instruction definition map.
    pub fn instruction_map(&self) -> HashMap<mir::Value, mir::LocalNodeId<mir::Instruction>> {
        self.definitions
            .iter()
            .enumerate()
            .filter_map(|(value, definition)| {
                definition
                    .and_then(ValueDefinition::instruction)
                    .map(|instruction| (mir::Value(value as u32), instruction))
            })
            .collect()
    }

    /// Iterate over SSA values and their definition sites.
    pub fn definitions(&self) -> impl Iterator<Item = (mir::Value, ValueDefinition)> + '_ {
        self.definitions
            .iter()
            .enumerate()
            .filter_map(|(value, definition)| {
                definition.map(|definition| (mir::Value(value as u32), definition))
            })
    }

    /// Return values written into locals.
    pub fn local_values(&self) -> &HashMap<mir::LocalId, Vec<mir::Value>> {
        &self.local_values
    }

    /// Return values passed to block parameters.
    pub fn block_parameter_values(&self) -> &HashMap<mir::Value, Vec<mir::Value>> {
        &self.block_parameter_values
    }

    /// Build block parameter definitions from predecessor arguments.
    fn build_block_parameter_values(
        function: &mir::Function,
        tree: &mir::Tree,
    ) -> HashMap<mir::Value, Vec<mir::Value>> {
        let mut values = HashMap::new();

        // collect arguments from every outgoing target
        for &block_id in function.blocks() {
            let block = tree.get(block_id);
            let terminator = tree.get(block.terminator);

            for (edge, target) in terminator.targets(tree, block_id) {
                Self::add_block_parameter_values(
                    &mut values,
                    terminator,
                    edge.successor,
                    target,
                    tree,
                );
            }
        }

        values
    }

    /// Add one target's arguments to the block parameter value map.
    fn add_block_parameter_values(
        values: &mut HashMap<mir::Value, Vec<mir::Value>>,
        terminator: &mir::Terminator,
        successor: mir::Successor,
        target: &mir::BlockTarget,
        tree: &mir::Tree,
    ) {
        let parameters = terminator
            .target_parameters(tree, successor, target)
            .unwrap_or_else(|| unreachable!("verified block target has invalid argument count"));
        let arguments = target.arguments(tree);

        // pair target arguments with the destination block parameters
        for (parameter, argument) in parameters.iter().zip(arguments) {
            values.entry(parameter.value).or_default().push(*argument);
        }
    }
}

impl Analysis for DefinitionTable {
    const INVALIDATED_BY: Mutation = Mutation::VALUE;
}

impl DefinitionTable {
    /// Compute value definitions for one function.
    pub(crate) fn compute(
        function: &mir::Function,
        tree: &mir::Tree,
        _analyses: &mut FunctionCache,
    ) -> Self {
        Self::build(function, tree)
    }
}

#[cfg(test)]
mod tests {
    use super::DefinitionTable;
    use crate::analyses::tests::TestProgram;

    /// Fallible allocation success results are not treated as edge arguments.
    #[test]
    fn test_block_parameter_values_skip_fallible_allocation_result() {
        let (tree, function_id) = TestProgram::parse_function(
            r#"
function test(v0: int64, v1: int32): int32 {
entry(v0: int64, v1: int32):
    new.slice.uninit.try int32, v0 => b1(v1) | b2(v1)

b1(v2: uninit<slice<int32, managed, mutable>>, v3: int32):
    return v3

b2(v4: int32):
    return v4
}
"#,
        );
        let function = tree.get(function_id);
        let definitions = DefinitionTable::build(function, &tree);
        let success = tree.get(function.block(1));
        let failure = tree.get(function.block(2));
        let success_result = success.parameters[0].value;
        let success_payload = success.parameters[1].value;
        let failure_payload = failure.parameters[0].value;

        // success result is produced by the terminator and has no edge argument
        assert!(
            !definitions
                .block_parameter_values()
                .contains_key(&success_result)
        );

        // explicit payloads on both edges come from the same source value
        assert_eq!(
            definitions.block_parameter_values()[&success_payload],
            definitions.block_parameter_values()[&failure_payload]
        );
    }
}
