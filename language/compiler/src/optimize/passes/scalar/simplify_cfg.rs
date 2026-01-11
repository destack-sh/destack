use std::collections::{HashMap, HashSet};

use destack_compiler_macros::declare_pass;
use destack_mir as mir;

use crate::optimize::analyses::{ConstantPropagation, RangeAnalysis, ValueRange};
use crate::optimize::{
    AnalysisPreservation, FunctionAnalyses, FunctionPass, PipelineContext,
    instruction_substitute_uses, terminator_substitute_uses,
};

// TODO #Performance: add path sensitive jump threading with edge specific range facts

declare_pass! {
    /// Simplify the control flow graph.
    ///
    /// This pass performs several CFG simplifications:
    /// 1. Branch folding: converts `branch cond, A, B` to `jump` when cond is constant or range-proven
    /// 2. Jump threading: threads jumps through empty or passthrough blocks
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

impl FunctionPass for SimplifyCfg {
    /// Run CFG simplification on a function.
    fn run(
        &self,
        function: &mut mir::Function,
        tree: &mut mir::NodeTree,
        _ctx: &PipelineContext<'_>,
    ) -> AnalysisPreservation {
        // get constant propagation and range analyses
        let (constants, ranges) = {
            let analyses = FunctionAnalyses::new(function, tree);
            (
                analyses.get::<ConstantPropagation>().clone(),
                analyses.get::<RangeAnalysis>().clone(),
            )
        };

        // run simplify CFG
        let changed = run_simplify_cfg(function, tree, &constants, &ranges);

        // select preservation based on CFG changes
        if changed {
            AnalysisPreservation::none()
        } else {
            AnalysisPreservation::all()
        }
    }

    /// Return the pass name.
    fn name(&self) -> &'static str {
        "SimplifyCfg"
    }

    /// Return the pass identifier.
    fn id(&self) -> &'static str {
        "simplify-cfg"
    }
}

/// SimplifyCFG logic. Returns true if changes were made.
fn run_simplify_cfg(
    function: &mut mir::Function,
    tree: &mut mir::NodeTree,
    constants: &ConstantPropagation,
    ranges: &RangeAnalysis,
) -> bool {
    let mut changed = false;

    // phase 1: branch folding
    // converts `branch always_true, A, B` -> `jump A`
    changed |= fold_branches(function, tree, constants, ranges);

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

    changed
}

/// Fold branches on constant conditions into unconditional jumps.
/// Returns true if any branches were folded.
fn fold_branches(
    function: &mir::Function,
    tree: &mut mir::NodeTree,
    constants: &ConstantPropagation,
    ranges: &RangeAnalysis,
) -> bool {
    let mut changed = false;

    // fold branches with constant or range-proven conditions
    for &block_id in &function.blocks {
        let block = tree.get(block_id);
        let exit_ranges = ranges.exit(block_id);
        match &block.terminator {
            mir::Terminator::Branch {
                condition,
                then_target,
                then_arguments,
                else_target,
                else_arguments,
            } => {
                // resolve condition constant
                let condition_constant = constants.constant_at_exit(block_id, *condition);
                let condition_value = match condition_constant {
                    Some(mir::Constant::Boolean { value }) => Some(*value),
                    _ => None,
                };
                let condition_value =
                    condition_value.or_else(|| bool_from_range(exit_ranges.get(*condition)));

                if let Some(is_true) = condition_value {
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
            mir::Terminator::Check {
                condition,
                success,
                failure,
                ..
            } => {
                // resolve condition constant
                let condition_constant = constants.constant_at_exit(block_id, *condition);
                let condition_value = match condition_constant {
                    Some(mir::Constant::Boolean { value }) => Some(*value),
                    _ => None,
                };
                let condition_value =
                    condition_value.or_else(|| bool_from_range(exit_ranges.get(*condition)));

                if let Some(is_true) = condition_value {
                    let (target, arguments) = if is_true {
                        (success.target, success.arguments.clone())
                    } else {
                        (failure.target, failure.arguments.clone())
                    };

                    let mut new_block = block.clone();
                    new_block.terminator = mir::Terminator::Jump { target, arguments };
                    tree.replace(block_id, new_block);
                    changed = true;
                }
            }
            mir::Terminator::Switch {
                value,
                default,
                default_arguments,
                cases,
            } => {
                let constant_value = constants.constant_at_exit(block_id, *value);
                let range_value = exit_ranges.get(*value);
                if let Some(new_terminator) = fold_switch(
                    *value,
                    *default,
                    default_arguments,
                    cases,
                    constant_value,
                    range_value,
                ) {
                    let mut new_block = block.clone();
                    new_block.terminator = new_terminator;
                    tree.replace(block_id, new_block);
                    changed = true;
                }
            }
            _ => {}
        }
    }

    changed
}

/// Convert a boolean range into a constant when possible.
fn bool_from_range(range: Option<&ValueRange>) -> Option<bool> {
    let ValueRange::Boolean {
        can_be_true,
        can_be_false,
    } = range?
    else {
        return None;
    };

    match (*can_be_true, *can_be_false) {
        (true, false) => Some(true),
        (false, true) => Some(false),
        _ => None,
    }
}

/// Fold switch terminators using constant or range information.
fn fold_switch(
    value: mir::Value,
    default: mir::LocalNodeId<mir::Block>,
    default_arguments: &[mir::Value],
    cases: &[mir::SwitchCase],
    constant_value: Option<&mir::Constant>,
    range_value: Option<&ValueRange>,
) -> Option<mir::Terminator> {
    // select a single known constant case
    if let Some(mir::Constant::Int {
        value, is_signed, ..
    }) = constant_value
    {
        return Some(resolve_switch_case(
            *value as i128,
            default,
            default_arguments,
            cases,
            *is_signed,
        ));
    }
    if let Some(mir::Constant::UInt { value, .. }) = constant_value {
        return Some(resolve_switch_case(
            *value as i128,
            default,
            default_arguments,
            cases,
            false,
        ));
    }

    // narrow cases by integer range when possible
    let ValueRange::Integer {
        min,
        max,
        is_signed,
        ..
    } = range_value?
    else {
        return None;
    };

    if min == max {
        return Some(resolve_switch_case(
            *min,
            default,
            default_arguments,
            cases,
            *is_signed,
        ));
    }

    let mut filtered_cases: Vec<mir::SwitchCase> = Vec::new();
    for case in cases {
        let case_value = if *is_signed {
            case.value as i128
        } else {
            case.value as u64 as i128
        };
        if case_value >= *min && case_value <= *max {
            filtered_cases.push(case.clone());
        }
    }

    if filtered_cases.len() == cases.len() {
        return None;
    }

    if filtered_cases.is_empty() {
        return Some(mir::Terminator::Jump {
            target: default,
            arguments: default_arguments.to_vec(),
        });
    }

    Some(mir::Terminator::Switch {
        value,
        default,
        default_arguments: default_arguments.to_vec(),
        cases: filtered_cases,
    })
}

/// Resolve a switch into a jump when the value is known.
fn resolve_switch_case(
    value: i128,
    default: mir::LocalNodeId<mir::Block>,
    default_arguments: &[mir::Value],
    cases: &[mir::SwitchCase],
    is_signed: bool,
) -> mir::Terminator {
    // scan cases for a matching value
    for case in cases {
        // normalize the case value for comparison
        let case_value = if is_signed {
            case.value as i128
        } else {
            case.value as u64 as i128
        };

        // return when the case matches
        if value == case_value {
            return mir::Terminator::Jump {
                target: case.target,
                arguments: case.arguments.clone(),
            };
        }
    }

    // fall back to the default target
    mir::Terminator::Jump {
        target: default,
        arguments: default_arguments.to_vec(),
    }
}

/// Thread jumps through empty or passthrough blocks.
///
/// If a block has no instructions and either has no parameters or just forwards
/// them, predecessors can bypass it. For jump terminators, we resolve chains
/// (A->B->C becomes A->C).
///
/// Returns true if any changes were made.
fn thread_jumps(function: &mir::Function, tree: &mut mir::NodeTree) -> bool {
    // find all empty blocks (no instructions) that can be threaded
    let mut threadable: HashMap<mir::LocalNodeId<mir::Block>, ThreadableBlock> = HashMap::new();

    for &block_id in &function.blocks {
        let block = tree.get(block_id);

        // block must have no instructions to be threadable
        if !block.instructions.is_empty() {
            continue;
        }

        match &block.terminator {
            mir::Terminator::Jump { target, arguments } => {
                if block.parameters.is_empty() && arguments.is_empty() {
                    threadable.insert(
                        block_id,
                        ThreadableBlock::Terminator(block.terminator.clone()),
                    );
                } else if is_passthrough_jump(block, arguments) {
                    threadable.insert(block_id, ThreadableBlock::Forward { target: *target });
                }
            }
            mir::Terminator::Return { .. } | mir::Terminator::Unreachable => {
                if block.parameters.is_empty() {
                    threadable.insert(
                        block_id,
                        ThreadableBlock::Terminator(block.terminator.clone()),
                    );
                }
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
                let resolved = resolve_jump_target(*target, arguments, &threadable);
                match resolved {
                    ResolvedTarget::Terminator(t) if arguments.is_empty() => Some(t),
                    ResolvedTarget::Jump {
                        target: new_target,
                        arguments: new_arguments,
                    } if new_target != *target || &new_arguments != arguments => {
                        Some(mir::Terminator::Jump {
                            target: new_target,
                            arguments: new_arguments,
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
                let then_resolved = resolve_jump_target(*then_target, then_arguments, &threadable);
                let else_resolved = resolve_jump_target(*else_target, else_arguments, &threadable);

                let (new_then, new_then_args) = match then_resolved {
                    ResolvedTarget::Jump { target, arguments } => (target, arguments),
                    _ => (*then_target, then_arguments.clone()),
                };
                let (new_else, new_else_args) = match else_resolved {
                    ResolvedTarget::Jump { target, arguments } => (target, arguments),
                    _ => (*else_target, else_arguments.clone()),
                };

                if new_then != *then_target
                    || new_else != *else_target
                    || new_then_args != *then_arguments
                    || new_else_args != *else_arguments
                {
                    Some(mir::Terminator::Branch {
                        condition: *condition,
                        then_target: new_then,
                        then_arguments: new_then_args,
                        else_target: new_else,
                        else_arguments: new_else_args,
                    })
                } else {
                    None
                }
            }
            mir::Terminator::Check {
                condition,
                constraint,
                success,
                failure,
            } => {
                let success_resolved =
                    resolve_jump_target(success.target, &success.arguments, &threadable);
                let failure_resolved =
                    resolve_jump_target(failure.target, &failure.arguments, &threadable);

                let (new_success, new_success_args) = match success_resolved {
                    ResolvedTarget::Jump { target, arguments } => (target, arguments),
                    _ => (success.target, success.arguments.clone()),
                };
                let (new_failure, new_failure_args) = match failure_resolved {
                    ResolvedTarget::Jump { target, arguments } => (target, arguments),
                    _ => (failure.target, failure.arguments.clone()),
                };

                if new_success != success.target
                    || new_failure != failure.target
                    || new_success_args != success.arguments
                    || new_failure_args != failure.arguments
                {
                    Some(mir::Terminator::Check {
                        condition: *condition,
                        constraint: constraint.clone(),
                        success: mir::CheckTarget {
                            target: new_success,
                            arguments: new_success_args,
                        },
                        failure: mir::CheckTarget {
                            target: new_failure,
                            arguments: new_failure_args,
                        },
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

/// Threadable block metadata.
enum ThreadableBlock {
    /// A terminator that can be absorbed by predecessors.
    Terminator(mir::Terminator),
    /// A passthrough jump that forwards its parameters unchanged.
    Forward {
        target: mir::LocalNodeId<mir::Block>,
    },
}

/// Result of resolving a jump target through threadable blocks.
enum ResolvedTarget {
    /// Resolved to a final jump target with updated arguments.
    Jump {
        /// The final target block.
        target: mir::LocalNodeId<mir::Block>,
        /// The arguments to pass to the target.
        arguments: Vec<mir::Value>,
    },
    /// Resolved to a terminator that can be absorbed (return or unreachable).
    Terminator(mir::Terminator),
}

/// Resolve a jump target by following through threadable blocks.
fn resolve_jump_target(
    target: mir::LocalNodeId<mir::Block>,
    arguments: &[mir::Value],
    threadable: &HashMap<mir::LocalNodeId<mir::Block>, ThreadableBlock>,
) -> ResolvedTarget {
    let mut current = target;
    let mut current_args = arguments.to_vec();
    let mut visited = HashSet::new();

    loop {
        if !visited.insert(current) {
            return ResolvedTarget::Jump {
                target: current,
                arguments: current_args,
            };
        }

        let Some(threadable_block) = threadable.get(&current) else {
            return ResolvedTarget::Jump {
                target: current,
                arguments: current_args,
            };
        };

        match threadable_block {
            ThreadableBlock::Forward {
                target: next_target,
            } => {
                current = *next_target;
            }
            ThreadableBlock::Terminator(terminator) => match terminator {
                mir::Terminator::Jump {
                    target: next_target,
                    arguments,
                } if arguments.is_empty() => {
                    current = *next_target;
                    current_args = Vec::new();
                }
                _ => {
                    return ResolvedTarget::Terminator(terminator.clone());
                }
            },
        }
    }
}

/// Return true if a jump forwards all block parameters unchanged.
fn is_passthrough_jump(block: &mir::Block, arguments: &[mir::Value]) -> bool {
    // require exact parameter and argument alignment
    if block.parameters.len() != arguments.len() {
        return false;
    }

    // verify each parameter is forwarded verbatim
    let mut is_forwarding = true;
    for (param, arg) in block.parameters.iter().zip(arguments.iter()) {
        // stop when a parameter does not match
        if param.value != *arg {
            is_forwarding = false;
            break;
        }
    }

    is_forwarding
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
        program.assert_output(expected);
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
        program.assert_output(expected);
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
        program.assert_output(expected);
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
        program.assert_output(expected);
    }

    /// Branch on a global const folds to the selected target.
    #[test]
    fn test_fold_global_const_branch() {
        let input = r#"global @flag: bool = true ; const
function @test() -> i32 {
block0:
    v0 = global.const @flag
    v1 = iconst 1i32
    v2 = iconst 2i32
    branch v0, block1, block2
block1:
    return v1
block2:
    return v2
}"#;
        let expected = r#"global @flag: bool = true ; const
function @test() -> i32 {
block0:
    v0 = global.const @flag
    v1 = iconst 1i32
    v2 = iconst 2i32
    return v1
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&SimplifyCfg);
        program.assert_output(expected);
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

    /// Branch on a block parameter constant folds to the selected target.
    #[test]
    fn test_fold_block_param_constant_branch() {
        let input = r#"function @test(v0: bool) -> i32 {
block0(v0: bool):
    v1 = iconst true
    branch v0, block1(v1), block2(v1)
block1(v2: bool):
    jump block3(v2)
block2(v3: bool):
    jump block3(v3)
block3(v4: bool):
    branch v4, block4, block5
block4:
    v5 = iconst 1i32
    return v5
block5:
    v6 = iconst 2i32
    return v6
}"#;
        let expected = r#"function @test(v0: bool) -> i32 {
block0(v0: bool):
    v1 = iconst true
    branch v0, block1(v1), block1(v1)
block1(v4: bool):
    v5 = iconst 1i32
    return v5
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&SimplifyCfg);
        program.assert_output(expected);
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
        program.assert_output(expected);
    }

    /// Loop back-edges keep loop blocks reachable.
    #[test]
    fn test_preserve_loop_structure() {
        let input = r#"function @test() -> i32 {
block0:
    v0 = iconst 0i32
    jump block1(v0)
block1(v1: i32):
    v2 = iconst 10i32
    v3 = icmp_slt v1, v2
    branch v3, block2, block3
block2:
    v4 = iconst 1i32
    v5 = iadd v1, v4
    jump block1(v5)
block3:
    return v1
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
        program.assert_output(expected);
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
        program.assert_output(expected);
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
        program.assert_output(expected);
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
        program.assert_output(expected);
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
        program.assert_output(expected);
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
        program.assert_output(expected);
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
        program.assert_output(expected);
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
        program.assert_output(expected);
    }

    /// Range-based branch folding collapses branches on bounded conditions.
    #[test]
    fn test_fold_range_branch_select() {
        let input = r#"function @test(v0: bool) -> i32 {
block0(v0: bool):
    v1 = iconst 0u32
    v2 = iconst 1u32
    v3 = select v0, v1, v2
    v4 = iconst 2u32
    v5 = icmp_ult v3, v4
    v6 = iconst 10i32
    v7 = iconst 20i32
    branch v5, block1, block2
block1:
    return v6
block2:
    return v7
}"#;
        let expected = r#"function @test(v0: bool) -> i32 {
block0(v0: bool):
    v1 = iconst 0u32
    v2 = iconst 1u32
    v3 = select v0, v1, v2
    v4 = iconst 2u32
    v5 = icmp_ult v3, v4
    v6 = iconst 10i32
    v7 = iconst 20i32
    return v6
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&SimplifyCfg);
        program.assert_output(expected);
    }

    /// Range-based switch folding prunes impossible cases.
    #[test]
    fn test_prune_switch_cases_by_range() {
        let input = r#"function @test(v0: bool) -> i32 {
block0(v0: bool):
    v1 = iconst 0u32
    v2 = iconst 1u32
    v3 = select v0, v1, v2
    switch v3, block3, 0 => block1, 1 => block2, 2 => block4
block1:
    v4 = iconst 10i32
    return v4
block2:
    v5 = iconst 11i32
    return v5
block3:
    v6 = iconst 12i32
    return v6
block4:
    v7 = iconst 13i32
    return v7
}"#;
        let expected = r#"function @test(v0: bool) -> i32 {
block0(v0: bool):
    v1 = iconst 0u32
    v2 = iconst 1u32
    v3 = select v0, v1, v2
    switch v3, block3, 0 => block1, 1 => block2
block1:
    v4 = iconst 10i32
    return v4
block2:
    v5 = iconst 11i32
    return v5
block3:
    v6 = iconst 12i32
    return v6
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&SimplifyCfg);
        program.assert_output(expected);
    }

    /// Passthrough blocks forward parameters directly to their successor.
    #[test]
    fn test_thread_passthrough_block_parameters() {
        let input = r#"function @test() -> i32 {
block0:
    v0 = iconst 5i32
    jump block1(v0)
block1(v1: i32):
    jump block2(v1)
block2(v2: i32):
    return v2
}"#;
        let expected = r#"function @test() -> i32 {
block0:
    v0 = iconst 5i32
    jump block1(v0)
block1(v2: i32):
    return v2
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&SimplifyCfg);
        program.assert_output(expected);
    }
}
