use std::collections::HashMap;

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
    /// Index every emitted operation in stable function layout order.
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

            for block_id in body.blocks() {
                let block = optimized.tree.get(*block_id);
                blocks.insert(*block_id, Point::new(function_id, operation));

                for instruction_id in &block.instructions {
                    let point = Point::new(function_id, operation);
                    instructions.insert(*instruction_id, point);
                    operation += 1;
                }

                let point = Point::new(function_id, operation);
                terminators.insert(*block_id, point);
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

    /// Return one block terminator point.
    pub(super) fn terminator(&self, block: mir::BlockId) -> Point {
        self.terminators[&block]
    }
}
