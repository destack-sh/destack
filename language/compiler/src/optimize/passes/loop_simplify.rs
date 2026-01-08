use std::collections::HashSet;

use destack_compiler_macros::declare_pass;
use destack_mir as mir;

use crate::optimize::analyses::{Loop, LoopAnalysis};
use crate::optimize::{
    AnalysisPreservation, FunctionPass, OptimizationContext, Pass, PassMetadata,
};

declare_pass! {
    /// Canonicalize loops into a simplified form.
    ///
    /// This pass transforms loops to have:
    /// 1. A preheader: a single block that precedes the loop header
    /// 2. A single latch: one back edge to the header
    ///
    /// These properties simplify subsequent loop transformations like LICM,
    /// loop rotation, and unrolling.
    ///
    /// A preheader is inserted when the loop header has:
    /// - Multiple predecessors from outside the loop
    /// - A single predecessor that also branches elsewhere (not a dedicated entry)
    /// - Is the function entry block
    ///
    /// Latches are merged when a loop has multiple back edges to its header.
    #[pass(id = "loop-simplify")]
    pub LoopSimplify,
    "Canonicalize loops (preheaders, single latch)"
}

impl Pass for LoopSimplify {
    fn metadata(&self) -> &'static PassMetadata {
        LoopSimplify::metadata()
    }
}

impl FunctionPass for LoopSimplify {
    fn run_on_function(
        &self,
        function: &mut mir::Function,
        tree: &mut mir::NodeTree,
        context: &OptimizationContext<'_>,
    ) -> AnalysisPreservation {
        let entry = match function.entry {
            Some(entry) => entry,
            None => return AnalysisPreservation::all(),
        };

        // get loop analysis
        let loops = context.analyses.get::<LoopAnalysis>(function, tree);
        if loops.num_loops() == 0 {
            return AnalysisPreservation::all();
        }

        // collect ALL information upfront before modifying anything
        // this avoids stale cache issues
        let mut preheader_work: Vec<PreheaderWork> = Vec::new();
        let mut latch_work: Vec<LatchWork> = Vec::new();

        for lp in loops.loops() {
            // check if preheader is needed
            if needs_preheader(lp, function, tree, entry) {
                preheader_work.push(PreheaderWork {
                    header: lp.header,
                    loop_blocks: lp.blocks.clone(),
                });
            }

            // check if latch merging is needed
            if !lp.has_single_latch() {
                latch_work.push(LatchWork {
                    header: lp.header,
                    latches: lp.latches.clone(),
                });
            }
        }

        // done with analysis, drop it before modifying
        drop(loops);

        // nothing to do
        if preheader_work.is_empty() && latch_work.is_empty() {
            return AnalysisPreservation::all();
        }

        // phase 1: insert preheaders
        for work in preheader_work {
            insert_preheader(work.header, &work.loop_blocks, function, tree, entry);
        }

        // phase 2: merge latches
        for work in latch_work {
            merge_latches(work.header, &work.latches, function, tree);
        }

        AnalysisPreservation::none()
    }
}

/// Work item for preheader insertion.
struct PreheaderWork {
    header: mir::LocalNodeId<mir::Block>,
    loop_blocks: HashSet<mir::LocalNodeId<mir::Block>>,
}

/// Work item for latch merging.
struct LatchWork {
    header: mir::LocalNodeId<mir::Block>,
    latches: Vec<mir::LocalNodeId<mir::Block>>,
}

/// Check if a loop needs a preheader.
///
/// A loop needs a preheader if:
/// - The header is the function entry block
/// - Multiple blocks outside the loop jump to the header
/// - A single outside predecessor also branches elsewhere
fn needs_preheader(
    lp: &Loop,
    function: &mir::Function,
    tree: &mir::NodeTree,
    entry: mir::LocalNodeId<mir::Block>,
) -> bool {
    // entry block always needs preheader if it's a loop header
    if lp.header == entry {
        return true;
    }

    // find outside predecessors
    let mut outside_preds: Vec<mir::LocalNodeId<mir::Block>> = Vec::new();
    for &block_id in &function.blocks {
        if lp.blocks.contains(&block_id) {
            continue;
        }

        let block = tree.get(block_id);
        if block.terminator.successors().contains(&lp.header) {
            outside_preds.push(block_id);
        }
    }

    // need preheader if multiple outside predecessors
    if outside_preds.len() > 1 {
        return true;
    }

    // need preheader if single predecessor has other successors (branch)
    if outside_preds.len() == 1 {
        let pred = outside_preds[0];
        let pred_block = tree.get(pred);
        if pred_block.terminator.successors().len() > 1 {
            return true;
        }
    }

    false
}

/// Insert a preheader block for a loop.
///
/// Creates a new block that:
/// 1. Has the same parameters as the header
/// 2. Receives all edges from outside the loop
/// 3. Unconditionally jumps to the header
fn insert_preheader(
    header: mir::LocalNodeId<mir::Block>,
    loop_blocks: &HashSet<mir::LocalNodeId<mir::Block>>,
    function: &mut mir::Function,
    tree: &mut mir::NodeTree,
    entry: mir::LocalNodeId<mir::Block>,
) {
    let header_block = tree.get(header);
    let header_params = header_block.parameters.clone();

    // create preheader with fresh parameters matching header's types
    let preheader_params: Vec<mir::TypedValue> = header_params
        .iter()
        .map(|p| mir::TypedValue {
            value: function.next_value(),
            ty: p.ty,
        })
        .collect();

    // preheader unconditionally jumps to header, forwarding its parameters
    let preheader_args: Vec<mir::Value> = preheader_params.iter().map(|p| p.value).collect();
    let preheader = mir::Block {
        parameters: preheader_params,
        instructions: vec![],
        terminator: mir::Terminator::Jump {
            target: header,
            arguments: preheader_args,
        },
    };
    let preheader_id = tree.insert(preheader);
    function.blocks.push(preheader_id);

    // redirect all outside predecessors to preheader
    for &block_id in &function.blocks {
        // skip preheader itself and blocks inside the loop
        if block_id == preheader_id || loop_blocks.contains(&block_id) {
            continue;
        }

        let block = tree.get(block_id);
        let new_terminator = redirect_terminator(&block.terminator, header, preheader_id);
        if new_terminator != block.terminator {
            let mut new_block = block.clone();
            new_block.terminator = new_terminator;
            tree.replace(block_id, new_block);
        }
    }

    // if header was the entry, preheader becomes entry
    if header == entry {
        function.entry = Some(preheader_id);
    }
}

/// Redirect terminator edges from old_target to new_target.
fn redirect_terminator(
    terminator: &mir::Terminator,
    old_target: mir::LocalNodeId<mir::Block>,
    new_target: mir::LocalNodeId<mir::Block>,
) -> mir::Terminator {
    match terminator {
        mir::Terminator::Jump { target, arguments } if *target == old_target => {
            mir::Terminator::Jump {
                target: new_target,
                arguments: arguments.clone(),
            }
        }

        mir::Terminator::Branch {
            condition,
            then_target,
            then_arguments,
            else_target,
            else_arguments,
        } => {
            let new_then = if *then_target == old_target {
                new_target
            } else {
                *then_target
            };
            let new_else = if *else_target == old_target {
                new_target
            } else {
                *else_target
            };

            if new_then != *then_target || new_else != *else_target {
                mir::Terminator::Branch {
                    condition: *condition,
                    then_target: new_then,
                    then_arguments: then_arguments.clone(),
                    else_target: new_else,
                    else_arguments: else_arguments.clone(),
                }
            } else {
                terminator.clone()
            }
        }

        mir::Terminator::Switch {
            value,
            default,
            default_arguments,
            cases,
        } => {
            let new_default = if *default == old_target {
                new_target
            } else {
                *default
            };
            let new_cases: Vec<_> = cases
                .iter()
                .map(|case| mir::SwitchCase {
                    value: case.value,
                    target: if case.target == old_target {
                        new_target
                    } else {
                        case.target
                    },
                    arguments: case.arguments.clone(),
                })
                .collect();
            let cases_changed = cases
                .iter()
                .zip(new_cases.iter())
                .any(|(old, new)| old.target != new.target);

            if new_default != *default || cases_changed {
                mir::Terminator::Switch {
                    value: *value,
                    default: new_default,
                    default_arguments: default_arguments.clone(),
                    cases: new_cases,
                }
            } else {
                terminator.clone()
            }
        }

        _ => terminator.clone(),
    }
}

/// Merge multiple latch blocks into a single latch.
///
/// Creates a new latch block that:
/// 1. Has the same parameters as the header
/// 2. All original latches redirect to it
/// 3. Unconditionally jumps to the header
fn merge_latches(
    header: mir::LocalNodeId<mir::Block>,
    latches: &[mir::LocalNodeId<mir::Block>],
    function: &mut mir::Function,
    tree: &mut mir::NodeTree,
) {
    if latches.len() <= 1 {
        return;
    }

    let header_block = tree.get(header);
    let header_params = header_block.parameters.clone();

    // create merged latch with fresh parameters matching header's types
    let latch_params: Vec<mir::TypedValue> = header_params
        .iter()
        .map(|p| mir::TypedValue {
            value: function.next_value(),
            ty: p.ty,
        })
        .collect();

    // merged latch jumps to header, forwarding its parameters
    let latch_args: Vec<mir::Value> = latch_params.iter().map(|p| p.value).collect();
    let new_latch = mir::Block {
        parameters: latch_params,
        instructions: vec![],
        terminator: mir::Terminator::Jump {
            target: header,
            arguments: latch_args,
        },
    };
    let new_latch_id = tree.insert(new_latch);
    function.blocks.push(new_latch_id);

    // redirect all original latches to the new merged latch
    for &latch_id in latches {
        let latch_block = tree.get(latch_id);
        let new_terminator = redirect_terminator(&latch_block.terminator, header, new_latch_id);
        if new_terminator != latch_block.terminator {
            let mut new_block = latch_block.clone();
            new_block.terminator = new_terminator;
            tree.replace(latch_id, new_block);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optimize::common::tests::TestProgram;

    /// Loop with multiple entry edges gets a preheader inserted.
    #[test]
    fn test_insert_preheader_for_multiple_entries() {
        // block0 and block1 both enter block2, so preheader needed
        let input = r#"function @test(v0: bool, v1: bool) -> void {
block0(v0: bool, v1: bool):
    branch v0, block1, block2
block1:
    jump block2
block2:
    branch v1, block2, block3
block3:
    return
}"#;
        let expected = r#"function @test(v0: bool, v1: bool) -> void {
block0(v0: bool, v1: bool):
    branch v0, block1, block4
block1:
    jump block4
block2:
    branch v1, block2, block3
block3:
    return
block4:
    jump block2
}"#;
        let mut program = TestProgram::new(input);
        program.run_pass(&LoopSimplify);
        program.assert_output(expected);
    }

    /// Loop already in simplified form is unchanged.
    #[test]
    fn test_preserve_already_canonical() {
        let input = r#"function @test(v0: bool) -> void {
block0(v0: bool):
    jump block1
block1:
    branch v0, block2, block3
block2:
    jump block1
block3:
    return
}"#;
        let mut program = TestProgram::new(input);
        program.run_pass(&LoopSimplify);
        program.assert_unchanged(input);
    }

    /// Self-loop at entry block gets a preheader.
    #[test]
    fn test_insert_preheader_for_entry_loop() {
        // block0 is entry and has self-loop, needs preheader
        let input = r#"function @test(v0: bool) -> void {
block0(v0: bool):
    branch v0, block0(v0), block1
block1:
    return
}"#;
        // preheader (block2) becomes new entry with fresh param v1
        let expected = r#"function @test(v0: bool) -> void {
block0(v0: bool):
    branch v0, block0(v0), block1
block1:
    return
block2(v1: bool):
    jump block0(v1)
}"#;
        let mut program = TestProgram::new(input);
        program.run_pass(&LoopSimplify);
        program.assert_output(expected);

        // verify entry changed to preheader
        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = program.tree.get(function_id);
        assert_ne!(function.entry.unwrap(), function.blocks[0]);
    }

    /// Multiple latches are merged into single latch.
    #[test]
    fn test_merge_multiple_latches() {
        // block2 and block3 both jump to block1 (two latches)
        let input = r#"function @test(v0: bool, v1: bool) -> void {
block0(v0: bool, v1: bool):
    jump block1
block1:
    branch v0, block2, block3
block2:
    jump block1
block3:
    branch v1, block1, block4
block4:
    return
}"#;
        // merged latch (block5) inserted, block2 and block3 redirect to it
        let expected = r#"function @test(v0: bool, v1: bool) -> void {
block0(v0: bool, v1: bool):
    jump block1
block1:
    branch v0, block2, block3
block2:
    jump block5
block3:
    branch v1, block5, block4
block4:
    return
block5:
    jump block1
}"#;
        let mut program = TestProgram::new(input);
        program.run_pass(&LoopSimplify);
        program.assert_output(expected);
    }

    /// Function without loops is unchanged.
    #[test]
    fn test_preserve_no_loops() {
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
        program.run_pass(&LoopSimplify);
        program.assert_unchanged(input);
    }

    /// Nested loops both get simplified.
    #[test]
    fn test_simplify_nested_loops() {
        // outer loop: header=block1, latches={block2, block3}
        // inner loop: header=block2, latch={block2} (self-loop)
        let input = r#"function @test(v0: bool, v1: bool, v2: bool) -> void {
block0(v0: bool, v1: bool, v2: bool):
    branch v0, block1, block4
block1:
    branch v1, block2, block3
block2:
    branch v2, block2, block1
block3:
    jump block1
block4:
    return
}"#;
        // block5 = preheader for outer loop (block1, header ID comes first)
        // block6 = preheader for inner loop (block2)
        // block7 = merged latch for outer loop
        let expected = r#"function @test(v0: bool, v1: bool, v2: bool) -> void {
block0(v0: bool, v1: bool, v2: bool):
    branch v0, block5, block4
block1:
    branch v1, block6, block3
block2:
    branch v2, block2, block7
block3:
    jump block7
block4:
    return
block5:
    jump block1
block6:
    jump block2
block7:
    jump block1
}"#;
        let mut program = TestProgram::new(input);
        program.run_pass(&LoopSimplify);
        program.assert_output(expected);
    }

    /// Header parameters are preserved through preheader.
    #[test]
    fn test_preserve_header_parameters() {
        // block2 has parameter v3, preheader must forward it
        // input has v0-v3, so next value is v4
        let input = r#"function @test(v0: bool, v1: i32) -> i32 {
block0(v0: bool, v1: i32):
    branch v0, block1(v1), block2(v1)
block1(v2: i32):
    jump block2(v2)
block2(v3: i32):
    branch v0, block2(v3), block3
block3:
    return v3
}"#;
        // preheader (block4) has parameter v4 and forwards to block2
        let expected = r#"function @test(v0: bool, v1: i32) -> i32 {
block0(v0: bool, v1: i32):
    branch v0, block1(v1), block4(v1)
block1(v2: i32):
    jump block4(v2)
block2(v3: i32):
    branch v0, block2(v3), block3
block3:
    return v3
block4(v4: i32):
    jump block2(v4)
}"#;
        let mut program = TestProgram::new(input);
        program.run_pass(&LoopSimplify);
        program.assert_output(expected);
    }

    /// Single predecessor with branch still gets preheader.
    #[test]
    fn test_insert_preheader_for_branch_predecessor() {
        // block0 branches to block1, not a dedicated entry
        let input = r#"function @test(v0: bool, v1: bool) -> void {
block0(v0: bool, v1: bool):
    branch v0, block1, block2
block1:
    branch v1, block1, block2
block2:
    return
}"#;
        // preheader (block3) inserted for block1's loop
        let expected = r#"function @test(v0: bool, v1: bool) -> void {
block0(v0: bool, v1: bool):
    branch v0, block3, block2
block1:
    branch v1, block1, block2
block2:
    return
block3:
    jump block1
}"#;
        let mut program = TestProgram::new(input);
        program.run_pass(&LoopSimplify);
        program.assert_output(expected);
    }

    /// While-style loop with dedicated entry is unchanged.
    #[test]
    fn test_preserve_while_loop_with_preheader() {
        let input = r#"function @test(v0: bool) -> void {
block0(v0: bool):
    jump block1
block1:
    branch v0, block2, block3
block2:
    jump block1
block3:
    return
}"#;
        let mut program = TestProgram::new(input);
        program.run_pass(&LoopSimplify);
        program.assert_unchanged(input);
    }

    /// Multiple latches with different arguments to header are merged correctly.
    #[test]
    fn test_merge_latches_with_different_arguments() {
        // block2 jumps to block1(v5), block4 jumps to block1(v7) - two latches
        // input has v0-v7, so next value is v8
        let input = r#"function @test(v0: bool, v1: bool) -> i32 {
block0(v0: bool, v1: bool):
    v2 = iconst 0i32
    jump block1(v2)
block1(v3: i32):
    branch v0, block2, block3
block2:
    v4 = iconst 1i32
    v5 = iadd v3, v4
    jump block1(v5)
block3:
    branch v1, block4, block5
block4:
    v6 = iconst 10i32
    v7 = iadd v3, v6
    jump block1(v7)
block5:
    return v3
}"#;
        // merged latch (block6) receives parameter v8 and forwards to header
        let expected = r#"function @test(v0: bool, v1: bool) -> i32 {
block0(v0: bool, v1: bool):
    v2 = iconst 0i32
    jump block1(v2)
block1(v3: i32):
    branch v0, block2, block3
block2:
    v4 = iconst 1i32
    v5 = iadd v3, v4
    jump block6(v5)
block3:
    branch v1, block4, block5
block4:
    v6 = iconst 10i32
    v7 = iadd v3, v6
    jump block6(v7)
block5:
    return v3
block6(v8: i32):
    jump block1(v8)
}"#;
        let mut program = TestProgram::new(input);
        program.run_pass(&LoopSimplify);
        program.assert_output(expected);
    }

    /// Switch terminator entering loop gets redirected to preheader.
    #[test]
    fn test_switch_entry_to_loop() {
        // switch default and case 0 both go to block1
        let input = r#"function @test(v0: i32, v1: bool) -> void {
block0(v0: i32, v1: bool):
    switch v0, block1, 0 => block1, 1 => block2
block1:
    branch v1, block1, block2
block2:
    return
}"#;
        // preheader (block3) inserted, switch redirected
        let expected = r#"function @test(v0: i32, v1: bool) -> void {
block0(v0: i32, v1: bool):
    switch v0, block3, 0 => block3, 1 => block2
block1:
    branch v1, block1, block2
block2:
    return
block3:
    jump block1
}"#;
        let mut program = TestProgram::new(input);
        program.run_pass(&LoopSimplify);
        program.assert_output(expected);
    }

    /// Loop with no outside predecessors (infinite loop from entry) gets preheader.
    #[test]
    fn test_infinite_loop_from_entry() {
        // block0 is entry and only jumps to itself
        // input has v0, so next value is v1
        let input = r#"function @test(v0: bool) -> void {
block0(v0: bool):
    jump block0(v0)
}"#;
        // preheader (block1) becomes entry with fresh param v1
        let expected = r#"function @test(v0: bool) -> void {
block0(v0: bool):
    jump block0(v0)
block1(v1: bool):
    jump block0(v1)
}"#;
        let mut program = TestProgram::new(input);
        program.run_pass(&LoopSimplify);
        program.assert_output(expected);

        // verify entry changed to preheader
        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = program.tree.get(function_id);
        assert_ne!(function.entry.unwrap(), function.blocks[0]);
    }
}
