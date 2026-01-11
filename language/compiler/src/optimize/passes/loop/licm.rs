use std::collections::HashSet;

use destack_compiler_macros::declare_pass;
use destack_mir as mir;

use crate::optimize::analyses::{
    AliasAnalysis, ConstantPropagation, DominatorTree, Loop, LoopAnalysis, RangeAnalysis,
    ValueRange,
};
use crate::optimize::common::{
    MemoryLocation, instruction_is_speculatable, instruction_may_affect_memory,
};
use crate::optimize::{AnalysisPreservation, FunctionAnalyses, FunctionPass, PipelineContext};

declare_pass! {
    /// Move loop-invariant computations outside of loops.
    ///
    /// An instruction is loop-invariant if all its operands are defined outside
    /// the loop or by other loop-invariant instructions. Loop-invariant instructions
    /// can be hoisted to the loop preheader, reducing redundant computation.
    ///
    /// This pass hoists:
    /// - Pure arithmetic (Binary, Unary, Cast)
    /// - Constants
    /// - Pure aggregate operations (FieldGet, FieldSet, FieldAddr, ElementGet, ElementSet, ElementAddr)
    /// - Immutable global references (GlobalConst, GlobalAddr)
    ///
    /// Loop invariant loads are hoisted when alias analysis proves the memory
    /// location is not clobbered inside the loop.
    ///
    /// Potentially trapping instructions (e.g. integer division) are only hoisted
    /// when range analysis proves the operation is safe.
    ///
    /// Requires canonical loop form (preheader, single latch) from LoopSimplify.
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
        _ctx: &PipelineContext<'_>,
    ) -> AnalysisPreservation {
        let entry = match function.entry {
            Some(entry) => entry,
            None => return AnalysisPreservation::all(),
        };

        // get analyses
        let (loops, domtree, ranges, constants, alias) = {
            let analyses = FunctionAnalyses::new(function, tree);
            (
                analyses.get::<LoopAnalysis>().clone(),
                analyses.get::<DominatorTree>().clone(),
                analyses.get::<RangeAnalysis>().clone(),
                analyses.get::<ConstantPropagation>().clone(),
                analyses.get::<AliasAnalysis>().clone(),
            )
        };
        if loops.num_loops() == 0 {
            return AnalysisPreservation::all();
        }

        // run LICM
        let changed = run_licm(
            entry, function, tree, &loops, &domtree, &ranges, &constants, &alias,
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
) -> bool {
    // collect hoisting work for each loop, outermost first
    let mut all_work: Vec<HoistWork> = Vec::new();
    let mut already_queued: HashSet<(mir::LocalNodeId<mir::Block>, usize)> = HashSet::new();

    for lp in loops.loops().iter() {
        // find preheader (immediate dominator outside the loop)
        let preheader = match find_preheader(lp, domtree, entry) {
            Some(p) => p,
            None => continue,
        };

        // collect values defined outside the loop
        let mut invariant_values: HashSet<mir::Value> = HashSet::new();

        // function parameters
        for param in &function.parameters {
            invariant_values.insert(param.value);
        }

        // values from blocks outside the loop
        for &block_id in &function.blocks {
            if lp.blocks.contains(&block_id) {
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

        // iteratively find loop-invariant instructions
        let mut changed = true;
        while changed {
            changed = false;
            for &block_id in &function.blocks {
                if !lp.blocks.contains(&block_id) {
                    continue;
                }
                let block = tree.get(block_id);

                for &instruction_id in &block.instructions {
                    let instruction = tree.get(instruction_id);
                    if let Some(destination) = instruction.destination() {
                        if invariant_values.contains(&destination) {
                            continue;
                        }
                        let operands_invariant = instruction
                            .uses()
                            .iter()
                            .all(|v| invariant_values.contains(v));
                        let can_hoist = instruction_is_hoistable(
                            instruction_id,
                            instruction,
                            &lp.blocks,
                            tree,
                            alias,
                            block_id,
                            ranges,
                            constants,
                        );
                        if can_hoist && operands_invariant {
                            invariant_values.insert(destination);
                            changed = true;
                        }
                    }
                }
            }
        }

        // collect instructions to hoist
        for &block_id in &function.blocks {
            if !lp.blocks.contains(&block_id) {
                continue;
            }
            let block = tree.get(block_id);

            for (index, &instruction_id) in block.instructions.iter().enumerate() {
                if already_queued.contains(&(block_id, index)) {
                    continue;
                }

                let instruction = tree.get(instruction_id);
                if let Some(destination) = instruction.destination() {
                    let operands_invariant = instruction
                        .uses()
                        .iter()
                        .all(|v| invariant_values.contains(v));

                    // hoist if pure and all operands are invariant
                    let can_hoist = instruction_is_hoistable(
                        instruction_id,
                        instruction,
                        &lp.blocks,
                        tree,
                        alias,
                        block_id,
                        ranges,
                        constants,
                    );
                    if can_hoist && invariant_values.contains(&destination) && operands_invariant {
                        already_queued.insert((block_id, index));
                        all_work.push(HoistWork {
                            source_block: block_id,
                            instruction_index: index,
                            target_preheader: preheader,
                        });
                    }
                }
            }
        }
    }

    if all_work.is_empty() {
        return false;
    }

    // sort descending by (block, index) so removal doesn't invalidate indices
    all_work.sort_by(|a, b| {
        b.source_block
            .cmp(&a.source_block)
            .then(b.instruction_index.cmp(&a.instruction_index))
    });

    let mut hoisted_count = 0;
    let mut current_block: Option<mir::LocalNodeId<mir::Block>> = None;
    let mut block_data: Option<mir::Block> = None;
    let mut preheader_insertions: Vec<(
        mir::LocalNodeId<mir::Block>,
        mir::LocalNodeId<mir::Instruction>,
    )> = Vec::new();

    for work in all_work {
        // flush previous block if switching
        if current_block != Some(work.source_block) {
            if let (Some(block_id), Some(data)) = (current_block, block_data.take()) {
                tree.replace(block_id, data);
            }
            current_block = Some(work.source_block);
            block_data = Some(tree.get(work.source_block).clone());
        }

        let data = block_data.as_mut().unwrap();
        let instr_id = data.instructions.remove(work.instruction_index);
        preheader_insertions.push((work.target_preheader, instr_id));
        hoisted_count += 1;
    }

    // flush last block
    if let (Some(block_id), Some(data)) = (current_block, block_data.take()) {
        tree.replace(block_id, data);
    }

    // insert into preheaders
    preheader_insertions.reverse();
    let mut preheaders_to_update: HashSet<mir::LocalNodeId<mir::Block>> = HashSet::new();
    for (preheader, _) in &preheader_insertions {
        preheaders_to_update.insert(*preheader);
    }

    for preheader_id in preheaders_to_update {
        let mut preheader = tree.get(preheader_id).clone();
        for (target, instr_id) in &preheader_insertions {
            if *target == preheader_id {
                preheader.instructions.push(*instr_id);
            }
        }
        tree.replace(preheader_id, preheader);
    }

    hoisted_count > 0
}

/// Return true when an instruction can be hoisted safely.
#[allow(clippy::too_many_arguments)]
fn instruction_is_hoistable(
    instruction_id: mir::LocalNodeId<mir::Instruction>,
    instruction: &mir::Instruction,
    loop_blocks: &HashSet<mir::LocalNodeId<mir::Block>>,
    tree: &mir::NodeTree,
    alias: &AliasAnalysis,
    block_id: mir::LocalNodeId<mir::Block>,
    ranges: &RangeAnalysis,
    constants: &ConstantPropagation,
) -> bool {
    // accept speculatable instructions immediately
    if instruction_is_speculatable(instruction) {
        return true;
    }

    // allow invariant loads when the location is not clobbered in the loop
    if let mir::Instruction::Load { pointer, .. } = instruction {
        return load_is_hoistable(instruction_id, *pointer, loop_blocks, tree, alias);
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
    pointer: mir::Value,
    loop_blocks: &HashSet<mir::LocalNodeId<mir::Block>>,
    tree: &mir::NodeTree,
    alias: &AliasAnalysis,
) -> bool {
    // scan loop instructions for clobbers
    let location = MemoryLocation::from_ptr(pointer);
    for block_id in loop_blocks {
        let block = tree.get(*block_id);
        for &instruction_id in &block.instructions {
            // skip the load itself
            if instruction_id == load_id {
                continue;
            }

            // skip instructions that do not affect memory
            let instruction = tree.get(instruction_id);
            let affects_memory = instruction_may_affect_memory(instruction);
            if !affects_memory {
                continue;
            }

            // reject any potential clobber
            let may_clobber = alias.may_clobber(instruction_id, &location);
            if may_clobber {
                return false;
            }
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

/// Resolve a best-effort integer range for a value at a block boundary.
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
struct HoistWork {
    /// The block containing the instruction to hoist.
    source_block: mir::LocalNodeId<mir::Block>,
    /// The index of the instruction to hoist.
    instruction_index: usize,
    /// The preheader to hoist the instruction to.
    target_preheader: mir::LocalNodeId<mir::Block>,
}

/// Find the preheader of a loop.
///
/// The preheader is the immediate dominator of the header that is outside the loop.
fn find_preheader(
    lp: &Loop,
    domtree: &DominatorTree,
    _entry: mir::LocalNodeId<mir::Block>,
) -> Option<mir::LocalNodeId<mir::Block>> {
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

    /// Operation using loop-variant value is not hoisted.
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

    /// Invariant in inner loop is hoisted to inner preheader.
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
        // v3 and v4 are invariant to the inner loop, hoist to block1 (inner preheader)
        // actually v4 uses v2 which is a function param, so both are invariant to outer too
        // they should be hoisted to block0
        let expected = r#"function @test(v0: bool, v1: bool, v2: i32) -> i32 {
block0(v0: bool, v1: bool, v2: i32):
    v3 = iconst 5i32
    v4 = iadd v2, v3
    jump block1
block1:
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
    v1 = managed.alloc i32
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
    v1 = stack.alloc i32
    v2 = stack.alloc i32
    v3 = iconst 1i32
    store v1, v3
    jump block1
block1:
    v4 = iconst 2i32
    store v2, v4
    v5 = load v1
    branch v0, block1, block2
block2:
    return v5
}"#;
        let expected = r#"function @test(v0: bool) -> i32 {
block0(v0: bool):
    v1 = stack.alloc i32
    v2 = stack.alloc i32
    v3 = iconst 1i32
    store v1, v3
    v4 = iconst 2i32
    v5 = load v1
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
    v1 = stack.alloc i32
    v2 = iconst 1i32
    store v1, v2
    jump block1
block1:
    v3 = load v1
    v4 = iconst 2i32
    store v1, v4
    branch v0, block1, block2
block2:
    return v3
}"#;
        let expected = r#"function @test(v0: bool) -> i32 {
block0(v0: bool):
    v1 = stack.alloc i32
    v2 = iconst 1i32
    store v1, v2
    v4 = iconst 2i32
    jump block1
block1:
    v3 = load v1
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

    /// Safe division is hoisted when the divisor is proven non-zero.
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
