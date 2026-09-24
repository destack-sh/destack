use crate as mir;

use crate::{Analysis, DominatorTable, Mutation};

/// Definitions for one MIR function.
#[derive(Debug, Clone, Default)]
pub struct DefinitionTable {
    /// Definition site for each SSA value.
    definitions: Vec<Option<ValueDefinition>>,

    /// First incoming edge offset for each value and the final input count.
    input_offsets: Vec<u32>,
    /// Incoming edges grouped by block parameter value.
    inputs: Vec<ValueInput>,
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

/// One incoming definition of a block parameter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ValueInput {
    /// The incoming control flow edge.
    pub edge: mir::Edge,
    /// The explicit argument, absent when the terminator produces the value.
    pub argument: Option<mir::Value>,
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
    /// Analyse value definitions and incoming block arguments for one function.
    pub fn analyse(function: &mir::Function, tree: &mir::Tree) -> Self {
        // allocate one definition entry for each SSA value
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

            // preserve the definitions shared by function and entry parameters
            if Some(block_id) != function.entry() {
                for (index, parameter) in block.parameters.iter().enumerate() {
                    definitions[parameter.value.0 as usize] =
                        Some(ValueDefinition::BlockParameter {
                            block: block_id,
                            index,
                        });
                }
            }

            // record each instruction result
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

        // index the incoming definitions of block parameters
        let (input_offsets, inputs) = Self::build_inputs(function, tree);

        Self {
            definitions,
            input_offsets,
            inputs,
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

    /// Return every incoming definition of one block parameter.
    pub fn inputs(&self, parameter: mir::Value) -> &[ValueInput] {
        let index = parameter.id() as usize;
        let offsets = self
            .input_offsets
            .get(index..=index + 1)
            .unwrap_or_else(|| unreachable!("value outside definition table: {parameter:?}"));
        let start = offsets[0] as usize;
        let end = offsets[1] as usize;

        &self.inputs[start..end]
    }

    /// Build block parameter definitions from predecessor arguments.
    fn build_inputs(function: &mir::Function, tree: &mir::Tree) -> (Vec<u32>, Vec<ValueInput>) {
        let value_count = function.value_capacity();
        let mut entries = Vec::new();

        // collect arguments from every outgoing target
        for &block_id in function.blocks() {
            let block = tree.get(block_id);
            let terminator = tree.get(block.terminator);

            for (edge, target) in terminator.targets(tree, block_id) {
                Self::add_inputs(&mut entries, terminator, edge, target, tree);
            }
        }

        // group inputs by block parameter value
        entries.sort_by_key(|(parameter, _)| parameter.id());
        let mut offsets = vec![0u32; value_count + 1];
        for (parameter, _) in &entries {
            offsets[parameter.id() as usize + 1] += 1;
        }

        // convert input counts into contiguous ranges
        for value in 0..value_count {
            offsets[value + 1] += offsets[value];
        }

        let values = entries.into_iter().map(|(_, value)| value).collect();

        (offsets, values)
    }

    /// Record explicit arguments and values produced by one incoming edge.
    fn add_inputs(
        entries: &mut Vec<(mir::Value, ValueInput)>,
        terminator: &mir::Terminator,
        edge: mir::Edge,
        target: &mir::BlockTarget,
        tree: &mir::Tree,
    ) {
        // require the edge's explicit arguments to match its destination parameters
        let parameters = terminator
            .target_parameters(tree, edge.successor, target)
            .unwrap_or_else(|| unreachable!("verified block target has invalid argument count"));
        let arguments = target.arguments(tree);
        let result_count = terminator.target_result_count(tree, edge.successor);

        // record values produced by the terminator on this edge
        for parameter in tree.get(target.block).parameters.iter().take(result_count) {
            entries.push((
                parameter.value,
                ValueInput {
                    edge,
                    argument: None,
                },
            ));
        }

        // record each explicit argument occurrence
        for (parameter, &argument) in parameters.iter().zip(arguments) {
            entries.push((
                parameter.value,
                ValueInput {
                    edge,
                    argument: Some(argument),
                },
            ));
        }
    }
}

impl Analysis for DefinitionTable {
    const INVALIDATED_BY: Mutation = Mutation::CONTROL.union(Mutation::VALUE);
}

#[cfg(test)]
mod tests {
    use super::{DefinitionTable, ValueDefinition, ValueInput};
    use crate::analyses::tests::TestModule;
    use crate::{Edge, Successor, Value};

    /// Distinguish function parameters from instruction results.
    #[test]
    fn test_identify_parameter_and_instruction_definitions() {
        let (tree, function_id) = TestModule::parse_function(
            r#"
function test(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = add v0, v0
    return v1
}
"#,
        );
        let function = tree.get(function_id);
        let definitions = DefinitionTable::analyse(function, &tree);
        let block = function.block(0);
        let instruction = tree.get(block).instructions[0];

        assert_eq!(
            definitions.definitions().collect::<Vec<_>>(),
            vec![
                (Value(0), ValueDefinition::FunctionParameter(0)),
                (
                    Value(1),
                    ValueDefinition::Instruction { block, instruction }
                ),
            ]
        );
        assert_eq!(definitions.inputs(Value(0)), &[]);
        assert_eq!(definitions.inputs(Value(1)), &[]);
    }

    /// Distinguish allocation results from explicit incoming arguments.
    #[test]
    fn test_identify_edge_results_and_arguments() {
        let (tree, function_id) = TestModule::parse_function(
            r#"
function test(v0: int64, v1: int32): int32 {
entry(v0: int64, v1: int32):
    new.slice.uninit.try int32, v0, local => b1(v1) | b2(v1)

b1(v2: uninit<slice<int32, managed, mutable, local>>, v3: int32):
    return v3

b2(v4: int32):
    return v4
}
"#,
        );
        let function = tree.get(function_id);
        let definitions = DefinitionTable::analyse(function, &tree);
        let success = tree.get(function.block(1));
        let failure = tree.get(function.block(2));
        let success_result = success.parameters[0].value;
        let success_payload = success.parameters[1].value;
        let failure_payload = failure.parameters[0].value;

        let source = function.block(0);
        let normal = Edge::new(source, Successor::NewSuccess, function.block(1));
        let failure = Edge::new(source, Successor::NewFailure, function.block(2));

        assert_eq!(
            definitions.inputs(success_result),
            &[ValueInput {
                edge: normal,
                argument: None
            }]
        );
        assert_eq!(
            definitions.inputs(success_payload),
            &[ValueInput {
                edge: normal,
                argument: Some(Value(1))
            }]
        );
        assert_eq!(
            definitions.inputs(failure_payload),
            &[ValueInput {
                edge: failure,
                argument: Some(Value(1))
            }]
        );
    }

    /// Preserve both incoming arguments when a branch selects the same destination.
    #[test]
    fn test_preserve_repeated_incoming_edges() {
        let (tree, function_id) = TestModule::parse_function(
            r#"
function test(v0: boolean, v1: int32, v2: int32): int32 {
entry(v0: boolean, v1: int32, v2: int32):
    branch v0 => join(v1) | join(v2)

join(v3: int32):
    return v3
}
"#,
        );
        let function = tree.get(function_id);
        let table = DefinitionTable::analyse(function, &tree);
        let entry = function.block(0);
        let join = function.block(1);

        assert_eq!(
            table.inputs(Value(3)),
            &[
                ValueInput {
                    edge: Edge::new(entry, Successor::BranchThen, join),
                    argument: Some(Value(1))
                },
                ValueInput {
                    edge: Edge::new(entry, Successor::BranchElse, join),
                    argument: Some(Value(2))
                },
            ]
        );
        assert_eq!(
            table.definitions().collect::<Vec<_>>(),
            vec![
                (Value(0), ValueDefinition::FunctionParameter(0)),
                (Value(1), ValueDefinition::FunctionParameter(1)),
                (Value(2), ValueDefinition::FunctionParameter(2)),
                (
                    Value(3),
                    ValueDefinition::BlockParameter {
                        block: join,
                        index: 0
                    }
                ),
            ]
        );
    }
}
