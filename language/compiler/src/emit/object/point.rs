use std::collections::HashMap;
use std::iter;

use destack_artifact::{MirOptimized, Point};
use destack_mir as mir;

/// Emitted points keyed by MIR operation identity.
#[derive(Debug)]
pub(super) struct PointIndex {
    /// Instruction points.
    instructions: HashMap<mir::LocalNodeId<mir::Instruction>, Point>,
    /// Block entry points.
    blocks: HashMap<mir::BlockId, Point>,
    /// Block terminator points.
    terminators: HashMap<mir::BlockId, Point>,
}

impl PointIndex {
    /// Index every emitted operation in executable block order.
    pub(super) fn build(optimized: &MirOptimized) -> Self {
        let mut instructions = HashMap::new();
        let mut blocks = HashMap::new();
        let mut terminators = HashMap::new();

        // assign function-local operation indices in emitted bytecode order
        for (function_id, function) in optimized.tree.iter_nodes::<mir::Function>() {
            let Some(body) = &function.body else {
                continue;
            };
            let mut operation = 0;

            let entry = body.entry();
            let ordered_blocks = iter::once(entry).chain(
                body.blocks()
                    .iter()
                    .copied()
                    .filter(|block| *block != entry),
            );

            for block_id in ordered_blocks {
                let block = optimized.tree.get(block_id);
                blocks.insert(block_id, Point::new(function_id, operation));

                for instruction_id in &block.instructions {
                    let point = Point::new(function_id, operation);
                    instructions.insert(*instruction_id, point);
                    operation += 1;
                }

                let point = Point::new(function_id, operation);
                terminators.insert(block_id, point);
                operation += 1;
            }
        }

        Self {
            instructions,
            blocks,
            terminators,
        }
    }

    /// Return one instruction point.
    pub(super) fn instruction(&self, instruction: mir::LocalNodeId<mir::Instruction>) -> Point {
        self.instructions[&instruction]
    }

    /// Return one block entry point.
    pub(super) fn block(&self, block: mir::BlockId) -> Point {
        self.blocks[&block]
    }

    /// Sort blocks into emitted operation order.
    pub(super) fn order_blocks(&self, blocks: &mut [mir::BlockId]) {
        blocks.sort_unstable_by_key(|block| self.block(*block).operation);
    }

    /// Return one block terminator point.
    pub(super) fn terminator(&self, block: mir::BlockId) -> Point {
        self.terminators[&block]
    }
}
