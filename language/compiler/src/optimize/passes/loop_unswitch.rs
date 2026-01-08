use std::collections::{HashMap, HashSet};

use destack_compiler_macros::declare_pass;
use destack_mir as mir;

use crate::optimize::analyses::{DominatorTree, Loop, LoopAnalysis};
use crate::optimize::common::{instruction_map, terminator_remap};
use crate::optimize::{
    AnalysisPreservation, FunctionPass, OptimizationContext, Pass, PassMetadata,
};

declare_pass! {
    /// Move loop-invariant conditionals outside of loops by duplicating the loop.
    ///
    /// This eliminates the branch inside the loop, improving branch prediction
    /// and enabling further optimizations on each specialized copy.
    ///
    /// ```mir
    /// function @before(v0: bool) -> void {
    /// block0(v0: bool):
    ///     jump block1
    /// block1:
    ///     branch v0, block2, block3
    /// block2:
    ///     jump block1
    /// block3:
    ///     return
    /// }
    /// ```
    /// becomes:
    /// ```mir
    /// function @after(v0: bool) -> void {
    /// block0(v0: bool):
    ///     branch v0, block1, block2
    /// block1:
    ///     jump block3
    /// block2:
    ///     return
    /// block3:
    ///     jump block1
    /// block4:
    ///     jump block5
    /// block5:
    ///     return
    /// }
    /// ```
    ///
    /// Restrictions:
    /// - Only unswitches loops with a branch in the header
    /// - Only unswitches when the condition is loop-invariant
    /// - Only unswitches small loops to avoid excessive code growth
    /// - Requires canonical loop form from LoopSimplify
    #[pass(id = "loop-unswitch")]
    pub LoopUnswitch,
    "Loop unswitching for invariant conditionals"
}

/// Maximum number of instructions in a loop to consider for unswitching.
const MAX_LOOP_SIZE: usize = 50;

impl Pass for LoopUnswitch {
    fn metadata(&self) -> &'static PassMetadata {
        LoopUnswitch::metadata()
    }
}

impl FunctionPass for LoopUnswitch {
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

        // find first unswitchable loop (innermost first)
        let mut candidate: Option<UnswitchCandidate> = None;
        for lp in loops.loops().iter().rev() {
            if let Some(c) = find_unswitchable_loop(lp, function, tree, &domtree) {
                candidate = Some(c);
                break;
            }
        }

        drop(domtree);
        drop(loops);

        // unswitch at most one loop per pass invocation
        let candidate = match candidate {
            Some(c) => c,
            None => return AnalysisPreservation::all(),
        };

        unswitch_loop(function, tree, &candidate);

        AnalysisPreservation::none()
    }
}

/// Information needed to unswitch a loop.
struct UnswitchCandidate {
    /// The preheader block.
    preheader: mir::LocalNodeId<mir::Block>,
    /// The loop header (contains the invariant branch).
    header: mir::LocalNodeId<mir::Block>,
    /// The invariant condition value.
    condition: mir::Value,
    /// The "then" successor of the branch.
    then_target: mir::LocalNodeId<mir::Block>,
    /// Arguments passed to then_target.
    then_arguments: Vec<mir::Value>,
    /// The "else" successor of the branch.
    else_target: mir::LocalNodeId<mir::Block>,
    /// Arguments passed to else_target.
    else_arguments: Vec<mir::Value>,
    /// All blocks in the loop.
    loop_blocks: HashSet<mir::LocalNodeId<mir::Block>>,
    /// Arguments passed from preheader to header.
    preheader_to_header_args: Vec<mir::Value>,
}

/// Check if a loop can be unswitched.
fn find_unswitchable_loop(
    lp: &Loop,
    function: &mir::Function,
    tree: &mir::NodeTree,
    domtree: &DominatorTree,
) -> Option<UnswitchCandidate> {
    // need a preheader
    let preheader = domtree.immediate_dominator(lp.header)?;
    if lp.blocks.contains(&preheader) {
        return None;
    }

    // header must have a branch terminator
    let header = lp.header;
    let header_block = tree.get(header);
    let (condition, then_target, then_arguments, else_target, else_arguments) =
        match &header_block.terminator {
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
            _ => return None,
        };

    // check loop size
    let loop_size: usize = lp
        .blocks
        .iter()
        .map(|&b| tree.get(b).instructions.len())
        .sum();
    if loop_size > MAX_LOOP_SIZE {
        return None;
    }

    // condition must be loop-invariant
    let invariant_values = collect_invariant_values(lp, function, tree);
    if !invariant_values.contains(&condition) {
        return None;
    }

    // both targets must be different (otherwise branch is effectively a jump)
    if then_target == else_target {
        return None;
    }

    // at least one branch must stay in the loop
    let then_in_loop = lp.blocks.contains(&then_target);
    let else_in_loop = lp.blocks.contains(&else_target);
    if !then_in_loop && !else_in_loop {
        return None;
    }

    // get preheader to header arguments
    let preheader_block = tree.get(preheader);
    let preheader_to_header_args = match &preheader_block.terminator {
        mir::Terminator::Jump { target, arguments } if *target == header => arguments.clone(),
        _ => return None,
    };

    Some(UnswitchCandidate {
        preheader,
        header,
        condition,
        then_target,
        then_arguments,
        else_target,
        else_arguments,
        loop_blocks: lp.blocks.clone(),
        preheader_to_header_args,
    })
}

/// Collect all values that are invariant (defined outside the loop).
fn collect_invariant_values(
    lp: &Loop,
    function: &mir::Function,
    tree: &mir::NodeTree,
) -> HashSet<mir::Value> {
    let mut invariant = HashSet::new();

    // function parameters
    for param in &function.parameters {
        invariant.insert(param.value);
    }

    // values from blocks outside the loop
    for &block_id in &function.blocks {
        if lp.blocks.contains(&block_id) {
            continue;
        }

        let block = tree.get(block_id);
        for param in &block.parameters {
            invariant.insert(param.value);
        }

        for &instruction_id in &block.instructions {
            let instruction = tree.get(instruction_id);
            if let Some(destination) = instruction.destination() {
                invariant.insert(destination);
            }
        }
    }

    invariant
}

/// Perform loop unswitching transformation.
fn unswitch_loop(
    function: &mut mir::Function,
    tree: &mut mir::NodeTree,
    candidate: &UnswitchCandidate,
) {
    // clone all loop blocks with fresh IDs and values
    let (block_map, value_map) = clone_loop_blocks(&candidate.loop_blocks, function, tree);

    // get the cloned header
    let cloned_header = block_map[&candidate.header];

    // modify original header: always take the "then" branch
    let mut header = tree.get(candidate.header).clone();
    header.terminator = mir::Terminator::Jump {
        target: candidate.then_target,
        arguments: candidate.then_arguments.clone(),
    };
    tree.replace(candidate.header, header);

    // modify cloned header: always take the "else" branch
    let mut cloned = tree.get(cloned_header).clone();
    let else_target = if candidate.loop_blocks.contains(&candidate.else_target) {
        block_map[&candidate.else_target]
    } else {
        candidate.else_target
    };
    let else_arguments: Vec<mir::Value> = candidate
        .else_arguments
        .iter()
        .map(|v| *value_map.get(v).unwrap_or(v))
        .collect();
    cloned.terminator = mir::Terminator::Jump {
        target: else_target,
        arguments: else_arguments,
    };
    tree.replace(cloned_header, cloned);

    // modify preheader: branch based on condition
    let mut preheader = tree.get(candidate.preheader).clone();
    preheader.terminator = mir::Terminator::Branch {
        condition: candidate.condition,
        then_target: candidate.header,
        then_arguments: candidate.preheader_to_header_args.clone(),
        else_target: cloned_header,
        else_arguments: candidate.preheader_to_header_args.clone(),
    };
    tree.replace(candidate.preheader, preheader);

    // remap terminators in cloned blocks (except header which we already handled)
    for (&original, &cloned_id) in &block_map {
        if original == candidate.header {
            continue;
        }

        let mut block = tree.get(cloned_id).clone();
        terminator_remap(&mut block.terminator, &block_map, &value_map);
        tree.replace(cloned_id, block);
    }

    // add cloned blocks to function (sorted for deterministic output)
    let mut cloned_blocks: Vec<_> = block_map.values().copied().collect();
    cloned_blocks.sort();
    for cloned_block in cloned_blocks {
        function.blocks.push(cloned_block);
    }
}

/// Clone all blocks in the loop, creating fresh block IDs and value IDs.
fn clone_loop_blocks(
    loop_blocks: &HashSet<mir::LocalNodeId<mir::Block>>,
    function: &mut mir::Function,
    tree: &mut mir::NodeTree,
) -> (
    HashMap<mir::LocalNodeId<mir::Block>, mir::LocalNodeId<mir::Block>>,
    HashMap<mir::Value, mir::Value>,
) {
    let mut block_map = HashMap::new();
    let mut value_map = HashMap::new();

    // sort blocks for deterministic output
    let mut sorted_blocks: Vec<_> = loop_blocks.iter().copied().collect();
    sorted_blocks.sort();

    // first pass: allocate new values and create placeholder blocks
    for block_id in &sorted_blocks {
        let block_id = *block_id;
        let original = tree.get(block_id);

        // create new block parameters with fresh values
        let new_params: Vec<mir::TypedValue> = original
            .parameters
            .iter()
            .map(|param| {
                let new_value = function.next_value();
                value_map.insert(param.value, new_value);
                mir::TypedValue {
                    value: new_value,
                    ty: param.ty,
                }
            })
            .collect();

        // allocate fresh values for instruction destinations
        for &instruction_id in &original.instructions {
            let instruction = tree.get(instruction_id);
            if let Some(destination) = instruction.destination() {
                let new_value = function.next_value();
                value_map.insert(destination, new_value);
            }
        }

        // create block with cloned terminator (will be remapped later)
        let new_block = mir::Block {
            parameters: new_params,
            instructions: Vec::new(),
            terminator: original.terminator.clone(),
        };
        let new_block_id = tree.insert(new_block);
        block_map.insert(block_id, new_block_id);
    }

    // second pass: clone instructions with remapped values
    for &block_id in &sorted_blocks {
        let new_block_id = block_map[&block_id];

        // collect instruction IDs to avoid borrowing tree during iteration
        let instruction_ids: Vec<_> = tree.get(block_id).instructions.clone();

        let mut new_instructions = Vec::new();
        for instruction_id in instruction_ids {
            let original_instruction = tree.get(instruction_id).clone();
            let new_instruction = instruction_map(&original_instruction, &value_map, tree);
            let new_instruction_id = tree.insert(new_instruction);
            new_instructions.push(new_instruction_id);
        }

        // update block with cloned instructions
        let mut new_block = tree.get(new_block_id).clone();
        new_block.instructions = new_instructions;
        tree.replace(new_block_id, new_block);
    }

    (block_map, value_map)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optimize::common::tests::TestProgram;
    use crate::optimize::passes::LoopSimplify;

    /// Loop with invariant condition in header is unswitched.
    #[test]
    fn test_unswitch_invariant_branch() {
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
        // after unswitching on v0:
        // - preheader branches on v0
        // - then branch: original loop with header jumping to block2 path
        // - else branch: cloned loop with header jumping to block3 path
        let expected = r#"function @test(v0: bool, v1: bool) -> void {
block0(v0: bool, v1: bool):
    branch v0, block1, block6
block1:
    jump block2
block2:
    jump block5
block3:
    branch v1, block5, block4
block4:
    return
block5:
    jump block1
block6:
    jump block8
block7:
    jump block9
block8:
    branch v1, block9, block4
block9:
    jump block6
}"#;
        let mut program = TestProgram::new(input);
        program.run_pass(&LoopSimplify);
        program.run_pass(&LoopUnswitch);
        program.assert_output(expected);
    }

    /// Loop with invariant condition creates two specialized loops.
    #[test]
    fn test_unswitch_creates_two_loops() {
        // loop where both branches stay in loop, with different bodies
        let input = r#"function @test(v0: bool, v1: bool) -> void {
block0(v0: bool, v1: bool):
    jump block1
block1:
    branch v0, block2, block3
block2:
    branch v1, block1, block4
block3:
    branch v1, block1, block4
block4:
    return
}"#;
        // after unswitching: two loops, one always taking block2 path, one always taking block3 path
        let expected = r#"function @test(v0: bool, v1: bool) -> void {
block0(v0: bool, v1: bool):
    branch v0, block1, block6
block1:
    jump block2
block2:
    branch v1, block5, block4
block3:
    branch v1, block5, block4
block4:
    return
block5:
    jump block1
block6:
    jump block8
block7:
    branch v1, block9, block4
block8:
    branch v1, block9, block4
block9:
    jump block6
}"#;
        let mut program = TestProgram::new(input);
        program.run_pass(&LoopSimplify);
        program.run_pass(&LoopUnswitch);
        program.assert_output(expected);
    }

    /// Loop with variant condition is preserved.
    #[test]
    fn test_preserve_variant_condition() {
        let input = r#"function @test(v0: i32, v1: bool) -> void {
block0(v0: i32, v1: bool):
    v2 = iconst 0i32
    jump block1(v2)
block1(v3: i32):
    v4 = iconst 10i32
    v5 = icmp_slt v3, v4
    branch v5, block2, block4
block2:
    v6 = iconst 1i32
    v7 = iadd v3, v6
    jump block1(v7)
block4:
    return
}"#;
        let mut program = TestProgram::new(input);
        program.run_pass(&LoopSimplify);
        let before = program.format();
        program.run_pass(&LoopUnswitch);
        program.assert_output(&before);
    }

    /// Loop without branch in header is preserved.
    #[test]
    fn test_preserve_jump_header() {
        let input = r#"function @test(v0: bool) -> void {
block0(v0: bool):
    jump block1
block1:
    jump block2
block2:
    branch v0, block1, block3
block3:
    return
}"#;
        let mut program = TestProgram::new(input);
        program.run_pass(&LoopSimplify);
        let before = program.format();
        program.run_pass(&LoopUnswitch);
        program.assert_output(&before);
    }

    /// Loop where both branch targets exit is preserved.
    #[test]
    fn test_preserve_both_targets_exit() {
        let input = r#"function @test(v0: bool) -> void {
block0(v0: bool):
    jump block1
block1:
    branch v0, block2, block3
block2:
    return
block3:
    return
}"#;
        let mut program = TestProgram::new(input);
        program.run_pass(&LoopSimplify);
        let before = program.format();
        program.run_pass(&LoopUnswitch);
        program.assert_output(&before);
    }

    /// Loop with identical branch targets is preserved.
    #[test]
    fn test_preserve_same_branch_targets() {
        let input = r#"function @test(v0: bool) -> void {
block0(v0: bool):
    jump block1
block1:
    branch v0, block2, block2
block2:
    branch v0, block1, block3
block3:
    return
}"#;
        let mut program = TestProgram::new(input);
        program.run_pass(&LoopSimplify);
        let before = program.format();
        program.run_pass(&LoopUnswitch);
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
        program.run_pass(&LoopUnswitch);
        program.assert_unchanged(input);
    }

    /// Loop with block parameters is unswitched correctly.
    #[test]
    fn test_unswitch_with_parameters() {
        let input = r#"function @test(v0: bool, v1: i32) -> i32 {
block0(v0: bool, v1: i32):
    v2 = iconst 0i32
    jump block1(v2)
block1(v3: i32):
    branch v0, block2(v3), block3(v3)
block2(v4: i32):
    v5 = iconst 1i32
    v6 = iadd v4, v5
    jump block1(v6)
block3(v7: i32):
    v8 = iconst 2i32
    v9 = iadd v7, v8
    v10 = icmp_slt v9, v1
    branch v10, block1(v9), block4(v9)
block4(v11: i32):
    return v11
}"#;
        // block parameters are correctly remapped in cloned loop
        let expected = r#"function @test(v0: bool, v1: i32) -> i32 {
block0(v0: bool, v1: i32):
    v2 = iconst 0i32
    branch v0, block1(v2), block6(v2)
block1(v3: i32):
    jump block2(v3)
block2(v4: i32):
    v5 = iconst 1i32
    v6 = iadd v4, v5
    jump block5(v6)
block3(v7: i32):
    v8 = iconst 2i32
    v9 = iadd v7, v8
    v10 = icmp_slt v9, v1
    branch v10, block5(v9), block4(v9)
block4(v11: i32):
    return v11
block5(v12: i32):
    jump block1(v12)
block6(v13: i32):
    jump block8(v13)
block7(v14: i32):
    v15 = iconst 1i32
    v16 = iadd v14, v15
    jump block9(v16)
block8(v17: i32):
    v18 = iconst 2i32
    v19 = iadd v17, v18
    v20 = icmp_slt v19, v1
    branch v20, block9(v19), block4(v19)
block9(v21: i32):
    jump block6(v21)
}"#;
        let mut program = TestProgram::new(input);
        program.run_pass(&LoopSimplify);
        program.run_pass(&LoopUnswitch);
        program.assert_output(expected);
    }

    /// Loop exceeding size limit is preserved.
    #[test]
    fn test_preserve_large_loop() {
        // create a loop with > MAX_LOOP_SIZE (50) instructions
        let mut instructions = String::new();
        for i in 0..60 {
            instructions.push_str(&format!("    v{} = iconst {}i32\n", i + 10, i));
        }

        let input = format!(
            r#"function @test(v0: bool, v1: bool) -> void {{
block0(v0: bool, v1: bool):
    jump block1
block1:
{instructions}    branch v0, block2, block3
block2:
    branch v1, block1, block4
block3:
    return
block4:
    return
}}"#
        );
        let mut program = TestProgram::new(&input);
        program.run_pass(&LoopSimplify);
        let before = program.format();
        program.run_pass(&LoopUnswitch);
        program.assert_output(&before);
    }

    /// Inner loop is unswitched before outer loop.
    #[test]
    fn test_unswitch_inner_loop_first() {
        let input = r#"function @test(v0: bool, v1: bool) -> void {
block0(v0: bool, v1: bool):
    jump block1
block1:
    jump block2
block2:
    branch v0, block3, block4
block3:
    jump block2
block4:
    branch v1, block1, block5
block5:
    return
}"#;
        // inner loop (block2-block3) is unswitched on v0
        // the outer loop structure is preserved
        let expected = r#"function @test(v0: bool, v1: bool) -> void {
block0(v0: bool, v1: bool):
    jump block1
block1:
    branch v0, block2, block6
block2:
    jump block3
block3:
    jump block2
block4:
    branch v1, block1, block5
block5:
    return
block6:
    jump block4
block7:
    jump block6
}"#;
        let mut program = TestProgram::new(input);
        program.run_pass(&LoopSimplify);
        program.run_pass(&LoopUnswitch);
        program.assert_output(expected);
    }
}
