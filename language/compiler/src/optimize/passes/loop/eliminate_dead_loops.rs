use destack_core::{FxIndexMap, FxIndexSet};

use crate::optimize::declare_pass;
use destack_mir as mir;
use destack_source::ProvenanceJournal;

use crate::optimize::{FunctionPass, MirOptimized, PipelineContext};
use destack_mir::{
    ConstantTable, DominatorTable, Loop, LoopTable, Mutation, instruction_has_side_effects,
};

declare_pass! {
    /// Delete loops that are proven to be skipped.
    ///
    /// ```mir
    /// function before(): void {
    /// b0:
    ///     v0: boolean = false
    ///     jump b1
    /// b1:
    ///     v1: int32 = 1
    ///     v2: int32 = add v1, v1
    ///     branch v0 => b1 | b2
    /// b2:
    ///     return
    /// }
    /// ```
    /// becomes:
    /// ```mir
    /// function after(): void {
    /// b0:
    ///     v0: boolean = false
    ///     jump b1
    /// b1:
    ///     return
    /// }
    /// ```
    #[pass(id = "eliminate-dead-loops")]
    pub EliminateDeadLoops,
    "Delete loops that compute nothing useful"
}

impl FunctionPass for EliminateDeadLoops {
    fn run(
        &self,
        function: &mut mir::Function,
        optimized: &mut MirOptimized,
        provenance: &mut ProvenanceJournal<'_>,
        _ctx: &PipelineContext<'_>,
        analyses: &mut mir::FunctionCache,
    ) -> Mutation {
        let tree = &mut optimized.tree;
        let accesses = &mut optimized.accesses;

        if function.entry().is_none() {
            return Mutation::NONE;
        }

        // get analyses
        let (loops, domtree, constants) = {
            (
                analyses.loops(function, tree).clone(),
                analyses.dominator(function, tree).clone(),
                analyses.constant(function, tree).clone(),
            )
        };
        if loops.num_loops() == 0 {
            return Mutation::NONE;
        }

        // run loop deletion
        let changed = run_eliminate_dead_loops(
            function, tree, accesses, provenance, &loops, &domtree, &constants,
        );
        if changed {
            Mutation::CONTROL | Mutation::VALUE
        } else {
            Mutation::NONE
        }
    }
}

/// Core loop deletion logic. Returns true if changes were made.
fn run_eliminate_dead_loops(
    function: &mut mir::Function,
    tree: &mut mir::Tree,
    accesses: &mut mir::AccessTable,
    provenance: &mut ProvenanceJournal<'_>,
    loops: &LoopTable,
    domtree: &DominatorTable,
    constants: &ConstantTable,
) -> bool {
    // collect deletable loops (innermost first to avoid invalidation issues)
    let mut deletable: Vec<DeleteCandidate> = Vec::new();

    for lp in loops.loops().iter().rev() {
        if let Some(candidate) = find_deletable_loop(lp, function, tree, domtree, constants) {
            deletable.push(candidate);
        }
    }

    if deletable.is_empty() {
        return false;
    }

    // delete loops
    for candidate in &deletable {
        delete_loop(function, tree, accesses, candidate, provenance);
    }

    true
}

/// Information needed to delete a loop.
struct DeleteCandidate {
    /// The preheader block (will be modified to jump to exit).
    preheader: mir::LocalNodeId<mir::Block>,
    /// The single exit block.
    exit_block: mir::LocalNodeId<mir::Block>,
    /// Arguments to pass to the exit block (initial values).
    exit_arguments: Vec<mir::Value>,
    /// All blocks in the loop (will become dead).
    loop_blocks: FxIndexSet<mir::LocalNodeId<mir::Block>>,
}

/// Check if a loop can be deleted and gather necessary information.
fn find_deletable_loop(
    lp: &Loop,
    function: &mir::Function,
    tree: &mir::Tree,
    domtree: &DominatorTable,
    constants: &ConstantTable,
) -> Option<DeleteCandidate> {
    // need a preheader (immediate dominator outside the loop)
    let preheader = domtree.immediate_dominator(lp.header)?;
    if lp.blocks.contains(&preheader) {
        return None;
    }

    // determine the exit block and arguments for the constant condition path
    let (exit_block, exit_arguments) = find_constant_exit(lp, tree, preheader, constants)?;

    // check for side effects in all loop blocks
    for &block_id in &lp.blocks {
        let block = tree.get(block_id);
        for &instruction_id in &block.instructions {
            let instruction = tree.get(instruction_id);
            if instruction_has_side_effects(instruction) {
                return None;
            }
        }
    }

    // collect all values defined in the loop
    let mut loop_defined_values: FxIndexSet<mir::Value> = FxIndexSet::default();
    for &block_id in &lp.blocks {
        let block = tree.get(block_id);

        // block parameters are loop-defined
        for param in &block.parameters {
            let value = param.value;

            loop_defined_values.insert(value);
        }

        // instruction destinations are loop-defined
        for &instruction_id in &block.instructions {
            let instruction = tree.get(instruction_id);
            if let Some(destination) = instruction.destination() {
                loop_defined_values.insert(destination);
            }
        }
    }

    // check if any loop-defined values are used outside the loop
    // first, ensure the exit arguments are available outside the loop
    for arg in &exit_arguments {
        if loop_defined_values.contains(arg) {
            return None;
        }
    }

    // second, check direct uses in blocks outside the loop
    for &block_id in function.blocks() {
        if lp.blocks.contains(&block_id) {
            continue;
        }

        let block = tree.get(block_id);

        // check instruction uses
        for &instruction_id in &block.instructions {
            let instruction = tree.get(instruction_id);
            for used_value in instruction.uses() {
                if loop_defined_values.contains(&used_value) {
                    return None;
                }
            }

            // check externalized arguments
            if let Some(args_slice) = instruction.argument_slice() {
                for &arg in tree.get_values(args_slice) {
                    if loop_defined_values.contains(&arg) {
                        return None;
                    }
                }
            }
        }

        // check terminator uses
        let terminator = tree.get(block.terminator);
        for used_value in terminator.uses(tree) {
            if loop_defined_values.contains(&used_value) {
                return None;
            }
        }
    }

    Some(DeleteCandidate {
        preheader,
        exit_block,
        exit_arguments,
        loop_blocks: lp.blocks.clone(),
    })
}

/// Find the exit block and arguments selected by a constant header condition.
fn find_constant_exit(
    lp: &Loop,
    tree: &mir::Tree,
    preheader: mir::LocalNodeId<mir::Block>,
    constants: &ConstantTable,
) -> Option<(mir::LocalNodeId<mir::Block>, Vec<mir::Value>)> {
    let header_block = tree.get(lp.header);
    let header_terminator = tree.get(header_block.terminator);

    // extract condition and targets from header terminator
    let (condition, then_target, then_args, else_target, else_args) = match header_terminator {
        mir::Terminator::Branch {
            condition,
            then_target,
            else_target,
        } => (
            condition,
            then_target.block,
            tree.get_values(then_target.arguments).to_vec(),
            else_target.block,
            tree.get_values(else_target.arguments).to_vec(),
        ),
        _ => return None,
    };

    // evaluate the condition as a constant
    let condition_constant = constants.constant_at_exit(lp.header, *condition)?;
    let condition_value = match condition_constant {
        mir::Constant::Boolean { value } => *value,
        _ => return None,
    };

    // determine which branch is taken
    let (taken_target, taken_arguments) = if condition_value {
        (then_target, then_args)
    } else {
        (else_target, else_args)
    };

    // ensure the taken target is outside the loop
    if lp.blocks.contains(&taken_target) {
        return None;
    }

    // map header parameters to their initial values from preheader
    let preheader_args = preheader_to_header_args(preheader, lp.header, tree)?;
    let mut initial_values: FxIndexMap<mir::Value, mir::Value> = FxIndexMap::default();
    for (param, arg) in header_block.parameters.iter().zip(preheader_args.iter()) {
        let parameter = param.value;

        initial_values.insert(parameter, *arg);
    }

    // resolve exit arguments using initial values
    let resolved_arguments: Vec<mir::Value> = taken_arguments
        .iter()
        .copied()
        .map(|value| *initial_values.get(&value).unwrap_or(&value))
        .collect();

    Some((taken_target, resolved_arguments))
}

/// Get arguments passed from preheader to header.
fn preheader_to_header_args(
    preheader: mir::LocalNodeId<mir::Block>,
    header: mir::LocalNodeId<mir::Block>,
    tree: &mir::Tree,
) -> Option<Vec<mir::Value>> {
    let preheader_block = tree.get(preheader);
    let preheader_terminator = tree.get(preheader_block.terminator);

    // find the terminator path that leads to the header
    match preheader_terminator {
        mir::Terminator::Jump { target } if target.block == header => {
            Some(tree.get_values(target.arguments).to_vec())
        }
        mir::Terminator::Branch {
            then_target,
            else_target,
            ..
        } => {
            // check then branch
            if then_target.block == header {
                Some(tree.get_values(then_target.arguments).to_vec())
            }
            // check else branch
            else if else_target.block == header {
                Some(tree.get_values(else_target.arguments).to_vec())
            } else {
                None
            }
        }
        mir::Terminator::Check {
            success, failure, ..
        } => {
            // check success path
            if success.block == header {
                Some(tree.get_values(success.arguments).to_vec())
            }
            // check failure path
            else if failure.block == header {
                Some(tree.get_values(failure.arguments).to_vec())
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Delete a loop by replacing the preheader's terminator with a jump to exit.
fn delete_loop(
    function: &mut mir::Function,
    tree: &mut mir::Tree,
    accesses: &mut mir::AccessTable,
    candidate: &DeleteCandidate,
    provenance: &mut ProvenanceJournal<'_>,
) {
    // update preheader to jump directly to exit
    let terminator = tree.get(candidate.preheader).terminator;
    let new_terminator = mir::Terminator::Jump {
        target: mir::BlockTarget::new(
            candidate.exit_block,
            tree.add_values(&candidate.exit_arguments),
        ),
    };
    tree.rewrite(terminator, new_terminator, provenance);

    // remove loop blocks from function (they're now unreachable)
    function.retain_blocks(
        |block| !candidate.loop_blocks.contains(&block),
        tree,
        accesses,
        provenance,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optimize::common::tests::TestProgram;
    use crate::optimize::passes::SimplifyLoops;

    /// Empty loop with no side effects is deleted.
    #[test]
    fn test_delete_empty_loop() {
        let input = r#"
function test(): void {
entry:
    v0: boolean = false
    jump b1

b1:
    branch v0 => b1 | b2

b2:
    return
}
"#;
        let expected = r#"
function test(): void {
entry:
    v0: boolean = false
    jump b2

b2:
    return
}
"#;
        let mut test = TestProgram::new(input);
        test.run_pass(&SimplifyLoops);
        test.run_pass(&EliminateDeadLoops);
        test.assert_output(expected);
    }

    /// Loop computing unused value is deleted.
    #[test]
    fn test_delete_unused_computation() {
        let input = r#"
function test(v0: int32): void {
entry(v0: int32):
    v1: boolean = false
    v2: int32 = 0
    jump b1(v2)

b1(v3: int32):
    v4: int32 = 1
    v5: int32 = add v3, v4
    branch v1 => b1(v5) | b2

b2:
    return
}
"#;
        let expected = r#"
function test(v0: int32): void {
entry(v0: int32):
    v1: boolean = false
    v2: int32 = 0
    jump b2

b2:
    return
}
"#;
        let mut test = TestProgram::new(input);
        test.run_pass(&SimplifyLoops);
        test.run_pass(&EliminateDeadLoops);
        test.assert_output(expected);
    }

    /// Loop with call is preserved (calls have side effects).
    #[test]
    fn test_preserve_call_side_effects() {
        let input = r#"
function test(): void {
entry:
    v0: boolean = false
    jump b1

b1:
    call sideEffect(): () => int32
    branch v0 => b1 | b2

b2:
    return
}

function sideEffect(): int32 {
entry:
    v0: int32 = 42
    return v0
}
"#;
        let mut test = TestProgram::new(input);
        test.run_pass(&SimplifyLoops);
        let before = test.format();
        test.run_pass(&EliminateDeadLoops);
        test.assert_output(&before);
    }

    /// Loop with store is preserved (stores have side effects).
    #[test]
    fn test_preserve_store_side_effects() {
        let input = r#"
function test(v0: ref<int32, borrowed, mutable>, v1: int32): void {
entry(v0: ref<int32, borrowed, mutable>, v1: int32):
    v2: boolean = false
    jump b1

b1:
    store v0, v1
    branch v2 => b1 | b2

b2:
    return
}
"#;
        let mut test = TestProgram::new(input);
        test.run_pass(&SimplifyLoops);
        let before = test.format();
        test.run_pass(&EliminateDeadLoops);
        test.assert_output(&before);
    }

    /// Loop with live-out value is preserved.
    #[test]
    fn test_preserve_live_out() {
        let input = r#"
function test(): int32 {
entry:
    v0: boolean = false
    jump b1

b1:
    v1: int32 = 1
    branch v0 => b1 | b2(v1)

b2(v2: int32):
    return v2
}
"#;
        let mut test = TestProgram::new(input);
        test.run_pass(&SimplifyLoops);
        let before = test.format();
        test.run_pass(&EliminateDeadLoops);
        test.assert_output(&before);
    }

    /// Loop with live-out value used in an exit block is preserved.
    #[test]
    fn test_preserve_live_out_via_direct_use() {
        let input = r#"
function test(): int32 {
entry:
    v0: boolean = false
    jump b1

b1:
    v1: int32 = 1
    branch v0 => b1 | b2

b2:
    v2: int32 = add v1, v1
    return v2
}
"#;
        let mut test = TestProgram::new(input);
        test.run_pass(&SimplifyLoops);
        let before = test.format();
        test.run_pass(&EliminateDeadLoops);
        test.assert_output(&before);
    }

    /// Loop with multiple parameterless exits is deleted.
    #[test]
    fn test_delete_multiple_parameterless_exits() {
        let input = r#"
function test(v0: boolean): void {
entry(v0: boolean):
    v1: boolean = false
    jump b1

b1:
    branch v1 => b2 | b3

b2:
    branch v0 => b1 | b4

b3:
    return

b4:
    return
}
"#;
        let expected = r#"
function test(v0: boolean): void {
entry(v0: boolean):
    v1: boolean = false
    jump b3

b3:
    return

b4:
    return
}
"#;
        let mut test = TestProgram::new(input);
        test.run_pass(&SimplifyLoops);
        test.run_pass(&EliminateDeadLoops);
        test.assert_output(expected);
    }

    /// Constant backedge loop is preserved.
    #[test]
    fn test_preserve_constant_backedge() {
        let input = r#"
function test(): void {
entry:
    v0: boolean = true
    jump b1

b1:
    branch v0 => b1 | b2

b2:
    return
}
"#;
        let mut test = TestProgram::new(input);
        test.run_pass(&SimplifyLoops);
        let before = test.format();
        test.run_pass(&EliminateDeadLoops);
        test.assert_output(&before);
    }

    /// Function without loops is unchanged.
    #[test]
    fn test_preserve_no_loops() {
        let input = r#"
function test(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = 1
    v2: int32 = add v0, v1
    return v2
}
"#;
        let mut test = TestProgram::new(input);
        test.run_pass(&EliminateDeadLoops);
        test.assert_unchanged(input);
    }

    /// Inner loop is deleted when outer loop has live-out.
    #[test]
    fn test_delete_inner_loop_preserve_outer() {
        let input = r#"
function test(v0: boolean, v1: boolean): int32 {
entry(v0: boolean, v1: boolean):
    v2: int32 = 0
    jump b1(v2)

b1(v3: int32):
    v4: boolean = false
    jump b2

b2:
    branch v4 => b2 | b3

b3:
    v5: int32 = 1
    v6: int32 = add v3, v5
    branch v0 => b1(v6) | b4(v6)

b4(v7: int32):
    return v7
}
"#;
        let expected = r#"
function test(v0: boolean, v1: boolean): int32 {
entry(v0: boolean, v1: boolean):
    v2: int32 = 0
    jump b1(v2)

b1(v3: int32):
    v4: boolean = false
    jump b3

b3:
    v5: int32 = 1
    v6: int32 = add v3, v5
    branch v0 => b1(v6) | b4(v6)

b4(v7: int32):
    return v7
}
"#;
        let mut test = TestProgram::new(input);
        test.run_pass(&SimplifyLoops);
        test.run_pass(&EliminateDeadLoops);
        test.assert_output(expected);
    }

    /// Exit block parameters receive initial values.
    #[test]
    fn test_delete_pass_initial_values_to_exit() {
        let input = r#"
function test(v0: int32): int32 {
entry(v0: int32):
    v1: boolean = false
    v2: int32 = 0
    jump b1(v2)

b1(v3: int32):
    v4: int32 = 1
    v5: int32 = add v3, v4
    branch v1 => b1(v5) | b2(v3)

b2(v6: int32):
    return v6
}
"#;
        let expected = r#"
function test(v0: int32): int32 {
entry(v0: int32):
    v1: boolean = false
    v2: int32 = 0
    jump b1(v2)

b1(v6: int32):
    return v6
}
"#;
        let mut test = TestProgram::new(input);
        test.run_pass(&SimplifyLoops);
        test.run_pass(&EliminateDeadLoops);
        test.assert_output(expected);
    }

    /// Loop with free is preserved because storage release has side effects.
    #[test]
    fn test_preserve_free_side_effects() {
        let input = r#"
function test(v0: ref<int32, unique, mutable>): void {
entry(v0: ref<int32, unique, mutable>):
    v1: boolean = false
    jump b1

b1:
    free v0
    branch v1 => b1 | b2

b2:
    return
}
"#;
        let mut test = TestProgram::new(input);
        test.run_pass(&SimplifyLoops);
        let before = test.format();
        test.run_pass(&EliminateDeadLoops);
        test.assert_output(&before);
    }
}
