use crate as mir;

use crate::{Analysis, Mutation};

/// Operand uses for SSA values in one MIR function.
#[derive(Debug, Clone, Default)]
pub struct UseTable {
    /// First use offset for each value and the final use count.
    offsets: Vec<u32>,
    /// Operand uses grouped by value id.
    uses: Vec<ValueUse>,
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

impl ValueUse {
    /// Return the block containing this use.
    pub fn block(self) -> mir::LocalNodeId<mir::Block> {
        match self {
            Self::Instruction { block, .. } | Self::Terminator { block, .. } => block,
        }
    }
}

impl UseTable {
    /// Build operand uses for one function.
    pub fn analyse(function: &mir::Function, tree: &mir::Tree) -> Self {
        let value_count = function.value_capacity();
        let mut counts = vec![0u32; value_count];

        // count uses for each value
        for &block_id in function.blocks() {
            let block = tree.get(block_id);

            for &instruction_id in &block.instructions {
                let instruction = tree.get(instruction_id);
                for value in instruction.reads(tree) {
                    counts[value.0 as usize] += 1;
                }
            }

            let terminator = tree.get(block.terminator);
            for value in terminator.uses(tree) {
                counts[value.0 as usize] += 1;
            }
        }

        // build dense value ranges
        let mut offsets = Vec::with_capacity(value_count + 1);
        offsets.push(0);
        for count in counts {
            let next = offsets[offsets.len() - 1] + count;
            offsets.push(next);
        }

        let mut cursors = offsets[..value_count].to_vec();
        let mut uses = vec![None; offsets[value_count] as usize];

        // fill each value range in program order
        for &block_id in function.blocks() {
            let block = tree.get(block_id);

            for &instruction_id in &block.instructions {
                let instruction = tree.get(instruction_id);
                for (index, value) in instruction.reads(tree).into_iter().enumerate() {
                    Self::record(
                        value,
                        ValueUse::Instruction {
                            block: block_id,
                            instruction: instruction_id,
                            index,
                        },
                        &mut cursors,
                        &mut uses,
                    );
                }
            }

            let terminator = tree.get(block.terminator);
            for (index, value) in terminator.uses(tree).into_iter().enumerate() {
                Self::record(
                    value,
                    ValueUse::Terminator {
                        block: block_id,
                        index,
                    },
                    &mut cursors,
                    &mut uses,
                );
            }
        }

        let uses = uses
            .into_iter()
            .map(|value_use| match value_use {
                Some(value_use) => value_use,
                None => unreachable!("missing value use"),
            })
            .collect();

        Self { offsets, uses }
    }

    /// Return operand uses for one value.
    pub fn uses(&self, value: impl Into<mir::Value>) -> &[ValueUse] {
        let value = value.into();
        let index = value.0 as usize;

        let Some(range) = self.offsets.get(index..=index + 1) else {
            unreachable!("value use outside function value table: {value:?}");
        };
        let start = range[0] as usize;
        let end = range[1] as usize;

        &self.uses[start..end]
    }

    /// Return how many operand occurrences read one value.
    pub fn count(&self, value: impl Into<mir::Value>) -> usize {
        self.uses(value).len()
    }

    /// Iterate over blocks containing uses of one value.
    pub fn blocks(
        &self,
        value: impl Into<mir::Value>,
    ) -> impl Iterator<Item = mir::LocalNodeId<mir::Block>> + '_ {
        self.uses(value).iter().map(|value_use| value_use.block())
    }

    /// Return whether one value is used outside a block.
    pub fn is_used_outside(
        &self,
        value: impl Into<mir::Value>,
        block: mir::LocalNodeId<mir::Block>,
    ) -> bool {
        self.blocks(value).any(|use_block| use_block != block)
    }

    /// Return whether the value has at least one operand use.
    pub fn is_used(&self, value: impl Into<mir::Value>) -> bool {
        !self.uses(value).is_empty()
    }

    /// Record one operand use.
    fn record(
        value: mir::Value,
        value_use: ValueUse,
        cursors: &mut [u32],
        uses: &mut [Option<ValueUse>],
    ) {
        let index = value.0 as usize;
        let Some(cursor) = cursors.get_mut(index) else {
            unreachable!("value use outside function value table: {value:?}");
        };
        uses[*cursor as usize] = Some(value_use);
        *cursor += 1;
    }
}

impl Analysis for UseTable {
    const INVALIDATED_BY: Mutation = Mutation::CONTROL.union(Mutation::VALUE);
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
        let uses = UseTable::analyse(function, &tree);
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
