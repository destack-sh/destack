use std::collections::{HashMap, HashSet};

use destack_compiler_macros::declare_pass;
use destack_mir as mir;

use crate::optimize::analyses::{DominatorTree, Loop, LoopAnalysis};
use crate::optimize::common::instruction_has_side_effects;
use crate::optimize::{
    AnalysisPreservation, FunctionPass, OptimizationContext, Pass, PassMetadata,
};

declare_pass! {
    /// Delete loops that compute nothing useful.
    ///
    /// A loop can be deleted if:
    /// 1. It has no side effects (no stores, calls, etc.)
    /// 2. No values defined in the loop are used outside the loop
    /// 3. The loop has exit(s) that can be redirected to:
    ///    - Single exit: always OK
    ///    - Multiple exits: OK if all exit blocks have no parameters
    ///
    /// When deleted, the loop is replaced with a direct jump from the preheader
    /// to the exit block, passing the initial values of any exit block parameters.
    ///
    /// ```mir
    /// function @before(v0: i32) -> void {
    /// block0(v0: i32):
    ///     v1 = iconst 0i32
    ///     jump block1(v1)
    /// block1(v2: i32):
    ///     v3 = iconst 1i32
    ///     v4 = iadd v2, v3
    ///     v5 = icmp_slt v4, v0
    ///     branch v5, block1(v4), block2
    /// block2:
    ///     return
    /// }
    /// ```
    /// becomes:
    /// ```mir
    /// function @after(v0: i32) -> void {
    /// block0(v0: i32):
    ///     jump block1
    /// block1:
    ///     return
    /// }
    /// ```
    #[pass(id = "loop-delete")]
    pub LoopDelete,
    "Delete loops that compute nothing useful"
}

impl Pass for LoopDelete {
    fn metadata(&self) -> &'static PassMetadata {
        LoopDelete::metadata()
    }
}

impl FunctionPass for LoopDelete {
    fn run_on_function(
        &self,
        function: &mut mir::Function,
        tree: &mut mir::NodeTree,
        context: &OptimizationContext<'_>,
    ) -> AnalysisPreservation {
        if function.entry.is_none() {
            return AnalysisPreservation::all();
        }

        let loops = context.analyses.get::<LoopAnalysis>(function, tree);
        if loops.num_loops() == 0 {
            return AnalysisPreservation::all();
        }
        let domtree = context.analyses.get::<DominatorTree>(function, tree);

        // collect deletable loops (innermost first to avoid invalidation issues)
        let mut deletable: Vec<DeleteCandidate> = Vec::new();

        for lp in loops.loops().iter().rev() {
            if let Some(candidate) = find_deletable_loop(lp, function, tree, &domtree) {
                deletable.push(candidate);
            }
        }

        drop(domtree);
        drop(loops);

        if deletable.is_empty() {
            return AnalysisPreservation::all();
        }

        // delete loops
        for candidate in &deletable {
            delete_loop(function, tree, candidate);
        }

        AnalysisPreservation::none()
    }
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

/// Collect arguments passed to blocks outside the loop from this terminator.
fn get_arguments_to_exits(
    terminator: &mir::Terminator,
    loop_blocks: &HashSet<mir::LocalNodeId<mir::Block>>,
) -> Vec<mir::Value> {
    let mut args = Vec::new();

    match terminator {
        mir::Terminator::Jump { target, arguments } => {
            if !loop_blocks.contains(target) {
                args.extend(arguments.iter().copied());
            }
        }

        mir::Terminator::Branch {
            then_target,
            then_arguments,
            else_target,
            else_arguments,
            ..
        } => {
            if !loop_blocks.contains(then_target) {
                args.extend(then_arguments.iter().copied());
            }
            if !loop_blocks.contains(else_target) {
                args.extend(else_arguments.iter().copied());
            }
        }

        mir::Terminator::Switch {
            default,
            default_arguments,
            cases,
            ..
        } => {
            if !loop_blocks.contains(default) {
                args.extend(default_arguments.iter().copied());
            }
            for case in cases {
                if !loop_blocks.contains(&case.target) {
                    args.extend(case.arguments.iter().copied());
                }
            }
        }

        mir::Terminator::Return { .. }
        | mir::Terminator::Yield { .. }
        | mir::Terminator::Unreachable => {}
    }

    args
}

/// Check if a loop can be deleted and gather necessary information.
fn find_deletable_loop(
    lp: &Loop,
    function: &mir::Function,
    tree: &mir::NodeTree,
    domtree: &DominatorTree,
) -> Option<DeleteCandidate> {
    // need a preheader (immediate dominator outside the loop)
    let preheader = domtree.immediate_dominator(lp.header)?;
    if lp.blocks.contains(&preheader) {
        return None;
    }

    // determine the exit block
    // prefer single exit, but allow multiple exits if they all have no parameters
    let exit_block = if lp.exit_blocks.len() == 1 {
        lp.exit_blocks[0]
    } else if lp.exit_blocks.len() > 1 {
        // multiple exits: only allow if ALL exit blocks have no parameters
        // (otherwise we'd need to compute consistent arguments for each)
        let all_parameterless = lp
            .exit_blocks
            .iter()
            .all(|&eb| tree.get(eb).parameters.is_empty());
        if !all_parameterless {
            return None;
        }
        // pick the smallest exit block ID for determinism (they're all equivalent)
        *lp.exit_blocks.iter().min().unwrap()
    } else {
        // no exits (infinite loop) - can't delete
        return None;
    };

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
    // first, check direct uses in blocks outside the loop
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

    // second, check if loop-defined values are passed as arguments to blocks outside the loop
    // (this catches values that escape via block parameters)
    for &block_id in &lp.blocks {
        let block = tree.get(block_id);
        let exit_args = get_arguments_to_exits(&block.terminator, &lp.blocks);
        for arg in exit_args {
            if loop_defined_values.contains(&arg) {
                return None;
            }
        }
    }

    // find the exit arguments from the preheader path
    // when we delete the loop, we need to pass the initial values to the exit block
    let exit_arguments = find_exit_arguments(lp, tree, preheader, exit_block)?;

    Some(DeleteCandidate {
        preheader,
        exit_block,
        exit_arguments,
        loop_blocks: lp.blocks.clone(),
    })
}

/// Find the arguments to pass to the exit block when deleting the loop.
///
/// These are the values that would reach the exit block if the loop executed zero times.
/// We trace back from the exiting block's branch to find the initial values.
fn find_exit_arguments(
    lp: &Loop,
    tree: &mir::NodeTree,
    preheader: mir::LocalNodeId<mir::Block>,
    exit_block: mir::LocalNodeId<mir::Block>,
) -> Option<Vec<mir::Value>> {
    // find an exiting block that goes to the exit
    let exiting_block = lp.exiting_blocks.iter().find(|&&eb| {
        let block = tree.get(eb);
        block.terminator.successors().contains(&exit_block)
    })?;

    let exiting = tree.get(*exiting_block);

    // get the arguments passed to exit from the exiting block
    let exit_args_from_exiting = match &exiting.terminator {
        mir::Terminator::Branch {
            then_target,
            then_arguments,
            else_target,
            else_arguments,
            ..
        } => {
            if *then_target == exit_block {
                then_arguments.clone()
            } else if *else_target == exit_block {
                else_arguments.clone()
            } else {
                return None;
            }
        }
        mir::Terminator::Jump { target, arguments } => {
            if *target == exit_block {
                arguments.clone()
            } else {
                return None;
            }
        }
        _ => return None,
    };

    // if exit block has no parameters, we're done
    let exit_block_data = tree.get(exit_block);
    if exit_block_data.parameters.is_empty() {
        return Some(Vec::new());
    }

    // the exit arguments may reference header parameters (loop phis)
    // we need to map these back to the preheader's initial values
    let header = lp.header;
    let header_block = tree.get(header);

    // get arguments passed from preheader to header
    let preheader_block = tree.get(preheader);
    let preheader_to_header_args = match &preheader_block.terminator {
        mir::Terminator::Jump { target, arguments } if *target == header => arguments.clone(),
        mir::Terminator::Branch {
            then_target,
            then_arguments,
            else_target,
            else_arguments,
            ..
        } => {
            if *then_target == header {
                then_arguments.clone()
            } else if *else_target == header {
                else_arguments.clone()
            } else {
                return None;
            }
        }
        _ => return None,
    };

    // build substitution map: header param -> preheader arg (initial value)
    let mut initial_values: HashMap<mir::Value, mir::Value> = HashMap::new();
    for (param, arg) in header_block
        .parameters
        .iter()
        .zip(preheader_to_header_args.iter())
    {
        initial_values.insert(param.value, *arg);
    }

    // substitute in exit arguments
    let result: Vec<mir::Value> = exit_args_from_exiting
        .iter()
        .map(|v| *initial_values.get(v).unwrap_or(v))
        .collect();

    Some(result)
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
        let input = r#"function @test(v0: bool) -> void {
block0(v0: bool):
    jump block1
block1:
    branch v0, block1, block2
block2:
    return
}"#;
        let expected = r#"function @test(v0: bool) -> void {
block0(v0: bool):
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
        let input = r#"function @test(v0: i32, v1: i32) -> void {
block0(v0: i32, v1: i32):
    v2 = iconst 0i32
    jump block1(v2)
block1(v3: i32):
    v4 = iconst 1i32
    v5 = iadd v3, v4
    v6 = icmp_slt v5, v1
    branch v6, block1(v5), block2
block2:
    return
}"#;
        let expected = r#"function @test(v0: i32, v1: i32) -> void {
block0(v0: i32, v1: i32):
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
        let input = r#"function @test(v0: bool) -> void {
block0(v0: bool):
    jump block1
block1:
    v1 = call @side_effect()
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
        let input = r#"function @test(v0: bool, v1: ref<raw i32>, v2: i32) -> void {
block0(v0: bool, v1: ref<raw i32>, v2: i32):
    jump block1
block1:
    store v1, v2
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

    /// Loop with live-out value is preserved.
    #[test]
    fn test_preserve_live_out() {
        let input = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    v1 = iconst 0i32
    v2 = iconst 1i32
    jump block1(v1)
block1(v3: i32):
    v4 = iadd v3, v2
    v5 = icmp_slt v4, v0
    branch v5, block1(v4), block2
block2:
    return v4
}"#;
        let mut program = TestProgram::new(input);
        program.run_pass(&LoopSimplify);
        let before = program.format();
        program.run_pass(&LoopDelete);
        program.assert_output(&before);
    }

    /// Loop with value passed to exit block parameter is preserved.
    #[test]
    fn test_preserve_live_out_via_exit_args() {
        let input = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    v1 = iconst 0i32
    jump block1(v1)
block1(v2: i32):
    v3 = iconst 1i32
    v4 = iadd v2, v3
    v5 = icmp_slt v4, v0
    branch v5, block1(v4), block2(v4)
block2(v6: i32):
    return v6
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
        let input = r#"function @test(v0: bool, v1: bool) -> void {
block0(v0: bool, v1: bool):
    jump block1
block1:
    branch v0, block2, block3
block2:
    branch v1, block1, block4
block3:
    return
block4:
    return
}"#;
        // both exits (block3, block4) have no parameters, so loop can be deleted
        let expected = r#"function @test(v0: bool, v1: bool) -> void {
block0(v0: bool, v1: bool):
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

    /// Loop with multiple exits where some have parameters is preserved.
    #[test]
    fn test_preserve_multiple_exits_with_params() {
        let input = r#"function @test(v0: bool, v1: bool, v2: i32) -> i32 {
block0(v0: bool, v1: bool, v2: i32):
    jump block1
block1:
    branch v0, block2, block3
block2:
    branch v1, block1, block4(v2)
block3:
    v3 = iconst 0i32
    return v3
block4(v4: i32):
    return v4
}"#;
        // block4 has a parameter, so we can't delete with multiple exits
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
    jump block2
block2:
    branch v1, block2, block3
block3:
    v4 = iconst 1i32
    v5 = iadd v3, v4
    branch v0, block1(v5), block4(v5)
block4(v6: i32):
    return v6
}"#;
        let expected = r#"function @test(v0: bool, v1: bool) -> i32 {
block0(v0: bool, v1: bool):
    v2 = iconst 0i32
    jump block1(v2)
block1(v3: i32):
    jump block2
block2:
    v4 = iconst 1i32
    v5 = iadd v3, v4
    branch v0, block1(v5), block3(v5)
block3(v6: i32):
    return v6
}"#;
        let mut program = TestProgram::new(input);
        program.run_pass(&LoopSimplify);
        program.run_pass(&LoopDelete);
        program.assert_output(expected);
    }

    /// Exit block parameters receive initial values.
    #[test]
    fn test_delete_pass_initial_values_to_exit() {
        let input = r#"function @test(v0: i32, v1: bool) -> i32 {
block0(v0: i32, v1: bool):
    v2 = iconst 0i32
    jump block1(v2)
block1(v3: i32):
    v4 = iconst 1i32
    v5 = iadd v3, v4
    branch v1, block1(v5), block2(v0)
block2(v6: i32):
    return v6
}"#;
        let expected = r#"function @test(v0: i32, v1: bool) -> i32 {
block0(v0: i32, v1: bool):
    v2 = iconst 0i32
    jump block1(v0)
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
        let input = r#"function @test(v0: bool, v1: i32) -> void {
block0(v0: bool, v1: i32):
    jump block1
block1:
    drop v1
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
}
