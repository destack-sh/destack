use std::collections::{HashMap, HashSet};

use destack_compiler_macros::declare_pass;
use destack_mir as mir;

use crate::optimize::analyses::{ConstantPropagation, DominatorTree, Loop, LoopAnalysis};
use crate::optimize::common::instruction_has_side_effects;
use crate::optimize::{AnalysisPreservation, FunctionPass, PipelineContext};

declare_pass! {
    /// Delete loops that are proven to be skipped.
    ///
    /// A loop can be deleted if:
    /// 1. The header branch condition is a constant that selects an exit
    /// 2. The selected exit block is outside the loop
    /// 3. The loop has no side effects
    /// 4. No values defined in the loop are used outside the loop
    ///
    /// When deleted, the loop is replaced with a direct jump from the preheader
    /// to the selected exit block, passing the initial values of any exit block parameters.
    ///
    /// ```mir
    /// function @before() -> void {
    /// block0:
    ///     v0 = iconst false
    ///     jump block1
    /// block1:
    ///     v1 = iconst 1i32
    ///     v2 = iadd v1, v1
    ///     branch v0, block1, block2
    /// block2:
    ///     return
    /// }
    /// ```
    /// becomes:
    /// ```mir
    /// function @after() -> void {
    /// block0:
    ///     v0 = iconst false
    ///     jump block1
    /// block1:
    ///     return
    /// }
    /// ```
    #[pass(id = "loop-delete")]
    pub LoopDelete,
    "Delete loops that compute nothing useful"
}

impl FunctionPass for LoopDelete {
    fn run(
        &self,
        function: &mut mir::Function,
        tree: &mut mir::NodeTree,
        ctx: &PipelineContext<'_>,
    ) -> AnalysisPreservation {
        if function.entry.is_none() {
            return AnalysisPreservation::all();
        }

        // get analyses
        let (loops, domtree, constants) = {
            let analyses = ctx.function_analyses(function, tree);
            (
                analyses.get::<LoopAnalysis>().clone(),
                analyses.get::<DominatorTree>().clone(),
                analyses.get::<ConstantPropagation>().clone(),
            )
        };
        if loops.num_loops() == 0 {
            return AnalysisPreservation::all();
        }

        // run loop deletion
        let changed = run_loop_delete(function, tree, &loops, &domtree, &constants);
        if changed {
            AnalysisPreservation::none()
        } else {
            AnalysisPreservation::all()
        }
    }

    fn name(&self) -> &'static str {
        "LoopDelete"
    }

    fn id(&self) -> &'static str {
        "loop-delete"
    }
}

/// Core loop deletion logic. Returns true if changes were made.
fn run_loop_delete(
    function: &mut mir::Function,
    tree: &mut mir::NodeTree,
    loops: &LoopAnalysis,
    domtree: &DominatorTree,
    constants: &ConstantPropagation,
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
        delete_loop(function, tree, candidate);
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
    loop_blocks: HashSet<mir::LocalNodeId<mir::Block>>,
}

/// Check if a loop can be deleted and gather necessary information.
fn find_deletable_loop(
    lp: &Loop,
    function: &mir::Function,
    tree: &mir::NodeTree,
    domtree: &DominatorTree,
    constants: &ConstantPropagation,
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
    let mut loop_defined_values: HashSet<mir::Value> = HashSet::new();
    for &block_id in &lp.blocks {
        let block = tree.get(block_id);

        // block parameters are loop-defined
        for param in &block.parameters {
            loop_defined_values.insert(param.value);
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
    for &block_id in &function.blocks {
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
                for &arg in tree.get_arguments(args_slice) {
                    if loop_defined_values.contains(&arg) {
                        return None;
                    }
                }
            }
        }

        // check terminator uses
        for used_value in block.terminator.uses() {
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
    tree: &mir::NodeTree,
    preheader: mir::LocalNodeId<mir::Block>,
    constants: &ConstantPropagation,
) -> Option<(mir::LocalNodeId<mir::Block>, Vec<mir::Value>)> {
    let header_block = tree.get(lp.header);

    // extract condition and targets from header terminator
    let (condition, then_target, then_args, else_target, else_args) = match &header_block.terminator
    {
        mir::Terminator::Branch {
            condition,
            then_target,
            then_arguments,
            else_target,
            else_arguments,
        } => (
            *condition,
            *then_target,
            then_arguments.clone(),
            *else_target,
            else_arguments.clone(),
        ),
        mir::Terminator::Check {
            condition,
            success,
            failure,
            ..
        } => (
            *condition,
            success.target,
            success.arguments.clone(),
            failure.target,
            failure.arguments.clone(),
        ),
        _ => return None,
    };

    // evaluate the condition as a constant
    let condition_constant = constants.constant_at_exit(lp.header, condition)?;
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
    let mut initial_values: HashMap<mir::Value, mir::Value> = HashMap::new();
    for (param, arg) in header_block.parameters.iter().zip(preheader_args.iter()) {
        initial_values.insert(param.value, *arg);
    }

    // resolve exit arguments using initial values
    let resolved_arguments: Vec<mir::Value> = taken_arguments
        .iter()
        .map(|v| *initial_values.get(v).unwrap_or(v))
        .collect();

    Some((taken_target, resolved_arguments))
}

/// Get arguments passed from preheader to header.
fn preheader_to_header_args(
    preheader: mir::LocalNodeId<mir::Block>,
    header: mir::LocalNodeId<mir::Block>,
    tree: &mir::NodeTree,
) -> Option<Vec<mir::Value>> {
    let preheader_block = tree.get(preheader);

    // find the terminator path that leads to the header
    match &preheader_block.terminator {
        mir::Terminator::Jump { target, arguments } if *target == header => Some(arguments.clone()),
        mir::Terminator::Branch {
            then_target,
            then_arguments,
            else_target,
            else_arguments,
            ..
        } => {
            // check then branch
            if *then_target == header {
                Some(then_arguments.clone())
            }
            // check else branch
            else if *else_target == header {
                Some(else_arguments.clone())
            } else {
                None
            }
        }
        mir::Terminator::Check {
            success, failure, ..
        } => {
            // check success path
            if success.target == header {
                Some(success.arguments.clone())
            }
            // check failure path
            else if failure.target == header {
                Some(failure.arguments.clone())
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
    tree: &mut mir::NodeTree,
    candidate: &DeleteCandidate,
) {
    // update preheader to jump directly to exit
    let mut preheader = tree.get(candidate.preheader).clone();
    preheader.terminator = mir::Terminator::Jump {
        target: candidate.exit_block,
        arguments: candidate.exit_arguments.clone(),
    };
    tree.replace(candidate.preheader, preheader);

    // remove loop blocks from function (they're now unreachable)
    function
        .blocks
        .retain(|b| !candidate.loop_blocks.contains(b));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optimize::common::tests::TestProgram;
    use crate::optimize::passes::LoopSimplify;

    /// Empty loop with no side effects is deleted.
    #[test]
    fn test_delete_empty_loop() {
        let input = r#"function @test() -> void {
block0:
    v0 = iconst false
    jump block1
block1:
    branch v0, block1, block2
block2:
    return
}"#;
        let expected = r#"function @test() -> void {
block0:
    v0 = iconst false
    jump block1
block1:
    return
}"#;
        let mut program = TestProgram::new(input);
        program.run_pass(&LoopSimplify);
        program.run_pass(&LoopDelete);
        program.assert_output(expected);
    }

    /// Loop computing unused value is deleted.
    #[test]
    fn test_delete_unused_computation() {
        let input = r#"function @test(v0: i32) -> void {
block0(v0: i32):
    v1 = iconst false
    v2 = iconst 0i32
    jump block1(v2)
block1(v3: i32):
    v4 = iconst 1i32
    v5 = iadd v3, v4
    branch v1, block1(v5), block2
block2:
    return
}"#;
        let expected = r#"function @test(v0: i32) -> void {
block0(v0: i32):
    v1 = iconst false
    v2 = iconst 0i32
    jump block1
block1:
    return
}"#;
        let mut program = TestProgram::new(input);
        program.run_pass(&LoopSimplify);
        program.run_pass(&LoopDelete);
        program.assert_output(expected);
    }

    /// Loop with call is preserved (calls have side effects).
    #[test]
    fn test_preserve_call_side_effects() {
        let input = r#"function @test() -> void {
block0:
    v0 = iconst false
    jump block1
block1:
    v1 = call @side_effect() -> fn() -> void
    branch v0, block1, block2
block2:
    return
}

function @side_effect() -> i32 {
block0:
    v0 = iconst 42i32
    return v0
}"#;
        let mut program = TestProgram::new(input);
        program.run_pass(&LoopSimplify);
        let before = program.format();
        program.run_pass(&LoopDelete);
        program.assert_output(&before);
    }

    /// Loop with store is preserved (stores have side effects).
    #[test]
    fn test_preserve_store_side_effects() {
        let input = r#"function @test(v0: ref<raw i32>, v1: i32) -> void {
block0(v0: ref<raw i32>, v1: i32):
    v2 = iconst false
    jump block1
block1:
    store v0, v1
    branch v2, block1, block2
block2:
    return
}"#;
        let mut program = TestProgram::new(input);
        program.run_pass(&LoopSimplify);
        let before = program.format();
        program.run_pass(&LoopDelete);
        program.assert_output(&before);
    }

    /// Loop with live-out value is preserved.
    #[test]
    fn test_preserve_live_out() {
        let input = r#"function @test() -> i32 {
block0:
    v0 = iconst false
    jump block1
block1:
    v1 = iconst 1i32
    branch v0, block1, block2(v1)
block2(v2: i32):
    return v2
}"#;
        let mut program = TestProgram::new(input);
        program.run_pass(&LoopSimplify);
        let before = program.format();
        program.run_pass(&LoopDelete);
        program.assert_output(&before);
    }

    /// Loop with live-out value used in an exit block is preserved.
    #[test]
    fn test_preserve_live_out_via_direct_use() {
        let input = r#"function @test() -> i32 {
block0:
    v0 = iconst false
    jump block1
block1:
    v1 = iconst 1i32
    branch v0, block1, block2
block2:
    v2 = iadd v1, v1
    return v2
}"#;
        let mut program = TestProgram::new(input);
        program.run_pass(&LoopSimplify);
        let before = program.format();
        program.run_pass(&LoopDelete);
        program.assert_output(&before);
    }

    /// Loop with multiple parameterless exits is deleted.
    #[test]
    fn test_delete_multiple_parameterless_exits() {
        let input = r#"function @test(v0: bool) -> void {
block0(v0: bool):
    v1 = iconst false
    jump block1
block1:
    branch v1, block2, block3
block2:
    branch v0, block1, block4
block3:
    return
block4:
    return
}"#;
        let expected = r#"function @test(v0: bool) -> void {
block0(v0: bool):
    v1 = iconst false
    jump block1
block1:
    return
block2:
    return
}"#;
        let mut program = TestProgram::new(input);
        program.run_pass(&LoopSimplify);
        program.run_pass(&LoopDelete);
        program.assert_output(expected);
    }

    /// Constant backedge loop is preserved.
    #[test]
    fn test_preserve_constant_backedge() {
        let input = r#"function @test() -> void {
block0:
    v0 = iconst true
    jump block1
block1:
    branch v0, block1, block2
block2:
    return
}"#;
        let mut program = TestProgram::new(input);
        program.run_pass(&LoopSimplify);
        let before = program.format();
        program.run_pass(&LoopDelete);
        program.assert_output(&before);
    }

    /// Function without loops is unchanged.
    #[test]
    fn test_preserve_no_loops() {
        let input = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    v1 = iconst 1i32
    v2 = iadd v0, v1
    return v2
}"#;
        let mut program = TestProgram::new(input);
        program.run_pass(&LoopDelete);
        program.assert_unchanged(input);
    }

    /// Inner loop is deleted when outer loop has live-out.
    #[test]
    fn test_delete_inner_loop_preserve_outer() {
        let input = r#"function @test(v0: bool, v1: bool) -> i32 {
block0(v0: bool, v1: bool):
    v2 = iconst 0i32
    jump block1(v2)
block1(v3: i32):
    v4 = iconst false
    jump block2
block2:
    branch v4, block2, block3
block3:
    v5 = iconst 1i32
    v6 = iadd v3, v5
    branch v0, block1(v6), block4(v6)
block4(v7: i32):
    return v7
}"#;
        let expected = r#"function @test(v0: bool, v1: bool) -> i32 {
block0(v0: bool, v1: bool):
    v2 = iconst 0i32
    jump block1(v2)
block1(v3: i32):
    v4 = iconst false
    jump block2
block2:
    v5 = iconst 1i32
    v6 = iadd v3, v5
    branch v0, block1(v6), block3(v6)
block3(v7: i32):
    return v7
}"#;
        let mut program = TestProgram::new(input);
        program.run_pass(&LoopSimplify);
        program.run_pass(&LoopDelete);
        program.assert_output(expected);
    }

    /// Exit block parameters receive initial values.
    #[test]
    fn test_delete_pass_initial_values_to_exit() {
        let input = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    v1 = iconst false
    v2 = iconst 0i32
    jump block1(v2)
block1(v3: i32):
    v4 = iconst 1i32
    v5 = iadd v3, v4
    branch v1, block1(v5), block2(v3)
block2(v6: i32):
    return v6
}"#;
        let expected = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    v1 = iconst false
    v2 = iconst 0i32
    jump block1(v2)
block1(v6: i32):
    return v6
}"#;
        let mut program = TestProgram::new(input);
        program.run_pass(&LoopSimplify);
        program.run_pass(&LoopDelete);
        program.assert_output(expected);
    }

    /// Loop with drop is preserved (drop has side effects).
    #[test]
    fn test_preserve_drop_side_effects() {
        let input = r#"function @test(v0: i32) -> void {
block0(v0: i32):
    v1 = iconst false
    jump block1
block1:
    raw.drop v0
    branch v1, block1, block2
block2:
    return
}"#;
        let mut program = TestProgram::new(input);
        program.run_pass(&LoopSimplify);
        let before = program.format();
        program.run_pass(&LoopDelete);
        program.assert_output(&before);
    }
}
