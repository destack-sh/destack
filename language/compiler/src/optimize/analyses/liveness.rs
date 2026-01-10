use std::collections::{HashMap, HashSet};

use destack_mir as mir;

use crate::optimize::{
    Analysis, AnalysisId, ControlFlowGraph, FunctionAnalyses, FunctionAnalysis,
    terminator_used_values, terminator_uses,
};

/// Liveness analysis for SSA values.
///
/// Computes which values are "live" (potentially used in the future) at each
/// program point. A value is live at a point if there's a path from that point
/// to a use of the value.
///
/// This is a classic backward dataflow analysis:
/// - live_in(B) = use(B) ∪ (live_out(B) - def(B))
/// - live_out(B) = ∪ live_in(S) for all successors S of B
#[derive(Debug)]
pub struct LivenessAnalysis {
    /// Values live at entry to each block.
    live_in: HashMap<mir::LocalNodeId<mir::Block>, HashSet<mir::Value>>,
    /// Values live at exit of each block.
    live_out: HashMap<mir::LocalNodeId<mir::Block>, HashSet<mir::Value>>,
}

impl LivenessAnalysis {
    /// Build liveness analysis for a function.
    fn build(function: &mir::Function, tree: &mir::NodeTree, _cfg: &ControlFlowGraph) -> Self {
        // compute use/def sets for each block
        let mut block_use: HashMap<mir::LocalNodeId<mir::Block>, HashSet<mir::Value>> =
            HashMap::new();
        let mut block_def: HashMap<mir::LocalNodeId<mir::Block>, HashSet<mir::Value>> =
            HashMap::new();
        for &block_id in &function.blocks {
            let (uses, defs) = compute_block_use_def(block_id, tree);
            block_use.insert(block_id, uses);
            block_def.insert(block_id, defs);
        }

        // initialize live_in and live_out
        let mut live_in: HashMap<mir::LocalNodeId<mir::Block>, HashSet<mir::Value>> =
            HashMap::new();
        let mut live_out: HashMap<mir::LocalNodeId<mir::Block>, HashSet<mir::Value>> =
            HashMap::new();

        for &block_id in &function.blocks {
            live_in.insert(block_id, HashSet::new());
            live_out.insert(block_id, HashSet::new());
        }

        // iterate until fixed point (backward dataflow)
        let mut changed = true;
        while changed {
            changed = false;

            // process blocks in reverse order for faster convergence
            for &block_id in function.blocks.iter().rev() {
                let block = tree.get(block_id);

                // live_out = union of live_in of successors
                let mut new_live_out = HashSet::new();
                for succ_id in block.terminator.successors() {
                    if let Some(succ_live_in) = live_in.get(&succ_id) {
                        new_live_out.extend(succ_live_in.iter().copied());
                    }
                }

                // add block arguments passed to successors
                // these are "uses" at the end of the current block
                match &block.terminator {
                    mir::Terminator::Jump { arguments, .. } => {
                        for &arg in arguments {
                            new_live_out.insert(arg);
                        }
                    }
                    mir::Terminator::Branch {
                        then_arguments,
                        else_arguments,
                        ..
                    } => {
                        for &arg in then_arguments {
                            new_live_out.insert(arg);
                        }
                        for &arg in else_arguments {
                            new_live_out.insert(arg);
                        }
                    }
                    mir::Terminator::Switch {
                        cases,
                        default_arguments,
                        ..
                    } => {
                        for case in cases {
                            for &arg in &case.arguments {
                                new_live_out.insert(arg);
                            }
                        }
                        for &arg in default_arguments {
                            new_live_out.insert(arg);
                        }
                    }
                    mir::Terminator::Yield {
                        resume_arguments, ..
                    } => {
                        for &arg in resume_arguments {
                            new_live_out.insert(arg);
                        }
                    }
                    _ => {}
                }

                // live_in = use ∪ (live_out - def)
                let block_uses = block_use.get(&block_id).unwrap();
                let block_defs = block_def.get(&block_id).unwrap();

                let mut new_live_in: HashSet<mir::Value> =
                    new_live_out.difference(block_defs).copied().collect();
                new_live_in.extend(block_uses.iter().copied());

                // check for changes
                if new_live_in != *live_in.get(&block_id).unwrap() {
                    live_in.insert(block_id, new_live_in);
                    changed = true;
                }
                if new_live_out != *live_out.get(&block_id).unwrap() {
                    live_out.insert(block_id, new_live_out);
                    changed = true;
                }
            }
        }

        Self { live_in, live_out }
    }

    /// Get values live at entry to a block.
    pub fn live_in(&self, block: mir::LocalNodeId<mir::Block>) -> &HashSet<mir::Value> {
        self.live_in.get(&block).unwrap_or_else(|| {
            // This should never happen in normal use, but provide a sensible default
            static EMPTY: std::sync::OnceLock<HashSet<mir::Value>> = std::sync::OnceLock::new();
            EMPTY.get_or_init(HashSet::new)
        })
    }

    /// Get values live at exit of a block.
    pub fn live_out(&self, block: mir::LocalNodeId<mir::Block>) -> &HashSet<mir::Value> {
        self.live_out.get(&block).unwrap_or_else(|| {
            static EMPTY: std::sync::OnceLock<HashSet<mir::Value>> = std::sync::OnceLock::new();
            EMPTY.get_or_init(HashSet::new)
        })
    }

    /// Check if a value is live at block entry.
    pub fn is_live_in(&self, block: mir::LocalNodeId<mir::Block>, value: mir::Value) -> bool {
        self.live_in
            .get(&block)
            .map(|s| s.contains(&value))
            .unwrap_or(false)
    }

    /// Check if a value is live at block exit.
    pub fn is_live_out(&self, block: mir::LocalNodeId<mir::Block>, value: mir::Value) -> bool {
        self.live_out
            .get(&block)
            .map(|s| s.contains(&value))
            .unwrap_or(false)
    }

    /// Check if a value is live after a specific instruction.
    ///
    /// A value is live after an instruction if it's used by any subsequent
    /// instruction in the block, or it's live-out of the block.
    pub fn is_live_after_instruction(
        &self,
        block_id: mir::LocalNodeId<mir::Block>,
        instruction_index: usize,
        value: mir::Value,
        tree: &mir::NodeTree,
    ) -> bool {
        let block = tree.get(block_id);

        // check subsequent instructions
        for &inst_id in block.instructions.iter().skip(instruction_index + 1) {
            let inst = tree.get(inst_id);
            if inst.uses().contains(&value) {
                return true;
            }
            // check call arguments (stored externally)
            if let Some(arg_slice) = inst.argument_slice()
                && tree.get_arguments(arg_slice).contains(&value)
            {
                return true;
            }
        }

        // check terminator
        if terminator_uses(&block.terminator, value) {
            return true;
        }

        // check live-out
        self.is_live_out(block_id, value)
    }

    /// Get all values that are live at some point in the function.
    pub fn all_live_values(&self) -> HashSet<mir::Value> {
        let mut result = HashSet::new();
        for live in self.live_in.values() {
            result.extend(live.iter().copied());
        }
        for live in self.live_out.values() {
            result.extend(live.iter().copied());
        }
        result
    }
}

/// Compute use/def sets for a block.
///
/// - use: values used before being defined in the block (upward exposed uses)
/// - def: values defined in the block
fn compute_block_use_def(
    block_id: mir::LocalNodeId<mir::Block>,
    tree: &mir::NodeTree,
) -> (HashSet<mir::Value>, HashSet<mir::Value>) {
    let block = tree.get(block_id);
    let mut uses = HashSet::new();
    let mut defs = HashSet::new();

    // block parameters are definitions
    for param in &block.parameters {
        defs.insert(param.value);
    }

    // process instructions
    for &inst_id in &block.instructions {
        let inst = tree.get(inst_id);

        // uses (only if not already defined locally)
        for used in inst.uses() {
            if !defs.contains(&used) {
                uses.insert(used);
            }
        }

        // call arguments are stored externally - handle them explicitly
        if let Some(arg_slice) = inst.argument_slice() {
            for &arg in tree.get_arguments(arg_slice) {
                if !defs.contains(&arg) {
                    uses.insert(arg);
                }
            }
        }

        // definition
        if let Some(dest) = inst.destination() {
            defs.insert(dest);
        }
    }

    // terminator uses
    for used in terminator_used_values(&block.terminator) {
        if !defs.contains(&used) {
            uses.insert(used);
        }
    }

    (uses, defs)
}

impl Analysis for LivenessAnalysis {
    const ID: AnalysisId = AnalysisId("liveness");
    const DEPENDENCIES: &'static [AnalysisId] = &[ControlFlowGraph::ID];
}

impl FunctionAnalysis for LivenessAnalysis {
    fn compute(
        function: &mir::Function,
        tree: &mir::NodeTree,
        analyses: &FunctionAnalyses<'_>,
    ) -> Self {
        let cfg = analyses.get::<ControlFlowGraph>();
        Self::build(function, tree, &cfg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optimize::common::tests::TestProgram;

    /// Single block: all values defined locally, none live-in, live-out empty at return.
    #[test]
    fn test_simple_liveness() {
        let program = TestProgram::new(
            r#"function @test() -> i32 {
block0:
    v0 = iconst 1i32
    v1 = iconst 2i32
    v2 = iadd v0, v1
    return v2
}"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = program.tree.get(function_id);
        let analyses = program.function_analyses(function);
        let liveness = analyses.get::<LivenessAnalysis>();

        let entry = function.entry.unwrap();

        // all values defined in this block, so none are live-in
        assert!(!liveness.is_live_in(entry, mir::Value::new(0)));
        assert!(!liveness.is_live_in(entry, mir::Value::new(1)));
        assert!(!liveness.is_live_in(entry, mir::Value::new(2)));

        // no successors (return), so live_out is empty
        assert!(liveness.live_out(entry).is_empty());

        // v0 is live after instruction 0 (used by iadd)
        assert!(liveness.is_live_after_instruction(entry, 0, mir::Value::new(0), &program.tree));

        // v2 is live after instruction 2 (used by return)
        assert!(liveness.is_live_after_instruction(entry, 2, mir::Value::new(2), &program.tree));
    }

    /// Value defined in one block is live-in to block that uses it.
    #[test]
    fn test_live_across_blocks() {
        let program = TestProgram::new(
            r#"function @test(v0: bool) -> i32 {
block0(v0: bool):
    v1 = iconst 42i32
    branch v0, block1, block2
block1:
    return v1
block2:
    v2 = iconst 0i32
    return v2
}"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = program.tree.get(function_id);
        let analyses = program.function_analyses(function);
        let liveness = analyses.get::<LivenessAnalysis>();

        let block0 = function.blocks[0];
        let block1 = function.blocks[1];
        let block2 = function.blocks[2];

        // v1 is live-out of block0 (used in block1)
        assert!(liveness.is_live_out(block0, mir::Value::new(1)));

        // v1 is live-in to block1
        assert!(liveness.is_live_in(block1, mir::Value::new(1)));

        // v1 is not live in block2 (not used there)
        assert!(!liveness.is_live_in(block2, mir::Value::new(1)));
    }

    /// Values in loop: block parameters not live-in, loop-carried values live-out.
    #[test]
    fn test_live_in_loop() {
        let program = TestProgram::new(
            r#"function @test(v0: bool) -> i32 {
block0(v0: bool):
    v1 = iconst 1i32
    jump block1(v1)
block1(v2: i32):
    v3 = iconst 2i32
    v4 = iadd v2, v3
    branch v0, block1(v4), block2
block2:
    return v4
}"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = program.tree.get(function_id);
        let analyses = program.function_analyses(function);
        let liveness = analyses.get::<LivenessAnalysis>();

        let block0 = function.blocks[0];
        let block1 = function.blocks[1];
        let block2 = function.blocks[2];

        // v2 is a block parameter of block1, so it's NOT live-in (it's defined at entry)
        assert!(!liveness.is_live_in(block1, mir::Value::new(2)));

        // v0 is used in block1's terminator but defined in block0, so it's live-in to block1
        assert!(liveness.is_live_in(block1, mir::Value::new(0)));

        // v4 is live-out of block1 (passed as argument to block1 and to block2)
        assert!(liveness.is_live_out(block1, mir::Value::new(4)));

        // v4 is live-in to block2 (used in return)
        assert!(liveness.is_live_in(block2, mir::Value::new(4)));

        // v1 is live-out of block0 (passed as argument to block1)
        assert!(liveness.is_live_out(block0, mir::Value::new(1)));
    }

    /// Unused value is not live after its definition.
    #[test]
    fn test_dead_value() {
        let program = TestProgram::new(
            r#"function @test() -> i32 {
block0:
    v0 = iconst 1i32
    v1 = iconst 2i32
    return v0
}"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = program.tree.get(function_id);
        let analyses = program.function_analyses(function);
        let liveness = analyses.get::<LivenessAnalysis>();

        let entry = function.entry.unwrap();

        // v0 is live after its definition (used by return)
        assert!(liveness.is_live_after_instruction(entry, 0, mir::Value::new(0), &program.tree));

        // v1 is dead (never used), not live after its definition
        assert!(!liveness.is_live_after_instruction(entry, 1, mir::Value::new(1), &program.tree));
    }

    /// Block parameters are defined at entry, not live-in.
    #[test]
    fn test_block_arguments_live() {
        let program = TestProgram::new(
            r#"function @test(v0: i32, v1: i32) -> i32 {
block0(v0: i32, v1: i32):
    v2 = iadd v0, v1
    return v2
}"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = program.tree.get(function_id);
        let analyses = program.function_analyses(function);
        let liveness = analyses.get::<LivenessAnalysis>();

        let entry = function.entry.unwrap();

        // block parameters are defined at entry, not live-in
        assert!(!liveness.is_live_in(entry, mir::Value::new(0)));
        assert!(!liveness.is_live_in(entry, mir::Value::new(1)));
    }

    /// is_live_after_instruction correctly tracks per-instruction liveness.
    #[test]
    fn test_is_live_after_instruction() {
        let program = TestProgram::new(
            r#"function @test() -> i32 {
block0:
    v0 = iconst 1i32
    v1 = iconst 2i32
    v2 = iadd v0, v1
    return v2
}"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = program.tree.get(function_id);
        let analyses = program.function_analyses(function);
        let liveness = analyses.get::<LivenessAnalysis>();

        let entry = function.entry.unwrap();

        // after v0 = iconst, v0 is still live (used by iadd)
        assert!(liveness.is_live_after_instruction(entry, 0, mir::Value::new(0), &program.tree));

        // after v2 = iadd, v0 is dead (already consumed)
        assert!(!liveness.is_live_after_instruction(entry, 2, mir::Value::new(0), &program.tree));

        // after v2 = iadd, v2 is live (used by return)
        assert!(liveness.is_live_after_instruction(entry, 2, mir::Value::new(2), &program.tree));
    }

    /// Diamond control flow: value live on some paths.
    ///
    /// Value defined before branch is live-in to blocks that use it,
    /// even if not all successors use it.
    #[test]
    fn test_diamond_control_flow() {
        let program = TestProgram::new(
            r#"function @test(v0: bool) -> i32 {
block0(v0: bool):
    v1 = iconst 10i32
    v2 = iconst 20i32
    branch v0, block1, block2
block1:
    jump block3(v1)
block2:
    jump block3(v2)
block3(v3: i32):
    return v3
}"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = program.tree.get(function_id);
        let analyses = program.function_analyses(function);
        let liveness = analyses.get::<LivenessAnalysis>();

        let block0 = function.blocks[0];
        let block1 = function.blocks[1];
        let block2 = function.blocks[2];

        // v1 is live-out of block0 (used in block1)
        assert!(liveness.is_live_out(block0, mir::Value::new(1)));

        // v2 is live-out of block0 (used in block2)
        assert!(liveness.is_live_out(block0, mir::Value::new(2)));

        // v1 is live-in to block1 but not block2
        assert!(liveness.is_live_in(block1, mir::Value::new(1)));
        assert!(!liveness.is_live_in(block2, mir::Value::new(1)));

        // v2 is live-in to block2 but not block1
        assert!(liveness.is_live_in(block2, mir::Value::new(2)));
        assert!(!liveness.is_live_in(block1, mir::Value::new(2)));
    }

    /// Multiple uses of same value.
    ///
    /// A value remains live until after its last use.
    #[test]
    fn test_multiple_uses() {
        let program = TestProgram::new(
            r#"function @test() -> i32 {
block0:
    v0 = iconst 5i32
    v1 = iadd v0, v0
    v2 = iadd v1, v0
    return v2
}"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = program.tree.get(function_id);
        let analyses = program.function_analyses(function);
        let liveness = analyses.get::<LivenessAnalysis>();

        let entry = function.entry.unwrap();

        // v0 is live after instruction 0 (used by both iadds)
        assert!(liveness.is_live_after_instruction(entry, 0, mir::Value::new(0), &program.tree));

        // v0 is live after instruction 1 (still used by second iadd)
        assert!(liveness.is_live_after_instruction(entry, 1, mir::Value::new(0), &program.tree));

        // v0 is dead after instruction 2 (all uses consumed)
        assert!(!liveness.is_live_after_instruction(entry, 2, mir::Value::new(0), &program.tree));
    }

    /// Call arguments are considered uses.
    ///
    /// Values passed to function calls must be live at the call site.
    #[test]
    fn test_call_arguments_live() {
        let program = TestProgram::new(
            r#"extern function @external(i32, i32) -> void
function @test() -> void {
block0:
    v0 = iconst 1i32
    v1 = iconst 2i32
    call @external(v0, v1)
    return
}"#,
        );

        // skip the extern function, get the test function
        let function_id = program.tree.iter_nodes::<mir::Function>().nth(1).unwrap().0;
        let function = program.tree.get(function_id);
        let analyses = program.function_analyses(function);
        let liveness = analyses.get::<LivenessAnalysis>();

        let entry = function.entry.unwrap();

        // v0 is live after instruction 0 (used as call argument)
        assert!(liveness.is_live_after_instruction(entry, 0, mir::Value::new(0), &program.tree));

        // v1 is live after instruction 1 (used as call argument)
        assert!(liveness.is_live_after_instruction(entry, 1, mir::Value::new(1), &program.tree));

        // both are dead after the call
        assert!(!liveness.is_live_after_instruction(entry, 2, mir::Value::new(0), &program.tree));
        assert!(!liveness.is_live_after_instruction(entry, 2, mir::Value::new(1), &program.tree));
    }
}
