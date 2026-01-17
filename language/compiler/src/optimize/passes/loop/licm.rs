use std::collections::{HashMap, HashSet};

use destack_compiler_macros::declare_pass;
use destack_mir as mir;

use crate::optimize::analyses::{
    AliasAnalysis, ConstantPropagation, DominatorTree, Loop, LoopAnalysis, MemoryAccess,
    MemoryAccessId, MemoryAccessLocation, MemorySSA, RangeAnalysis, ValueRange,
};
use crate::optimize::common::{
    build_instruction_block_map, instruction_allows_read_only_motion,
    instruction_is_read_only_access, instruction_is_speculatable,
};
use crate::optimize::{AnalysisPreservation, FunctionPass, PipelineContext};

declare_pass! {
    /// Move loop invariant computations outside of loops.
    ///
    /// An instruction is loop invariant if all its operands are defined outside the loop or by other loop invariant instructions.
    /// Loop invariant instructions can be hoisted to the loop preheader, reducing redundant computation.
    ///
    /// This pass hoists pure arithmetic, casts, and selects.
    /// It hoists constants and immutable global constants.
    /// It hoists address computations for fields and elements.
    /// It hoists loads and local gets that are invariant and not clobbered in the loop.
    ///
    /// Load invariance is checked with Memory SSA and alias analysis.
    /// Potentially trapping instructions such as integer division are only hoisted when range analysis proves the operation is safe and the block executes on every iteration.
    ///
    /// ```mir
    /// function @before(v0: bool, v1: i32) -> i32 {
    /// block0(v0: bool, v1: i32):
    ///     v2 = iconst 3i32
    ///     jump block1
    /// block1:
    ///     v3 = iadd v1, v2
    ///     branch v0, block1, block2
    /// block2:
    ///     return v3
    /// }
    /// ```
    /// becomes:
    /// ```mir
    /// function @after(v0: bool, v1: i32) -> i32 {
    /// block0(v0: bool, v1: i32):
    ///     v2 = iconst 3i32
    ///     v3 = iadd v1, v2
    ///     jump block1
    /// block1:
    ///     branch v0, block1, block2
    /// block2:
    ///     return v3
    /// }
    /// ```
    ///
    /// Requires canonical loop form from LoopSimplify.
    #[pass(id = "licm")]
    pub Licm,
    "Loop invariant code motion"
}

impl FunctionPass for Licm {
    /// Run loop invariant code motion on a function.
    fn run(
        &self,
        function: &mut mir::Function,
        tree: &mut mir::NodeTree,
        ctx: &PipelineContext<'_>,
    ) -> AnalysisPreservation {
        let entry = match function.entry {
            Some(entry) => entry,
            None => return AnalysisPreservation::all(),
        };

        // get analyses
        let (loops, domtree, ranges, constants, alias, memory_ssa) = {
            let analyses = ctx.function_analyses(function, tree);
            (
                analyses.get::<LoopAnalysis>().clone(),
                analyses.get::<DominatorTree>().clone(),
                analyses.get::<RangeAnalysis>().clone(),
                analyses.get::<ConstantPropagation>().clone(),
                analyses.get::<AliasAnalysis>().clone(),
                analyses.get::<MemorySSA>(),
            )
        };
        if loops.num_loops() == 0 {
            return AnalysisPreservation::all();
        }

        // run LICM
        let changed = run_licm(
            entry,
            function,
            tree,
            &loops,
            &domtree,
            &ranges,
            &constants,
            &alias,
            memory_ssa.as_ref(),
        );

        if changed {
            AnalysisPreservation::none()
        } else {
            AnalysisPreservation::all()
        }
    }

    /// Return the pass name.
    fn name(&self) -> &'static str {
        "Licm"
    }

    /// Return the pass identifier.
    fn id(&self) -> &'static str {
        "licm"
    }
}

/// Core LICM logic. Returns true if changes were made.
#[allow(clippy::too_many_arguments)]
fn run_licm(
    entry: mir::LocalNodeId<mir::Block>,
    function: &mir::Function,
    tree: &mut mir::NodeTree,
    loops: &LoopAnalysis,
    domtree: &DominatorTree,
    ranges: &RangeAnalysis,
    constants: &ConstantPropagation,
    alias: &AliasAnalysis,
    memory_ssa: &MemorySSA,
) -> bool {
    // order loops from inner to outer
    let mut loop_order: Vec<&Loop> = loops.loops().iter().collect();
    loop_order.sort_by(|a, b| b.depth.cmp(&a.depth).then(a.header.cmp(&b.header)));

    // build instruction to block mapping
    let instruction_blocks = build_instruction_block_map(function, tree);

    // build dominator preorder index
    let block_order = build_dominator_preorder(entry, function, domtree);

    // collect hoisting work for each loop
    let mut all_work: Vec<HoistWork> = Vec::new();

    for lp in loop_order {
        // find the loop preheader
        let preheader = match find_preheader(lp, domtree) {
            Some(preheader) => preheader,
            None => continue,
        };

        // collect blocks owned by this loop but not by subloops
        let loop_blocks = collect_loop_blocks(function, loops, lp);

        // skip loops without owned blocks
        if loop_blocks.is_empty() {
            continue;
        }

        // compute blocks that execute every iteration
        let guaranteed_blocks = compute_guaranteed_blocks(lp, domtree);

        // seed invariants with values defined outside the loop
        let mut invariant_values = collect_invariant_seed_values(function, tree, &lp.blocks);

        // fixpoint to find loop invariant instructions
        let mut changed = true;
        while changed {
            changed = false;

            // scan loop blocks for new invariants
            for &block_id in &loop_blocks {
                let block = tree.get(block_id);

                // scan instructions in the block
                for &instruction_id in &block.instructions {
                    let instruction = tree.get(instruction_id);
                    let Some(destination) = instruction.destination() else {
                        continue;
                    };

                    if invariant_values.contains(&destination) {
                        continue;
                    }

                    let operands_invariant = instruction
                        .uses()
                        .iter()
                        .all(|value| invariant_values.contains(value));
                    if !operands_invariant {
                        continue;
                    }

                    let can_hoist = instruction_is_hoistable(
                        instruction_id,
                        instruction,
                        &lp.blocks,
                        &guaranteed_blocks,
                        tree,
                        alias,
                        memory_ssa,
                        &instruction_blocks,
                        block_id,
                        ranges,
                        constants,
                    );
                    if can_hoist {
                        invariant_values.insert(destination);
                        changed = true;
                    }
                }
            }
        }

        // collect instructions to hoist
        // scan loop blocks for hoistable instructions
        for &block_id in &loop_blocks {
            let block = tree.get(block_id);

            // scan block instructions in order
            for (index, &instruction_id) in block.instructions.iter().enumerate() {
                let instruction = tree.get(instruction_id);
                let Some(destination) = instruction.destination() else {
                    continue;
                };

                if !invariant_values.contains(&destination) {
                    continue;
                }

                let operands_invariant = instruction
                    .uses()
                    .iter()
                    .all(|value| invariant_values.contains(value));
                if !operands_invariant {
                    continue;
                }

                let can_hoist = instruction_is_hoistable(
                    instruction_id,
                    instruction,
                    &lp.blocks,
                    &guaranteed_blocks,
                    tree,
                    alias,
                    memory_ssa,
                    &instruction_blocks,
                    block_id,
                    ranges,
                    constants,
                );
                if can_hoist {
                    all_work.push(HoistWork {
                        source_block: block_id,
                        instruction_index: index,
                        instruction_id,
                        target_preheader: preheader,
                    });
                }
            }
        }
    }

    // exit early when no instructions move
    if all_work.is_empty() {
        return false;
    }

    // sort for insertion order
    let mut insertion_order = all_work.clone();
    insertion_order.sort_by_key(|work| {
        let block_index = block_order
            .get(&work.source_block)
            .copied()
            .unwrap_or(usize::MAX);
        (work.target_preheader, block_index, work.instruction_index)
    });

    // build insertion and removal sets
    let mut insertion_by_preheader: HashMap<
        mir::LocalNodeId<mir::Block>,
        Vec<mir::LocalNodeId<mir::Instruction>>,
    > = HashMap::new();
    let mut to_remove: HashSet<mir::LocalNodeId<mir::Instruction>> = HashSet::new();

    for work in insertion_order {
        to_remove.insert(work.instruction_id);
        insertion_by_preheader
            .entry(work.target_preheader)
            .or_default()
            .push(work.instruction_id);
    }

    // remove hoisted instructions
    for &block_id in &function.blocks {
        let block = tree.get_mut(block_id);
        block.instructions.retain(|id| !to_remove.contains(id));
    }

    // insert instructions into preheaders
    for (preheader_id, instructions) in insertion_by_preheader {
        let mut preheader = tree.get(preheader_id).clone();
        for instruction_id in instructions {
            preheader.instructions.push(instruction_id);
        }
        tree.replace(preheader_id, preheader);
    }

    true
}

/// Collect loop blocks owned by this loop and not by subloops.
fn collect_loop_blocks(
    function: &mir::Function,
    loops: &LoopAnalysis,
    lp: &Loop,
) -> Vec<mir::LocalNodeId<mir::Block>> {
    let mut blocks = Vec::new();

    // scan blocks in function order
    for &block_id in &function.blocks {
        if !lp.blocks.contains(&block_id) {
            continue;
        }

        let innermost = loops
            .innermost_loop(block_id)
            .map(|loop_data| loop_data.header);
        if innermost != Some(lp.header) {
            continue;
        }

        blocks.push(block_id);
    }

    blocks
}

/// Collect values that are defined outside the loop.
fn collect_invariant_seed_values(
    function: &mir::Function,
    tree: &mir::NodeTree,
    loop_blocks: &HashSet<mir::LocalNodeId<mir::Block>>,
) -> HashSet<mir::Value> {
    let mut invariant_values = HashSet::new();

    // include function parameters
    for param in &function.parameters {
        invariant_values.insert(param.value);
    }

    // include values defined outside the loop
    for &block_id in &function.blocks {
        if loop_blocks.contains(&block_id) {
            continue;
        }

        let block = tree.get(block_id);

        for param in &block.parameters {
            invariant_values.insert(param.value);
        }

        for &instruction_id in &block.instructions {
            let instruction = tree.get(instruction_id);
            if let Some(destination) = instruction.destination() {
                invariant_values.insert(destination);
            }
        }
    }

    invariant_values
}

/// Compute blocks that execute on every iteration of the loop.
fn compute_guaranteed_blocks(
    lp: &Loop,
    domtree: &DominatorTree,
) -> HashSet<mir::LocalNodeId<mir::Block>> {
    let mut guaranteed = HashSet::new();

    // select blocks that dominate all latches and exits
    for &block_id in &lp.blocks {
        let dominates_latches = lp
            .latches
            .iter()
            .all(|latch| domtree.dominates(block_id, *latch));
        if !dominates_latches {
            continue;
        }

        let dominates_exits = lp
            .exiting_blocks
            .iter()
            .all(|exit_block| domtree.dominates(block_id, *exit_block));
        if !dominates_exits {
            continue;
        }

        guaranteed.insert(block_id);
    }

    guaranteed
}

/// Build a dominator tree preorder index.
fn build_dominator_preorder(
    entry: mir::LocalNodeId<mir::Block>,
    function: &mir::Function,
    domtree: &DominatorTree,
) -> HashMap<mir::LocalNodeId<mir::Block>, usize> {
    let mut children: HashMap<mir::LocalNodeId<mir::Block>, Vec<mir::LocalNodeId<mir::Block>>> =
        HashMap::new();

    // prepare empty child lists
    for &block_id in &function.blocks {
        children.insert(block_id, Vec::new());
    }

    // build the idom child mapping
    for &block_id in &function.blocks {
        if let Some(idom) = domtree.immediate_dominator(block_id) {
            children.entry(idom).or_default().push(block_id);
        }
    }

    // sort children for deterministic order
    for child_list in children.values_mut() {
        child_list.sort();
    }

    // traverse dominator tree in preorder
    let mut order = HashMap::new();
    let mut stack = vec![entry];
    let mut index = 0;

    while let Some(block_id) = stack.pop() {
        if order.contains_key(&block_id) {
            continue;
        }

        order.insert(block_id, index);
        index += 1;

        let Some(child_list) = children.get(&block_id) else {
            continue;
        };

        for &child in child_list.iter().rev() {
            stack.push(child);
        }
    }

    order
}

/// Return true when an instruction can be hoisted safely.
#[allow(clippy::too_many_arguments)]
fn instruction_is_hoistable(
    instruction_id: mir::LocalNodeId<mir::Instruction>,
    instruction: &mir::Instruction,
    loop_blocks: &HashSet<mir::LocalNodeId<mir::Block>>,
    guaranteed_blocks: &HashSet<mir::LocalNodeId<mir::Block>>,
    tree: &mir::NodeTree,
    alias: &AliasAnalysis,
    memory_ssa: &MemorySSA,
    instruction_blocks: &HashMap<mir::LocalNodeId<mir::Instruction>, mir::LocalNodeId<mir::Block>>,
    block_id: mir::LocalNodeId<mir::Block>,
    ranges: &RangeAnalysis,
    constants: &ConstantPropagation,
) -> bool {
    // accept speculatable instructions immediately
    if instruction_is_speculatable(instruction) {
        return true;
    }

    // require guaranteed execution for non speculatable operations
    if !guaranteed_blocks.contains(&block_id) {
        return false;
    }

    // handle invariant loads and local gets
    if matches!(
        instruction,
        mir::Instruction::Load { .. } | mir::Instruction::LocalGet { .. }
    ) {
        return load_is_hoistable(
            instruction_id,
            loop_blocks,
            tree,
            alias,
            memory_ssa,
            instruction_blocks,
        );
    }

    // handle other read only memory operations
    if instruction_is_read_only_access(instruction_id, memory_ssa) {
        if !instruction_allows_read_only_motion(instruction_id, instruction, tree) {
            return false;
        }

        return read_only_access_is_hoistable(
            instruction_id,
            loop_blocks,
            tree,
            alias,
            memory_ssa,
            instruction_blocks,
        );
    }

    // handle divisions with explicit safety checks
    match instruction {
        mir::Instruction::Binary {
            operator,
            left,
            right,
            ..
        } if matches!(
            operator,
            mir::BinaryOperator::SignedDivide
                | mir::BinaryOperator::UnsignedDivide
                | mir::BinaryOperator::SignedRemainder
                | mir::BinaryOperator::UnsignedRemainder
        ) =>
        {
            division_is_safe(*operator, *left, *right, block_id, ranges, constants)
        }
        _ => false,
    }
}

/// Return true when a loop invariant load is not clobbered in the loop.
fn load_is_hoistable(
    load_id: mir::LocalNodeId<mir::Instruction>,
    loop_blocks: &HashSet<mir::LocalNodeId<mir::Block>>,
    tree: &mir::NodeTree,
    alias: &AliasAnalysis,
    memory_ssa: &MemorySSA,
    instruction_blocks: &HashMap<mir::LocalNodeId<mir::Instruction>, mir::LocalNodeId<mir::Block>>,
) -> bool {
    // read memory ssa access for the load
    let Some(accesses) = memory_ssa.accesses_for_instruction(load_id) else {
        return false;
    };

    let mut load_access = None;
    for access_id in accesses {
        if matches!(memory_ssa.access(*access_id), MemoryAccess::Use(_)) {
            if load_access.is_some() {
                return false;
            }
            load_access = Some(*access_id);
        }
    }

    let Some(load_access) = load_access else {
        return false;
    };

    let MemoryAccess::Use(use_access) = memory_ssa.access(load_access) else {
        return false;
    };

    if use_access.effect.is_volatile || use_access.effect.is_barrier {
        return false;
    }

    if matches!(use_access.effect.location, MemoryAccessLocation::Unknown) {
        return false;
    }

    // reject loads when the loop clobbers the access
    if loop_clobbers_access(load_id, load_access, loop_blocks, tree, memory_ssa, alias) {
        return false;
    }

    // resolve the clobbering access before the load
    let clobber = memory_ssa.clobbering_access_for_use(load_access, alias, tree);
    match memory_ssa.access(clobber) {
        MemoryAccess::LiveOnEntry => true,
        MemoryAccess::Def(def_access) => {
            let Some(block_id) = instruction_blocks.get(&def_access.instruction) else {
                return false;
            };

            !loop_blocks.contains(block_id)
        }
        _ => false,
    }
}

/// Return true when any def in the loop may clobber a use access.
fn loop_clobbers_access(
    origin_instruction: mir::LocalNodeId<mir::Instruction>,
    use_access: MemoryAccessId,
    loop_blocks: &HashSet<mir::LocalNodeId<mir::Block>>,
    tree: &mir::NodeTree,
    memory_ssa: &MemorySSA,
    alias: &AliasAnalysis,
) -> bool {
    // scan loop blocks for clobbering defs
    for block_id in loop_blocks {
        let block = tree.get(*block_id);

        for &instruction_id in &block.instructions {
            if instruction_id == origin_instruction {
                continue;
            }

            let Some(accesses) = memory_ssa.accesses_for_instruction(instruction_id) else {
                continue;
            };

            for access_id in accesses {
                if memory_ssa.def_clobbers_access(*access_id, use_access, alias, tree) {
                    return true;
                }
            }
        }
    }

    false
}

/// Return true when a loop invariant read only access is not clobbered in the loop.
fn read_only_access_is_hoistable(
    instruction_id: mir::LocalNodeId<mir::Instruction>,
    loop_blocks: &HashSet<mir::LocalNodeId<mir::Block>>,
    tree: &mir::NodeTree,
    alias: &AliasAnalysis,
    memory_ssa: &MemorySSA,
    instruction_blocks: &HashMap<mir::LocalNodeId<mir::Instruction>, mir::LocalNodeId<mir::Block>>,
) -> bool {
    // read memory ssa access for the instruction
    let Some(accesses) = memory_ssa.accesses_for_instruction(instruction_id) else {
        return false;
    };

    // collect use accesses
    let mut use_accesses = Vec::new();
    for access_id in accesses {
        match memory_ssa.access(*access_id) {
            MemoryAccess::Use(use_access) => {
                if use_access.effect.is_volatile || use_access.effect.is_barrier {
                    return false;
                }
                if matches!(use_access.effect.location, MemoryAccessLocation::Unknown) {
                    return false;
                }
                use_accesses.push(*access_id);
            }
            MemoryAccess::Def(_) => return false,
            MemoryAccess::Phi(_) | MemoryAccess::LiveOnEntry => {}
        }
    }

    if use_accesses.is_empty() {
        return false;
    }

    // require no clobbers in the loop
    for use_access in &use_accesses {
        if loop_clobbers_access(
            instruction_id,
            *use_access,
            loop_blocks,
            tree,
            memory_ssa,
            alias,
        ) {
            return false;
        }

        let clobber = memory_ssa.clobbering_access_for_use(*use_access, alias, tree);
        match memory_ssa.access(clobber) {
            MemoryAccess::LiveOnEntry => {}
            MemoryAccess::Def(def_access) => {
                let Some(block_id) = instruction_blocks.get(&def_access.instruction) else {
                    return false;
                };
                if loop_blocks.contains(block_id) {
                    return false;
                }
            }
            _ => return false,
        }
    }

    true
}

/// Return true when a division or remainder cannot trap in the loop.
fn division_is_safe(
    operator: mir::BinaryOperator,
    left: mir::Value,
    right: mir::Value,
    block_id: mir::LocalNodeId<mir::Block>,
    ranges: &RangeAnalysis,
    constants: &ConstantPropagation,
) -> bool {
    // resolve operand ranges
    let left_range = integer_range_for_value(left, block_id, ranges, constants);
    let right_range = integer_range_for_value(right, block_id, ranges, constants);
    let (left_range, right_range) = match (left_range, right_range) {
        (Some(left), Some(right)) => (left, right),
        _ => return false,
    };

    // require compatible widths and signedness
    let is_same_width = left_range.width == right_range.width;
    let is_same_signedness = left_range.is_signed == right_range.is_signed;
    if !is_same_width || !is_same_signedness {
        return false;
    }

    // reject zero divisors
    if right_range.contains_value(0) {
        return false;
    }

    // reject signed overflow case min value divided by negative one
    if matches!(
        operator,
        mir::BinaryOperator::SignedDivide | mir::BinaryOperator::SignedRemainder
    ) {
        let Some(min_value) = signed_min_for_width(left_range.width) else {
            return false;
        };

        // check the negative one overflow case
        let divisor_is_negative_one = right_range.contains_value(-1);
        let dividend_is_min = left_range.contains_value(min_value);
        if divisor_is_negative_one && dividend_is_min {
            return false;
        }
    }

    true
}

/// Resolve a best effort integer range for a value at a block boundary.
fn integer_range_for_value(
    value: mir::Value,
    block_id: mir::LocalNodeId<mir::Block>,
    ranges: &RangeAnalysis,
    constants: &ConstantPropagation,
) -> Option<IntegerRange> {
    // prefer constant propagation facts
    let constant = constants
        .constant_at_exit(block_id, value)
        .or_else(|| constants.constant_at_entry(block_id, value));
    if let Some(constant) = constant {
        return integer_range_from_constant(constant);
    }

    // fall back to range analysis facts
    if let Some(range) = ranges.exit(block_id).get(value)
        && let Some(int_range) = integer_range_from_value_range(range)
    {
        return Some(int_range);
    }

    // no known range
    None
}

/// Return the minimum signed value for a given bit width.
fn signed_min_for_width(width: u8) -> Option<i128> {
    // reject unsupported widths
    let bits = u32::from(width);
    let is_zero = bits == 0;
    let is_too_wide = bits > 127;
    if is_zero || is_too_wide {
        return None;
    }

    // compute minimum signed value
    let value = 1i128.checked_shl(bits - 1)?;

    Some(-value)
}

/// Integer range with bit width and signedness metadata.
#[derive(Clone, Copy)]
struct IntegerRange {
    /// The minimum value in the range.
    min: i128,
    /// The maximum value in the range.
    max: i128,
    /// The integer width in bits.
    width: u8,
    /// Whether the range is signed.
    is_signed: bool,
}

impl IntegerRange {
    /// Return true if the range contains a value.
    fn contains_value(self, value: i128) -> bool {
        // compare against inclusive bounds
        value >= self.min && value <= self.max
    }
}

/// Convert a value range into an integer range.
fn integer_range_from_value_range(range: &ValueRange) -> Option<IntegerRange> {
    // require integer ranges
    let ValueRange::Integer {
        min,
        max,
        width,
        is_signed,
    } = range
    else {
        return None;
    };

    // map to integer range
    let result = IntegerRange {
        min: *min,
        max: *max,
        width: *width,
        is_signed: *is_signed,
    };

    Some(result)
}

/// Convert a constant into an integer range.
fn integer_range_from_constant(constant: &mir::Constant) -> Option<IntegerRange> {
    // convert integer constants into ranges
    match constant {
        mir::Constant::Int {
            value,
            width,
            is_signed,
        } => Some(IntegerRange {
            min: *value as i128,
            max: *value as i128,
            width: *width,
            is_signed: *is_signed,
        }),
        mir::Constant::UInt { value, width } => Some(IntegerRange {
            min: *value as i128,
            max: *value as i128,
            width: *width,
            is_signed: false,
        }),
        _ => None,
    }
}

/// Work item for hoisting an instruction.
#[derive(Clone)]
struct HoistWork {
    /// The block containing the instruction to hoist.
    source_block: mir::LocalNodeId<mir::Block>,
    /// The index of the instruction to hoist.
    instruction_index: usize,
    /// The instruction to hoist.
    instruction_id: mir::LocalNodeId<mir::Instruction>,
    /// The preheader to hoist the instruction to.
    target_preheader: mir::LocalNodeId<mir::Block>,
}

/// Find the preheader of a loop.
///
/// The preheader is the immediate dominator of the header that is outside the loop.
fn find_preheader(lp: &Loop, domtree: &DominatorTree) -> Option<mir::LocalNodeId<mir::Block>> {
    let idom = domtree.immediate_dominator(lp.header)?;
    if lp.blocks.contains(&idom) {
        // idom is inside the loop, no proper preheader
        None
    } else {
        Some(idom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optimize::common::tests::TestProgram;
    use crate::optimize::passes::LoopSimplify;

    /// Constant in loop is hoisted to preheader.
    #[test]
    fn test_hoist_constant() {
        let input = r#"function @test(v0: bool) -> i32 {
block0(v0: bool):
    jump block1
block1:
    v1 = iconst 42i32
    branch v0, block1, block2
block2:
    return v1
}"#;
        // v1 = iconst 42 should be hoisted to block0
        let expected = r#"function @test(v0: bool) -> i32 {
block0(v0: bool):
    v1 = iconst 42i32
    jump block1
block1:
    branch v0, block1, block2
block2:
    return v1
}"#;
        let mut program = TestProgram::new(input);
        program.run_pass(&LoopSimplify);
        program.run_pass(&Licm);
        program.assert_output(expected);
    }

    /// Binary operation on invariant operands is hoisted.
    #[test]
    fn test_hoist_binary_invariant() {
        let input = r#"function @test(v0: bool, v1: i32, v2: i32) -> i32 {
block0(v0: bool, v1: i32, v2: i32):
    jump block1
block1:
    v3 = iadd v1, v2
    branch v0, block1, block2
block2:
    return v3
}"#;
        // v3 = iadd v1, v2 is invariant (v1, v2 are function params)
        let expected = r#"function @test(v0: bool, v1: i32, v2: i32) -> i32 {
block0(v0: bool, v1: i32, v2: i32):
    v3 = iadd v1, v2
    jump block1
block1:
    branch v0, block1, block2
block2:
    return v3
}"#;
        let mut program = TestProgram::new(input);
        program.run_pass(&LoopSimplify);
        program.run_pass(&Licm);
        program.assert_output(expected);
    }

    /// Chain of invariant operations is hoisted.
    #[test]
    fn test_hoist_chain() {
        let input = r#"function @test(v0: bool, v1: i32) -> i32 {
block0(v0: bool, v1: i32):
    jump block1
block1:
    v2 = iconst 10i32
    v3 = iadd v1, v2
    v4 = imul v3, v2
    branch v0, block1, block2
block2:
    return v4
}"#;
        // all three instructions are invariant
        let expected = r#"function @test(v0: bool, v1: i32) -> i32 {
block0(v0: bool, v1: i32):
    v2 = iconst 10i32
    v3 = iadd v1, v2
    v4 = imul v3, v2
    jump block1
block1:
    branch v0, block1, block2
block2:
    return v4
}"#;
        let mut program = TestProgram::new(input);
        program.run_pass(&LoopSimplify);
        program.run_pass(&Licm);
        program.assert_output(expected);
    }

    /// Operation using loop variant value is not hoisted.
    #[test]
    fn test_no_hoist_variant() {
        let input = r#"function @test(v0: bool, v1: i32) -> i32 {
block0(v0: bool, v1: i32):
    jump block1(v1)
block1(v2: i32):
    v3 = iconst 1i32
    v4 = iadd v2, v3
    branch v0, block1(v4), block2
block2:
    return v4
}"#;
        // v3 is invariant and can be hoisted
        // v4 depends on v2 which is a loop phi, so v4 cannot be hoisted
        let expected = r#"function @test(v0: bool, v1: i32) -> i32 {
block0(v0: bool, v1: i32):
    v3 = iconst 1i32
    jump block1(v1)
block1(v2: i32):
    v4 = iadd v2, v3
    branch v0, block1(v4), block2
block2:
    return v4
}"#;
        let mut program = TestProgram::new(input);
        program.run_pass(&LoopSimplify);
        program.run_pass(&Licm);
        program.assert_output(expected);
    }

    /// Function without loops is unchanged.
    #[test]
    fn test_no_loops() {
        let input = r#"function @test(v0: i32, v1: i32) -> i32 {
block0(v0: i32, v1: i32):
    v2 = iadd v0, v1
    return v2
}"#;
        let mut program = TestProgram::new(input);
        program.run_pass(&Licm);
        program.assert_unchanged(input);
    }

    /// Already hoisted code is unchanged.
    #[test]
    fn test_already_hoisted() {
        let input = r#"function @test(v0: bool, v1: i32, v2: i32) -> i32 {
block0(v0: bool, v1: i32, v2: i32):
    v3 = iadd v1, v2
    jump block1
block1:
    branch v0, block1, block2
block2:
    return v3
}"#;
        let mut program = TestProgram::new(input);
        program.run_pass(&LoopSimplify);
        program.run_pass(&Licm);
        program.assert_unchanged(input);
    }

    /// Invariant in inner loop is hoisted to the inner preheader.
    #[test]
    fn test_hoist_nested_inner() {
        let input = r#"function @test(v0: bool, v1: bool, v2: i32) -> i32 {
block0(v0: bool, v1: bool, v2: i32):
    jump block1
block1:
    jump block2
block2:
    v3 = iconst 5i32
    v4 = iadd v2, v3
    branch v1, block2, block3
block3:
    branch v0, block1, block4
block4:
    return v4
}"#;
        let expected = r#"function @test(v0: bool, v1: bool, v2: i32) -> i32 {
block0(v0: bool, v1: bool, v2: i32):
    jump block1
block1:
    v3 = iconst 5i32
    v4 = iadd v2, v3
    jump block2
block2:
    branch v1, block2, block3
block3:
    branch v0, block1, block4
block4:
    return v4
}"#;
        let mut program = TestProgram::new(input);
        program.run_pass(&LoopSimplify);
        program.run_pass(&Licm);
        program.assert_output(expected);
    }

    /// Read only intrinsics are hoisted when invariant.
    #[test]
    fn test_hoist_read_only_intrinsic() {
        let input = r#"function @test(v0: bool) -> i32 {
block0(v0: bool):
    v1 = stack.alloc i32 -> ref<raw addrspace(stack) i32>
    v2 = iconst 4i64
    jump block1
block1:
    v3 = intrinsic.memcmp(v1, v1, v2)
    branch v0, block1, block2
block2:
    return v3
}"#;

        let expected = r#"function @test(v0: bool) -> i32 {
block0(v0: bool):
    v1 = stack.alloc i32 -> ref<raw addrspace(stack) i32>
    v2 = iconst 4i64
    v3 = intrinsic.memcmp(v1, v1, v2)
    jump block1
block1:
    branch v0, block1, block2
block2:
    return v3
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&LoopSimplify);
        program.run_pass(&Licm);
        program.assert_output(expected);
    }

    /// Calls are not hoisted (side effects).
    #[test]
    fn test_no_hoist_call() {
        let input = r#"function @test(v0: bool) -> i32 {
block0(v0: bool):
    jump block1
block1:
    v1 = call @get_value()
    branch v0, block1, block2
block2:
    return v1
}

function @get_value() -> i32 {
block0:
    v0 = iconst 42i32
    return v0
}"#;
        // call should not be hoisted
        let mut program = TestProgram::new(input);
        program.run_pass(&LoopSimplify);
        let before = program.format();
        program.run_pass(&Licm);
        program.assert_output(&before);
    }

    /// Allocations are not hoisted (each iteration needs fresh allocation).
    #[test]
    fn test_no_hoist_alloc() {
        let input = r#"function @test(v0: bool) -> ref<managed i32> {
block0(v0: bool):
    jump block1
block1:
    v1 = managed.alloc i32 -> ref<managed i32>
    branch v0, block1, block2
block2:
    return v1
}"#;
        // managed.alloc should stay in loop: each iteration allocates a new object
        let mut program = TestProgram::new(input);
        program.run_pass(&LoopSimplify);
        let before = program.format();
        program.run_pass(&Licm);
        program.assert_output(&before);
    }

    /// Invariant load with no clobbering stores is hoisted.
    #[test]
    fn test_hoist_invariant_load() {
        let input = r#"function @test(v0: bool) -> i32 {
block0(v0: bool):
    v1 = stack.alloc i32 -> ref<raw addrspace(stack) i32>
    v2 = stack.alloc i32 -> ref<raw addrspace(stack) i32>
    v3 = iconst 1i32
    store v1, v3
    jump block1
block1:
    v4 = iconst 2i32
    store v2, v4
    v5 = load v1 -> i32
    branch v0, block1, block2
block2:
    return v5
}"#;
        let expected = r#"function @test(v0: bool) -> i32 {
block0(v0: bool):
    v1 = stack.alloc i32 -> ref<raw addrspace(stack) i32>
    v2 = stack.alloc i32 -> ref<raw addrspace(stack) i32>
    v3 = iconst 1i32
    store v1, v3
    v4 = iconst 2i32
    v5 = load v1 -> i32
    jump block1
block1:
    store v2, v4
    branch v0, block1, block2
block2:
    return v5
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&LoopSimplify);
        program.run_pass(&Licm);
        program.assert_output(expected);
    }

    /// Invariant load is not hoisted when the loop writes the same location.
    #[test]
    fn test_skip_hoist_clobbered_load() {
        let input = r#"function @test(v0: bool) -> i32 {
block0(v0: bool):
    v1 = stack.alloc i32 -> ref<raw addrspace(stack) i32>
    v2 = iconst 1i32
    store v1, v2
    jump block1
block1:
    v3 = load v1 -> i32
    v4 = iconst 2i32
    store v1, v4
    branch v0, block1, block2
block2:
    return v3
}"#;
        let expected = r#"function @test(v0: bool) -> i32 {
block0(v0: bool):
    v1 = stack.alloc i32 -> ref<raw addrspace(stack) i32>
    v2 = iconst 1i32
    store v1, v2
    v4 = iconst 2i32
    jump block1
block1:
    v3 = load v1 -> i32
    store v1, v4
    branch v0, block1, block2
block2:
    return v3
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&LoopSimplify);
        program.run_pass(&Licm);
        program.assert_output(expected);
    }

    /// Load in a conditional block is not hoisted.
    #[test]
    fn test_skip_hoist_conditional_load() {
        let input = r#"function @test(v0: bool, v1: bool) -> i32 {
block0(v0: bool, v1: bool):
    v2 = stack.alloc i32 -> ref<raw addrspace(stack) i32>
    v3 = iconst 1i32
    store v2, v3
    jump block1
block1:
    branch v0, block2, block3
block2:
    v4 = load v2 -> i32
    jump block4(v4)
block3:
    jump block4(v3)
block4(v5: i32):
    branch v1, block1, block5
block5:
    return v5
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&LoopSimplify);
        program.run_pass(&Licm);
        program.assert_unchanged(input);
    }

    /// Invariant local get is hoisted to the preheader.
    #[test]
    fn test_hoist_local_get() {
        let input = r#"function @test(v0: bool) -> i32 {
    local0: i32 ; owned
block0(v0: bool):
    v1 = iconst 3i32
    local.set local0, v1
    jump block1
block1:
    v2 = local.get local0
    branch v0, block1, block2
block2:
    return v2
}"#;
        let expected = r#"function @test(v0: bool) -> i32 {
    local0: i32 ; owned
block0(v0: bool):
    v1 = iconst 3i32
    local.set local0, v1
    v2 = local.get local0
    jump block1
block1:
    branch v0, block1, block2
block2:
    return v2
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&LoopSimplify);
        program.run_pass(&Licm);
        program.assert_output(expected);
    }

    /// Load with a clobbering call is not hoisted.
    #[test]
    fn test_skip_hoist_load_with_call() {
        let input = r#"function @test(v0: bool) -> i32 {
block0(v0: bool):
    v1 = stack.alloc i32 -> ref<raw addrspace(stack) i32>
    v2 = iconst 1i32
    store v1, v2
    jump block1
block1:
    v3 = load v1 -> i32
    call @touch(v1)
    branch v0, block1, block2
block2:
    return v3
}
function @touch(v0: ref<raw i32>) -> void {
block0(v0: ref<raw i32>):
    return
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&LoopSimplify);
        program.run_pass(&Licm);
        program.assert_unchanged(input);
    }

    /// Load with a disjoint alias scope is hoisted.
    #[test]
    fn test_hoist_load_with_noalias_scope() {
        let input = r#"function @test(v0: bool, v1: ref<raw i32>, v2: ref<raw i32>) -> i32 {
block0(v0: bool, v1: ref<raw i32>, v2: ref<raw i32>):
    jump block1
block1:
    v3 = load v1 -> i32
    v4 = iconst 1i32
    store v2, v4
    branch v0, block1, block2
block2:
    return v3
}"#;
        let expected = r#"function @test(v0: bool, v1: ref<raw i32>, v2: ref<raw i32>) -> i32 {
block0(v0: bool, v1: ref<raw i32>, v2: ref<raw i32>):
    v3 = load v1 -> i32
    v4 = iconst 1i32
    jump block1
block1:
    store v2, v4
    branch v0, block1, block2
block2:
    return v3
}"#;

        let mut program = TestProgram::new(input);
        let function_id = program.first_function_id();
        let function = program.tree.get(function_id);
        let block = program.tree.get(function.blocks[1]);
        let load_v1 = block.instructions[0];
        let store_v2 = block.instructions[2];

        let domain = program.tree.memory_table.alias_scopes.create_domain(None);
        let scope_a = program
            .tree
            .memory_table
            .alias_scopes
            .create_scope(domain, None);

        program.insert_pointer_access(
            load_v1,
            mir::MemoryAccessKind::Read,
            mir::Value::new(1),
            None,
            vec![scope_a],
            Vec::new(),
            None,
        );

        program.insert_pointer_access(
            store_v2,
            mir::MemoryAccessKind::Write,
            mir::Value::new(2),
            None,
            Vec::new(),
            vec![scope_a],
            None,
        );

        program.run_pass(&LoopSimplify);
        program.run_pass(&Licm);
        program.assert_output(expected);
    }

    /// Multiple independent loops each get their invariants hoisted.
    #[test]
    fn test_multiple_loops() {
        let input = r#"function @test(v0: bool, v1: bool, v2: i32) -> i32 {
block0(v0: bool, v1: bool, v2: i32):
    jump block1
block1:
    v3 = iconst 10i32
    branch v0, block1, block2
block2:
    jump block3
block3:
    v4 = iconst 20i32
    v5 = iadd v2, v4
    branch v1, block3, block4
block4:
    v6 = iadd v3, v5
    return v6
}"#;
        // v3 hoisted from loop1 to block0
        // v4, v5 hoisted from loop3 to block2
        let expected = r#"function @test(v0: bool, v1: bool, v2: i32) -> i32 {
block0(v0: bool, v1: bool, v2: i32):
    v3 = iconst 10i32
    jump block1
block1:
    branch v0, block1, block2
block2:
    v4 = iconst 20i32
    v5 = iadd v2, v4
    jump block3
block3:
    branch v1, block3, block4
block4:
    v6 = iadd v3, v5
    return v6
}"#;
        let mut program = TestProgram::new(input);
        program.run_pass(&LoopSimplify);
        program.run_pass(&Licm);
        program.assert_output(expected);
    }

    /// Safe division is hoisted when the divisor is proven not zero.
    #[test]
    fn test_hoist_safe_division() {
        let input = r#"function @test() -> i32 {
block0:
    v0 = iconst 0i32
    v1 = iconst 1i32
    v2 = iconst 10i32
    v3 = iconst 2i32
    jump block1(v0)
block1(v4: i32):
    v5 = sdiv v2, v3
    v6 = icmp_slt v4, v1
    branch v6, block2, block3
block2:
    v7 = iadd v4, v1
    jump block1(v7)
block3:
    return v5
}"#;
        let expected = r#"function @test() -> i32 {
block0:
    v0 = iconst 0i32
    v1 = iconst 1i32
    v2 = iconst 10i32
    v3 = iconst 2i32
    v5 = sdiv v2, v3
    jump block1(v0)
block1(v4: i32):
    v6 = icmp_slt v4, v1
    branch v6, block2, block3
block2:
    v7 = iadd v4, v1
    jump block1(v7)
block3:
    return v5
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&LoopSimplify);
        program.run_pass(&Licm);
        program.assert_output(expected);
    }

    /// Potentially trapping division remains in the loop.
    #[test]
    fn test_skip_trapping_division() {
        let input = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    v1 = iconst 0i32
    v2 = iconst 1i32
    v3 = iconst 10i32
    jump block1(v1)
block1(v4: i32):
    v5 = sdiv v3, v0
    v6 = icmp_slt v4, v2
    branch v6, block2, block3
block2:
    v7 = iadd v4, v2
    jump block1(v7)
block3:
    return v5
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&LoopSimplify);
        program.run_pass(&Licm);
        program.assert_output(input);
    }
}
