use crate as mir;

use super::{Analysis, DominatorTable, FunctionCache, Mutation};

/// Definitions for one MIR function.
#[derive(Debug, Clone, Default)]
pub struct DefinitionTable {
    /// Definition site for each SSA value.
    definitions: Vec<Option<ValueDefinition>>,
    /// First input offset for each value and the final input count.
    block_parameter_offsets: Vec<u32>,
    /// Inputs grouped by block parameter value.
    block_parameter_values: Vec<mir::Value>,
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

        // record function parameter definitions
        for (index, parameter) in function.parameters.iter().enumerate() {
            definitions[parameter.value.0 as usize] =
                Some(ValueDefinition::FunctionParameter(index));
        }

        // record block parameter and instruction definitions
        for &block_id in function.blocks() {
            let block = tree.get(block_id);

            // entry parameters mirror the function parameters
            if Some(block_id) != function.entry() {
                for (index, parameter) in block.parameters.iter().enumerate() {
                    definitions[parameter.value.0 as usize] =
                        Some(ValueDefinition::BlockParameter {
                            block: block_id,
                            index,
                        });
                }
            }

            for &instruction_id in &block.instructions {
                let instruction = tree.get(instruction_id);
                if let Some(destination) = instruction.destination() {
                    definitions[destination.0 as usize] = Some(ValueDefinition::Instruction {
                        block: block_id,
                        instruction: instruction_id,
                    });
                }
            }
        }

        let (block_parameter_offsets, block_parameter_values) =
            Self::build_block_parameter_values(function, tree);

        Self {
            definitions,
            block_parameter_offsets,
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

    /// Return whether one value is available at a block exit.
    pub fn is_available_at_exit(
        &self,
        value: impl Into<mir::Value>,
        block: mir::LocalNodeId<mir::Block>,
        dominators: &DominatorTable,
    ) -> bool {
        match self.definition(value) {
            Some(ValueDefinition::FunctionParameter(_)) => true,
            Some(ValueDefinition::BlockParameter {
                block: definition, ..
            })
            | Some(ValueDefinition::Instruction {
                block: definition, ..
            }) => dominators.dominates(definition, block),
            None => false,
        }
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

    /// Return values passed to one block parameter.
    pub fn block_parameter_values(&self, parameter: mir::Value) -> &[mir::Value] {
        let index = parameter.id() as usize;
        let offsets = self
            .block_parameter_offsets
            .get(index..=index + 1)
            .unwrap_or_else(|| unreachable!("value outside definition table: {parameter:?}"));
        let start = offsets[0] as usize;
        let end = offsets[1] as usize;

        &self.block_parameter_values[start..end]
    }

    /// Build block parameter definitions from predecessor arguments.
    fn build_block_parameter_values(
        function: &mir::Function,
        tree: &mir::Tree,
    ) -> (Vec<u32>, Vec<mir::Value>) {
        let value_count = function.value_capacity();
        let mut entries = Vec::new();

        // collect arguments from every outgoing target
        for &block_id in function.blocks() {
            let block = tree.get(block_id);
            let terminator = tree.get(block.terminator);

            for (edge, target) in terminator.targets(tree, block_id) {
                Self::add_block_parameter_values(
                    &mut entries,
                    terminator,
                    edge.successor,
                    target,
                    tree,
                );
            }
        }

        // group inputs by block parameter value
        entries.sort_by_key(|(parameter, _)| parameter.id());
        let mut offsets = vec![0u32; value_count + 1];
        for (parameter, _) in &entries {
            offsets[parameter.id() as usize + 1] += 1;
        }
        for value in 0..value_count {
            offsets[value + 1] += offsets[value];
        }

        let values = entries.into_iter().map(|(_, value)| value).collect();

        (offsets, values)
    }

    /// Add one target's arguments to the block parameter value map.
    fn add_block_parameter_values(
        entries: &mut Vec<(mir::Value, mir::Value)>,
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
            entries.push((parameter.value, *argument));
        }
    }
}

impl Analysis for DefinitionTable {
    const INVALIDATED_BY: Mutation = Mutation::CONTROL.union(Mutation::VALUE);
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
    use super::{DefinitionTable, ValueDefinition};
    use crate::analyses::tests::TestProgram;

    /// Preserve function parameters through their entry-block mirrors.
    #[test]
    fn test_defines_function_parameters() {
        let (tree, function_id) = TestProgram::parse_function(
            r#"
function test(v0: int32): int32 {
entry(v0: int32):
    return v0
}
"#,
        );
        let function = tree.get(function_id);
        let definitions = DefinitionTable::build(function, &tree);
        let parameter = function.parameters[0].value;

        assert_eq!(
            definitions.definition(parameter),
            Some(ValueDefinition::FunctionParameter(0))
        );
    }

    /// Fallible allocation success results are not treated as edge arguments.
    #[test]
    fn test_block_parameter_values_skip_fallible_allocation_result() {
        let (tree, function_id) = TestProgram::parse_function(
            r#"
function test(v0: int64, v1: int32): int32 {
entry(v0: int64, v1: int32):
    new.slice.uninit.try int32, v0 => b1(v1) | b2(v1)

b1(v2: uninit<slice<int32, managed, mutable, local>>, v3: int32):
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
            definitions
                .block_parameter_values(success_result)
                .is_empty()
        );

        // explicit payloads on both edges come from the same source value
        assert_eq!(
            definitions.block_parameter_values(success_payload),
            definitions.block_parameter_values(failure_payload)
        );
    }
}
