use std::collections::{HashMap, HashSet};

use destack_compiler_macros::declare_pass;
use destack_mir as mir;

use crate::optimize::analyses::{
    ControlFlowGraph, DominatorTree, Loop, LoopAnalysis, RangeAnalysis,
};
use crate::optimize::common::{
    CallsiteHotness, SuccessorArguments, block_execution_counts, block_hotness_from_counts,
    bool_from_range, build_value_definition_map, clone_loop_blocks, instruction_is_speculatable,
    instruction_map_with_locals, terminator_arguments_for_successor_checked, terminator_remap,
};
use crate::optimize::{AnalysisPreservation, FunctionPass, PipelineContext};

declare_pass! {
    /// Move loop invariant conditionals outside of loops by duplicating the loop.
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
    /// 1) Only unswitches branches with loop invariant conditions.
    /// 2) Only unswitches small loops to avoid excessive code growth.
    /// 3) Limits unswitching to a small number of disjoint loops per run.
    /// 4) Avoids cold unswitching when profile data is present.
    /// 5) Requires canonical loop form from LoopSimplify.
    #[pass(id = "loop-unswitch")]
    pub LoopUnswitch,
    "Loop unswitching for invariant conditionals"
}

/// Maximum number of instructions in a loop to consider for unswitching.
const MAX_LOOP_SIZE: usize = 50;
/// Maximum loop size for hot loops when profile data is present.
const MAX_LOOP_SIZE_HOT: usize = 200;
/// Maximum loop size for cold loops when profile data is present.
const MAX_LOOP_SIZE_COLD: usize = 30;
/// Maximum number of unswitch operations per function invocation.
const MAX_UNSWITCHES_PER_FUNCTION: usize = 2;
/// Minimum branch execution count for unswitching with profiles.
const MIN_BRANCH_COUNT_FOR_UNSWITCH: u64 = 16;

impl FunctionPass for LoopUnswitch {
    /// Run loop unswitching on a function.
    fn run(
        &self,
        function: &mut mir::Function,
        tree: &mut mir::NodeTree,
        ctx: &PipelineContext<'_>,
    ) -> AnalysisPreservation {
        // skip empty functions
        if function.entry.is_none() {
            return AnalysisPreservation::all();
        }

        // run loop unswitching
        let changed = run_loop_unswitch(function, tree, ctx);

        // select preservation based on unswitch changes
        if changed {
            AnalysisPreservation::none()
        } else {
            AnalysisPreservation::all()
        }
    }

    /// Return the pass name.
    fn name(&self) -> &'static str {
        "LoopUnswitch"
    }

    /// Return the pass identifier.
    fn id(&self) -> &'static str {
        "loop-unswitch"
    }
}

/// Core loop unswitching logic. Returns true if changes were made.
fn run_loop_unswitch(
    function: &mut mir::Function,
    tree: &mut mir::NodeTree,
    ctx: &PipelineContext<'_>,
) -> bool {
    // track progress and exclusions
    let mut changed = false;
    let mut unswitched = 0;
    let mut unswitched_headers: HashSet<mir::LocalNodeId<mir::Block>> = HashSet::new();
    let mut unswitched_blocks: Vec<HashSet<mir::LocalNodeId<mir::Block>>> = Vec::new();

    // iterate unswitch attempts within the limit
    while unswitched < MAX_UNSWITCHES_PER_FUNCTION {
        // refresh analyses after each transform
        let (loops, domtree, cfg, ranges) = {
            let analyses = ctx.function_analyses(function, tree);
            (
                analyses.get::<LoopAnalysis>().clone(),
                analyses.get::<DominatorTree>().clone(),
                analyses.get::<ControlFlowGraph>().clone(),
                analyses.get::<RangeAnalysis>().clone(),
            )
        };

        // compute profile driven heuristics
        let heuristics =
            UnswitchHeuristics::new(function, tree, ctx.profile(), ctx.inline_hotness_policy());

        // stop when there are no loops to process
        if loops.num_loops() == 0 {
            break;
        }

        // prepare loop ordering for selection
        let mut candidate: Option<UnswitchCandidate> = None;
        let mut ordered_loops: Vec<&Loop> = loops.loops().iter().collect();
        ordered_loops.sort_by_key(|lp| (std::cmp::Reverse(lp.depth), lp.blocks.len(), lp.header));

        // find first unswitchable loop
        for lp in ordered_loops {
            // skip loops rejected by earlier transformations
            if !loop_is_candidate(lp, &unswitched_headers, &unswitched_blocks) {
                continue;
            }

            // capture the first loop that can be unswitched
            if let Some(c) =
                find_unswitchable_loop(lp, function, tree, &cfg, &domtree, &ranges, &heuristics)
            {
                candidate = Some(c);
                break;
            }
        }

        // stop when no candidate was found
        let Some(candidate) = candidate else {
            break;
        };

        // apply the unswitch transform
        function.recompute_next_value_id(tree);
        unswitched_headers.insert(candidate.header);
        unswitched_blocks.push(candidate.loop_blocks.clone());
        unswitch_loop(function, tree, &candidate);
        unswitched += 1;
        changed = true;
    }

    changed
}

/// Return true when a loop is disjoint from previously unswitched blocks.
fn loop_is_disjoint(lp: &Loop, unswitched: &[HashSet<mir::LocalNodeId<mir::Block>>]) -> bool {
    // check all prior unswitched block sets
    unswitched
        .iter()
        .all(|blocks| blocks.is_disjoint(&lp.blocks))
}

/// Return true when a loop can be unswitched given prior transformations.
fn loop_is_candidate(
    lp: &Loop,
    unswitched_headers: &HashSet<mir::LocalNodeId<mir::Block>>,
    unswitched_blocks: &[HashSet<mir::LocalNodeId<mir::Block>>],
) -> bool {
    // reject overlap with previously unswitched blocks
    if !loop_is_disjoint(lp, unswitched_blocks) {
        return false;
    }

    // scan headers that must not appear in the loop
    for header in unswitched_headers {
        // abort when a prior header is inside the loop
        if lp.blocks.contains(header) {
            return false;
        }
    }

    // accept remaining loops
    true
}

/// Information needed to unswitch a loop.
struct UnswitchCandidate {
    /// The preheader block.
    preheader: mir::LocalNodeId<mir::Block>,
    /// The loop header.
    header: mir::LocalNodeId<mir::Block>,
    /// The block containing the invariant branch (may be header or another block).
    branch_block: mir::LocalNodeId<mir::Block>,
    /// The invariant condition value.
    condition: mir::Value,
    /// Instruction to hoist into the preheader when needed.
    hoisted_condition: Option<HoistedCondition>,
    /// The "then" successor of the branch.
    then_target: mir::LocalNodeId<mir::Block>,
    /// Arguments passed to then_target.
    then_arguments: Vec<mir::Value>,
    /// The "else" successor of the branch.
    else_target: mir::LocalNodeId<mir::Block>,
    /// Arguments passed to else_target.
    else_arguments: Vec<mir::Value>,
    /// The check kind if the invariant branch is a check.
    check_kind: Option<mir::CheckConstraint>,
    /// All blocks in the loop.
    loop_blocks: HashSet<mir::LocalNodeId<mir::Block>>,
    /// Arguments passed from preheader to header.
    preheader_to_header_args: Vec<mir::Value>,
}

#[derive(Debug, Clone)]
struct HoistedCondition {
    /// The instruction defining the condition.
    instruction: mir::Instruction,
    /// The original destination value.
    destination: mir::Value,
    /// Remapping for invariant operands.
    value_map: HashMap<mir::Value, mir::Value>,
}

/// Profile driven heuristics for unswitch selection.
struct UnswitchHeuristics<'a> {
    /// Execution counts for blocks in the function.
    block_counts: HashMap<mir::LocalNodeId<mir::Block>, u64>,
    /// Entry count for the function.
    entry_count: u64,
    /// Hotness thresholds to apply.
    hotness_policy: &'a crate::optimize::common::CallsiteHotnessPolicy,
}

impl<'a> UnswitchHeuristics<'a> {
    /// Create heuristics from profile data.
    fn new(
        function: &mir::Function,
        tree: &mir::NodeTree,
        profile: Option<&mir::ProfileTable>,
        hotness_policy: &'a crate::optimize::common::CallsiteHotnessPolicy,
    ) -> Self {
        // compute block counts from profile data
        let block_counts = block_execution_counts(function, tree, profile, hotness_policy);
        let entry_count = function
            .entry
            .and_then(|entry| block_counts.get(&entry).copied())
            .unwrap_or(0);

        Self {
            block_counts,
            entry_count,
            hotness_policy,
        }
    }

    /// Return true when profile data is available.
    fn has_profile(&self) -> bool {
        !self.block_counts.is_empty()
    }

    /// Return the hotness for a block when profile data is available.
    fn block_hotness(&self, block: mir::LocalNodeId<mir::Block>) -> CallsiteHotness {
        let Some(count) = self.block_counts.get(&block).copied() else {
            return CallsiteHotness::Unknown;
        };

        block_hotness_from_counts(count, self.entry_count, self.hotness_policy)
    }

    /// Return the loop size limit for a header block.
    fn loop_size_limit(&self, header: mir::LocalNodeId<mir::Block>) -> usize {
        if !self.has_profile() {
            return MAX_LOOP_SIZE;
        }

        match self.block_hotness(header) {
            CallsiteHotness::Hot => MAX_LOOP_SIZE_HOT,
            CallsiteHotness::Cold => MAX_LOOP_SIZE_COLD,
            CallsiteHotness::Unknown => MAX_LOOP_SIZE,
        }
    }

    /// Return true when the branch block is too cold to unswitch.
    fn branch_is_too_cold(&self, branch: mir::LocalNodeId<mir::Block>) -> bool {
        if !self.has_profile() {
            return false;
        }

        let count = self.block_counts.get(&branch).copied().unwrap_or(0);
        count < MIN_BRANCH_COUNT_FOR_UNSWITCH
            || matches!(self.block_hotness(branch), CallsiteHotness::Cold)
    }
}

/// Check if a loop can be unswitched.
fn find_unswitchable_loop(
    lp: &Loop,
    function: &mir::Function,
    tree: &mir::NodeTree,
    cfg: &ControlFlowGraph,
    domtree: &DominatorTree,
    ranges: &RangeAnalysis,
    heuristics: &UnswitchHeuristics<'_>,
) -> Option<UnswitchCandidate> {
    // need a preheader
    let preheader = domtree.immediate_dominator(lp.header)?;
    if lp.blocks.contains(&preheader) {
        return None;
    }

    let header = lp.header;

    // check loop size
    let loop_size: usize = lp
        .blocks
        .iter()
        .map(|&b| tree.get(b).instructions.len())
        .sum();
    let loop_size_limit = heuristics.loop_size_limit(header);
    if loop_size > loop_size_limit {
        return None;
    }

    // get preheader to header arguments
    let preheader_block = tree.get(preheader);
    let preheader_to_header_args = match &preheader_block.terminator {
        mir::Terminator::Jump { target, arguments } if *target == header => arguments.clone(),
        _ => return None,
    };

    // collect invariant values (defined outside the loop)
    let base_invariant_values = collect_base_invariant_values(lp, function, tree);
    let header_param_rewrites = collect_header_param_rewrites(
        lp,
        tree,
        cfg,
        &base_invariant_values,
        &preheader_to_header_args,
    );

    let mut preheader_values = base_invariant_values.clone();
    for arg in header_param_rewrites.values() {
        preheader_values.insert(*arg);
    }

    let value_definitions = build_value_definition_map(function, tree);

    // scan all loop blocks for an invariant branch (prefer header first for stability)
    let mut sorted_blocks: Vec<_> = lp.blocks.iter().copied().collect();
    sorted_blocks.sort();
    // put header first if present
    if let Some(pos) = sorted_blocks.iter().position(|&b| b == header) {
        sorted_blocks.remove(pos);
        sorted_blocks.insert(0, header);
    }

    for &block_id in &sorted_blocks {
        let block = tree.get(block_id);

        // must have a branch terminator
        let (condition, then_target, then_arguments, else_target, else_arguments, check_kind) =
            match &block.terminator {
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
                    None,
                ),
                mir::Terminator::Check {
                    condition,
                    constraint,
                    success,
                    failure,
                } => (
                    *condition,
                    success.target,
                    success.arguments.clone(),
                    failure.target,
                    failure.arguments.clone(),
                    Some(constraint.clone()),
                ),
                _ => continue,
            };

        // condition must be loop invariant after header parameter rewrite
        let condition_value = header_param_rewrites
            .get(&condition)
            .copied()
            .unwrap_or(condition);
        let hoisted_condition = if preheader_values.contains(&condition_value) {
            None
        } else {
            try_hoist_invariant_condition(
                condition,
                &value_definitions,
                &header_param_rewrites,
                &preheader_values,
                tree,
            )
        };
        if !preheader_values.contains(&condition_value) && hoisted_condition.is_none() {
            continue;
        }
        if bool_from_range(ranges.entry(block_id).get(condition_value)).is_some() {
            continue;
        }
        let check_kind =
            check_kind.map(|constraint| remap_check_constraint(constraint, &header_param_rewrites));
        if let Some(kind) = &check_kind
            && !kind
                .uses()
                .iter()
                .all(|value| preheader_values.contains(value))
        {
            continue;
        }

        // both targets must be different (otherwise branch is effectively a jump)
        if then_target == else_target {
            continue;
        }

        // at least one branch must stay in the loop
        let then_in_loop = lp.blocks.contains(&then_target);
        let else_in_loop = lp.blocks.contains(&else_target);
        if !then_in_loop && !else_in_loop {
            continue;
        }

        if heuristics.branch_is_too_cold(block_id) {
            continue;
        }

        return Some(UnswitchCandidate {
            preheader,
            header,
            branch_block: block_id,
            condition: condition_value,
            hoisted_condition,
            then_target,
            then_arguments,
            else_target,
            else_arguments,
            check_kind,
            loop_blocks: lp.blocks.clone(),
            preheader_to_header_args,
        });
    }

    None
}

/// Collect values defined outside the loop.
fn collect_base_invariant_values(
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

/// Hoist a loop invariant condition into the preheader when possible.
fn try_hoist_invariant_condition(
    condition: mir::Value,
    value_definitions: &HashMap<mir::Value, mir::LocalNodeId<mir::Instruction>>,
    header_param_rewrites: &HashMap<mir::Value, mir::Value>,
    invariant_values: &HashSet<mir::Value>,
    tree: &mir::NodeTree,
) -> Option<HoistedCondition> {
    // resolve the instruction defining the condition
    let instruction_id = value_definitions.get(&condition)?;
    let instruction = tree.get(*instruction_id);
    if !instruction_is_speculatable(instruction) {
        return None;
    }

    // seed the value map with header rewrites
    let mut value_map = HashMap::new();
    for (from, to) in header_param_rewrites {
        value_map.insert(*from, *to);
    }

    // require all operands to be invariant under rewrite
    let all_invariant = instruction.uses().iter().all(|value| {
        let mapped = header_param_rewrites.get(value).copied().unwrap_or(*value);
        invariant_values.contains(&mapped)
    });
    if !all_invariant {
        return None;
    }

    // capture the hoistable instruction
    Some(HoistedCondition {
        instruction: instruction.clone(),
        destination: condition,
        value_map,
    })
}

/// Collect invariant header parameter rewrites based on preheader arguments.
fn collect_header_param_rewrites(
    lp: &Loop,
    tree: &mir::NodeTree,
    cfg: &ControlFlowGraph,
    invariant_values: &HashSet<mir::Value>,
    preheader_args: &[mir::Value],
) -> HashMap<mir::Value, mir::Value> {
    // skip when there are no header parameters
    let header_block = tree.get(lp.header);
    if header_block.parameters.is_empty() {
        return HashMap::new();
    }

    // verify preheader argument count matches
    if preheader_args.len() != header_block.parameters.len() {
        return HashMap::new();
    }

    // build rewrite mapping for invariant parameters
    let mut rewrites = HashMap::new();
    for (index, param) in header_block.parameters.iter().enumerate() {
        let preheader_arg = preheader_args[index];

        // require invariant preheader argument
        if !invariant_values.contains(&preheader_arg) {
            continue;
        }

        // verify all predecessors pass invariant values
        let mut is_invariant = true;
        for &pred in cfg.predecessors(lp.header) {
            // read arguments flowing into the header
            let args = match terminator_arguments_for_successor_checked(
                &tree.get(pred).terminator,
                lp.header,
            ) {
                SuccessorArguments::Consistent(args) => args,
                SuccessorArguments::Missing | SuccessorArguments::Conflict => {
                    return HashMap::new();
                }
            };

            // reject mismatched argument counts
            if args.len() != header_block.parameters.len() {
                return HashMap::new();
            }

            // check for variant argument values
            let arg = args[index];

            // detect arguments that differ from invariant candidates
            let is_preheader_match = arg == preheader_arg;
            let is_param_match = arg == param.value;
            if !is_preheader_match && !is_param_match {
                is_invariant = false;
                break;
            }
        }

        // record invariant rewrite
        if is_invariant {
            rewrites.insert(param.value, preheader_arg);
        }
    }

    // return rewrites for header parameters
    rewrites
}

/// Remap values inside a check constraint using the rewrite map.
fn remap_check_constraint(
    constraint: mir::CheckConstraint,
    rewrites: &HashMap<mir::Value, mir::Value>,
) -> mir::CheckConstraint {
    // remap values through rewrite map
    let remap =
        |value: mir::Value| -> mir::Value { rewrites.get(&value).copied().unwrap_or(value) };

    // rebuild the constraint with remapped values
    match constraint {
        mir::CheckConstraint::Bounds {
            index,
            length,
            collection,
            is_signed,
        } => mir::CheckConstraint::Bounds {
            index: remap(index),
            length: remap(length),
            collection: remap(collection),
            is_signed,
        },
        mir::CheckConstraint::Null { value } => mir::CheckConstraint::Null {
            value: remap(value),
        },
        mir::CheckConstraint::DivZero { divisor } => mir::CheckConstraint::DivZero {
            divisor: remap(divisor),
        },
        mir::CheckConstraint::ShiftRange {
            value,
            bit_width,
            is_signed,
        } => mir::CheckConstraint::ShiftRange {
            value: remap(value),
            bit_width,
            is_signed,
        },
        mir::CheckConstraint::Narrow {
            value,
            to_width,
            is_signed,
        } => mir::CheckConstraint::Narrow {
            value: remap(value),
            to_width,
            is_signed,
        },
        mir::CheckConstraint::Overflow {
            operator,
            left,
            right,
            is_signed,
        } => mir::CheckConstraint::Overflow {
            operator,
            left: remap(left),
            right: remap(right),
            is_signed,
        },
        mir::CheckConstraint::Type { value, expected } => mir::CheckConstraint::Type {
            value: remap(value),
            expected,
        },
        mir::CheckConstraint::Union { value, expected } => mir::CheckConstraint::Union {
            value: remap(value),
            expected,
        },
        mir::CheckConstraint::Vtable { receiver, expected } => mir::CheckConstraint::Vtable {
            receiver: remap(receiver),
            expected,
        },
        mir::CheckConstraint::Itab { receiver, expected } => mir::CheckConstraint::Itab {
            receiver: remap(receiver),
            expected,
        },
    }
}

/// Perform loop unswitching transformation.
fn unswitch_loop(
    function: &mut mir::Function,
    tree: &mut mir::NodeTree,
    candidate: &UnswitchCandidate,
) {
    // clone all loop blocks with fresh IDs and values
    let (block_map, value_map) = clone_loop_blocks(&candidate.loop_blocks, function, tree);

    // get the cloned header and cloned branch block
    let cloned_header = block_map[&candidate.header];
    let cloned_branch_block = block_map[&candidate.branch_block];

    // modify original branch block: always take the "then" branch
    let mut branch_block = tree.get(candidate.branch_block).clone();
    branch_block.terminator = mir::Terminator::Jump {
        target: candidate.then_target,
        arguments: candidate.then_arguments.clone(),
    };
    tree.replace(candidate.branch_block, branch_block);

    // modify cloned branch block: always take the "else" branch
    let mut cloned = tree.get(cloned_branch_block).clone();
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
    tree.replace(cloned_branch_block, cloned);

    // modify preheader: branch based on condition
    let mut preheader = tree.get(candidate.preheader).clone();
    let condition_value = if let Some(hoisted) = &candidate.hoisted_condition {
        // hoist invariant condition into the preheader
        let mut value_map = hoisted.value_map.clone();
        let new_value = function.next_value();
        value_map.insert(hoisted.destination, new_value);
        let local_map = HashMap::new();
        let hoisted_inst =
            instruction_map_with_locals(&hoisted.instruction, &value_map, &local_map, tree);
        let hoisted_id = tree.insert(hoisted_inst);
        preheader.instructions.push(hoisted_id);
        new_value
    } else {
        candidate.condition
    };
    preheader.terminator = if let Some(constraint) = candidate.check_kind.clone() {
        mir::Terminator::Check {
            condition: condition_value,
            constraint,
            success: mir::CheckTarget {
                target: candidate.header,
                arguments: candidate.preheader_to_header_args.clone(),
            },
            failure: mir::CheckTarget {
                target: cloned_header,
                arguments: candidate.preheader_to_header_args.clone(),
            },
        }
    } else {
        mir::Terminator::Branch {
            condition: condition_value,
            then_target: candidate.header,
            then_arguments: candidate.preheader_to_header_args.clone(),
            else_target: cloned_header,
            else_arguments: candidate.preheader_to_header_args.clone(),
        }
    };
    tree.replace(candidate.preheader, preheader);

    // remap terminators in cloned blocks (except the branch block which we already handled)
    for (&original, &cloned_id) in &block_map {
        if original == candidate.branch_block {
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
        // preheader branches on v0
        // then branch is the original loop with header jumping to block2 path
        // else branch is the cloned loop with header jumping to block3 path
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
        let mut test = TestProgram::new(input);
        test.run_pass(&LoopSimplify);
        test.run_pass(&LoopUnswitch);
        test.assert_output(expected);
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
        let mut test = TestProgram::new(input);
        test.run_pass(&LoopSimplify);
        test.run_pass(&LoopUnswitch);
        test.assert_output(expected);
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
        let mut test = TestProgram::new(input);
        test.run_pass(&LoopSimplify);
        let before = test.format();
        test.run_pass(&LoopUnswitch);
        test.assert_output(&before);
    }

    /// Loop with invariant branch NOT in header is still unswitched.
    #[test]
    fn test_unswitch_non_header_branch() {
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
        // branch is in block2 (not header), but v0 is invariant
        // unswitch on the non header branch
        let expected = r#"function @test(v0: bool) -> void {
block0(v0: bool):
    branch v0, block1, block4
block1:
    jump block2
block2:
    jump block1
block3:
    return
block4:
    jump block5
block5:
    jump block3
}"#;
        let mut test = TestProgram::new(input);
        test.run_pass(&LoopSimplify);
        test.run_pass(&LoopUnswitch);
        test.assert_output(expected);
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
        let mut test = TestProgram::new(input);
        test.run_pass(&LoopSimplify);
        let before = test.format();
        test.run_pass(&LoopUnswitch);
        test.assert_output(&before);
    }

    /// Loop with identical branch targets (all branches) is preserved.
    #[test]
    fn test_preserve_same_branch_targets() {
        // all branches in the loop have identical targets (effectively jumps)
        let input = r#"function @test(v0: bool, v1: bool) -> void {
block0(v0: bool, v1: bool):
    jump block1
block1:
    branch v0, block2, block2
block2:
    branch v1, block1, block1
block3:
    return
}"#;
        // no unswitchable branch: both have same targets
        let mut test = TestProgram::new(input);
        test.run_pass(&LoopSimplify);
        let before = test.format();
        test.run_pass(&LoopUnswitch);
        test.assert_output(&before);
    }

    /// Non header branch is unswitched when header branch has same targets.
    #[test]
    fn test_unswitch_skips_same_targets_finds_other() {
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
        // block1's branch has same targets (skipped)
        // block2's branch has different targets and invariant condition (unswitched)
        let expected = r#"function @test(v0: bool) -> void {
block0(v0: bool):
    branch v0, block1, block4
block1:
    branch v0, block2, block2
block2:
    jump block1
block3:
    return
block4:
    branch v0, block5, block5
block5:
    jump block3
}"#;
        let mut test = TestProgram::new(input);
        test.run_pass(&LoopSimplify);
        test.run_pass(&LoopUnswitch);
        test.assert_output(expected);
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
        let mut test = TestProgram::new(input);
        test.run_pass(&LoopUnswitch);
        test.assert_unchanged(input);
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
        let mut test = TestProgram::new(input);
        test.run_pass(&LoopSimplify);
        test.run_pass(&LoopUnswitch);
        test.assert_output(expected);
    }

    /// Loop header parameters can be rewritten to preheader arguments.
    #[test]
    fn test_unswitch_header_param_condition() {
        let input = r#"function @test(v0: bool) -> void {
block0(v0: bool):
    jump block1(v0)
block1(v1: bool):
    branch v1, block2, block3
block2:
    jump block1(v1)
block3:
    return
}"#;
        let expected = r#"function @test(v0: bool) -> void {
block0(v0: bool):
    branch v0, block1(v0), block4(v0)
block1(v1: bool):
    jump block2
block2:
    jump block1(v1)
block3:
    return
block4(v2: bool):
    jump block3
block5:
    jump block4(v2)
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&LoopSimplify);
        test.run_pass(&LoopUnswitch);
        test.assert_output(expected);
    }

    /// Loop checks with invariant condition are unswitched.
    #[test]
    fn test_unswitch_check_terminator() {
        let input = r#"function @test(v0: bool, v1: u32, v2: [u8; 8]) -> void {
block0(v0: bool, v1: u32, v2: [u8; 8]):
    jump block1(v1)
block1(v3: u32):
    check v0, bounds.unsigned v3, v1, v2, block2, block3
block2:
    jump block1(v3)
block3:
    return
}"#;

        let expected = r#"function @test(v0: bool, v1: u32, v2: [u8; 8]) -> void {
block0(v0: bool, v1: u32, v2: [u8; 8]):
    check v0, bounds.unsigned v1, v1, v2, block1(v1), block4(v1)
block1(v3: u32):
    jump block2
block2:
    jump block1(v3)
block3:
    return
block4(v4: u32):
    jump block3
block5:
    jump block4(v4)
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&LoopSimplify);
        test.run_pass(&LoopUnswitch);
        test.assert_output(expected);
    }

    /// Loop conditions computed in the header can be hoisted to the preheader.
    #[test]
    fn test_unswitch_hoists_header_condition() {
        let input = r#"function @test(v0: bool) -> void {
block0(v0: bool):
    jump block1
block1:
    v1 = select v0, v0, v0
    branch v1, block2, block3
block2:
    jump block1
block3:
    return
}"#;

        let expected = r#"function @test(v0: bool) -> void {
block0(v0: bool):
    v3 = select v0, v0, v0
    branch v3, block1, block4
block1:
    v1 = select v0, v0, v0
    jump block2
block2:
    jump block1
block3:
    return
block4:
    v2 = select v0, v0, v0
    jump block3
block5:
    jump block4
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&LoopSimplify);
        test.run_pass(&LoopUnswitch);
        test.assert_output(expected);
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
        let mut test = TestProgram::new(&input);
        test.run_pass(&LoopSimplify);
        let before = test.format();
        test.run_pass(&LoopUnswitch);
        test.assert_output(&before);
    }

    /// Inner loop is unswitched first, then an outer loop may also unswitch.
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
        // inner loop block2 to block3 is unswitched on v0
        // the outer loop may also unswitch in a subsequent iteration
        let expected = r#"function @test(v0: bool, v1: bool) -> void {
block0(v0: bool, v1: bool):
    branch v0, block1, block8
block1:
    jump block2
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
block8:
    jump block10
block9:
    branch v1, block8, block5
block10:
    jump block9
}"#;
        let mut test = TestProgram::new(input);
        test.run_pass(&LoopSimplify);
        test.run_pass(&LoopUnswitch);
        test.assert_output(expected);
    }

    /// Constant conditions are not unswitched.
    #[test]
    fn test_unswitch_skips_constant_condition() {
        let input = r#"function @test() -> void {
block0:
    v0 = iconst true
    jump block1
block1:
    branch v0, block2, block3
block2:
    jump block1
block3:
    return
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&LoopSimplify);
        test.run_pass(&LoopUnswitch);
        test.assert_output(input);
    }

    /// Cold branches are not unswitched when profile data is available.
    #[test]
    fn test_unswitch_skips_cold_branch_with_profile() {
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

        let mut test = TestProgram::new(input);
        test.run_pass(&LoopSimplify);

        let function_id = test.entry_function_id();
        let entry_block = test.entry_block_id(function_id);
        let header_block = match &test.tree.get(entry_block).terminator {
            mir::Terminator::Jump { target, .. } => *target,
            _ => panic!("missing loop header jump"),
        };

        let mut profile = mir::ProfileTable::new(mir::ProfileSource::Instrumentation);
        profile.blocks.insert(
            entry_block,
            mir::BlockProfile {
                execution_count: mir::ProfileCount::new(100, mir::ProfileConfidence::Precise),
            },
        );
        profile.blocks.insert(
            header_block,
            mir::BlockProfile {
                execution_count: mir::ProfileCount::new(1, mir::ProfileConfidence::Precise),
            },
        );

        test.run_pass_with_profile(&LoopUnswitch, profile);
        test.assert_output(input);
    }
}
