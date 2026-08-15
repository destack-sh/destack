use crate as mir;

use super::{Analysis, FunctionCache, Mutation};

/// Operand uses for SSA values in one MIR function.
#[derive(Debug, Clone, Default)]
pub struct UseTable {
    /// Operand uses indexed by value id.
    uses: Vec<Vec<ValueUse>>,
}

/// One operand occurrence of an SSA value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ValueUse {
    /// An instruction reads the value.
    Instruction {
        /// The block that owns the instruction.
        block: mir::BlockId,
        /// The instruction that reads the value.
        instruction: mir::LocalNodeId<mir::Instruction>,
        /// The operand index among the instruction's read values.
        index: usize,
    },
    /// A terminator reads the value.
    Terminator {
        /// The block that owns the terminator.
        block: mir::BlockId,
        /// The operand index among the terminator's read values.
        index: usize,
    },
}

impl UseTable {
    /// Build operand uses for one function.
    pub fn build(function: &mir::Function, tree: &mir::Tree) -> Self {
        let mut value_uses = Self {
            uses: vec![Vec::new(); function.value_capacity()],
        };

        // scan every executable block
        for &block_id in function.blocks() {
            let block = tree.get(block_id);

            // scan instruction operands
            for &instruction_id in &block.instructions {
                let instruction = tree.get(instruction_id);
                for (index, value) in instruction.reads(tree).into_iter().enumerate() {
                    value_uses.record(
                        value,
                        ValueUse::Instruction {
                            block: block_id,
                            instruction: instruction_id,
                            index,
                        },
                    );
                }
            }

            // scan terminator operands
            let terminator = tree.get(block.terminator);
            for (index, value) in terminator.uses(tree).into_iter().enumerate() {
                value_uses.record(
                    value,
                    ValueUse::Terminator {
                        block: block_id,
                        index,
                    },
                );
            }
        }

        value_uses
    }

    /// Return operand uses for one value.
    pub fn uses(&self, value: impl Into<mir::Value>) -> &[ValueUse] {
        let value = value.into();
        let index = value.0 as usize;

        match self.uses.get(index) {
            Some(uses) => uses,
            None => unreachable!("value use outside function value table: {value:?}"),
        }
    }

    /// Return how many operand occurrences read one value.
    pub fn count(&self, value: impl Into<mir::Value>) -> usize {
        self.uses(value).len()
    }

    /// Return whether the value has at least one operand use.
    pub fn is_used(&self, value: impl Into<mir::Value>) -> bool {
        !self.uses(value).is_empty()
    }

    /// Record one operand use.
    fn record(&mut self, value: mir::Value, value_use: ValueUse) {
        let index = value.0 as usize;

        let Some(uses) = self.uses.get_mut(index) else {
            unreachable!("value use outside function value table: {value:?}");
        };

        uses.push(value_use);
    }
}

impl Analysis for UseTable {
    const INVALIDATED_BY: Mutation = Mutation::VALUE;
}

impl UseTable {
    /// Compute value uses for one function.
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
    use crate as mir;

    use super::{UseTable, ValueUse};
    use crate::analyses::tests::TestProgram;

    /// Value uses include instruction operands and terminator operands.
    #[test]
    fn test_collect_value_uses() {
        let (tree, function_id) = TestProgram::parse_function(
            r#"
function test(v0: int32, v1: int32): int32 {
entry(v0: int32, v1: int32):
    v2: int32 = add v0, v1
    v3: int32 = add v2, v1
    jump b1(v3)

b1(v4: int32):
    return v4
}
"#,
        );
        let function = tree.get(function_id);
        let uses = UseTable::build(function, &tree);
        let entry = function.block(0);
        let entry_block = tree.get(entry);
        let exit = function.block(1);

        // v1 is used by both arithmetic instructions
        assert_eq!(
            uses.uses(mir::Value(1)),
            &[
                ValueUse::Instruction {
                    block: entry,
                    instruction: entry_block.instructions[0],
                    index: 1,
                },
                ValueUse::Instruction {
                    block: entry,
                    instruction: entry_block.instructions[1],
                    index: 1,
                }
            ]
        );

        // v3 is passed as a block argument
        assert_eq!(
            uses.uses(mir::Value(3)),
            &[ValueUse::Terminator {
                block: entry,
                index: 0
            }]
        );

        // v4 is returned by the exit block
        assert_eq!(
            uses.uses(mir::Value(4)),
            &[ValueUse::Terminator {
                block: exit,
                index: 0
            }]
        );
    }
}
