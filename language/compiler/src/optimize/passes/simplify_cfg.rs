use std::collections::{HashMap, HashSet};

use destack_compiler_macros::declare_pass;
use destack_mir as mir;

use crate::optimize::{
    AnalysisPreservation, FunctionPass, OptimizationContext, Pass, PassMetadata,
    instruction_substitute_uses, terminator_substitute_uses,
};

declare_pass! {
    /// Simplify the control flow graph.
    ///
    /// This pass performs several CFG simplifications:
    /// 1. Constant branch folding: converts `branch const, A, B` to `jump`
    /// 2. Jump threading: threads jumps through empty blocks
    /// 3. Block merging: merges blocks with single predecessor/successor
    /// 4. Unreachable block elimination: removes blocks not reachable from entry
    ///
    /// ```mir
    /// function @before(v0: i32) -> i32 {
    /// block0(v0: i32):
    ///     v1 = iconst true
    ///     branch v1, block1, block2
    /// block1:
    ///     return v0
    /// block2:
    ///     v2 = iconst 0i32
    ///     return v2
    /// }
    /// ```
    /// becomes:
    /// ```mir
    /// function @after(v0: i32) -> i32 {
    /// block0(v0: i32):
    ///     return v0
    /// }
    /// ```
    #[pass(id = "simplify-cfg")]
    pub SimplifyCfg,
    "Simplify control flow graph"
}

impl Pass for SimplifyCfg {
    fn metadata(&self) -> &'static PassMetadata {
        SimplifyCfg::metadata()
    }
}

impl FunctionPass for SimplifyCfg {
    fn run_on_function(
        &self,
        function: &mut mir::Function,
        tree: &mut mir::NodeTree,
        _context: &OptimizationContext<'_>,
    ) -> AnalysisPreservation {
        let mut changed = false;

        // phase 1: constant branch folding
        // converts `branch const_true, A, B` -> `jump A`
        changed |= fold_constant_branches(function, tree);

        // phase 2: jump threading
        // threads jumps through empty blocks
        changed |= thread_jumps(function, tree);

        // phase 3: block merging
        // merges blocks with single predecessor/successor
        if let Some(entry) = function.entry {
            changed |= merge_blocks(function, tree, entry);
        }

        // phase 4: eliminate unreachable blocks
        if let Some(entry) = function.entry {
            changed |= eliminate_unreachable_blocks(function, tree, entry);
        }

        if changed {
            AnalysisPreservation::none() // CFG changed
        } else {
            AnalysisPreservation::all()
        }
    }
}

/// Fold branches on constant conditions into unconditional jumps.
/// Returns true if any branches were folded.
fn fold_constant_branches(function: &mir::Function, tree: &mut mir::NodeTree) -> bool {
    let mut changed = false;

    // collect all constant values
    let mut constants: HashSet<mir::Value> = HashSet::new();
    let mut true_values: HashSet<mir::Value> = HashSet::new();
    for &block_id in &function.blocks {
        let block = tree.get(block_id);
        for &instruction_id in &block.instructions {
            let instruction = tree.get(instruction_id);
            if let mir::Instruction::Const { destination, value } = instruction {
                constants.insert(*destination);
                if let mir::Constant::Boolean { value: true } = value {
                    true_values.insert(*destination);
                }
            }
        }
    }

    // fold constant branches
    for &block_id in &function.blocks {
        let block = tree.get(block_id);
        if let mir::Terminator::Branch {
            condition,
            then_target,
            then_arguments,
            else_target,
            else_arguments,
        } = &block.terminator
            && constants.contains(condition)
        {
            let is_true = true_values.contains(condition);
            let (target, arguments) = if is_true {
                (*then_target, then_arguments.clone())
            } else {
                (*else_target, else_arguments.clone())
            };

            let mut new_block = block.clone();
            new_block.terminator = mir::Terminator::Jump { target, arguments };
            tree.replace(block_id, new_block);
            changed = true;
        }
    }

    changed
}

/// Thread jumps through empty blocks.
///
/// If a block has no instructions, no parameters, and a simple terminator,
/// predecessors can absorb that terminator directly. For jump terminators,
/// we also resolve chains (A->B->C becomes A->C).
///
/// Returns true if any changes were made.
fn thread_jumps(function: &mir::Function, tree: &mut mir::NodeTree) -> bool {
    // find all empty blocks (no instructions, no parameters) that can be threaded
    let mut threadable: HashMap<mir::LocalNodeId<mir::Block>, mir::Terminator> = HashMap::new();

    for &block_id in &function.blocks {
        let block = tree.get(block_id);

        // block must have no instructions and no parameters to be threadable
        if !block.instructions.is_empty() || !block.parameters.is_empty() {
            continue;
        }

        // check if terminator can be threaded through
        match &block.terminator {
            mir::Terminator::Jump { arguments, .. } if arguments.is_empty() => {
                threadable.insert(block_id, block.terminator.clone());
            }
            mir::Terminator::Return { .. } | mir::Terminator::Unreachable => {
                threadable.insert(block_id, block.terminator.clone());
            }
            _ => {}
        }
    }

    if threadable.is_empty() {
        return false;
    }

    // rewrite terminators
    let mut changed = false;

    for &block_id in &function.blocks {
        let block = tree.get(block_id);

        let new_terminator = match &block.terminator {
            mir::Terminator::Jump { target, arguments } => {
                // resolve the target, following jump chains
                let resolved = resolve_jump_target(*target, &threadable);
                match resolved {
                    ResolvedTarget::Terminator(t) if arguments.is_empty() => {
                        // can absorb any terminator when no arguments
                        Some(t)
                    }
                    ResolvedTarget::Terminator(mir::Terminator::Jump {
                        target: new_target,
                        ..
                    }) => {
                        // redirect to new target, keeping our arguments
                        Some(mir::Terminator::Jump {
                            target: new_target,
                            arguments: arguments.clone(),
                        })
                    }
                    ResolvedTarget::Block(new_target) if new_target != *target => {
                        Some(mir::Terminator::Jump {
                            target: new_target,
                            arguments: arguments.clone(),
                        })
                    }
                    _ => None,
                }
            }
            mir::Terminator::Branch {
                condition,
                then_target,
                then_arguments,
                else_target,
                else_arguments,
            } => {
                // for branches, resolve each target
                let then_resolved = resolve_jump_target(*then_target, &threadable);
                let else_resolved = resolve_jump_target(*else_target, &threadable);

                let new_then = match then_resolved {
                    ResolvedTarget::Block(t) if t != *then_target => Some(t),
                    ResolvedTarget::Terminator(mir::Terminator::Jump { target, .. }) => {
                        Some(target)
                    }
                    _ => None,
                };
                let new_else = match else_resolved {
                    ResolvedTarget::Block(t) if t != *else_target => Some(t),
                    ResolvedTarget::Terminator(mir::Terminator::Jump { target, .. }) => {
                        Some(target)
                    }
                    _ => None,
                };

                if new_then.is_some() || new_else.is_some() {
                    Some(mir::Terminator::Branch {
                        condition: *condition,
                        then_target: new_then.unwrap_or(*then_target),
                        then_arguments: then_arguments.clone(),
                        else_target: new_else.unwrap_or(*else_target),
                        else_arguments: else_arguments.clone(),
                    })
                } else {
                    None
                }
            }
            _ => None,
        };

        if let Some(terminator) = new_terminator {
            let mut new_block = block.clone();
            new_block.terminator = terminator;
            tree.replace(block_id, new_block);
            changed = true;
        }
    }

    changed
}

/// Result of resolving a jump target through threadable blocks.
enum ResolvedTarget {
    /// Resolved to a final block (followed jump chain but ended at non-threadable block).
    Block(mir::LocalNodeId<mir::Block>),
    /// Resolved to a terminator that can be absorbed (return, unreachable, or final jump).
    Terminator(mir::Terminator),
}

/// Resolve a jump target by following through threadable blocks.
fn resolve_jump_target(
    target: mir::LocalNodeId<mir::Block>,
    threadable: &HashMap<mir::LocalNodeId<mir::Block>, mir::Terminator>,
) -> ResolvedTarget {
    let mut current = target;
    let mut visited = HashSet::new();

    loop {
        let Some(terminator) = threadable.get(&current) else {
            // not threadable, return current block
            return ResolvedTarget::Block(current);
        };

        match terminator {
            mir::Terminator::Jump {
                target: next,
                arguments,
            } if arguments.is_empty() => {
                // follow the jump chain
                if !visited.insert(current) {
                    // cycle detected, stop here
                    return ResolvedTarget::Block(current);
                }
                current = *next;
            }
            // non-jump terminator - can be absorbed by predecessor
            _ => {
                return ResolvedTarget::Terminator(terminator.clone());
            }
        }
    }
}

/// Merge blocks where predecessor has single successor and successor has single predecessor.
///
/// If block A unconditionally jumps to block B, and B has no other predecessors,
/// we can merge B's instructions and terminator into A.
///
/// Returns true if any blocks were merged.
fn merge_blocks(
    function: &mut mir::Function,
    tree: &mut mir::NodeTree,
    entry: mir::LocalNodeId<mir::Block>,
) -> bool {
    // build predecessor count for each block
    let mut predecessor_count: HashMap<mir::LocalNodeId<mir::Block>, usize> = HashMap::new();
    for &block_id in &function.blocks {
        predecessor_count.entry(block_id).or_insert(0);
        let block = tree.get(block_id);
        for successor in block.terminator.successors() {
            *predecessor_count.entry(successor).or_insert(0) += 1;
        }
    }

    let mut changed = false;
    let mut merged_away: HashSet<mir::LocalNodeId<mir::Block>> = HashSet::new();

    // iterate until no more merges possible
    loop {
        let mut merged_this_round = false;

        for &block_id in &function.blocks {
            if merged_away.contains(&block_id) {
                continue;
            }

            // extract info from block without holding borrow
            let (target, arguments, block_clone) = {
                let block = tree.get(block_id);
                let mir::Terminator::Jump { target, arguments } = &block.terminator else {
                    continue;
                };
                (*target, arguments.clone(), block.clone())
            };

            // don't merge into ourselves
            if target == block_id {
                continue;
            }

            // target must have exactly one predecessor (us)
            if predecessor_count.get(&target).copied().unwrap_or(0) != 1 {
                continue;
            }

            // don't merge away the entry block
            if target == entry {
                continue;
            }

            // target must not already be merged away
            if merged_away.contains(&target) {
                continue;
            }

            // extract target block info
            let (param_to_arg, target_instructions, target_terminator) = {
                let target_block = tree.get(target);
                let param_to_arg: HashMap<mir::Value, mir::Value> = target_block
                    .parameters
                    .iter()
                    .zip(arguments.iter())
                    .map(|(param, arg)| (param.value, *arg))
                    .collect();
                (
                    param_to_arg,
                    target_block.instructions.clone(),
                    target_block.terminator.clone(),
                )
            };

            // merge: append target's instructions to our block, take target's terminator
            let mut new_block = block_clone;

            // copy and substitute instructions from target
            for instruction_id in target_instructions {
                let instruction = tree.get(instruction_id).clone();
                let new_instruction = instruction_substitute_uses(&instruction, &param_to_arg);
                let new_id = tree.insert(new_instruction);
                new_block.instructions.push(new_id);
            }

            // substitute and take target's terminator
            new_block.terminator = terminator_substitute_uses(&target_terminator, &param_to_arg);

            tree.replace(block_id, new_block);

            // mark target as merged away
            merged_away.insert(target);
            merged_this_round = true;
            changed = true;
        }

        if !merged_this_round {
            break;
        }
    }

    // remove merged blocks from function
    if !merged_away.is_empty() {
        function
            .blocks
            .retain(|block_id| !merged_away.contains(block_id));
    }

    changed
}

/// Eliminate blocks not reachable from the entry block.
/// Returns true if any blocks were removed.
fn eliminate_unreachable_blocks(
    function: &mut mir::Function,
    tree: &mir::NodeTree,
    entry: mir::LocalNodeId<mir::Block>,
) -> bool {
    // find all reachable blocks via BFS from entry
    let mut reachable = HashSet::new();
    let mut worklist = vec![entry];

    while let Some(block_id) = worklist.pop() {
        if !reachable.insert(block_id) {
            continue; // already visited
        }

        let block = tree.get(block_id);
        for successor in block.terminator.successors() {
            if !reachable.contains(&successor) {
                worklist.push(successor);
            }
        }
    }

    // remove unreachable blocks from the function
    let original_len = function.blocks.len();
    function
        .blocks
        .retain(|&block_id| reachable.contains(&block_id));

    function.blocks.len() != original_len
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optimize::common::tests::TestProgram;

    /// Unreachable blocks are eliminated from the function.
    #[test]
    fn test_eliminate_unreachable_block() {
        // block1 is empty (just returns), block2 is unreachable
        let input = r#"function @test() -> i32 {
block0:
    v0 = iconst 1i32
    jump block1
block1:
    return v0
block2:
    v1 = iconst 2i32
    return v1
}"#;
        // block0's jump threads to return, block1 and block2 become unreachable
        let expected = r#"function @test() -> i32 {
block0:
    v0 = iconst 1i32
    return v0
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&SimplifyCfg);
        program.assert_eq(expected);
    }

    /// Chains of unreachable blocks are all eliminated.
    #[test]
    fn test_eliminate_unreachable_chain() {
        let input = r#"function @test() -> i32 {
block0:
    v0 = iconst 1i32
    return v0
block1:
    jump block2
block2:
    v1 = iconst 2i32
    return v1
}"#;
        let expected = r#"function @test() -> i32 {
block0:
    v0 = iconst 1i32
    return v0
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&SimplifyCfg);
        program.assert_eq(expected);
    }

    /// Branch on constant true folds to unconditional jump to then target.
    #[test]
    fn test_fold_constant_true_branch() {
        // branch on true folds to jump, then threads through empty return block
        let input = r#"function @test() -> i32 {
block0:
    v0 = iconst true
    v1 = iconst 1i32
    v2 = iconst 2i32
    branch v0, block1, block2
block1:
    return v1
block2:
    return v2
}"#;
        // branch folds to jump, then threads to return
        let expected = r#"function @test() -> i32 {
block0:
    v0 = iconst true
    v1 = iconst 1i32
    v2 = iconst 2i32
    return v1
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&SimplifyCfg);
        program.assert_eq(expected);
    }

    /// Branch on constant false folds to unconditional jump to else target.
    #[test]
    fn test_fold_constant_false_branch() {
        // branch on false folds to jump, then threads through empty return block
        let input = r#"function @test() -> i32 {
block0:
    v0 = iconst false
    v1 = iconst 1i32
    v2 = iconst 2i32
    branch v0, block1, block2
block1:
    return v1
block2:
    return v2
}"#;
        // branch folds to jump to block2, then threads to return v2
        let expected = r#"function @test() -> i32 {
block0:
    v0 = iconst false
    v1 = iconst 1i32
    v2 = iconst 2i32
    return v2
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&SimplifyCfg);
        program.assert_eq(expected);
    }

    /// Branch on non-constant condition is preserved.
    #[test]
    fn test_preserve_non_constant_branch() {
        // v0 is a parameter, not a constant
        let input = r#"function @test(v0: bool) -> i32 {
block0(v0: bool):
    v1 = iconst 1i32
    v2 = iconst 2i32
    branch v0, block1, block2
block1:
    return v1
block2:
    return v2
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&SimplifyCfg);
        program.assert_unchanged(input);
    }

    /// Constant branch folding makes the else target unreachable.
    #[test]
    fn test_fold_and_eliminate_combined() {
        // branch on true folds to jump to block1, then threads to return
        let input = r#"function @test() -> i32 {
block0:
    v0 = iconst true
    v1 = iconst 42i32
    branch v0, block1, block2
block1:
    return v1
block2:
    v2 = iconst 0i32
    return v2
}"#;
        // branch folds, jump threads through empty block1, block2 eliminated
        let expected = r#"function @test() -> i32 {
block0:
    v0 = iconst true
    v1 = iconst 42i32
    return v1
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&SimplifyCfg);
        program.assert_eq(expected);
    }

    /// Loop back-edges keep loop blocks reachable.
    #[test]
    fn test_preserve_loop_structure() {
        let input = r#"function @test() -> i32 {
block0:
    v0 = iconst 0i32
    jump block1
block1:
    v1 = iconst 10i32
    v2 = icmp_slt v0, v1
    branch v2, block2, block3
block2:
    v3 = iconst 1i32
    v4 = iadd v0, v3
    jump block1
block3:
    return v0
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&SimplifyCfg);
        program.assert_unchanged(input);
    }

    /// Single-block functions with no branches are unchanged.
    #[test]
    fn test_preserve_single_block() {
        let input = r#"function @test() -> i32 {
block0:
    v0 = iconst 42i32
    return v0
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&SimplifyCfg);
        program.assert_unchanged(input);
    }

    /// Nested constant branches all fold, making intermediate blocks unreachable.
    #[test]
    fn test_fold_nested_constant_branches() {
        // v0=true -> block1, v1=false -> block4, block2 and block3 become unreachable
        let input = r#"function @test() -> i32 {
block0:
    v0 = iconst true
    v1 = iconst false
    branch v0, block1, block2
block1:
    branch v1, block3, block4
block2:
    v2 = iconst 2i32
    return v2
block3:
    v3 = iconst 3i32
    return v3
block4:
    v4 = iconst 4i32
    return v4
}"#;
        let expected = r#"function @test() -> i32 {
block0:
    v0 = iconst true
    v1 = iconst false
    jump block1
block1:
    v4 = iconst 4i32
    return v4
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&SimplifyCfg);
        program.assert_eq(expected);
    }

    /// Diamond CFG with non-constant condition is preserved.
    #[test]
    fn test_preserve_diamond_cfg() {
        let input = r#"function @test(v0: bool) -> i32 {
block0(v0: bool):
    v1 = iconst 1i32
    v2 = iconst 2i32
    branch v0, block1, block2
block1:
    jump block3(v1)
block2:
    jump block3(v2)
block3(v3: i32):
    return v3
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&SimplifyCfg);
        program.assert_unchanged(input);
    }

    /// Multiple disconnected unreachable regions are all eliminated.
    #[test]
    fn test_eliminate_multiple_unreachable_regions() {
        let input = r#"function @test() -> i32 {
block0:
    v0 = iconst 1i32
    return v0
block1:
    v1 = iconst 2i32
    jump block2
block2:
    return v1
block3:
    v2 = iconst 3i32
    jump block4
block4:
    return v2
}"#;
        let expected = r#"function @test() -> i32 {
block0:
    v0 = iconst 1i32
    return v0
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&SimplifyCfg);
        program.assert_eq(expected);
    }

    /// Constant branch with block arguments preserves arguments on folded jump.
    #[test]
    fn test_fold_branch_with_block_arguments() {
        let input = r#"function @test() -> i32 {
block0:
    v0 = iconst true
    v1 = iconst 42i32
    v2 = iconst 0i32
    branch v0, block1(v1), block1(v2)
block1(v3: i32):
    return v3
}"#;
        // after folding branch to jump, block merging merges block1 into block0
        let expected = r#"function @test() -> i32 {
block0:
    v0 = iconst true
    v1 = iconst 42i32
    v2 = iconst 0i32
    return v1
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&SimplifyCfg);
        program.assert_eq(expected);
    }

    /// Jump through empty block is threaded to final target.
    #[test]
    fn test_thread_simple_jump() {
        // block1 and block2 are both empty threadable blocks
        let input = r#"function @test() -> i32 {
block0:
    v0 = iconst 42i32
    jump block1
block1:
    jump block2
block2:
    return v0
}"#;
        // block0's jump threads all the way to return, both intermediates become unreachable
        let expected = r#"function @test() -> i32 {
block0:
    v0 = iconst 42i32
    return v0
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&SimplifyCfg);
        program.assert_eq(expected);
    }

    /// Chain of empty jump blocks all thread to final target.
    #[test]
    fn test_thread_jump_chain() {
        // all intermediate blocks are empty and threadable
        let input = r#"function @test() -> i32 {
block0:
    v0 = iconst 42i32
    jump block1
block1:
    jump block2
block2:
    jump block3
block3:
    return v0
}"#;
        // block0's jump threads through entire chain to return
        let expected = r#"function @test() -> i32 {
block0:
    v0 = iconst 42i32
    return v0
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&SimplifyCfg);
        program.assert_eq(expected);
    }

    /// Block with instructions is not threaded through, but its successor can be.
    #[test]
    fn test_preserve_block_with_instructions() {
        // block1 has instructions so can't be threaded through
        // but after threading block1's jump to return, block0 and block1 merge
        let input = r#"function @test() -> i32 {
block0:
    v0 = iconst 1i32
    jump block1
block1:
    v1 = iadd v0, v0
    jump block2
block2:
    return v1
}"#;
        // block1's jump threads to return, then block0 and block1 merge
        let expected = r#"function @test() -> i32 {
block0:
    v0 = iconst 1i32
    v1 = iadd v0, v0
    return v1
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&SimplifyCfg);
        program.assert_eq(expected);
    }

    /// Branch targets through empty blocks are threaded.
    #[test]
    fn test_thread_branch_targets() {
        let input = r#"function @test(v0: bool) -> i32 {
block0(v0: bool):
    v1 = iconst 1i32
    v2 = iconst 2i32
    branch v0, block1, block2
block1:
    jump block3
block2:
    jump block4
block3:
    return v1
block4:
    return v2
}"#;
        let expected = r#"function @test(v0: bool) -> i32 {
block0(v0: bool):
    v1 = iconst 1i32
    v2 = iconst 2i32
    branch v0, block1, block2
block1:
    return v1
block2:
    return v2
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&SimplifyCfg);
        program.assert_eq(expected);
    }

    /// Block with parameters threads through its empty successor to a return.
    #[test]
    fn test_thread_parameterized_block_successor() {
        // block1 has params so can't be threaded through, but block2 is empty and threadable
        let input = r#"function @test(v0: bool) -> i32 {
block0(v0: bool):
    v1 = iconst 1i32
    v2 = iconst 2i32
    branch v0, block1(v1), block1(v2)
block1(v3: i32):
    jump block2
block2:
    return v3
}"#;
        // block1's jump is threaded directly to the return, block2 becomes unreachable
        let expected = r#"function @test(v0: bool) -> i32 {
block0(v0: bool):
    v1 = iconst 1i32
    v2 = iconst 2i32
    branch v0, block1(v1), block1(v2)
block1(v3: i32):
    return v3
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&SimplifyCfg);
        program.assert_eq(expected);
    }
}
