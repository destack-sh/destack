use tspp_artifact::MirOptimized;
use tspp_mir::{FunctionCache, FunctionId, Instruction, Mutation, Terminator};

use crate::CompilerResult;
use crate::optimize::pipeline::FunctionPass;

/// Insert runtime polls at loop headers and before tail calls.
pub(in crate::optimize) struct InsertSafepoints;

impl FunctionPass for InsertSafepoints {
    /// Insert polls using the selected function's current control flow.
    fn run(
        &self,
        function: FunctionId,
        module: &mut MirOptimized,
        analyses: &mut FunctionCache,
    ) -> CompilerResult<Mutation> {
        // preserve generated destructor bodies
        if module.drops.is_destructor(function) {
            return Ok(Mutation::NONE);
        }

        // derive loop headers and reachable blocks before inserting polls
        let body = module.tree.get(function);
        let dominator = analyses.dominator(function, &module.tree);
        let loops = analyses.loops(function, &module.tree);
        let blocks = body.blocks().to_vec();
        let mut mutation = Mutation::NONE;

        // poll at each loop header
        for entry in loops.loops() {
            let poll = module.tree.insert(Instruction::Poll);
            module
                .tree
                .get_mut(entry.header)
                .instructions
                .insert(0, poll);
            mutation = Mutation::VALUE;
        }

        // poll before each reachable tail call
        for block in blocks {
            let terminator = module.tree.get(block).terminator;
            if dominator.dominates(block, block)
                && matches!(module.tree.get(terminator), Terminator::TailCall { .. })
            {
                let poll = module.tree.insert(Instruction::Poll);
                module.tree.get_mut(block).instructions.push(poll);
                mutation = Mutation::VALUE;
            }
        }

        Ok(mutation)
    }
}
